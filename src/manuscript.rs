//! Source expansion, prose normalization, and the shared review hierarchy.
use crate::model::{DocumentTarget, MappedText, SourceSpan, TargetAnchor, TextMapping};
use anyhow::{Context, Result, ensure};
use std::{
    collections::BTreeMap,
    ops::Range,
    path::{Path, PathBuf},
};

impl MappedText {
    pub fn append(&mut self, other: &Self) {
        let offset = self.text.len();
        self.text.push_str(&other.text);
        for mut map in other.mappings.iter().cloned() {
            map.start += offset;
            map.end += offset;
            if let Some(last) = self.mappings.last_mut() {
                let linear = |m: &TextMapping| {
                    m.source.start_line == m.source.end_line
                        && m.source
                            .end_byte
                            .unwrap_or(0)
                            .saturating_sub(m.source.start_byte.unwrap_or(0))
                            == m.end - m.start
                };
                if linear(last)
                    && linear(&map)
                    && last.end == map.start
                    && last.source.file == map.source.file
                    && last.source.end_byte == map.source.start_byte
                    && last.source.end_line == map.source.start_line
                {
                    last.end = map.end;
                    last.source.end_byte = map.source.end_byte;
                    last.source.end_column = map.source.end_column;
                    continue;
                }
            }
            self.mappings.push(map);
        }
    }

    pub fn slice(&self, range: Range<usize>) -> Self {
        let mut result = Self {
            text: self.text[range.clone()].into(),
            mappings: vec![],
        };
        let first = self.mappings.partition_point(|map| map.end <= range.start);
        for map in self.mappings[first..]
            .iter()
            .take_while(|map| map.start < range.end)
        {
            let start = map.start.max(range.start);
            let end = map.end.min(range.end);
            if start >= end {
                continue;
            }
            let mut source = map.source.clone();
            if source.start_line == source.end_line
                && source
                    .end_byte
                    .unwrap_or(0)
                    .saturating_sub(source.start_byte.unwrap_or(0))
                    == map.end - map.start
            {
                source.start_byte = source.start_byte.map(|v| v + start - map.start);
                source.end_byte = source.start_byte.map(|v| v + end - start);
                source.start_column = source.start_column.map(|v| v + start - map.start);
                source.end_column = source.start_column.map(|v| v + end - start);
            }
            result.mappings.push(TextMapping {
                start: start - range.start,
                end: end - range.start,
                source,
            });
        }
        result
    }

    pub fn spans(&self) -> Vec<SourceSpan> {
        let mut spans: Vec<SourceSpan> = vec![];
        for mapping in &self.mappings {
            if let Some(last) = spans.last_mut() {
                if last.file == mapping.source.file && last.end_byte == mapping.source.start_byte {
                    last.end_byte = mapping.source.end_byte;
                    last.end_line = mapping.source.end_line;
                    last.end_column = mapping.source.end_column;
                    continue;
                }
            }
            if spans.last() != Some(&mapping.source) {
                spans.push(mapping.source.clone());
            }
        }
        spans
    }

    fn trim(&self) -> Self {
        let start = self.text.len() - self.text.trim_start().len();
        let end = self.text.trim_end().len().max(start);
        self.slice(start..end)
    }
}

pub struct SourceDocument {
    pub content: MappedText,
    pub line_starts: Vec<usize>,
}

impl SourceDocument {
    pub fn load(main: &Path, project: &Path) -> Result<Self> {
        let mut content = MappedText::default();
        expand(main, project, &mut vec![], &mut content)?;
        let mut line_starts = vec![0];
        line_starts.extend(content.text.match_indices('\n').map(|(i, _)| i + 1));
        Ok(Self {
            content,
            line_starts,
        })
    }

    pub fn line_range(&self, start: usize, end: usize) -> Range<usize> {
        self.line_starts
            .get(start.saturating_sub(1))
            .copied()
            .unwrap_or(0)
            ..self
                .line_starts
                .get(end)
                .copied()
                .unwrap_or(self.content.text.len())
    }
}

