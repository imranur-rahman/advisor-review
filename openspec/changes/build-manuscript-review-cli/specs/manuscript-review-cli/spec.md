## Purpose

Provides researchers with a repeatable command-line workflow for reviewing LaTeX manuscripts and receiving portable, evidence-backed findings in both machine-readable and human-readable formats.

## ADDED Requirements

### Requirement: CLI accepts a LaTeX project and review configuration
The CLI SHALL accept a project directory, a guideline directory, optional provider/model configuration, and an output directory. It SHALL discover the configured manuscript source and compiled PDF or report a clear validation error when required inputs are unavailable.

#### Scenario: Review a valid project
- **WHEN** a user supplies a project containing LaTeX sources, a compiled PDF, and a guideline directory
- **THEN** the CLI SHALL run the configured checks and write review outputs to the requested output directory

#### Scenario: Required manuscript input is missing
- **WHEN** the project does not contain the configured source or PDF input
- **THEN** the CLI SHALL exit non-successfully with an actionable error identifying the missing input

### Requirement: CLI produces JSON and Markdown findings
The CLI SHALL produce a JSON findings file and a Markdown report for every completed review, including successful reviews with zero findings. The JSON structure SHALL be stable enough for future tools to consume without parsing Markdown.

#### Scenario: Review produces findings
- **WHEN** one or more rules identify issues
- **THEN** the JSON SHALL contain typed findings and the Markdown SHALL present each finding with its rule, target, evidence, explanation, and suggestion

#### Scenario: Review produces no findings
- **WHEN** all enabled checks pass
- **THEN** the CLI SHALL write valid empty-findings JSON and a Markdown report stating that no findings were produced

### Requirement: Findings preserve review status and provenance
Each finding SHALL include a rule identifier, originating guideline, status, severity, confidence when available, target anchor, evidence, and suggested correction when available. Supported statuses SHALL distinguish definite violations, concerns, suggestions, uncertain results, and skipped checks.

#### Scenario: Finding comes from a guideline file
- **WHEN** a rule reports an issue
- **THEN** the finding SHALL identify the guideline file and rule that produced it

#### Scenario: A semantic check cannot run
- **WHEN** a rule requires a provider capability that is not configured or available
- **THEN** the result SHALL be recorded as skipped or uncertain with a reason rather than being reported as a violation

### Requirement: CLI communicates review outcomes through exit status
The CLI SHALL provide a predictable exit status that distinguishes successful review execution from invalid input or execution failure. Finding severity SHALL be available in the output for CI policies without forcing the initial CLI to impose a single failure threshold.

#### Scenario: Review completes with findings
- **WHEN** all requested checks execute successfully and findings exist
- **THEN** the CLI SHALL report a completed review and expose finding severities in its outputs

#### Scenario: Review cannot execute
- **WHEN** configuration or processing prevents the review from completing
- **THEN** the CLI SHALL exit non-successfully and identify the failure
