use advisor_review::{latex, model::DocumentTarget};
use std::fs;

fn parse(text: &str) -> Vec<DocumentTarget> {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("main.tex"), text).unwrap();
    latex::parse_project(&dir.path().join("main.tex"), dir.path()).unwrap()
}

#[test]
fn sections_contain_body_and_children_with_separate_headings() {
    let targets = parse(
        "\\section{Methods}\nFirst procedure. Another sentence.\n\n\\subsection{Measurements}\nMeasured 3.14 units.\n\\section{Results}\nFinal result.",
    );
    let methods = targets
        .iter()
        .find(|t| t.target_type == "section" && t.title.as_deref() == Some("Methods"))
        .unwrap();
    assert!(methods.text.contains("Measured 3.14 units."));
    assert!(!methods.text.contains("Final result"));
    assert!(
        !methods
            .direct_prose
            .as_ref()
            .unwrap()
            .text
            .contains("Measured")
    );
    let measurements = targets
        .iter()
        .find(|t| t.title.as_deref() == Some("Measurements") && t.target_type == "section")
        .unwrap();
    assert_eq!(measurements.parent_id.as_ref(), Some(&methods.id));
    assert_eq!(
        targets
            .iter()
            .find(|t| t.target_type == "heading")
            .unwrap()
            .text,
        "Methods"
    );
    assert_eq!(
        targets
            .iter()
            .filter(|t| t.target_type == "sentence")
            .count(),
        4
    );
}

#[test]
fn cross_file_paragraph_and_section_keep_reading_order_and_sources() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("main.tex"), "\\begin{document}\n\\section{Methods}\nThe experiment\n\\input{body}\nends here.\n\\end{document}").unwrap();
    fs::write(
        dir.path().join("body.tex"),
        "uses calibrated instruments and\n",
    )
    .unwrap();
    let targets = latex::parse_project(&dir.path().join("main.tex"), dir.path()).unwrap();
    let paragraph = targets
        .iter()
        .find(|t| t.target_type == "paragraph")
        .unwrap();
    assert_eq!(
        paragraph.text,
        "The experiment uses calibrated instruments and ends here."
    );
    assert!(paragraph.anchor.source.is_none());
    assert_eq!(
        paragraph
            .anchor
            .sources
            .iter()
            .map(|s| s.file.as_str())
            .collect::<Vec<_>>(),
        vec!["main.tex", "body.tex", "main.tex"]
    );
    assert_eq!(
        targets
            .iter()
            .filter(|t| t.target_type == "sentence")
            .count(),
        1
    );
}

#[test]
fn scientific_sentences_preserve_math_citations_and_exact_offsets() {
    let text = "Dr. Smith measured 3.14 units, e.g. with $x = 1.2$. This follows \\cite{Smith.2020}. Unicode café works!";
    let targets = parse(text);
    let sentences: Vec<_> = targets
        .iter()
        .filter(|t| t.target_type == "sentence")
        .collect();
    assert_eq!(
        sentences.len(),
        3,
        "{:?}",
        sentences.iter().map(|t| &t.text).collect::<Vec<_>>()
    );
    assert!(sentences[0].text.starts_with("Dr. Smith"));
    assert_eq!(sentences[1].text, "This follows [Smith.2020].");
    assert_eq!(
        sentences[1].anchor.source.as_ref().unwrap().start_byte,
        Some(text.find("This follows").unwrap())
    );
    assert!(
        sentences[2].anchor.source.as_ref().unwrap().start_column
            > sentences[1].anchor.source.as_ref().unwrap().start_column
    );
}

#[test]
fn normalized_prose_retains_source_view_and_mapping() {
    let targets = parse(
        "\\section[Short]{A \\emph{nested} title}\n\\textbf{Very clear} prose.\\par Next paragraph.",
    );
    let paragraphs: Vec<_> = targets
        .iter()
        .filter(|t| t.target_type == "paragraph")
        .collect();
    assert_eq!(paragraphs.len(), 2);
    assert_eq!(paragraphs[0].text, "Very clear prose.");
    assert!(paragraphs[0].raw_text.contains("\\textbf"));
    assert_eq!(
        targets
            .iter()
            .find(|t| t.target_type == "heading")
            .unwrap()
            .text,
        "A nested title"
    );
    let serialized = serde_json::to_string(&targets).unwrap();
    let restored: Vec<DocumentTarget> = serde_json::from_str(&serialized).unwrap();
    assert_eq!(restored.len(), targets.len());
}

#[test]
fn repeated_includes_are_preserved_but_cycles_are_rejected() {
    let dir = tempfile::tempdir().unwrap();
    let main = dir.path().join("main.tex");
    let body = dir.path().join("body.tex");
    fs::write(&main, "\\input{body}\n\n\\input{body}\n").unwrap();
    fs::write(&body, "Repeated sentence.\n").unwrap();
    let targets = latex::parse_project(&main, dir.path()).unwrap();
    let sentences: Vec<_> = targets
        .iter()
        .filter(|t| t.target_type == "sentence")
        .collect();
    assert_eq!(sentences.len(), 2);
    assert_ne!(sentences[0].id, sentences[1].id);
    assert_eq!(sentences[0].text, sentences[1].text);
    assert_eq!(sentences[0].anchor.sources, sentences[1].anchor.sources);
    fs::write(&body, "\\input{main}\n").unwrap();
    let error = latex::parse_project(&main, dir.path()).unwrap_err();
    assert!(format!("{error:#}").contains("cyclic include"));
}