fn expand(
    path: &Path,
    project: &Path,
    active: &mut Vec<PathBuf>,
    out: &mut MappedText,
) -> Result<()> {
    let canonical = path
        .canonicalize()
        .with_context(|| format!("read included source {}", path.display()))?;
    ensure!(
        !active.contains(&canonical),
        "cyclic include: {}",
        path.display()
    );
    ensure!(active.len() < 128, "include nesting exceeds 128 files");
    active.push(canonical);
    let raw = std::fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
    let file = path
        .strip_prefix(project)
        .unwrap_or(path)
        .display()
        .to_string();
    let mut original = MappedText::default();
    let (mut line, mut column) = (1, 1);
    for (offset, ch) in raw.char_indices() {
        let end_column = if ch == '\n' {
            1
        } else {
            column + ch.len_utf8()
        };
        let end_line = line + usize::from(ch == '\n');
        original.text.push(ch);
        original.mappings.push(TextMapping {
            start: offset,
            end: offset + ch.len_utf8(),
            source: SourceSpan {
                file: file.clone(),
                start_line: line,
                end_line,
                start_byte: Some(offset),
                end_byte: Some(offset + ch.len_utf8()),
                start_column: Some(column),
                end_column: Some(end_column),
            },
        });
        line = end_line;
        column = end_column;
    }
    let mut i = 0;
    while i < raw.len() {
        if raw[i..].starts_with('%') {
            i = raw[i..].find('\n').map(|n| i + n + 1).unwrap_or(raw.len());
            continue;
        }
        if raw[i..].starts_with('\\') {
            let (name, after) = command(&raw, i);
            if name == "begin" {
                if let Some((arg, end)) = group(&raw, after, '{', '}') {
                    if is_literal(&raw[arg]) {
                        let marker =
                            format!("\\end{{{}}}", &raw[group(&raw, after, '{', '}').unwrap().0]);
                        let stop = raw[end..]
                            .find(&marker)
                            .map(|n| end + n + marker.len())
                            .context("unclosed literal environment")?;
                        out.append(&original.slice(i..stop));
                        i = stop;
                        continue;
                    }
                }
            }
            if matches!(name.as_str(), "input" | "include") {
                let (arg, end) =
                    group(&raw, after, '{', '}').context("input/include requires a braced path")?;
                let mut child = project.join(raw[arg].trim());
                if child.extension().is_none() {
                    child.set_extension("tex");
                }
                expand(&child, project, active, out)
                    .with_context(|| format!("include in {} at byte {i}", path.display()))?;
                let line_start = raw[..i].rfind('\n').map(|n| n + 1).unwrap_or(0);
                let line_end = raw[end..].find('\n').map(|n| end + n).unwrap_or(raw.len());
                i = if raw[line_start..i].trim().is_empty() && raw[end..line_end].trim().is_empty()
                {
                    (line_end + 1).min(raw.len())
                } else {
                    end
                };
                continue;
            }
            // Consume escaped percent/backslash as a unit, so it isn't a comment.
            out.append(&original.slice(i..after));
            i = after;
            continue;
        }
        let end = i + raw[i..].chars().next().unwrap().len_utf8();
        out.append(&original.slice(i..end));
        i = end;
    }
    active.pop();
    Ok(())
}

fn command(text: &str, start: usize) -> (String, usize) {
    let mut end = start + 1;
    while end < text.len() && text.as_bytes()[end].is_ascii_alphabetic() {
        end += 1;
    }
    if end == start + 1 && end < text.len() {
        end += text[end..].chars().next().unwrap().len_utf8();
    }
    (text[start + 1..end].into(), end)
}

fn group(text: &str, start: usize, open: char, close: char) -> Option<(Range<usize>, usize)> {
    let start = start + text[start..].len() - text[start..].trim_start().len();
    if !text[start..].starts_with(open) {
        return None;
    }
    let mut depth = 1;
    let mut i = start + open.len_utf8();
    while i < text.len() {
        let ch = text[i..].chars().next()?;
        if ch == '\\' {
            i = command(text, i).1;
            continue;
        }
        if ch == open {
            depth += 1;
        }
        if ch == close {
            depth -= 1;
        }
        if depth == 0 {
            return Some((start + open.len_utf8()..i, i + close.len_utf8()));
        }
        i += ch.len_utf8();
    }
    None
}

