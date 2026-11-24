use crate::manuscript::section_title;
use crate::model::{ReviewFinding, ReviewReport};
use anyhow::Result;
use std::fs;
use std::path::Path;

pub fn write(report: &ReviewReport, output: &Path) -> Result<()> {
    fs::create_dir_all(output)?;
    fs::write(
        output.join("findings.json"),
        serde_json::to_string_pretty(report)?,
    )?;
    fs::write(output.join("findings.md"), markdown(report))?;
    Ok(())
}

pub fn markdown(report: &ReviewReport) -> String {
    let incomplete = report.is_incomplete();
    let mut out = format!(
        "# Manuscript Review\n\n**Status:** {}\n\n",
        if incomplete {
            "⚠️ Incomplete — some checks need attention"
        } else {
            "✅ Complete"
        }
    );
    let mut severity_counts = std::collections::BTreeMap::new();
    for finding in &report.findings {
        *severity_counts
            .entry(finding.severity.as_str())
            .or_insert(0usize) += 1;
    }
    let severity_summary = if severity_counts.is_empty() {
        "no findings".into()
    } else {
        severity_counts
            .iter()
            .map(|(severity, count)| format!("{count} {severity}"))
            .collect::<Vec<_>>()
            .join(", ")
    };
    let provider = match (&report.provider.provider, &report.provider.model) {
        (Some(provider), Some(model)) => format!("{provider} / {model}"),
        (Some(provider), None) => provider.clone(),
        _ => "deterministic checks only".into(),
    };
    out.push_str(&format!(
        "**Findings:** {} ({severity_summary})  \n**Provider:** {provider}  \n**Project:** `{}`  \n**Source:** `{}`  \n**PDF:** `{}`\n\n",
        report.findings.len(), report.project, report.main_tex, report.pdf
    ));
    if incomplete {
        out.push_str("**Next step:** review the issues below, fix configuration or unsupported checks, then rerun.\n\n");
    } else if report.findings.is_empty() {
        out.push_str("No findings were produced. This means all selected checks completed without reporting a finding.\n\n");
    } else {
        out.push_str("Findings include their status, source location, evidence, and any suggested action.\n\n");
    }
    if !report.coverage.is_empty() {
        out.push_str("## Coverage by scope\n\n| Scope | Selection | Rules | Targets | Evaluations | Complete | Partial | Skipped | Failed | N/A |\n|---|---|---:|---:|---:|---:|---:|---:|---:|---:|\n");
        for (scope, c) in &report.coverage {
            out.push_str(&format!(
                "| {} | {} | {} | {} | {} | {} | {} | {} | {} | {} |\n",
                scope,
                if c.selected {
                    "Selected"
                } else {
                    "Not selected"
                },
                c.rules,
                c.targets,
                c.evaluations,
                c.completed,
                c.partial,
                c.skipped,
                c.failed,
                c.not_applicable
            ));
        }
        out.push('\n');
    }
    let mut groups = std::collections::BTreeMap::new();
    for finding in &report.findings {
        let section =
            section_title(&finding.target, &report.targets).unwrap_or_else(|| "Manuscript".into());
        groups
            .entry((finding.target.target_type.as_str(), section))
            .or_insert_with(Vec::new)
            .push(finding);
    }
    for ((scope, section), findings) in groups {
        out.push_str(&format!("## {scope} — {section}\n\n"));
        for finding in findings {
            out.push_str(&finding_markdown(finding));
        }
    }
    if !report.conflicts.is_empty() {
        out.push_str("## Rule Conflicts\n\n");
        for conflict in &report.conflicts {
            out.push_str(&format!(
                "- {}: {}\n",
                conflict.rule_ids.join(", "),
                conflict.resolution
            ));
        }
        out.push('\n');
    }
    if !report.stale_rules.is_empty() {
        out.push_str("## Stale Compiled Rules\n\n");
        for stale in &report.stale_rules {
            out.push_str(&format!(
                "- `{}` from `{}` lines {}-{}: {}\n",
                stale.rule_id, stale.file, stale.start_line, stale.end_line, stale.reason
            ));
        }
        out.push('\n');
    }
    if !report.candidates.is_empty() {
        out.push_str("## Rule Candidates\n\n");
        for c in &report.candidates {
            out.push_str(&format!("- `{}` from `{}`: {}\n", c.id, c.source, c.text));
        }
        out.push('\n');
    }
    if !report.issues.is_empty() {
        out.push_str("## Review Issues\n\n");
        for issue in &report.issues {
            out.push_str(&format!("- **{}**: {}\n", issue.kind, issue.message));
        }
    }
    out
}

