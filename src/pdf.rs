use anyhow::Result;
use std::path::Path;

#[derive(Debug, Clone)]
pub struct PdfInfo {
    pub page_count: usize,
    pub mapping_quality: String,
}

pub fn inspect(path: &Path) -> Result<PdfInfo> {
    let bytes = std::fs::read(path)?;
    let text = String::from_utf8_lossy(&bytes);
    let page_count = text
        .matches("/Type /Page")
        .count()
        .saturating_sub(text.matches("/Type /Pages").count());
    Ok(PdfInfo {
        page_count,
        mapping_quality: "unavailable".into(),
    })
}