fn is_literal(name: &str) -> bool {
    matches!(name, "lstlisting" | "verbatim" | "verbatim*" | "minted")
}
fn is_special(name: &str) -> bool {
    is_literal(name)
        || matches!(
            name,
            "figure"
                | "figure*"
                | "table"
                | "table*"
                | "tabular"
                | "tabular*"
                | "equation"
                | "equation*"
                | "align"
                | "align*"
                | "displaymath"
        )
}

#[derive(Clone)]
struct Block {
    kind: &'static str,
    start: usize,
    end: usize,
    depth: usize,
    prose: MappedText,
}

fn normalized(source: &MappedText, range: Range<usize>) -> MappedText {
    let text = &source.text;
    let mut out = MappedText::default();
    let mut i = range.start;
    while i < range.end {
        let ch = text[i..].chars().next().unwrap();
        if ch == '\\' {
            let (name, mut end) = command(text, i);
            if matches!(
                name.as_str(),
                "label" | "index" | "bibliography" | "bibliographystyle"
            ) {
                if let Some((_, stop)) = group(text, end, '{', '}') {
                    end = stop;
                }
            } else if matches!(name.as_str(), "%" | "&" | "_" | "#" | "$" | "{" | "}") {
                out.append(&source.slice(i + 1..end));
            } else if matches!(
                name.as_str(),
                "cite" | "citep" | "citet" | "ref" | "autoref" | "cref"
            ) {
                while let Some((_, stop)) = group(text, end, '[', ']') {
                    end = stop;
                }
                if let Some((arg, stop)) = group(text, end, '{', '}') {
                    out.text.push('[');
                    out.append(&source.slice(arg));
                    out.text.push(']');
                    end = stop;
                }
            } else if matches!(name.as_str(), "(" | "[") {
                let close = if name == "(" { "\\)" } else { "\\]" };
                if let Some(n) = text[end..range.end].find(close) {
                    end += n + close.len();
                    out.append(&source.slice(i..end));
                }
            } else if name == "href" {
                if let Some((_, stop)) = group(text, end, '{', '}') {
                    end = stop;
                }
            } else if !matches!(
                name.as_str(),
                "textbf"
                    | "textit"
                    | "emph"
                    | "textrm"
                    | "textsf"
                    | "texttt"
                    | "text"
                    | "underline"
                    | "url"
                    | "footnote"
                    | "item"
                    | "noindent"
                    | "protect"
                    | "small"
                    | "large"
                    | "LaTeX"
                    | "TeX"
            ) {
                // Unknown commands remain visible; do not pretend to expand TeX.
                out.append(&source.slice(i..end));
            } else if matches!(name.as_str(), "LaTeX" | "TeX") {
                out.append(&source.slice(i + 1..end));
            }
            i = end.min(range.end);
            continue;
        }
        if ch == '$' {
            let delimiter = if text[i..].starts_with("$$") {
                "$$"
            } else {
                "$"
            };
            let next = i + delimiter.len();
            if let Some(n) = text[next..range.end].find(delimiter) {
                let end = next + n + delimiter.len();
                out.append(&source.slice(i..end));
                i = end;
                continue;
            }
        }
        let end = i + ch.len_utf8();
        if ch.is_whitespace() || ch == '~' {
            if !out.text.is_empty() && !out.text.ends_with(' ') {
                let mut space = source.slice(i..end);
                space.text = " ".into();
                for map in &mut space.mappings {
                    map.start = 0;
                    map.end = 1;
                }
                out.append(&space);
            }
        } else if !matches!(ch, '{' | '}') {
            out.append(&source.slice(i..end));
        }
        i = end;
    }
    out.trim()
}

