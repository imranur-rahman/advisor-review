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
    if report.findings.is_empty() {
        out.push_str("No findings were produced.\n");
    }
    for finding in &report.findings {
        out.push_str(&finding_markdown(finding));
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
    let source = f
        .target
        .anchor
        .source
        .as_ref()
        .map(|s| format!("{}:{}-{}", s.file, s.start_line, s.end_line))
        .unwrap_or_else(|| "unmapped".into());
    let pdf = f
        .target
        .anchor
        .pdf
        .as_ref()
        .map(|p| format!("page {} ({})", p.page, p.mapping_quality));
    format!(
        "## {} `{}`\n\n**Target:** {} `{}`  \n**Source:** `{}`{}  \n**Evidence:** {}\n\n{}\n\n{}\n\n",
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
    )
}
