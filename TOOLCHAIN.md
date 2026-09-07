# Analysis toolchain

The first implementation uses a Rust-native core with these boundaries:

- `walkdir` discovers LaTeX source and guideline files.
- The LaTeX analyzer expands includes into an ordered, source-mapped stream, then builds manuscript, nested section, paragraph, and sentence targets alongside specialized objects. Normalized prose and source views retain UTF-8 byte offsets and multi-file locations without requiring a TeX installation.
- Independent rule execution supports scope filters, section selectors, and explicit context. Semantic responses can contain multiple findings whose quoted evidence is validated against supplied text.
- Bounded semantic chunk review and synthesis preserve findings while marking incomplete global coverage explicitly; budgets are UTF-8 prompt bytes, not model-specific token estimates.
- PNG and JPEG headers provide raster dimensions for deterministic figure checks.
- `lopdf` parses PDF catalogs and page trees without external executables. Rendered layout extraction and source-to-PDF mapping remain unavailable and are reported explicitly.
- `serde`, `serde_json`, and `serde_yaml` provide the schema 2.0 report and structured-rule contract, including hierarchy, source mappings, and per-scope evaluation coverage.

This keeps deterministic checks portable and allows later optional integrations with Poppler, TeX engines, OCR, or image inspection tools without coupling the core model to one external executable.
