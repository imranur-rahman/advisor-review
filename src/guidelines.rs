use crate::model::{RuleCandidate, RuleConflict, RuleDefinition};
use anyhow::{Context, Result};
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
    for entry in WalkDir::new(dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
    {
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
    for i in 0..registry.active.len() {
        for j in (i + 1)..registry.active.len() {
            if registry.active[i].scope == registry.active[j].scope
                && registry.active[i].priority != registry.active[j].priority
            {
                registry.conflicts.push(RuleConflict {
                    rule_ids: vec![registry.active[i].id.clone(), registry.active[j].id.clone()],
                    target_scope: registry.active[i].scope.clone(),
                    resolution: format!(
                        "higher priority wins: {}",
                        if registry.active[i].priority > registry.active[j].priority {
                            &registry.active[i].id
                        } else {
                            &registry.active[j].id
                        }
                    ),
                });
            }
        }
    }
    Ok(registry)
}

fn load_yaml_file(path: &Path, source: &str, registry: &mut RuleRegistry) -> Result<()> {
    let raw = fs::read_to_string(path).with_context(|| format!("read guideline {source}"))?;
    let value: Value = serde_yaml::from_str(&raw)
        .with_context(|| format!("parse structured guideline {source}"))?;
    if let Some(items) = value.get("rules").and_then(Value::as_sequence) {
        for item in items {
            add_rule(item.clone(), source, registry)?;
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
            let value: Value =
                serde_yaml::from_str(&block).with_context(|| format!("parse rule in {source}"))?;
            add_rule(value, source, registry)?;
            continue;
        }
        if in_rule {
            block.push_str(line);
            block.push('\n');
        } else if !line.trim().is_empty() && !line.trim_start().starts_with('#') {
            prose.push(line.trim());
        }
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
    if rule.id.trim().is_empty()
        || rule.scope.trim().is_empty()
        || rule.kind.trim().is_empty()
        || rule.check.check_type.trim().is_empty()
    {
        registry.issues.push(format!(
            "invalid rule in {source}: id, scope, kind, and check.type are required"
        ));
        return Ok(());
    }
    rule.source = Some(source.into());
    rule.active = true;
    registry.active.push(rule);
    Ok(())
}

fn simple_id(source: &str) -> String {
    source
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .collect()
}
