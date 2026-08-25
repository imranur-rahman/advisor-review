## Purpose

Creates a common manuscript representation that connects LaTeX source, rendered PDF locations, and manuscript assets so review comments can target meaningful document elements beyond individual lines.

## ADDED Requirements

### Requirement: System extracts typed manuscript targets
The system SHALL identify source spans and typed targets for sections, paragraphs, sentences, figures, tables, table rows, table cells where available, equations, citations, references, and code/listing environments.

#### Scenario: Manuscript contains mixed environments
- **WHEN** a LaTeX project contains prose, figures, tables, equations, and code listings
- **THEN** the document representation SHALL expose each supported element with its type and source location

#### Scenario: Included source files are used
- **WHEN** the main LaTeX file includes content from other source files
- **THEN** extracted targets SHALL retain the actual contributing file and source range

### Requirement: Findings support source and rendered anchors
The system SHALL allow a finding to include a source file and line range and, when available, a PDF page and rendered bounding box. A finding MAY include multiple anchors when an issue relates to source, rendered output, and an asset.

#### Scenario: Source location is available
- **WHEN** a rule identifies a problem in a LaTeX environment
- **THEN** the finding SHALL point to the environment's source file and line range

#### Scenario: Rendered location is available
- **WHEN** a PDF analyzer identifies a layout or visual problem
- **THEN** the finding SHALL include the affected PDF page and rendered location when measurable

### Requirement: System analyzes figures, tables, and code as specialized targets
The system SHALL expose facts needed for specialized checks, including figure dimensions and effective resolution, table dimensions and rendered properties, and code block or line content with its surrounding context.

#### Scenario: Figure resolution is checked
- **WHEN** a figure rule requests effective DPI
- **THEN** the analyzer SHALL provide the source asset, rendered size when available, and calculated or unavailable effective DPI

#### Scenario: Table or code target has a finding
- **WHEN** a rule identifies an issue in a table, table cell, code block, or code line
- **THEN** the finding SHALL attach to that typed target rather than only to an unrelated paragraph or file line

### Requirement: Analysis limitations are explicit
The system SHALL record when source-to-PDF mapping, asset extraction, OCR, or other analysis is unavailable or approximate. It SHALL not present an unavailable location or measurement as exact.

#### Scenario: PDF mapping is approximate
- **WHEN** a rendered location cannot be mapped precisely to a source span
- **THEN** the finding SHALL identify the mapping as approximate and retain the best available page or environment anchor
