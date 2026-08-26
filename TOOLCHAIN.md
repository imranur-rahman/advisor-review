# Initial analysis toolchain

The first implementation uses a Rust-native core with these boundaries:

- `walkdir` discovers LaTeX source and guideline files.
- The LaTeX analyzer preserves source file and line ranges and recognizes common environments without requiring a TeX installation.
- PNG and JPEG headers provide raster dimensions for deterministic figure checks.
- PDF input is validated as a project artifact; rendered layout extraction remains an explicit extension point because reliable source-to-PDF mapping depends on the PDF toolchain available on the host.
- `serde`, `serde_json`, and `serde_yaml` provide the versioned report and structured-rule contract.

This keeps deterministic checks portable and allows later optional integrations with Poppler, TeX engines, OCR, or image inspection tools without coupling the core model to one external executable.
