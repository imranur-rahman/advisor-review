# Initial analysis toolchain

The first implementation uses a Rust-native core with these boundaries:

- `walkdir` discovers LaTeX source and guideline files.
- The LaTeX analyzer preserves source file and line ranges and recognizes common environments without requiring a TeX installation.
- PNG and JPEG headers provide raster dimensions for deterministic figure checks.
- `lopdf` parses PDF catalogs and page trees without external executables. Rendered layout extraction and source-to-PDF mapping remain unavailable and are reported explicitly.
- `serde`, `serde_json`, and `serde_yaml` provide the versioned report and structured-rule contract.

This keeps deterministic checks portable and allows later optional integrations with Poppler, TeX engines, OCR, or image inspection tools without coupling the core model to one external executable.
