## Why

Sentence rules are skipped, section rules see only headings, and paragraph/document reviews lack hierarchy, context, and measurable coverage. Researchers need independent checks at all four levels with traceable evidence.

## What Changes

- Extract an ordered manuscript hierarchy across included files, normalized prose, raw source, and precise source mappings.
- Implement sentence and full-section targets, retain heading-only targets, and support manuscript as an alias for document.
- Add scope filtering, section selectors, text views, context policies, and subsection control.
- Support contextual semantic requests, multiple evidence-anchored findings, bounded chunk review with synthesis, and explicit incomplete coverage.
- Report hierarchy and evaluation coverage by level in JSON and Markdown.
- **BREAKING**: report schema 2.0 and full-section semantics; document migration for heading-only and raw-source checks.

## Capabilities

### New Capabilities

- `hierarchical-review`: Extraction, independent execution, contextual semantic review, evidence validation, and coverage for manuscript, section, paragraph, and sentence scopes.

### Modified Capabilities

None; no main specs are published yet.

## Impact

Changes affect the Rust document model, LaTeX extraction, rule validation, execution, provider requests, CLI, reports, tests, and configuration documentation. Existing specialized asset checks remain supported. No live provider or publishing operation is required.
