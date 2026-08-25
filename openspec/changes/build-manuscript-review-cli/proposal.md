## Why

Researchers often receive detailed, recurring feedback from advisors, journals, and collaborators, but applying that feedback consistently across a LaTeX manuscript is manual and difficult to audit. A reusable local-first CLI can turn those guidelines into repeatable checks over source LaTeX, rendered PDF content, figures, tables, equations, and code environments while preserving actionable locations and evidence.

## What Changes

- Add a command-line manuscript review workflow for LaTeX projects containing source files and a compiled PDF.
- Load and combine multiple advisor, journal, conference, and project guideline files from a directory.
- Support both natural-language guidelines and explicit structured rule definitions.
- Review prose, LaTeX structure, figures, tables, equations, citations, references, and code/listing environments.
- Support deterministic checks such as regex, LaTeX structure, PDF layout, image dimensions, effective DPI, and rendered text size.
- Support semantic checks through configurable OpenAI, Anthropic, OpenRouter, Ollama, and compatible providers.
- Emit anchored findings in stable JSON and human-readable Markdown formats.
- Identify findings by typed targets such as paragraphs, figures, tables, cells, equations, code blocks, source ranges, and PDF pages.
- Preserve rule provenance, severity, confidence, evidence, explanation, suggestions, and provider/model metadata.
- Allow deterministic checks to run without an API key and mark unavailable semantic checks as skipped or uncertain.
- Keep future UI integration possible through the JSON finding contract.
- Exclude web UI, automatic source rewriting, and collaborative guideline management from the initial scope.

## Capabilities

### New Capabilities

- `manuscript-review-cli`: Run configurable reviews over LaTeX projects and produce JSON/Markdown findings.
- `guideline-and-rule-packs`: Load, compose, validate, and track provenance for natural-language and structured rules from multiple guideline files.
- `document-analysis-and-anchors`: Parse manuscript content and assets into typed review targets with source and PDF location metadata.
- `review-provider-configuration`: Configure semantic-review providers, API credentials, model names, and capability-aware execution.

### Modified Capabilities

- None.

## Impact

- Introduces the initial CLI, document-analysis pipeline, rule engine, provider adapter boundary, and report generators.
- Requires LaTeX/PDF parsing and inspection dependencies plus optional image, OCR, and vision-language tooling.
- Adds a stable JSON finding schema that future editor or web interfaces can consume.
- Requires careful handling of manuscript privacy, API keys, provider failures, unsupported capabilities, and imperfect source-to-PDF mappings.
