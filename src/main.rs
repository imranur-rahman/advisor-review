use advisor_review::{
    config::ProviderConfig,
    discover, guidelines, latex,
    model::{ReviewIssue, ReviewReport},
    pdf,
    providers::ConfiguredProvider,
    report, review,
};
use anyhow::Result;
use clap::{CommandFactory, Parser, Subcommand};
use std::path::PathBuf;

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
        #[arg(long)]
        provider: Option<String>,
        #[arg(long)]
        model: Option<String>,
    },
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
        }) => run_review(project, guidelines, output, main_tex, pdf, provider, model)?,
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
    provider: Option<String>,
    model: Option<String>,
) -> Result<i32> {
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
    let provider_config = ProviderConfig::from_values(provider, model);
    let provider_adapter = provider_config.name.as_ref().map(|_| ConfiguredProvider {
        config: provider_config.clone(),
    });
    let (findings, mut issues) = review::run(
        &targets,
        &registry,
        provider_adapter
            .as_ref()
            .map(|p| p as &dyn advisor_review::providers::SemanticProvider),
    );
    issues.extend(registry.issues.drain(..).map(|message| ReviewIssue {
        kind: "guideline".into(),
        message,
        rule_id: None,
    }));
    if pdf_info.page_count == 0 {
        issues.push(ReviewIssue { kind: "pdf_mapping".into(), message: "PDF page structure could not be extracted; rendered anchors are unavailable or approximate".into(), rule_id: None });
    }
    let mut result = ReviewReport::new(
        inputs.project.display().to_string(),
        inputs.main_tex.display().to_string(),
        inputs.pdf.display().to_string(),
        provider_config.metadata(),
    );
    result.findings = findings;
    result.candidates = registry.candidates;
    result.conflicts = registry.conflicts;
    result.issues = issues;
    report::write(&result, &output)?;
    println!(
        "wrote {} findings to {}",
        result.findings.len(),
        output.display()
    );
    Ok(0)
}
