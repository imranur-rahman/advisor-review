use crate::guideline_items;
use crate::model::{RuleCandidate, RuleConflict, RuleDefinition, RuleProvenance, StaleRule};
use anyhow::{Context, Result, bail, ensure};
use regex::Regex;
use serde_yaml::Value;
use std::fs;
use std::path::Path;
use walkdir::WalkDir;

#[derive(Debug, Default)]
pub struct RuleRegistry {
    pub active: Vec<RuleDefinition>,
    pub candidates: Vec<RuleCandidate>,
    pub issues: Vec<String>,
    pub conflicts: Vec<RuleConflict>,
    /// Valid rules with `status: draft`. They never run in a review.
    pub drafts: Vec<RuleDefinition>,
    /// Rules whose guideline item changed after compilation.
    pub stale: Vec<StaleRule>,
}

pub fn load(dir: &Path) -> Result<RuleRegistry> {
    let mut registry = RuleRegistry::default();
    for entry in WalkDir::new(dir).sort_by_file_name() {
        let entry = match entry {
            Ok(entry) => entry,
            Err(err) => {
                registry
                    .issues
                    .push(format!("read guideline directory: {err}"));
                continue;
            }
        };
        if !entry.file_type().is_file() {
            continue;
        }
        let path = entry.path();
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or_default();
        let source = path.display().to_string();
        match ext {
            "yaml" | "yml" => {
                if let Err(err) = load_yaml_file(path, &source, &mut registry) {
                    registry.issues.push(format!("{err:#}"));
                }
            }
            "md" | "markdown" => {
                if let Err(err) = load_markdown_file(path, &source, &mut registry) {
                    registry.issues.push(format!("{err:#}"));
                }
            }
            _ => {}
        }
    }
    registry.active.sort_by(|a, b| a.id.cmp(&b.id));
    // Duplicate identities are ambiguous: reject all definitions, not whichever
    // happened to be discovered last.
    let mut counts = std::collections::BTreeMap::new();
    for rule in &registry.active {
        *counts.entry(rule.id.clone()).or_insert(0) += 1;
    }
    registry.active.retain(|rule| {
        if counts[&rule.id] > 1 {
            registry.issues.push(format!(
                "duplicate rule id {} in {}",
                rule.id,
                rule.source.as_deref().unwrap_or("unknown")
            ));
            false
        } else {
            true
        }
    });
    let mut groups = std::collections::BTreeMap::new();
    for rule in &registry.active {
        if matches!(rule.check.check_type.as_str(), "contains" | "forbid") {
            groups
                .entry((
                    &rule.scope,
                    &rule.kind,
                    &rule.check.pattern,
                    &rule.section,
                    &rule.text_view,
                    rule.include_subsections,
                ))
                .or_insert_with(Vec::new)
                .push(rule);
        }
    }
    let mut suppressed = std::collections::BTreeSet::new();
    for rules in groups.values() {
        if !rules.iter().any(|rule| conflicts(rules[0], rule)) {
            continue;
        }
        let winner = rules.iter().max_by_key(|rule| rule.priority).unwrap();
        let tied = rules
            .iter()
            .any(|rule| rule.priority == winner.priority && conflicts(winner, rule));
        let losers: Vec<_> = rules
            .iter()
            .filter(|rule| tied || conflicts(winner, rule))
            .map(|rule| rule.id.clone())
            .collect();
        let resolution = if tied {
            registry.issues.push(format!(
                "unresolved conflict for rules {}; assign distinct top priorities",
                losers.join(", ")
            ));
            "equal top priority: conflicting group skipped".into()
        } else {
            format!(
                "higher priority wins: {}; suppressed: {}",
                winner.id,
                losers.join(", ")
            )
        };
        suppressed.extend(losers);
        registry.conflicts.push(RuleConflict {
            rule_ids: rules.iter().map(|rule| rule.id.clone()).collect(),
            target_scope: winner.scope.clone(),
            resolution,
        });
    }
    registry
        .active
        .retain(|rule| !suppressed.contains(&rule.id));
    Ok(registry)
}

fn load_yaml_file(path: &Path, source: &str, registry: &mut RuleRegistry) -> Result<()> {
    let raw = fs::read_to_string(path).with_context(|| format!("read guideline {source}"))?;
    let value: Value = serde_yaml::from_str(&raw)
        .with_context(|| format!("parse structured guideline {source}"))?;
    let rules = value.get("rules").and_then(Value::as_sequence);
    let uncheckable = value.get("uncheckable").and_then(Value::as_sequence);
    if rules.is_some() || uncheckable.is_some() {
        for item in rules.into_iter().flatten() {
            if let Err(err) = add_rule(item.clone(), source, registry) {
                registry.issues.push(format!("{err:#}"));
            }
        }
        for item in uncheckable.into_iter().flatten() {
            if let Err(err) = add_uncheckable(item.clone(), source, registry) {
                registry.issues.push(format!("{err:#}"));
            }
        }
    } else {
        add_rule(value, source, registry)?;
    }
    Ok(())
}

