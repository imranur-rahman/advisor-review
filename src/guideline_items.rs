//! Splits Markdown guidelines into compilable items and hashes item text.

use sha2::{Digest, Sha256};

/// One list item or prose paragraph from a Markdown guideline file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GuidelineItem {
    /// 1-based position among the file's items.
    pub number: usize,
    /// Nearest heading above the item, without `#` markers.
    pub heading: Option<String>,
    /// Normalized item text: each line trimmed, joined with `\n`.
    pub text: String,
    pub start_line: usize,
    pub end_line: usize,
}

/// Trims each line and joins with `\n`, so indentation changes do not mark rules stale.
pub fn normalize(lines: &[&str]) -> String {
    lines
        .iter()
        .map(|l| l.trim())
        .collect::<Vec<_>>()
        .join("\n")
}

pub fn hash(text: &str) -> String {
    use std::fmt::Write;
    Sha256::digest(text.as_bytes())
        .iter()
        .fold(String::with_capacity(64), |mut out, b| {
            let _ = write!(out, "{b:02x}");
            out
        })
}

/// Hash of the normalized text at a 1-based inclusive line range, if the range exists.
pub fn hash_lines(source: &str, start_line: usize, end_line: usize) -> Option<String> {
    let lines: Vec<&str> = source.lines().collect();
    if start_line == 0 || end_line < start_line || end_line > lines.len() {
        return None;
    }
    Some(hash(&normalize(&lines[start_line - 1..end_line])))
}

fn is_list_marker(line: &str) -> bool {
    if let Some(rest) = line
        .strip_prefix("- ")
        .or_else(|| line.strip_prefix("* "))
        .or_else(|| line.strip_prefix("+ "))
    {
        return !rest.trim().is_empty();
    }
    let digits = line.chars().take_while(char::is_ascii_digit).count();
    digits > 0
        && matches!(line[digits..].chars().next(), Some('.' | ')'))
        && line[digits + 1..].starts_with(' ')
}

fn is_fence(line: &str) -> bool {
    let t = line.trim_start();
    t.starts_with("```") || t.starts_with("~~~")
}

fn heading_text(line: &str) -> Option<String> {
    let hashes = line.chars().take_while(|c| *c == '#').count();
    (hashes > 0 && hashes <= 6 && line[hashes..].starts_with(' '))
        .then(|| line[hashes..].trim().to_string())
}

/// Splits Markdown into items. Headings give context, fenced blocks (including
/// ```` ```rule ```` blocks) and HTML comments are skipped.
pub fn split(markdown: &str) -> Vec<GuidelineItem> {
    #[derive(PartialEq)]
    enum Kind {
        List,
        Paragraph,
    }
    let lines: Vec<&str> = markdown.lines().collect();
    let mut items = vec![];
    let mut heading: Option<String> = None;
    // (kind, start index, end index inclusive)
    let mut current: Option<(Kind, usize, usize)> = None;
    let mut in_fence = false;
    let mut in_comment = false;
    let mut saw_blank = false;

    let flush = |current: &mut Option<(Kind, usize, usize)>,
                 items: &mut Vec<GuidelineItem>,
                 heading: &Option<String>| {
        if let Some((_, start, end)) = current.take() {
            items.push(GuidelineItem {
                number: items.len() + 1,
                heading: heading.clone(),
                text: normalize(&lines[start..=end]),
                start_line: start + 1,
                end_line: end + 1,
            });
        }
    };

    for (i, line) in lines.iter().enumerate() {
        if in_fence {
            if is_fence(line) {
                in_fence = false;
            }
            continue;
        }
        if in_comment {
            if line.contains("-->") {
                in_comment = false;
            }
            continue;
        }
        if is_fence(line) {
            flush(&mut current, &mut items, &heading);
            in_fence = true;
            continue;
        }
        if line.trim_start().starts_with("<!--") {
            flush(&mut current, &mut items, &heading);
            in_comment = !line.contains("-->");
            continue;
        }
        if line.trim().is_empty() {
            saw_blank = true;
            continue;
        }
        if let Some(text) = heading_text(line) {
            flush(&mut current, &mut items, &heading);
            heading = Some(text);
            saw_blank = false;
            continue;
        }
        let indented = line.starts_with("  ") || line.starts_with('\t');
        if is_list_marker(line) {
            flush(&mut current, &mut items, &heading);
            current = Some((Kind::List, i, i));
        } else if let Some((kind, _, end)) = current.as_mut() {
            // Indented lines continue a list item, even after a blank line. A
            // paragraph continues only until a blank line.
            if (*kind == Kind::List && (indented || !saw_blank))
                || (*kind == Kind::Paragraph && !saw_blank)
            {
                *end = i;
            } else {
                flush(&mut current, &mut items, &heading);
                current = Some((Kind::Paragraph, i, i));
            }
        } else {
            current = Some((Kind::Paragraph, i, i));
        }
        saw_blank = false;
    }
    flush(&mut current, &mut items, &heading);
    items
}

/// Finds the item whose hash matches, for rules whose lines moved.
pub fn find_by_hash<'a>(items: &'a [GuidelineItem], sha256: &str) -> Option<&'a GuidelineItem> {
    items.iter().find(|item| hash(&item.text) == sha256)
}
