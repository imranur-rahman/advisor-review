use anyhow::{Context, Result, bail};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

#[derive(Debug, Clone)]
pub struct ProjectInputs {
    pub project: PathBuf,
    pub main_tex: PathBuf,
    pub pdf: PathBuf,
    pub guidelines: PathBuf,
}

pub fn discover(
    project: &Path,
    guidelines: &Path,
    main_tex: Option<&Path>,
    pdf: Option<&Path>,
) -> Result<ProjectInputs> {
    if !project.is_dir() {
        bail!("project directory does not exist: {}", project.display());
    }
    if !guidelines.is_dir() {
        bail!(
            "guideline directory does not exist: {}",
            guidelines.display()
        );
    }
    let project = project
        .canonicalize()
        .context("resolve project directory")?;
    let explicit_main = main_tex.is_some();
    let mut main_tex = project.join(main_tex.unwrap_or_else(|| Path::new("main.tex")));
    if !main_tex.is_file() {
        if explicit_main {
            bail!(
                "explicit main LaTeX source not found: {}",
                main_tex.display()
            );
        }
        let mut candidates = Vec::new();
        for entry in WalkDir::new(&project).sort_by_file_name() {
            let entry = entry.context("search project for LaTeX source")?;
            if entry.file_type().is_file()
                && entry.path().extension().and_then(|e| e.to_str()) == Some("tex")
            {
                candidates.push(entry.into_path());
            }
        }
        match candidates.as_slice() {
            [path] => main_tex = path.clone(),
            [] => bail!(
                "main LaTeX source not found; expected {}",
                main_tex.display()
            ),
            _ => bail!("multiple LaTeX sources found; select the entry point with --main-tex"),
        }
    }
    let pdf = project.join(pdf.unwrap_or_else(|| Path::new("main.pdf")));
    if !pdf.is_file() {
        bail!("compiled PDF not found; expected {}", pdf.display());
    }
    Ok(ProjectInputs {
        project: project.to_path_buf(),
        main_tex,
        pdf,
        guidelines: guidelines.to_path_buf(),
    })
}

pub fn read_text(path: &Path) -> Result<String> {
    std::fs::read_to_string(path).with_context(|| format!("read {}", path.display()))
}