fn blocks(source: &MappedText) -> Result<Vec<Block>> {
    let text = &source.text;
    let body_start = text
        .find("\\begin{document}")
        .map(|n| n + "\\begin{document}".len())
        .unwrap_or(0);
    let body_end = text.rfind("\\end{document}").unwrap_or(text.len());
    let mut blocks = vec![];
    let mut start = body_start;
    let mut i = body_start;
    let flush = |start: usize, end: usize, blocks: &mut Vec<Block>| {
        let prose = normalized(source, start..end);
        if !prose.text.is_empty() {
            blocks.push(Block {
                kind: "paragraph",
                start,
                end,
                depth: 0,
                prose,
            });
        }
    };
    while i < body_end {
        if text[i..].starts_with('\\') {
            let (name, mut end) = command(text, i);
            if matches!(
                name.as_str(),
                "chapter" | "section" | "subsection" | "subsubsection"
            ) {
                flush(start, i, &mut blocks);
                if text[end..].starts_with('*') {
                    end += 1;
                }
                if let Some((_, stop)) = group(text, end, '[', ']') {
                    end = stop;
                }
                let (arg, stop) = group(text, end, '{', '}').context("unclosed section heading")?;
                let depth = match name.as_str() {
                    "chapter" => 0,
                    "section" => 1,
                    "subsection" => 2,
                    _ => 3,
                };
                blocks.push(Block {
                    kind: "heading",
                    start: i,
                    end: stop,
                    depth,
                    prose: normalized(source, arg),
                });
                i = stop;
                start = i;
                continue;
            }
            if matches!(
                name.as_str(),
                "par" | "begin" | "end" | "maketitle" | "bibliography" | "bibliographystyle"
            ) {
                flush(start, i, &mut blocks);
                if let Some((arg, stop)) = group(text, end, '{', '}') {
                    end = stop;
                    if name == "begin" && is_special(&text[arg.clone()]) {
                        let marker = format!("\\end{{{}}}", &text[arg]);
                        end = text[end..]
                            .find(&marker)
                            .map(|n| end + n + marker.len())
                            .context("unclosed specialized environment")?;
                    }
                }
                i = end;
                start = i;
                continue;
            }
            if matches!(name.as_str(), "(" | "[") {
                let close = if name == "(" { "\\)" } else { "\\]" };
                if let Some(n) = text[end..].find(close) {
                    i = end + n + close.len();
                    continue;
                }
            }
            i = end;
            continue;
        }
        if text[i..].starts_with('$') {
            let delimiter = if text[i..].starts_with("$$") {
                "$$"
            } else {
                "$"
            };
            let next = i + delimiter.len();
            if let Some(n) = text[next..].find(delimiter) {
                i = next + n + delimiter.len();
                continue;
            }
        }
        if text[i..].starts_with('\n') {
            let whitespace_end = i + text[i..].len() - text[i..].trim_start().len();
            if text[i..whitespace_end]
                .bytes()
                .filter(|b| *b == b'\n')
                .count()
                >= 2
            {
                flush(start, i, &mut blocks);
                i = whitespace_end;
                start = i;
                continue;
            }
        }
        i += text[i..].chars().next().unwrap().len_utf8();
    }
    flush(start, body_end, &mut blocks);
    Ok(blocks)
}

fn sentence_ranges(text: &str) -> Vec<Range<usize>> {
    let mut result = vec![];
    let mut start = 0;
    let mut i = 0;
    let mut brackets: usize = 0;
    while i < text.len() {
        if text[i..].starts_with('$') || text[i..].starts_with("\\(") {
            let delimiter = if text[i..].starts_with("\\(") {
                "\\("
            } else if text[i..].starts_with("$$") {
                "$$"
            } else {
                "$"
            };
            let close = if delimiter == "\\(" { "\\)" } else { delimiter };
            let next = i + delimiter.len();
            if let Some(n) = text[next..].find(close) {
                i = next + n + close.len();
                continue;
            }
        }
        let ch = text[i..].chars().next().unwrap();
        if ch == '[' {
            brackets += 1;
        }
        if ch == ']' {
            brackets = brackets.saturating_sub(1);
        }
        if matches!(ch, '.' | '?' | '!') && brackets == 0 {
            let prefix = &text[..i];
            let word = prefix
                .split_whitespace()
                .next_back()
                .unwrap_or("")
                .trim_matches(|c: char| !c.is_alphanumeric() && c != '.')
                .to_lowercase();
            let abbreviation = ch == '.'
                && (matches!(
                    word.as_str(),
                    "e.g"
                        | "i.e"
                        | "dr"
                        | "mr"
                        | "mrs"
                        | "ms"
                        | "prof"
                        | "fig"
                        | "figs"
                        | "eq"
                        | "eqs"
                        | "sec"
                        | "secs"
                        | "vs"
                        | "al"
                        | "no"
                        | "vol"
                        | "pp"
                ) || (word.chars().count() == 1 && word.chars().all(char::is_alphabetic)));
            let mut end = i + ch.len_utf8();
            while end < text.len()
                && matches!(
                    text[end..].chars().next().unwrap(),
                    '.' | '?' | '!' | '"' | '\'' | '”' | '’' | ')'
                )
            {
                end += text[end..].chars().next().unwrap().len_utf8();
            }
            if !abbreviation && (end == text.len() || text[end..].starts_with(char::is_whitespace))
            {
                result.push(start..end);
                start = end;
                while start < text.len() && text[start..].starts_with(char::is_whitespace) {
                    start += text[start..].chars().next().unwrap().len_utf8();
                }
                i = start;
                continue;
            }
        }
        i += ch.len_utf8();
    }
    if start < text.len() {
        result.push(start..text.len());
    }
    result
}

