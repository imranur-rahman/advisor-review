use advisor_review::{
    compile,
    config::ProviderConfig,
    discover, guidelines, latex,
    model::{ReviewIssue, ReviewReport},
    pdf,
    providers::ConfiguredProvider,
    report, review, rule_tests,
};
use anyhow::Result;
use clap::{CommandFactory, Parser, Subcommand, ValueEnum};
use std::path::{Path, PathBuf};

#[derive(Debug, Parser)]
#[command(
    name = "advisor-review",
    version,
    about = "Review LaTeX manuscripts against advisor and publication guidelines"
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Debug, Subcommand)]
enum Command {
    Review {
        #[arg(short, long, default_value = ".")]
        project: PathBuf,
        #[arg(short, long, default_value = "guidelines")]
        guidelines: PathBuf,
        #[arg(short, long, default_value = "review")]
        output: PathBuf,
        #[arg(long)]
        main_tex: Option<PathBuf>,
        #[arg(long)]
        pdf: Option<PathBuf>,
        #[arg(
            long,
            help = "Semantic provider (API provider, ollama, claude-code, or codex-cli)"
        )]
        provider: Option<String>,
        #[arg(long, help = "Provider model; optional for installed CLI providers")]
        model: Option<String>,
        #[arg(long, value_delimiter = ',')]
        scopes: Vec<String>,
        #[arg(long, default_value_t = 32768)]
        max_input_bytes: usize,
        #[arg(long, value_enum, default_value = "text")]
        format: OutputFormat,
    },
    /// Compile Markdown guideline items into draft rule packs with an installed agent CLI
    Compile {
        /// Markdown guideline file, or a directory of them
        #[arg(short, long)]
        guidelines: PathBuf,
        #[arg(long, help = "Compiler provider: claude-code or codex-cli")]
        provider: Option<String>,
        #[arg(long, help = "Provider model; omit to use the CLI's default")]
        model: Option<String>,
        #[arg(long, default_value_t = 3)]
        max_repair_rounds: usize,
        /// Replace existing compiled packs
        #[arg(long)]
        force: bool,
        #[arg(long, default_value_t = 32768)]
        max_input_bytes: usize,
    },
    /// Work with rule packs
    Rules {
        #[command(subcommand)]
        command: RulesCommand,
    },
}

