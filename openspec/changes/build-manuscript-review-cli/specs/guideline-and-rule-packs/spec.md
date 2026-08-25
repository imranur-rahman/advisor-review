## Purpose

Allows researchers to compose reusable advisor, journal, conference, and project guidance into an explicit, traceable set of manuscript-review rules.

## ADDED Requirements

### Requirement: Guideline directories support multiple sources
The system SHALL load multiple guideline files from a configured directory and preserve each rule's source file and metadata. It SHALL support combining advisor-specific, publication-specific, and project-specific guidance in one review.

#### Scenario: Multiple guideline sources are supplied
- **WHEN** a guideline directory contains several supported files
- **THEN** the system SHALL load applicable content from all files and retain source provenance for generated rules and findings

#### Scenario: A guideline file is invalid
- **WHEN** a guideline file contains invalid structured content
- **THEN** the system SHALL report the file and validation error without silently treating the malformed rule as active

### Requirement: Rules support natural-language and structured definitions
The system SHALL support natural-language guideline prose and explicit structured rule definitions. Natural-language interpretation SHALL produce identifiable candidate rules that can be reviewed before activation, while structured rules SHALL be directly validated before execution.

#### Scenario: Natural-language guidance is interpreted
- **WHEN** a guideline contains prose describing a review preference
- **THEN** the system SHALL represent the interpretation as a candidate rule with source provenance and an explanation of the inferred check

#### Scenario: Structured rule is valid
- **WHEN** a guideline contains a valid rule definition with an identifier, scope, evaluation kind, and check
- **THEN** the system SHALL make the rule available to the review engine

### Requirement: Rule metadata defines scope and evaluation needs
Each active rule SHALL identify its target scope, evaluation kind, severity, and required evidence or provider capabilities when applicable. Supported target scopes SHALL include document, section, paragraph, sentence, figure, table, table row, table cell, equation, code block, code line, citation, reference, and PDF page.

#### Scenario: Asset rule targets a figure
- **WHEN** a rule declares figure scope and an image-quality check
- **THEN** the rule SHALL execute against figure assets and attach findings to the corresponding figure target

#### Scenario: Semantic rule requires unavailable capability
- **WHEN** a rule requires vision-language analysis but the configured provider cannot perform it
- **THEN** the rule SHALL be skipped or marked uncertain with the missing capability recorded

### Requirement: Rule precedence and conflicts are visible
The system SHALL support explicit rule priority or precedence metadata and SHALL not silently hide conflicts between active rules. Conflicting rules SHALL be reported with the participating rule identifiers and the selected resolution when a deterministic resolution exists.

#### Scenario: Higher-priority publication rule conflicts with advisor rule
- **WHEN** two active rules make incompatible requirements for the same target
- **THEN** the system SHALL apply the configured precedence and record the conflict and resolution in review metadata

### Requirement: Rules cover deterministic and semantic review
The rule model SHALL support deterministic checks over text, LaTeX structure, PDF layout, and image metadata, as well as semantic text, visual, and cross-modal checks.

#### Scenario: Deterministic figure rule runs without an API key
- **WHEN** a figure rule checks effective DPI or rendered text size
- **THEN** the system SHALL execute the check without requiring a semantic provider

#### Scenario: Semantic paragraph rule runs with a configured provider
- **WHEN** a paragraph rule requests semantic evaluation and a compatible provider is configured
- **THEN** the system SHALL return a structured pass, violation, concern, suggestion, or uncertain result with evidence and explanation
