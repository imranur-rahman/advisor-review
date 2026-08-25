## Purpose

Provides configurable semantic-review backends so researchers can choose hosted or local models while keeping rule execution and findings independent of a specific provider.

## ADDED Requirements

### Requirement: Provider and model are configurable
The system SHALL allow users to select a provider and model through CLI or configuration settings. It SHALL support OpenAI, Anthropic, OpenRouter, Ollama, and compatible provider endpoints where the required capabilities are available.

#### Scenario: User selects a hosted model
- **WHEN** the user supplies a provider and model name for a semantic review
- **THEN** the review SHALL use that provider/model combination and record it in run metadata

#### Scenario: User selects a local model
- **WHEN** the user configures Ollama or another compatible local endpoint
- **THEN** semantic checks SHALL use the local endpoint without requiring a hosted API key

### Requirement: Credentials are supplied without embedding them in rules
The system SHALL accept credentials through environment variables or a local user configuration mechanism and SHALL avoid writing secret values into findings, Markdown reports, logs, or guideline files.

#### Scenario: API key is configured
- **WHEN** a hosted provider requires an API key and a valid key is available
- **THEN** the provider adapter SHALL authenticate without exposing the key in review outputs

#### Scenario: API key is missing
- **WHEN** a requested hosted semantic provider has no credential
- **THEN** the system SHALL report the provider configuration problem and continue deterministic checks where possible

### Requirement: Provider capabilities are explicit
Provider adapters SHALL expose supported capabilities such as semantic text, vision-language analysis, context limits, and structured output. The review engine SHALL use this information to skip or downgrade unsupported checks predictably.

#### Scenario: Provider lacks vision support
- **WHEN** a rule requires visual figure interpretation and the selected provider supports text only
- **THEN** the rule SHALL be marked skipped or uncertain with a capability explanation

### Requirement: Provider failures are isolated
A provider timeout, rate limit, malformed response, or service error SHALL be represented as a review execution issue for the affected checks and SHALL not erase deterministic findings already produced.

#### Scenario: Semantic request times out
- **WHEN** a provider request times out during a review
- **THEN** deterministic findings SHALL remain in the output and the affected semantic checks SHALL include an actionable failure status
