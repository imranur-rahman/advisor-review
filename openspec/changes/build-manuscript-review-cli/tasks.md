## 1. Project Foundation and CLI Contract

- [x] 1.1 Establish the application package, command entry point, configuration loading, and test layout; verify the CLI help command runs successfully.
- [x] 1.2 Define versioned JSON schemas/models for run metadata, rules, typed targets, anchors, findings, skipped checks, conflicts, and execution issues; verify representative JSON fixtures validate.
- [x] 1.3 Define CLI input discovery and validation for project directory, main LaTeX source, compiled PDF, guideline directory, provider/model settings, and output directory; verify missing-input tests return actionable errors.
- [x] 1.4 Decide and document the initial LaTeX/PDF/image toolchain through a small compatibility spike; verify it can inspect a sample project and record the decision in project documentation.

## 2. Manuscript Model and Source Anchors

- [x] 2.1 Implement LaTeX project discovery including included source files and raw source ranges; verify a fixture with multiple included files preserves contributing file and line locations.
- [x] 2.2 Implement typed extraction for sections, paragraphs, sentences, figures, tables, rows/cells where available, equations, citations, references, and code/listing environments; verify mixed-environment fixtures produce expected target types.
- [x] 2.3 Implement PDF metadata and rendered-location extraction with exact, approximate, and unavailable mapping states; verify PDF-only findings never claim an exact source mapping when one is unavailable.
- [x] 2.4 Implement figure, table, and code analyzer facts including asset dimensions, effective DPI inputs/calculation, rendered size, table properties, and code context; verify fixture facts are exposed to rules.

## 3. Guideline Loading and Rule Registry

- [x] 3.1 Implement guideline-directory discovery, supported-file loading, metadata, and source provenance; verify multiple advisor and publication files load into one registry.
- [x] 3.2 Define and validate structured rule syntax for identifiers, scope, evaluation kind, severity, precedence, parameters, required evidence, and provider capabilities; verify malformed rules are reported and not activated.
- [x] 3.3 Implement natural-language guideline candidate extraction with explicit candidate status and provenance; verify candidates require confirmation before becoming active rules.
- [x] 3.4 Implement rule precedence, conflict detection, and conflict metadata; verify a higher-priority publication rule produces a visible resolution over a conflicting advisor rule.

## 4. Deterministic Review Engine

- [x] 4.1 Implement the rule execution interface over typed manuscript targets and analyzer facts; verify a fixture rule can produce a finding with rule provenance and a source anchor.
- [x] 4.2 Implement regex/text and LaTeX-structure checks; verify positive, negative, and unsupported-target cases.
- [x] 4.3 Implement PDF/layout checks and figure checks for dimensions, effective DPI, rendered text size, and available asset metadata; verify deterministic figure checks run without provider credentials.
- [x] 4.4 Implement table, equation, citation/reference, and code/listing rule hooks; verify findings attach to typed table/cell, equation, citation, or code targets.
- [x] 4.5 Preserve partial results and execution issues when individual checks fail; verify one failing check does not erase completed deterministic findings.

## 5. Semantic Provider Adapters

- [x] 5.1 Define the provider adapter interface for model selection, structured responses, semantic text, vision-language capability, context limits, and request errors; verify a fake provider satisfies the interface.
- [x] 5.2 Implement provider configuration and credential resolution for OpenAI, Anthropic, OpenRouter, Ollama, and compatible endpoints; verify API keys are never present in logs or serialized outputs.
- [x] 5.3 Implement target-scoped semantic requests with structured pass/violation/concern/suggestion/uncertain results, evidence, explanation, and confidence; verify mock responses map to the finding schema.
- [x] 5.4 Implement capability-aware skipping and isolated timeout, rate-limit, malformed-response, and provider-error handling; verify deterministic findings remain when semantic requests fail.

## 6. Reports, Exit Status, and Integration

- [x] 6.1 Implement JSON report serialization with schema version, run metadata, provider/model metadata, findings, skipped checks, conflicts, and execution issues; verify zero-finding and multi-finding fixtures serialize valid JSON.
- [x] 6.2 Implement Markdown rendering from JSON only, including grouped findings, typed targets, source/PDF anchors, evidence, suggestions, and limitations; verify Markdown output matches the JSON fixture.
- [x] 6.3 Define and implement initial CLI exit-status behavior for completed reviews, invalid inputs, and execution failures; verify documented exit codes with automated CLI tests.
- [x] 6.4 Add an end-to-end fixture project with guidelines, source, PDF, deterministic rules, and mocked semantic checks; verify one command produces both expected JSON and Markdown artifacts.
- [x] 6.5 Document configuration, environment variables, privacy behavior, supported rule types, provider setup, output schema, and current source-to-PDF limitations; verify the documented example runs against the fixture project.
