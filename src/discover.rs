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
    let main_tex = main_tex
        .map(PathBuf::from)
        .unwrap_or_else(|| project.join("main.tex"));
    let main_tex = if main_tex.is_absolute() {
        main_tex
    } else {
        project.join(main_tex)
    };
    if !main_tex.is_file() {
        let candidate = WalkDir::new(project)
            .into_iter()
            .filter_map(|e| e.ok())
            .map(|e| e.path().to_path_buf())
            .find(|p| p.extension().and_then(|e| e.to_str()) == Some("tex"));
        match candidate {
            Some(path) => return discover(project, guidelines, Some(&path), pdf),
            None => bail!(
                "main LaTeX source not found; expected {}",
                main_tex.display()
            ),
        }
    }
    let pdf = pdf
        .map(PathBuf::from)
        .unwrap_or_else(|| project.join("main.pdf"));
    let pdf = if pdf.is_absolute() {
        pdf
    } else {
        project.join(pdf)
    };
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
