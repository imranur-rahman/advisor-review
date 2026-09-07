use lopdf::{Document, Object, Stream, dictionary};
use std::{fs, path::Path, process::Command};

pub fn write_pdf(path: &Path) {
    let mut doc = Document::with_version("1.5");
    let pages_id = doc.new_object_id();
    let font_id = doc.add_object(
        dictionary! {"Type" => "Font", "Subtype" => "Type1", "BaseFont" => "Helvetica"},
    );
    let content_id = doc.add_object(Stream::new(
        dictionary! {},
        b"BT /F1 12 Tf 50 750 Td (Introduction) Tj ET".to_vec(),
    ));
    let page_id = doc.add_object(dictionary! {
        "Type" => "Page", "Parent" => pages_id,
        "MediaBox" => vec![0.into(), 0.into(), 612.into(), 792.into()],
        "Resources" => dictionary! {"Font" => dictionary! {"F1" => font_id}},
        "Contents" => content_id
    });
    doc.objects.insert(
        pages_id,
        Object::Dictionary(dictionary! {
            "Type" => "Pages", "Kids" => vec![page_id.into()], "Count" => 1
        }),
    );
    let catalog = doc.add_object(dictionary! {"Type" => "Catalog", "Pages" => pages_id});
    doc.trailer.set("Root", catalog);
    doc.compress();
    doc.save(path).unwrap();
}

#[allow(dead_code)]
pub fn fixture(root: &Path) {
    fs::create_dir_all(root.join("paper/sections")).unwrap();
    fs::create_dir_all(root.join("guidelines")).unwrap();
    fs::write(
        root.join("paper/main.tex"),
        include_str!("../fixtures/main.tex"),
    )
    .unwrap();
    fs::write(
        root.join("paper/sections/body.tex"),
        include_str!("../fixtures/body.tex"),
    )
    .unwrap();
    fs::write(
        root.join("guidelines/rules.yaml"),
        include_str!("../fixtures/rules.yaml"),
    )
    .unwrap();
    write_pdf(&root.join("paper/main.pdf"));
}

#[allow(dead_code)]
pub fn cli(root: &Path) -> Command {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_advisor-review"));
    cmd.current_dir(root).args([
        "review",
        "--project",
        "./paper",
        "--guidelines",
        "./guidelines",
        "--output",
        "./review",
    ]);
    for key in [
        "ADVISOR_REVIEW_PROVIDER",
        "ADVISOR_REVIEW_MODEL",
        "ADVISOR_REVIEW_ENDPOINT",
        "ADVISOR_REVIEW_API_KEY",
        "OPENAI_API_KEY",
        "ANTHROPIC_API_KEY",
        "OPENROUTER_API_KEY",
    ] {
        cmd.env_remove(key);
    }
    cmd
}
