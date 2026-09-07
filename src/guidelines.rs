use crate::model::{RuleCandidate, RuleConflict, RuleDefinition};
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
                .entry((&rule.scope, &rule.kind, &rule.check.pattern))
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
    if let Some(items) = value.get("rules").and_then(Value::as_sequence) {
        for item in items {
            if let Err(err) = add_rule(item.clone(), source, registry) {
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
    validate_rule(&rule).with_context(|| format!("invalid rule {} in {source}", rule.id))?;
    rule.source = Some(source.into());
    rule.active = true;
    registry.active.push(rule);
    Ok(())
}

pub fn validate_rule(rule: &RuleDefinition) -> Result<()> {
    ensure!(!rule.id.trim().is_empty(), "id is required");
    ensure!(
        matches!(
            rule.scope.as_str(),
            "document"
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
        "regex" | "forbid" | "contains" => {
            let pattern = rule
                .check
                .pattern
                .as_deref()
                .filter(|s| !s.is_empty())
                .context("nonempty check.pattern is required")?;
            if rule.check.check_type == "regex" {
                Regex::new(pattern).context("invalid regex")?;
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
