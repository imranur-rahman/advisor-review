use crate::manuscript::{self, SourceDocument};
use crate::model::{DocumentTarget, SourceSpan, TargetAnchor};
use anyhow::{Result, bail};
use regex::Regex;
use serde_json::json;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

pub fn parse_project(main: &Path, project: &Path) -> Result<Vec<DocumentTarget>> {
    let source = SourceDocument::load(main, project)?;
    let mut targets = vec![];
    parse_flat(&source.content.text, project, &mut targets)?;
    targets.retain(|t| {
        !matches!(t.target_type.as_str(), "document" | "section" | "paragraph")
            && t.facts.get("environment").and_then(|v| v.as_str()) != Some("document")
    });
    let mut ids = std::collections::HashSet::new();
    targets.retain(|target| ids.insert(target.id.clone()));
    manuscript::build(&source, targets)
}

fn parse_flat(content: &str, project: &Path, targets: &mut Vec<DocumentTarget>) -> Result<()> {
    let rel = "expanded".to_string();
    let cleaned = clean_lines(content);
    let lines: Vec<&str> = cleaned.iter().map(String::as_str).collect();
    let env_re = Regex::new(r"\\begin\{([A-Za-z*]+)\}")?;
    let label_re = Regex::new(r"\\label\{([^}]+)\}")?;
    let section_re = Regex::new(r"\\(section|subsection|subsubsection|chapter)\*?\{([^}]*)\}")?;
    let include_re = Regex::new(r"\\(?:input|include)\{([^}]+)\}")?;
    let graphics_re = Regex::new(r"\\includegraphics(?:\[([^]]*)\])?\{([^}]+)\}")?;
    let cite_re = Regex::new(r"\\(cite|citep|citet)\{([^}]+)\}")?;
    let ref_re = Regex::new(r"\\(ref|autoref|cref)\{([^}]+)\}")?;
    let spans = environment_spans(&lines)?;
    let mut paragraph_start = None;
    let mut paragraph = String::new();
    for (idx, line) in lines.iter().enumerate() {
        let line_no = idx + 1;
        // Literal code must not be interpreted as TeX or ordinary prose.
        if spans
            .iter()
            .any(|s| is_code(&s.name) && idx > s.start && idx <= s.end)
        {
            continue;
        }
        for cap in section_re.captures_iter(line) {
            targets.push(DocumentTarget {
                id: format!(
                    "section:{}:{}:{}",
                    rel,
                    line_no,
                    cap.get(0).unwrap().start()
                ),
                target_type: "section".into(),
                label: None,
                text: cap[2].to_string(),
                anchor: TargetAnchor {
                    source: Some(SourceSpan {
                        file: rel.clone(),
                        start_line: line_no,
                        end_line: line_no,
                        ..Default::default()
                    }),
                    ..Default::default()
                },
                facts: BTreeMap::new(),
                ..Default::default()
            });
        }
        for cap in include_re.captures_iter(line) {
            flush_paragraph(
                &mut paragraph_start,
                &mut paragraph,
                &rel,
                line_no.saturating_sub(1),
                targets,
            );
            let _ = cap;
        }
        for cap in graphics_re.captures_iter(line) {
            let asset = resolve_asset(project, &cap[2]);
            let figure = spans.iter().rev().find(|s| {
                matches!(s.name.as_str(), "figure" | "figure*") && s.start <= idx && idx <= s.end
            });
            let (start, end) = figure.map(|s| (s.start, s.end)).unwrap_or((idx, idx));
            let mut facts = BTreeMap::new();
            if let Some(figure) = figure {
                facts.insert("environment".into(), json!(figure.name));
            }
            let options = cap.get(1).map(|m| m.as_str()).unwrap_or("");
            facts.insert("options".into(), json!(options));
            if let Some((w, h)) = image_dimensions(&asset) {
                facts.insert("pixel_width".into(), json!(w));
                facts.insert("pixel_height".into(), json!(h));
            }
            if let Some(width) = physical_width_in(options) {
                facts.insert("physical_width_in".into(), json!(width));
                if let Some(w) = facts.get("pixel_width").and_then(|v| v.as_f64()) {
                    facts.insert("effective_dpi".into(), json!(w / width));
                }
            }
            targets.push(DocumentTarget {
                id: format!("figure:{}:{}:{}", rel, line_no, cap.get(0).unwrap().start()),
                target_type: "figure".into(),
                label: lines[start..=end]
                    .iter()
                    .find_map(|l| label_re.captures(l).map(|c| c[1].to_string())),
                text: lines[start..=end].join("\n"),
                anchor: TargetAnchor {
                    source: Some(SourceSpan {
                        file: rel.clone(),
                        start_line: start + 1,
                        end_line: end + 1,
                        ..Default::default()
                    }),
                    asset: Some(asset.display().to_string()),
                    ..Default::default()
                },
                facts,
                ..Default::default()
            });
        }
        for cap in cite_re.captures_iter(line) {
            targets.push(DocumentTarget {
                id: format!("citation:{rel}:{line_no}:{}", cap.get(0).unwrap().start()),
                target_type: "citation".into(),
                label: Some(cap[2].to_string()),
                text: line.trim().into(),
                anchor: TargetAnchor {
                    source: Some(SourceSpan {
                        file: rel.clone(),
                        start_line: line_no,
                        end_line: line_no,
                        ..Default::default()
                    }),
                    ..Default::default()
                },
                facts: BTreeMap::new(),
                ..Default::default()
            });
        }
        for cap in ref_re.captures_iter(line) {
            targets.push(DocumentTarget {
                id: format!("reference:{rel}:{line_no}:{}", cap.get(0).unwrap().start()),
                target_type: "reference".into(),
                label: Some(cap[2].to_string()),
                text: line.trim().into(),
                anchor: TargetAnchor {
                    source: Some(SourceSpan {
                        file: rel.clone(),
                        start_line: line_no,
                        end_line: line_no,
                        ..Default::default()
                    }),
                    ..Default::default()
                },
                facts: BTreeMap::new(),
                ..Default::default()
            });
        }
        for cap in env_re.captures_iter(line) {
            let env = cap[1].to_string();
            let end = spans
                .iter()
                .find(|s| s.name == env && s.start == idx)
                .map(|s| s.end + 1)
                .unwrap_or(line_no);
            let kind = if env.contains("table") || env == "tabular" {
                "table"
            } else if env.contains("equation") || env == "math" {
                "equation"
            } else if is_code(&env) {
                "code_block"
            } else if matches!(env.as_str(), "figure" | "figure*") {
                "figure"
            } else {
                "environment"
            };
            let label = lines[idx..end.min(lines.len())]
                .iter()
                .find_map(|l| label_re.captures(l).map(|c| c[1].to_string()));
            let text = lines[idx..end.min(lines.len())].join("\n");
            // Asset-backed figures are emitted at includegraphics with the full
            // enclosing caption; do not emit the same figure a second time.
            if kind != "figure" || !graphics_re.is_match(&text) {
                targets.push(DocumentTarget {
                    id: format!("{kind}:{rel}:{line_no}:{}", cap.get(0).unwrap().start()),
                    target_type: kind.into(),
                    label,
                    text,
                    anchor: TargetAnchor {
                        source: Some(SourceSpan {
                            file: rel.clone(),
                            start_line: line_no,
                            end_line: end,
                            ..Default::default()
                        }),
                        ..Default::default()
                    },
                    facts: BTreeMap::from([(String::from("environment"), json!(env))]),
                    ..Default::default()
                });
            }
            if kind == "table" {
                for (offset, row) in lines[idx..end.min(lines.len())].iter().enumerate() {
                    if row.contains('&') {
                        for (column, cell) in row.split('&').enumerate() {
                            targets.push(DocumentTarget {
                                id: format!("table-cell:{rel}:{}:{}", line_no + offset, column + 1),
                                target_type: "table_cell".into(),
                                label: None,
                                text: cell.trim().trim_end_matches("\\\\").trim().into(),
                                anchor: TargetAnchor {
                                    source: Some(SourceSpan {
                                        file: rel.clone(),
                                        start_line: line_no + offset,
                                        end_line: line_no + offset,
                                        ..Default::default()
                                    }),
                                    ..Default::default()
                                },
                                facts: BTreeMap::from([(
                                    String::from("column"),
                                    json!(column + 1),
                                )]),
                                ..Default::default()
                            });
                        }
                    }
                }
            }
        }
        let trimmed = line.trim();
        let structural = section_re.is_match(line)
            || include_re.is_match(line)
            || env_re.is_match(line)
            || trimmed.starts_with("\\end{")
            || [
                "\\documentclass",
                "\\usepackage",
                "\\label",
                "\\bibliography",
                "\\title",
                "\\author",
                "\\maketitle",
            ]
            .iter()
            .any(|prefix| trimmed.starts_with(prefix));
        let in_specialized = spans.iter().any(|s| {
            (is_code(&s.name)
                || s.name.contains("table")
                || s.name == "tabular"
                || s.name.contains("figure")
                || s.name.contains("equation")
                || s.name == "math")
                && s.start <= idx
                && idx <= s.end
        });
        if trimmed.is_empty() || structural || in_specialized {
            flush_paragraph(
                &mut paragraph_start,
                &mut paragraph,
                &rel,
                line_no.saturating_sub(1),
                targets,
            );
        } else {
            if paragraph_start.is_none() {
                paragraph_start = Some(line_no);
            }
            if !paragraph.is_empty() {
                paragraph.push(' ');
            }
            paragraph.push_str(trimmed);
        }
    }
    flush_paragraph(
        &mut paragraph_start,
        &mut paragraph,
        &rel,
        lines.len(),
        targets,
    );
    Ok(())
}

