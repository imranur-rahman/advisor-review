## Context

The repository currently contains no application implementation, so this change establishes the initial architecture for a reusable researcher-facing CLI. The proposal defines the user-visible behavior; this design defines the boundaries needed to support source LaTeX, rendered PDF, assets, multiple rule sources, and hosted or local semantic providers.

## Goals / Non-Goals

**Goals:**

- Establish a local-first CLI workflow with JSON as the stable output contract and Markdown as a generated report.
- Normalize LaTeX, PDF, and asset analysis into typed targets with source and rendered anchors.
- Keep deterministic analysis independent from semantic-provider availability.
- Support explicit rules and human-readable guideline prose without silently activating inferred policies.
- Make provider adapters interchangeable and capability-aware.

**Non-Goals:**

- Building a web or desktop interface.
- Automatically editing or rewriting the manuscript.
- Solving perfect source-to-PDF mapping for every LaTeX construct.
- Supporting collaborative accounts, hosted storage, or guideline marketplaces.

## Decisions

### Use a common intermediate manuscript model

All analyzers should consume a normalized document model containing typed targets, source spans, optional PDF anchors, asset references, and extracted facts. This keeps rules independent of parser details and allows future UI clients to consume the same finding anchors.

Alternative considered: let every rule parse LaTeX or PDF independently. This would be simpler initially but would duplicate parsing logic and make cross-modal findings inconsistent.

### Keep JSON authoritative and generate Markdown from it

The review engine will emit a versioned JSON document containing run metadata, rule metadata, findings, skipped checks, conflicts, and execution issues. Markdown will be a renderer over that document rather than a second independently generated result.

Alternative considered: make Markdown primary and parse it later. That would make future integrations fragile and lose typed anchors.

### Separate rule compilation from rule execution

Guideline loading will produce candidate and active rules with provenance, scope, precedence, and capability requirements. Structured rules can be validated directly; natural-language guidance can produce candidates requiring user confirmation before activation. Execution will operate only on the active registry.

Alternative considered: send all guideline prose and manuscript content to one prompt. That is less deterministic, harder to test, and obscures which advisor statement caused a finding.

### Use specialized analyzers behind a shared interface

The initial analyzer boundary should cover LaTeX structure, prose segmentation, PDF/layout, figures, tables, equations, code/listings, and citations. Each analyzer exposes facts and targets; rules decide how those facts are evaluated.

Alternative considered: implement only a generic text extractor. That would not support the requested DPI, font-size, table, layout, or code-environment checks reliably.

### Treat provider capability as runtime data

Provider adapters will advertise supported semantic text, vision-language, structured-output, and context capabilities. Rules declare their requirements, allowing unsupported checks to be skipped or marked uncertain while deterministic review continues.

Alternative considered: require every provider to implement every rule. This would exclude local models and make provider support brittle.

### Preserve imperfect mappings explicitly

Anchors will distinguish exact, approximate, and unavailable mappings. A PDF-only issue can point to a page and bounding box even when no exact source line is known; the report must communicate that limitation.

## Risks / Trade-offs

- [LaTeX parsing complexity] → Start with common document structures and preserve raw source ranges; treat unsupported constructs as explicit limitations.
- [PDF-to-source mismatch] → Store independent source and PDF anchors with mapping quality metadata.
- [LLM false positives] → Require structured outputs, retain evidence, expose confidence/status, and never auto-edit in the initial release.
- [Conflicting advisor and publication rules] → Support explicit priorities and report conflicts and resolutions.
- [Manuscript privacy] → Make local deterministic checks work without a provider, document transmission behavior, and never include secrets in outputs.
- [Provider cost and failures] → Chunk semantic requests by target, record provider/model metadata, isolate failures, and preserve completed findings.
- [Large manuscripts exceeding context limits] → Use target-scoped review, configurable chunking, and explicit context requirements per rule.

## Migration Plan

There is no existing application behavior to migrate. The initial release can be introduced as a new CLI and output schema. The JSON schema version should be included from the first release so future UI or report changes can remain backward-compatible.

## Open Questions

- Which LaTeX parser and PDF/image toolchain best balances installation simplicity with coverage?
- Should natural-language rule candidates be stored in the project guideline directory or in a separate user rule registry?
- What default severity and exit-code policy should be used for CI integrations?
