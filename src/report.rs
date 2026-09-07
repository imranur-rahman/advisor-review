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
    let mut out = format!(
        "# Manuscript Review\n\n- Project: `{}`\n- Main LaTeX: `{}`\n- PDF: `{}`\n- Findings: {}\n\n",
        report.project,
        report.main_tex,
        report.pdf,
        report.findings.len()
    );
    if report.is_incomplete() {
        out.push_str("**Review incomplete:** some rules could not be evaluated. See Review Issues below.\n\n");
    }
    if report.findings.is_empty() {
        out.push_str("No findings were produced.\n");
    }
    if !report.coverage.is_empty() {
        out.push_str("## Coverage by scope\n\n| Scope | Selected | Targets | Rules | Evaluations | Completed | Partial | Skipped | Failed | Not applicable |\n|---|---|---:|---:|---:|---:|---:|---:|---:|---:|\n");
        for (scope, c) in &report.coverage {
            out.push_str(&format!(
                "| {} | {} | {} | {} | {} | {} | {} | {} | {} | {} |\n",
                scope,
                c.selected,
                c.targets,
                c.rules,
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
    let mut out = format!(
        "### {} `{}`\n\n**Target:** {} `{}`  \n**Source:** `{}`{}  \n**Evidence:** {}\n\n{}\n\n{}\n\n",
        f.severity,
        f.rule_id,
        f.target.target_type,
        f.target.id,
        source,
        pdf.map(|p| format!("  \n**PDF:** {}", p))
            .unwrap_or_default(),
        f.evidence,
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
