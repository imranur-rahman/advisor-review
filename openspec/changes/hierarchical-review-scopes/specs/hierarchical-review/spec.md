## Purpose

Provide independent, source-anchored manuscript, section, paragraph, and sentence reviews with explicit execution coverage.

## ADDED Requirements

### Requirement: Ordered document hierarchy
The system SHALL expose manuscript, nested section, paragraph, sentence, and heading targets with parent relationships, order, normalized text, raw text, and source mappings across included files. Specialized objects SHALL retain their anchors and enclosing section.

#### Scenario: Section and paragraph cross file boundaries
- **WHEN** a section or paragraph continues through an included source file
- **THEN** it remains one logical target with contributing source spans in reading order

#### Scenario: Scientific sentence boundaries
- **WHEN** prose contains abbreviations, decimals, citations, math, and multiple sentences on one source line
- **THEN** extracted sentences preserve those constructs and have distinct source locations

### Requirement: Independent rule execution
The system SHALL execute sentence, paragraph, section, and document rules independently, support manuscript as a document alias, and allow CLI scope filtering. Rules SHALL support prose/source views, section selection, subsection inclusion, and explicit context policies. Heading-only rules SHALL remain available.

#### Scenario: Sentence-only review
- **WHEN** the user selects sentence scope
- **THEN** only sentence rules execute and unselected levels are reported as unselected, not failed

#### Scenario: Local failure does not block global checks
- **WHEN** a sentence rule fails
- **THEN** selected section and manuscript rules still execute

### Requirement: Contextual and bounded semantic review
Semantic review SHALL distinguish target text from context, permit multiple findings, validate cited evidence against supplied text, and enforce a configurable input budget. Oversized section/document targets SHALL be reviewed in bounded chunks followed by synthesis with traceable evidence. Summary-only synthesis SHALL NOT claim complete global coverage.

#### Scenario: Context does not change finding ownership
- **WHEN** a sentence rule uses paragraph context
- **THEN** findings remain attached to that sentence and evidence references supplied targets

#### Scenario: Oversized manuscript
- **WHEN** the full manuscript exceeds the configured budget
- **THEN** bounded chunk checks and synthesis preserve findings, and the final report identifies partial global coverage even if all chunks pass

### Requirement: Versioned reports and coverage
JSON schema 2.0 SHALL expose the target hierarchy and per-scope selected/completed/skipped/failed evaluations. Markdown SHALL group findings by scope and section and show multi-span evidence. Incomplete selected checks SHALL preserve findings and return exit code 1.

#### Scenario: Independent findings overlap
- **WHEN** sentence and paragraph findings overlap in source location
- **THEN** both remain visible under their own scopes

#### Scenario: Compatibility migration
- **WHEN** users migrate existing heading or raw-source rules
- **THEN** documentation explains heading scope and the explicit source view