fn load_markdown_file(path: &Path, source: &str, registry: &mut RuleRegistry) -> Result<()> {
    let raw = fs::read_to_string(path).with_context(|| format!("read guideline {source}"))?;
    let mut in_rule = false;
    let mut block = String::new();
    let mut prose = vec![];
    for line in raw.lines() {
        if line.trim() == "```rule" {
            in_rule = true;
            block.clear();
            continue;
        }
        if in_rule && line.trim() == "```" {
            in_rule = false;
            let result = serde_yaml::from_str(&block)
                .with_context(|| format!("parse rule in {source}"))
                .and_then(|value| add_rule(value, source, registry));
            if let Err(err) = result {
                registry.issues.push(format!("{err:#}"));
            }
            continue;
        }
        if in_rule {
            block.push_str(line);
            block.push('\n');
        } else if !line.trim().is_empty() && !line.trim_start().starts_with('#') {
            prose.push(line.trim());
        }
    }
    if in_rule {
        registry
            .issues
            .push(format!("unterminated rule block in {source}"));
    }
    if !prose.is_empty() {
        registry.candidates.push(RuleCandidate {
            id: format!("candidate:{}", simple_id(source)),
            source: source.into(),
            text: prose.join(" "),
            reason: "Natural-language guidance requires explicit review before activation".into(),
            status: "candidate".into(),
        });
    }
    Ok(())
}