fn flush_paragraph(
    start: &mut Option<usize>,
    text: &mut String,
    file: &str,
    end: usize,
    targets: &mut Vec<DocumentTarget>,
) {
    if let Some(s) = start.take() {
        if !text.trim().is_empty() {
            targets.push(DocumentTarget {
                id: format!("paragraph:{file}:{s}"),
                target_type: "paragraph".into(),
                label: None,
                text: text.trim().into(),
                anchor: TargetAnchor {
                    source: Some(SourceSpan {
                        file: file.into(),
                        start_line: s,
                        end_line: end.max(s),
                        ..Default::default()
                    }),
                    ..Default::default()
                },
                facts: BTreeMap::new(),
                ..Default::default()
            });
        }
    }
    text.clear();
}

fn image_dimensions(path: &Path) -> Option<(u32, u32)> {
    let data = std::fs::read(path).ok()?;
    if data.starts_with(&[137, 80, 78, 71, 13, 10, 26, 10]) && data.len() >= 24 {
        return Some((
            u32::from_be_bytes(data[16..20].try_into().ok()?),
            u32::from_be_bytes(data[20..24].try_into().ok()?),
        ));
    }
    if data.starts_with(&[0xFF, 0xD8]) {
        let mut i = 2;
        while i + 9 < data.len() {
            if data[i] != 0xFF {
                i += 1;
                continue;
            }
            let marker = data[i + 1];
            let len = u16::from_be_bytes([data[i + 2], data[i + 3]]) as usize;
            if (0xC0..=0xC3).contains(&marker) && i + 8 < data.len() {
                return Some((
                    u16::from_be_bytes([data[i + 7], data[i + 8]]) as u32,
                    u16::from_be_bytes([data[i + 5], data[i + 6]]) as u32,
                ));
            }
            i += 2 + len;
        }
    }
    None
}