fn target(
    kind: &str,
    id: String,
    parent: Option<String>,
    order: usize,
    prose: MappedText,
    raw: MappedText,
) -> DocumentTarget {
    let sources = raw.spans();
    // Compatibility anchor is a line range only when all contributing spans
    // belong to one file; precise disjoint spans are always retained.
    let source = if kind != "document" && sources.len() == 1 {
        let mut span = sources[0].clone();
        if let Some(last) = sources.last() {
            span.end_byte = last.end_byte;
            span.end_line = last.end_line;
            span.end_column = last.end_column;
        }
        Some(span)
    } else {
        None
    };
    DocumentTarget {
        id,
        target_type: kind.into(),
        parent_id: parent,
        order,
        text: prose.text,
        text_mappings: prose.mappings,
        raw_text: raw.text,
        raw_mappings: raw.mappings,
        anchor: TargetAnchor {
            source,
            sources,
            ..Default::default()
        },
        ..Default::default()
    }
}

pub fn build(
    source: &SourceDocument,
    specialized: Vec<DocumentTarget>,
) -> Result<Vec<DocumentTarget>> {
    let blocks = blocks(&source.content)?;
    let headings: Vec<_> = blocks.iter().filter(|b| b.kind == "heading").collect();
    let mut result = vec![];
    let mut sections: Vec<(Range<usize>, usize, String)> = vec![];
    let mut document_prose = MappedText::default();
    for block in &blocks {
        if !document_prose.text.is_empty() {
            document_prose.text.push_str("\n\n");
        }
        document_prose.append(&block.prose);
    }
    result.push(target(
        "document",
        "document".into(),
        None,
        0,
        document_prose,
        source.content.clone(),
    ));
    for (index, heading) in headings.iter().enumerate() {
        let end = headings[index + 1..]
            .iter()
            .find(|h| h.depth <= heading.depth)
            .map(|h| h.start)
            .unwrap_or(source.content.text.len());
        let direct_end = headings
            .get(index + 1)
            .map(|h| h.start)
            .unwrap_or(end)
            .min(end);
        let id = format!("section-{}", index + 1);
        let parent = sections
            .iter()
            .rev()
            .find(|(range, depth, _)| range.contains(&heading.start) && *depth < heading.depth)
            .map(|(_, _, id)| id.clone())
            .unwrap_or("document".into());
        let compose = |stop| {
            let mut text = MappedText::default();
            for block in blocks
                .iter()
                .filter(|b| heading.start <= b.start && b.start < stop)
            {
                if !text.text.is_empty() {
                    text.text.push_str("\n\n");
                }
                text.append(&block.prose);
            }
            text
        };
        let mut section = target(
            "section",
            id.clone(),
            Some(parent),
            heading.start,
            compose(end),
            source.content.slice(heading.start..end),
        );
        section.title = Some(heading.prose.text.clone());
        section.depth = Some(heading.depth);
        section.direct_prose = Some(compose(direct_end));
        section.direct_source = Some(source.content.slice(heading.start..direct_end));
        result.push(section);
        let mut title = target(
            "heading",
            format!("heading-{}", index + 1),
            Some(id.clone()),
            heading.start,
            heading.prose.clone(),
            source.content.slice(heading.start..heading.end),
        );
        title.title = Some(heading.prose.text.clone());
        title.depth = Some(heading.depth);
        result.push(title);
        sections.push((heading.start..end, heading.depth, id));
    }
    let parent_at = |offset| {
        sections
            .iter()
            .rev()
            .find(|(range, _, _)| range.contains(&offset))
            .map(|(_, _, id)| id.clone())
            .unwrap_or("document".into())
    };
    for (index, block) in blocks.iter().filter(|b| b.kind == "paragraph").enumerate() {
        let id = format!("paragraph-{}", index + 1);
        // Trim raw whitespace without losing offsets into contributing files.
        let raw = source.content.slice(block.start..block.end).trim();
        result.push(target(
            "paragraph",
            id.clone(),
            Some(parent_at(block.start)),
            block.start,
            block.prose.clone(),
            raw,
        ));
        for (sentence, range) in sentence_ranges(&block.prose.text).into_iter().enumerate() {
            let prose = block.prose.slice(range);
            // Sentence raw view is the original source slices behind its prose.
            let spans = prose.spans();
            let raw = raw_for_spans(&source.content.slice(block.start..block.end), &spans);
            result.push(target(
                "sentence",
                format!("{id}:sentence-{}", sentence + 1),
                Some(id.clone()),
                block.start + sentence + 1,
                prose,
                raw,
            ));
        }
    }
    for mut object in specialized {
        let Some(span) = object.anchor.source.as_ref() else {
            continue;
        };
        let range = source.line_range(span.start_line, span.end_line);
        let raw = source.content.slice(range.clone()).trim();
        object.id = format!("{}:{}", object.target_type, object.id);
        object.parent_id = Some(parent_at(range.start));
        object.order = range.start;
        object.raw_text = raw.text.clone();
        object.raw_mappings = raw.mappings.clone();
        object.anchor.sources = raw.spans();
        let remapped = target(
            &object.target_type,
            object.id.clone(),
            object.parent_id.clone(),
            object.order,
            normalized(&source.content, range),
            raw,
        );
        object.anchor.source = remapped.anchor.source;
        // Preserve specialized code/table text; map it only if it is a literal
        // slice of the raw view. Never invent mappings for derived cell text.
        if let Some(start) = object.raw_text.find(&object.text) {
            object.text_mappings = MappedText {
                text: object.raw_text.clone(),
                mappings: object.raw_mappings.clone(),
            }
            .slice(start..start + object.text.len())
            .mappings;
        }
        result.push(object);
    }
    result.sort_by_key(|t| {
        (
            t.order,
            match t.target_type.as_str() {
                "document" => 0,
                "section" => 1,
                "heading" => 2,
                "paragraph" => 3,
                "sentence" => 4,
                _ => 5,
            },
        )
    });
    for (order, target) in result.iter_mut().enumerate() {
        target.order = order;
    }
    Ok(result)
}

