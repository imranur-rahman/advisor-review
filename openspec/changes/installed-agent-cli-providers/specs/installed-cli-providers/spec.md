## ADDED Requirements

### Requirement: Installed agent CLIs are semantic providers
The system SHALL support `claude-code` and `codex-cli` provider names that invoke the corresponding installed command-line client for semantic review. These providers SHALL reuse the CLI's existing authentication and SHALL NOT require an API key configured in advisor-review. A model MAY be omitted to use the CLI's configured default.

#### Scenario: Review with an installed CLI
- **WHEN** a user selects `claude-code` or `codex-cli` and the corresponding executable is available and authenticated
- **THEN** the semantic prompt is sent to that CLI and its response is processed using the normal review validation

#### Scenario: CLI is unavailable or fails
- **WHEN** the selected executable is missing, unauthenticated, or exits unsuccessfully
- **THEN** the review records a provider issue with actionable CLI-specific context and preserves findings already produced

### Requirement: CLI invocation limits access and persistence
The system SHALL pass review prompts through standard input, run the CLI from an isolated temporary working directory, disable session persistence, and constrain tools or sandbox access so that the CLI cannot modify the manuscript project. It SHALL NOT silently fall back to another provider.

#### Scenario: Review prompt is isolated
- **WHEN** either CLI is invoked
- **THEN** it receives only the bounded prompt generated for that review request, and its working directory is not the manuscript project

### Requirement: CLI results use existing evidence validation
The system SHALL parse the CLI's returned review JSON and apply the same response-shape and exact-quote evidence checks used for other semantic providers.

#### Scenario: CLI returns invalid or unsupported evidence
- **WHEN** the CLI response is malformed or cites absent text, unknown targets, or invalid offsets
- **THEN** the result is reported as a provider-response issue rather than accepted as a finding
