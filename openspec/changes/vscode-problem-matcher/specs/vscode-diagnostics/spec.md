## Purpose

Expose source-anchored manuscript review findings in VS Code so authors can navigate directly to issues while retaining full details in the existing reports.

## ADDED Requirements

### Requirement: Emit VS Code problem matcher diagnostics
The CLI SHALL provide an opt-in VS Code output format that emits one parseable diagnostic for each finding with a source location. Each diagnostic SHALL include a source path relative to the project, a one-based line and column, a supported severity, and a concise message identifying the rule and review guidance.

#### Scenario: Finding has a source location
- **WHEN** a review runs with VS Code output selected and a finding has a source anchor
- **THEN** the CLI SHALL emit a single-line diagnostic that a VS Code problem matcher can map to that file and location

#### Scenario: Finding has no source location
- **WHEN** a finding has no source anchor
- **THEN** the CLI SHALL omit it from diagnostic output while retaining it in JSON and Markdown reports

#### Scenario: Finding severity is mapped for VS Code
- **WHEN** a finding has error or warning severity
- **THEN** the CLI SHALL emit that severity; suggestion, info, and other severities SHALL be emitted as informational diagnostics

### Requirement: Keep review advisory and reports complete
VS Code diagnostics SHALL provide navigation and review guidance only. The CLI SHALL NOT change manuscript files or propose replacement text. The CLI SHALL continue to write the existing JSON and Markdown reports regardless of output format.

#### Scenario: VS Code task runs a review
- **WHEN** a user configures a VS Code task with the documented problem matcher and selects VS Code output
- **THEN** source-anchored findings SHALL appear as navigable diagnostics while full findings and coverage remain available in the report files

#### Scenario: User applies a change
- **WHEN** a user reviews a diagnostic
- **THEN** they SHALL choose and apply any manuscript change themselves