fn raw_for_spans(source: &MappedText, spans: &[SourceSpan]) -> MappedText {
    let Some(first) = spans.first() else {
        return MappedText::default();
    };
    let last = spans.last().unwrap();
    let start = source
        .mappings
        .iter()
        .find(|m| {
            m.source.file == first.file
                && m.source.start_byte <= first.start_byte
                && m.source.end_byte > first.start_byte
        })
        .map(|m| m.start + first.start_byte.unwrap() - m.source.start_byte.unwrap());
    let end = source
        .mappings
        .iter()
        .rev()
        .find(|m| {
            m.source.file == last.file
                && m.source.start_byte < last.end_byte
                && m.source.end_byte >= last.end_byte
        })
        .map(|m| m.end - (m.source.end_byte.unwrap() - last.end_byte.unwrap()));
    match (start, end) {
        (Some(start), Some(end)) if start <= end => source.slice(start..end),
        _ => MappedText::default(),
    }
}

pub fn mapped_view(target: &DocumentTarget, source: bool) -> MappedText {
    if source {
        MappedText {
            text: target.raw_text.clone(),
            mappings: target.raw_mappings.clone(),
        }
    } else {
        MappedText {
            text: target.text.clone(),
            mappings: target.text_mappings.clone(),
        }
    }
}

pub fn section_title(target: &DocumentTarget, targets: &[DocumentTarget]) -> Option<String> {
    let index: BTreeMap<_, _> = targets.iter().map(|t| (t.id.as_str(), t)).collect();
    let mut node = target;
    for _ in 0..targets.len() + 1 {
        if node.target_type == "section" {
            return node.title.clone();
        }
        node = *index.get(node.parent_id.as_deref()?)?;
    }
    None
}