pub fn vscode_diagnostics(report: &ReviewReport) -> Vec<String> {
    report
        .findings
        .iter()
        .filter_map(|finding| {
            let source = finding.target.anchor.sources.first().or(finding
                .target
                .anchor
                .source
                .as_ref())?;
            if source.start_line == 0 {
                return None;
            }
            let severity = match finding.severity.as_str() {
                "error" => "error",
                "warning" => "warning",
                _ => "info",
            };
            let mut parts = vec![
                format!("[{}]", finding.rule_id),
                finding.status.clone(),
                finding.explanation.clone(),
            ];
            let evidence = if finding.evidence_spans.is_empty() {
                finding.evidence.clone()
            } else {
                finding
                    .evidence_spans
                    .iter()
                    .map(|span| span.quote.as_str())
                    .collect::<Vec<_>>()
                    .join("; ")
            };
            if !evidence.trim().is_empty() {
                parts.push(format!("Evidence: {evidence}"));
            }
            if let Some(suggestion) = finding.suggestion.as_deref() {
                parts.push(format!("Suggestion: {suggestion}"));
            }
            let message = parts
                .join(" ")
                .split_whitespace()
                .collect::<Vec<_>>()
                .join(" ");
            Some(format!(
                "{}({},{}): {}: {}",
                source.file,
                source.start_line,
                source.start_column.unwrap_or(1),
                severity,
                message
            ))
        })
        .collect()
}

fn finding_markdown(f: &ReviewFinding) -> String {
    let spans = if f.target.anchor.sources.is_empty() {
        f.target.anchor.source.iter().collect::<Vec<_>>()
    } else {
        f.target.anchor.sources.iter().collect()
    };
    let source = if spans.is_empty() {
        "unmapped".into()
    } else {
        spans
            .iter()
            .map(|s| {
                format!(
                    "{}:{}:{}–{}:{}",
                    s.file,
                    s.start_line,
                    s.start_column.unwrap_or(1),
                    s.end_line,
                    s.end_column.unwrap_or(1)
                )
            })
            .collect::<Vec<_>>()
            .join(", ")
    };
    let pdf = f
        .target
        .anchor
        .pdf
        .as_ref()
        .map(|p| format!("page {} ({})", p.page, p.mapping_quality));
    let status = if f.status.is_empty() {
        "finding"
    } else {
        &f.status
    };
    let confidence = f
        .confidence
        .map(|v| format!("  \n**Confidence:** {:.0}%", v * 100.0))
        .unwrap_or_default();
    let evidence = if f.evidence_spans.is_empty() {
        format!("**Evidence:** {}", f.evidence)
    } else {
        String::new()
    };
    let mut out = format!(
        "### {} · {} — `{}`\n\n**Target:** {} `{}`  \n**Source:** `{}`{}{}{}\n\n{}\n\n{}\n\n",
        f.severity,
        status,
        f.rule_id,
        f.target.target_type,
        f.target.id,
        source,
        pdf.map(|p| format!("  \n**PDF:** {}", p))
            .unwrap_or_default(),
        confidence,
        evidence,
        f.explanation,
        f.suggestion
            .as_deref()
            .map(|s| format!("**Suggestion:** {}", s))
            .unwrap_or_default()
    );
    for evidence in &f.evidence_spans {
        let locations = evidence
            .sources
            .iter()
            .map(|s| {
                format!(
                    "{}:{}:{}",
                    s.file,
                    s.start_line,
                    s.start_column.unwrap_or(1)
                )
            })
            .collect::<Vec<_>>()
            .join(", ");
        out.push_str(&format!(
            "- Evidence in `{}` at {}: {}\n",
            evidence.target_id,
            if locations.is_empty() {
                "unmapped"
            } else {
                &locations
            },
            evidence.quote
        ));
    }
    out.push('\n');
    out
}
