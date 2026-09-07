use anyhow::{Context, Result, ensure};
use std::path::Path;

#[derive(Debug, Clone)]
pub struct PdfInfo {
    pub page_count: usize,
    pub mapping_quality: String,
}

pub fn inspect(path: &Path) -> Result<PdfInfo> {
    let document =
        lopdf::Document::load(path).with_context(|| format!("parse PDF {}", path.display()))?;
    ensure!(!document.is_encrypted(), "encrypted PDFs are not supported");
    document
        .catalog()
        .context("PDF catalog is missing or invalid")?;
    let pages = document.get_pages();
    ensure!(!pages.is_empty(), "PDF has no readable pages");
    for id in pages.values() {
        let page = document.get_dictionary(*id).context("invalid PDF page")?;
        ensure!(
            page.get(b"Type").and_then(lopdf::Object::as_name)? == b"Page",
            "invalid PDF page type"
        );
    }
    let page_count = pages.len();
    Ok(PdfInfo {
        page_count,
        mapping_quality: "unavailable".into(),
    })
}
