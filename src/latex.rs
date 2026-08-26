use crate::discover::read_text;
use crate::model::{DocumentTarget, SourceSpan, TargetAnchor};
use anyhow::Result;
use regex::Regex;
use serde_json::json;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

pub fn parse_project(main: &Path, project: &Path) -> Result<Vec<DocumentTarget>> {
    let mut targets = vec![];
    let mut visited = vec![];
    parse_file(main, project, &mut visited, &mut targets)?;
    Ok(targets)
}

fn parse_file(
    path: &Path,
    project: &Path,
    visited: &mut Vec<PathBuf>,
    targets: &mut Vec<DocumentTarget>,
) -> Result<()> {
    let canonical = std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    if visited.contains(&canonical) {
        return Ok(());
    }
    visited.push(canonical);
    let content = read_text(path)?;
    let rel = path
        .strip_prefix(project)
        .unwrap_or(path)
        .display()
        .to_string();
    let lines: Vec<&str> = content.lines().collect();
    let env_re = Regex::new(r"\\begin\{([A-Za-z*]+)\}")?;
    let label_re = Regex::new(r"\\label\{([^}]+)\}")?;
    let section_re = Regex::new(r"\\(section|subsection|subsubsection|chapter)\*?\{([^}]*)\}")?;
    let include_re = Regex::new(r"\\(?:input|include)\{([^}]+)\}")?;
    let graphics_re = Regex::new(r"\\includegraphics(?:\[([^]]*)\])?\{([^}]+)\}")?;
    let cite_re = Regex::new(r"\\(cite|citep|citet)\{([^}]+)\}")?;
    let ref_re = Regex::new(r"\\(ref|autoref|cref)\{([^}]+)\}")?;
    let mut paragraph_start = None;
    let mut paragraph = String::new();
    for (idx, line) in lines.iter().enumerate() {
        let line_no = idx + 1;
        if let Some(cap) = section_re.captures(line) {
            targets.push(DocumentTarget {
                id: format!("section:{}:{}", rel, line_no),
                target_type: "section".into(),
                label: None,
                text: cap[2].to_string(),
                anchor: TargetAnchor {
                    source: Some(SourceSpan {
                        file: rel.clone(),
                        start_line: line_no,
                        end_line: line_no,
                    }),
                    ..Default::default()
                },
                facts: BTreeMap::new(),
            });
        }
        if let Some(cap) = include_re.captures(line) {
            let mut child = project.join(&cap[1]);
            if child.extension().is_none() {
                child.set_extension("tex");
            }
            if child.is_file() {
                parse_file(&child, project, visited, targets)?;
            }
        }
        if let Some(cap) = graphics_re.captures(line) {
            let asset = project.join(&cap[2]);
            let mut facts = BTreeMap::new();
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
                id: format!("figure:{}:{}", rel, line_no),
                target_type: "figure".into(),
                label: None,
                text: line.trim().into(),
                anchor: TargetAnchor {
                    source: Some(SourceSpan {
                        file: rel.clone(),
                        start_line: line_no,
                        end_line: line_no,
                    }),
                    asset: Some(asset.display().to_string()),
                    ..Default::default()
                },
                facts,
            });
        }
        if let Some(cap) = cite_re.captures(line) {
            targets.push(DocumentTarget {
                id: format!("citation:{rel}:{line_no}"),
                target_type: "citation".into(),
                label: Some(cap[2].to_string()),
                text: line.trim().into(),
                anchor: TargetAnchor {
                    source: Some(SourceSpan {
                        file: rel.clone(),
                        start_line: line_no,
                        end_line: line_no,
                    }),
                    ..Default::default()
                },
                facts: BTreeMap::new(),
            });
        }
        if let Some(cap) = ref_re.captures(line) {
            targets.push(DocumentTarget {
                id: format!("reference:{rel}:{line_no}"),
                target_type: "reference".into(),
                label: Some(cap[2].to_string()),
                text: line.trim().into(),
                anchor: TargetAnchor {
                    source: Some(SourceSpan {
                        file: rel.clone(),
                        start_line: line_no,
                        end_line: line_no,
                    }),
                    ..Default::default()
                },
                facts: BTreeMap::new(),
            });
        }
        if let Some(cap) = env_re.captures(line) {
            let env = cap[1].to_string();
            let end = lines[idx..]
                .iter()
                .position(|l| l.contains(&format!("\\end{{{env}}}")))
                .map(|v| idx + v + 1)
                .unwrap_or(line_no);
            let kind = if env.contains("table") || env == "tabular" {
                "table"
            } else if env.contains("equation") || env == "math" {
                "equation"
            } else if env.contains("lst") || env == "verbatim" || env == "minted" {
                "code_block"
            } else {
                "environment"
            };
            let label = lines[idx..end.min(lines.len())]
                .iter()
                .find_map(|l| label_re.captures(l).map(|c| c[1].to_string()));
            let text = lines[idx..end.min(lines.len())].join("\n");
            targets.push(DocumentTarget {
                id: label
                    .clone()
                    .unwrap_or_else(|| format!("{kind}:{rel}:{line_no}")),
                target_type: kind.into(),
                label,
                text,
                anchor: TargetAnchor {
                    source: Some(SourceSpan {
                        file: rel.clone(),
                        start_line: line_no,
                        end_line: end,
                    }),
                    ..Default::default()
                },
                facts: BTreeMap::from([(String::from("environment"), json!(env))]),
            });
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
                                    }),
                                    ..Default::default()
                                },
                                facts: BTreeMap::from([(
                                    String::from("column"),
                                    json!(column + 1),
                                )]),
                            });
                        }
                    }
                }
            }
        }
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('%') {
            flush_paragraph(
                &mut paragraph_start,
                &mut paragraph,
                &rel,
                line_no.saturating_sub(1),
                targets,
            );
        } else if !trimmed.starts_with('\\') {
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
                    }),
                    ..Default::default()
                },
                facts: BTreeMap::new(),
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
        return v.trim().parse().ok();
    }
    if let Some(v) = width.strip_suffix("cm") {
        return v.trim().parse::<f64>().ok().map(|n| n / 2.54);
    }
    None
}

pub fn source_files(project: &Path) -> Vec<PathBuf> {
    WalkDir::new(project)
        .into_iter()
        .filter_map(|e| e.ok())
        .map(|e| e.path().to_path_buf())
        .filter(|p| p.extension().and_then(|e| e.to_str()) == Some("tex"))
        .collect()
}