fn add_rule(value: Value, source: &str, registry: &mut RuleRegistry) -> Result<()> {
    let mut rule: RuleDefinition =
        serde_yaml::from_value(value).with_context(|| format!("invalid rule in {source}"))?;
    if rule.scope == "code" {
        rule.scope = "code_block".into();
    }
    if rule.scope == "manuscript" {
        rule.scope = "document".into();
    }
    validate_rule(&rule).with_context(|| format!("invalid rule {} in {source}", rule.id))?;
    rule.source = Some(source.into());
    if let Some(provenance) = rule.provenance.clone() {
        let file = provenance_path(source, &provenance);
        rule.source = Some(format!(
            "{}:{}-{}",
            file.display(),
            provenance.start_line,
            provenance.end_line
        ));
        if let Some(reason) = stale_reason(&file, &provenance) {
            registry.stale.push(StaleRule {
                rule_id: rule.id.clone(),
                file: file.display().to_string(),
                start_line: provenance.start_line,
                end_line: provenance.end_line,
                reason,
            });
        }
    }
    if rule.status.as_deref() == Some("draft") {
        registry.candidates.push(RuleCandidate {
            id: rule.id.clone(),
            source: rule.source.clone().unwrap_or_default(),
            text: rule
                .description
                .clone()
                .or_else(|| rule.name.clone())
                .unwrap_or_default(),
            reason: "Compiled draft rule; set status: active to run it".into(),
            status: "draft".into(),
        });
        registry.drafts.push(rule);
        return Ok(());
    }
    rule.active = true;
    registry.active.push(rule);
    Ok(())
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct UncheckableEntry {
    id: String,
    text: String,
    reason: String,
    provenance: RuleProvenance,
}

fn add_uncheckable(value: Value, source: &str, registry: &mut RuleRegistry) -> Result<()> {
    let entry: UncheckableEntry = serde_yaml::from_value(value)
        .with_context(|| format!("invalid uncheckable entry in {source}"))?;
    let file = provenance_path(source, &entry.provenance);
    registry.candidates.push(RuleCandidate {
        id: entry.id,
        source: format!(
            "{}:{}-{}",
            file.display(),
            entry.provenance.start_line,
            entry.provenance.end_line
        ),
        text: entry.text,
        reason: entry.reason,
        status: "uncheckable".into(),
    });
    Ok(())
}

/// Provenance paths are relative to the directory of the compiled pack.
fn provenance_path(source: &str, provenance: &RuleProvenance) -> std::path::PathBuf {
    Path::new(source)
        .parent()
        .unwrap_or_else(|| Path::new(""))
        .join(&provenance.file)
}

/// Returns why a compiled rule is stale, or `None` when its guideline item is unchanged.
/// An item that moved to other lines with the same text is not stale.
pub fn stale_reason(file: &Path, provenance: &RuleProvenance) -> Option<String> {
    let Ok(text) = fs::read_to_string(file) else {
        return Some("source guideline file is missing or unreadable".into());
    };
    if guideline_items::hash_lines(&text, provenance.start_line, provenance.end_line).as_deref()
        == Some(provenance.sha256.as_str())
    {
        return None;
    }
    let items = guideline_items::split(&text);
    if guideline_items::find_by_hash(&items, &provenance.sha256).is_some() {
        return None;
    }
    Some("source guideline item changed after compilation".into())
}

pub fn validate_rule(rule: &RuleDefinition) -> Result<()> {
    ensure!(!rule.id.trim().is_empty(), "id is required");
    ensure!(
        matches!(
            rule.scope.as_str(),
            "document"
                | "manuscript"
                | "heading"
                | "section"
                | "paragraph"
                | "sentence"
                | "figure"
                | "table"
                | "table_row"
                | "table_cell"
                | "equation"
                | "code"
                | "code_block"
                | "code_line"
                | "citation"
                | "reference"
                | "pdf_page"
                | "environment"
        ),
        "unknown scope: {}",
        rule.scope
    );
    ensure!(
        matches!(rule.text_view.as_str(), "prose" | "source"),
        "text_view must be prose or source"
    );
    ensure!(
        matches!(
            rule.context.as_str(),
            "none" | "paragraph" | "neighbors" | "section"
        ),
        "context must be none, paragraph, neighbors, or section"
    );
    ensure!(
        rule.section.as_deref().is_none_or(|s| !s.trim().is_empty()),
        "section selector cannot be empty"
    );
    ensure!(
        rule.scope != "document" && rule.scope != "manuscript" || rule.section.is_none(),
        "document scope cannot select a section"
    );
    ensure!(
        rule.include_subsections || rule.scope == "section",
        "include_subsections: false requires section scope"
    );
    ensure!(
        matches!(
            rule.kind.as_str(),
            "text" | "asset" | "structure" | "semantic-text" | "semantic-vision" | "cross-modal"
        ),
        "unknown kind: {}",
        rule.kind
    );
    ensure!(
        matches!(
            rule.severity.as_str(),
            "error" | "warning" | "suggestion" | "info"
        ),
        "unknown severity: {}",
        rule.severity
    );
    ensure!(
        rule.status
            .as_deref()
            .is_none_or(|s| matches!(s, "draft" | "active")),
        "status must be draft or active"
    );
    if let Some(tests) = &rule.tests {
        ensure!(
            tests
                .should_fire
                .iter()
                .chain(&tests.should_pass)
                .all(|t| !t.trim().is_empty()),
            "example tests cannot be empty strings"
        );
    }
    if rule.provenance.is_some() {
        ensure!(
            rule.tests
                .as_ref()
                .is_some_and(|t| !t.should_fire.is_empty() && !t.should_pass.is_empty()),
            "compiled rules require at least one tests.should_fire and one tests.should_pass example"
        );
    }
    ensure!(
        rule.check.max.is_none() || rule.check.check_type == "count",
        "check.max is only supported for count checks"
    );
    ensure!(
        rule.check.parameters.is_empty(),
        "unknown check parameters: {:?}",
        rule.check.parameters.keys().collect::<Vec<_>>()
    );
    let semantic = matches!(
        rule.kind.as_str(),
        "semantic-text" | "semantic-vision" | "cross-modal"
    );
    ensure!(
        semantic || rule.requires.is_empty(),
        "provider capability requirements are only supported for semantic rules"
    );
    ensure!(
        semantic == (rule.check.check_type == "semantic"),
        "semantic kinds require check.type: semantic; deterministic kinds require a deterministic check"
    );
    match rule.check.check_type.as_str() {
        "regex" | "forbid" | "contains" | "count" => {
            let pattern = rule
                .check
                .pattern
                .as_deref()
                .filter(|s| !s.is_empty())
                .context("nonempty check.pattern is required")?;
            if matches!(rule.check.check_type.as_str(), "regex" | "count") {
                Regex::new(pattern).context("invalid regex")?;
            }
            if rule.check.check_type == "count" {
                ensure!(
                    rule.check.max.is_some_and(|max| max >= 0),
                    "count requires a non-negative integer check.max"
                );
            }
        }
        "min_pixels" => {
            ensure!(
                rule.check
                    .value
                    .as_ref()
                    .and_then(serde_json::Value::as_u64)
                    .is_some_and(|n| n > 0),
                "min_pixels requires a positive integer check.value"
            );
            ensure!(rule.scope == "figure", "asset checks require figure scope");
        }
        "min_effective_dpi" => {
            ensure!(
                rule.check
                    .value
                    .as_ref()
                    .and_then(serde_json::Value::as_f64)
                    .is_some_and(|n| n.is_finite() && n > 0.0),
                "min_effective_dpi requires a positive check.value"
            );
            ensure!(rule.scope == "figure", "asset checks require figure scope");
        }
        "environment_exists" => {}
        "semantic" => ensure!(
            rule.description
                .as_deref()
                .is_some_and(|s| !s.trim().is_empty()),
            "semantic rules require a description"
        ),
        other => bail!("unknown check.type: {other}"),
    }
    Ok(())
}

fn conflicts(a: &RuleDefinition, b: &RuleDefinition) -> bool {
    a.scope == b.scope
        && a.kind == b.kind
        && a.section == b.section
        && a.text_view == b.text_view
        && a.include_subsections == b.include_subsections
        && a.check.pattern == b.check.pattern
        && matches!(
            (a.check.check_type.as_str(), b.check.check_type.as_str()),
            ("contains", "forbid") | ("forbid", "contains")
        )
}

fn simple_id(source: &str) -> String {
    source
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .collect()
}