fn physical_width_in(options: &str) -> Option<f64> {
    let width = options
        .split(',')
        .find_map(|part| part.trim().strip_prefix("width="))?;
    if let Some(v) = width.strip_suffix("in") {
        return v
            .trim()
            .parse::<f64>()
            .ok()
            .filter(|n| n.is_finite() && *n > 0.0);
    }
    if let Some(v) = width.strip_suffix("cm") {
        return v
            .trim()
            .parse::<f64>()
            .ok()
            .filter(|n| n.is_finite() && *n > 0.0)
            .map(|n| n / 2.54);
    }
    None
}

fn resolve_asset(project: &Path, name: &str) -> PathBuf {
    let path = project.join(name);
    if path.is_file() || path.extension().is_some() {
        return path;
    }
    ["pdf", "png", "jpg", "jpeg"]
        .iter()
        .map(|ext| path.with_extension(ext))
        .find(|candidate| candidate.is_file())
        .unwrap_or(path)
}

fn is_code(name: &str) -> bool {
    matches!(name, "lstlisting" | "verbatim" | "verbatim*" | "minted")
}

fn strip_comment(line: &str) -> &str {
    let mut backslashes = 0;
    for (idx, ch) in line.char_indices() {
        if ch == '%' && backslashes % 2 == 0 {
            return &line[..idx];
        }
        backslashes = if ch == '\\' { backslashes + 1 } else { 0 };
    }
    line
}

fn clean_lines(content: &str) -> Vec<String> {
    let mut literal: Option<String> = None;
    content
        .lines()
        .map(|line| {
            if let Some(name) = &literal {
                let result = line.to_string();
                if line.contains(&format!("\\end{{{name}}}")) {
                    literal = None;
                }
                return result;
            }
            let line = strip_comment(line);
            for name in ["lstlisting", "verbatim", "verbatim*", "minted"] {
                if line.contains(&format!("\\begin{{{name}}}"))
                    && !line.contains(&format!("\\end{{{name}}}"))
                {
                    literal = Some(name.into());
                }
            }
            line.to_string()
        })
        .collect()
}

struct EnvironmentSpan {
    name: String,
    start: usize,
    end: usize,
}

fn environment_spans(lines: &[&str]) -> Result<Vec<EnvironmentSpan>> {
    let token = Regex::new(r"\\(begin|end)\{([A-Za-z*]+)\}")?;
    let mut stack: Vec<(String, usize)> = Vec::new();
    let mut spans = Vec::new();
    for (idx, line) in lines.iter().enumerate() {
        for cap in token.captures_iter(line) {
            if let Some((name, _)) = stack.last() {
                if is_code(name) && !(cap[1] == *"end" && cap[2] == *name) {
                    continue;
                }
            }
            if &cap[1] == "begin" {
                stack.push((cap[2].into(), idx));
            } else {
                let Some((name, start)) = stack.pop() else {
                    bail!("unmatched environment end at line {}", idx + 1);
                };
                if name != cap[2] {
                    bail!(
                        "mismatched environment at line {}: expected end of {name}",
                        idx + 1
                    );
                }
                spans.push(EnvironmentSpan {
                    name,
                    start,
                    end: idx,
                });
            }
        }
    }
    if let Some((name, start)) = stack.last() {
        bail!("unclosed environment {name} at line {}", start + 1);
    }
    spans.sort_by_key(|span| span.start);
    Ok(spans)
}

pub fn source_files(project: &Path) -> Vec<PathBuf> {
    WalkDir::new(project)
        .into_iter()
        .filter_map(|e| e.ok())
        .map(|e| e.path().to_path_buf())
        .filter(|p| p.extension().and_then(|e| e.to_str()) == Some("tex"))
        .collect()
}