#[derive(Debug, Subcommand)]
enum RulesCommand {
    /// Run every rule's should_fire and should_pass examples, including draft rules
    Test {
        /// Guideline file or directory
        #[arg(short, long, default_value = "guidelines")]
        guidelines: PathBuf,
        #[arg(long, help = "Semantic provider for semantic rule examples")]
        provider: Option<String>,
        #[arg(long)]
        model: Option<String>,
        #[arg(long, default_value_t = 32768)]
        max_input_bytes: usize,
    },
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum OutputFormat {
    Text,
    Vscode,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let code = match cli.command {
        Some(Command::Review {
            project,
            guidelines,
            output,
            main_tex,
            pdf,
            provider,
            model,
            scopes,
            max_input_bytes,
            format,
        }) => run_review(
            project,
            guidelines,
            output,
            main_tex,
            pdf,
            ProviderConfig::from_values(provider, model),
            review::ReviewOptions {
                scopes,
                max_input_bytes,
            },
            format,
        )?,
        Some(Command::Compile {
            guidelines,
            provider,
            model,
            max_repair_rounds,
            force,
            max_input_bytes,
        }) => run_compile(
            &guidelines,
            ProviderConfig::from_values(provider, model),
            compile::CompileOptions {
                max_repair_rounds,
                max_input_bytes,
            },
            force,
        ),
        Some(Command::Rules {
            command:
                RulesCommand::Test {
                    guidelines,
                    provider,
                    model,
                    max_input_bytes,
                },
        }) => run_rules_test(
            &guidelines,
            ProviderConfig::from_values(provider, model),
            max_input_bytes,
        ),
        None => {
            Cli::command().print_help()?;
            println!();
            0
        }
    };
    if code != 0 {
        std::process::exit(code);
    }
    Ok(())
}

fn run_review(
    project: PathBuf,
    guidelines_dir: PathBuf,
    output: PathBuf,
    main_tex: Option<PathBuf>,
    pdf: Option<PathBuf>,
    provider_config: ProviderConfig,
    options: review::ReviewOptions,
    format: OutputFormat,
) -> Result<i32> {
    if let Err(err) = options.validate() {
        eprintln!("error: {err}");
        return Ok(2);
    }
    if let Err(err) = provider_config.validate() {
        eprintln!("error: {err}");
        return Ok(2);
    }
    let inputs = match discover::discover(
        &project,
        &guidelines_dir,
        main_tex.as_deref(),
        pdf.as_deref(),
    ) {
        Ok(v) => v,
        Err(err) => {
            eprintln!("error: {err:#}");
            return Ok(2);
        }
    };
    let mut registry = guidelines::load(&inputs.guidelines)?;
    let targets = latex::parse_project(&inputs.main_tex, &inputs.project)?;
    let pdf_info = pdf::inspect(&inputs.pdf)?;
    let provider_adapter = provider_config.name.as_ref().map(|_| ConfiguredProvider {
        config: provider_config.clone(),
    });
    let run = review::run_with_options(
        &targets,
        &registry,
        provider_adapter
            .as_ref()
            .map(|p| p as &dyn advisor_review::providers::SemanticProvider),
        &options,
    );
    let mut issues = run.issues;
    issues.extend(registry.issues.drain(..).map(|message| ReviewIssue {
        kind: "guideline".into(),
        message,
        rule_id: None,
    }));
    issues.push(ReviewIssue { kind: "pdf_mapping".into(), message: format!("PDF validated ({} pages); source-to-PDF mapping is {}. Rendered layout and page text checks are not implemented.", pdf_info.page_count, pdf_info.mapping_quality), rule_id: None });
    if run.coverage.values().all(|c| c.rules == 0) {
        issues.push(ReviewIssue {
            kind: "skipped".into(),
            message: "no executable rules matched the selected scopes; review is incomplete".into(),
            rule_id: None,
        });
    }
    let mut result = ReviewReport::new(
        inputs.project.display().to_string(),
        inputs.main_tex.display().to_string(),
        inputs.pdf.display().to_string(),
        provider_config.metadata(),
    );
    result.findings = run.findings;
    result.coverage = run.coverage;
    result.evaluations = run.evaluations;
    result.targets = targets;
    result.candidates = registry.candidates;
    result.conflicts = registry.conflicts;
    result.stale_rules = registry.stale;
    result.issues = issues;
    report::write(&result, &output)?;
    match format {
        OutputFormat::Text => println!(
            "{}: {} findings; report: {}/findings.md",
            if result.is_incomplete() {
                "review incomplete"
            } else {
                "review complete"
            },
            result.findings.len(),
            output.display()
        ),
        OutputFormat::Vscode => {
            for diagnostic in report::vscode_diagnostics(&result) {
                println!("{diagnostic}");
            }
            println!(
                "advisor-review: {} ({} findings); details: {}/findings.md",
                if result.is_incomplete() {
                    "review incomplete"
                } else {
                    "review complete"
                },
                result.findings.len(),
                output.display()
            );
        }
    }
    if result.is_incomplete() {
        eprintln!(
            "review incomplete: see Review Issues in {}/findings.md and findings.json for details",
            output.display()
        );
        Ok(1)
    } else {
        Ok(0)
    }
}

fn markdown_sources(path: &Path) -> Vec<PathBuf> {
    if path.is_file() {
        return vec![path.to_path_buf()];
    }
    walkdir::WalkDir::new(path)
        .sort_by_file_name()
        .into_iter()
        .filter_map(Result::ok)
        .filter(|e| e.file_type().is_file())
        .map(|e| e.into_path())
        .filter(|p| {
            p.extension()
                .and_then(|e| e.to_str())
                .is_some_and(|e| matches!(e, "md" | "markdown"))
        })
        .collect()
}

fn run_compile(
    guidelines_path: &Path,
    provider_config: ProviderConfig,
    options: compile::CompileOptions,
    force: bool,
) -> i32 {
    if let Err(err) = provider_config.validate() {
        eprintln!("error: {err}");
        return 2;
    }
    if !matches!(
        provider_config.name.as_deref(),
        Some("claude-code" | "codex-cli")
    ) {
        eprintln!("error: compile needs --provider claude-code or --provider codex-cli");
        return 2;
    }
    if !guidelines_path.exists() {
        eprintln!("error: {} does not exist", guidelines_path.display());
        return 2;
    }
    let sources: Vec<_> = markdown_sources(guidelines_path)
        .into_iter()
        .filter(|p| {
            std::fs::read_to_string(p)
                .is_ok_and(|text| !advisor_review::guideline_items::split(&text).is_empty())
        })
        .collect();
    if sources.is_empty() {
        eprintln!(
            "error: no Markdown guideline items found in {}",
            guidelines_path.display()
        );
        return 2;
    }
    if !force {
        let existing: Vec<_> = sources
            .iter()
            .map(|s| compile::output_path(s))
            .filter(|o| o.exists())
            .collect();
        if !existing.is_empty() {
            for path in existing {
                eprintln!(
                    "error: {} exists; use --force to replace it",
                    path.display()
                );
            }
            return 2;
        }
    }
    let provider = ConfiguredProvider {
        config: provider_config,
    };
    for source in &sources {
        let output = compile::output_path(source);
        let compiled = match compile::compile_file(source, &provider, &options) {
            Ok(compiled) => compiled,
            Err(err) => {
                eprintln!(
                    "error: compile {} failed; no pack written: {err}",
                    source.display()
                );
                return 1;
            }
        };
        if let Err(err) = std::fs::write(&output, &compiled.yaml) {
            eprintln!("error: write {}: {err}", output.display());
            return 1;
        }
        let s = &compiled.summary;
        println!(
            "{} -> {}: {} items; {} deterministic rules; {} semantic rules; {} uncheckable; {} failed repair; {} CLI calls",
            source.display(),
            output.display(),
            s.items,
            s.deterministic,
            s.semantic,
            s.uncheckable,
            s.failed_repair,
            s.cli_calls
        );
        if !s.overlapping.is_empty() {
            println!(
                "  active rules from the same source may overlap: {}",
                s.overlapping.join(", ")
            );
        }
    }
    println!(
        "Compiled rules are drafts. Check each rule and its examples, add real sentences from your drafts to should_fire, then set status: active."
    );
    0
}

fn run_rules_test(
    guidelines_path: &Path,
    provider_config: ProviderConfig,
    max_input_bytes: usize,
) -> i32 {
    if let Err(err) = provider_config.validate() {
        eprintln!("error: {err}");
        return 2;
    }
    if max_input_bytes < 1024 {
        eprintln!("error: max-input-bytes must be at least 1024");
        return 2;
    }
    if !guidelines_path.exists() {
        eprintln!("error: {} does not exist", guidelines_path.display());
        return 2;
    }
    let registry = match guidelines::load(guidelines_path) {
        Ok(registry) => registry,
        Err(err) => {
            eprintln!("error: {err:#}");
            return 2;
        }
    };
    if !registry.issues.is_empty() {
        for issue in &registry.issues {
            eprintln!("error: {issue}");
        }
        return 2;
    }
    let provider = provider_config.name.as_ref().map(|_| ConfiguredProvider {
        config: provider_config.clone(),
    });
    let provider = provider
        .as_ref()
        .map(|p| p as &dyn advisor_review::providers::SemanticProvider);
    let (mut passed, mut failed, mut skipped, mut untested) = (0, 0, 0, 0);
    for rule in registry.active.iter().chain(&registry.drafts) {
        let results = rule_tests::run(rule, provider, max_input_bytes);
        if results.is_empty() {
            untested += 1;
            continue;
        }
        for result in results {
            match &result.outcome {
                rule_tests::Outcome::Passed => {
                    passed += 1;
                    println!("PASS {} {}", result.rule_id, result.example);
                }
                rule_tests::Outcome::Failed(detail) => {
                    failed += 1;
                    println!(
                        "FAIL {} {} {:?}: {detail}",
                        result.rule_id, result.example, result.text
                    );
                }
                rule_tests::Outcome::Skipped(detail) => {
                    skipped += 1;
                    println!("SKIP {} {}: {detail}", result.rule_id, result.example);
                }
            }
        }
    }
    println!(
        "{passed} passed, {failed} failed, {skipped} skipped; {untested} rules have no examples"
    );
    if failed > 0 || skipped > 0 { 1 } else { 0 }
}
