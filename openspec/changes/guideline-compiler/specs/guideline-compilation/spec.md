## Purpose

Turns natural-language guideline items into draft rule packs that a user can review, test, and activate, so that written advisor guidance can run in a review without hand-written rules.

## ADDED Requirements

### Requirement: Compile command produces a draft rule pack
The system SHALL provide an `advisor-review compile` command that takes one or more Markdown guideline files and a provider. For each input file, it SHALL write one draft rule pack in YAML next to the source, named `<source-stem>.compiled.yaml`. The command SHALL support the `claude-code` and `codex-cli` providers and SHALL reuse their existing authentication and default model.

#### Scenario: Compile a Markdown guideline file
- **WHEN** a user runs `advisor-review compile --guidelines guidelines/writing-donts.md --provider claude-code`
- **THEN** the system writes `guidelines/writing-donts.compiled.yaml` with one or more entries for each guideline item in the source

#### Scenario: Provider is unavailable
- **WHEN** the selected CLI is missing, unauthenticated, or exits unsuccessfully
- **THEN** the command exits with code 1, reports the CLI-specific error, and does not write a partial rule pack

#### Scenario: Output file already exists
- **WHEN** the target `<source-stem>.compiled.yaml` already exists and `--force` is not given
- **THEN** the command exits with code 2 and does not change the existing file

### Requirement: Guideline items are the unit of compilation
The system SHALL split a Markdown guideline file into guideline items. A guideline item SHALL be one list item or one prose paragraph outside fenced blocks. Headings SHALL be context for the items under them and SHALL NOT be items. Existing ```` ```rule ```` blocks SHALL NOT be sent to the compiler.

#### Scenario: Bulleted guideline list
- **WHEN** a guideline file has a heading followed by nine bullet items
- **THEN** the compiler receives nine guideline items, and each item carries the heading as context

### Requirement: Each guideline item compiles into one tier
For each guideline item, the compiled pack SHALL contain exactly one of these outcomes:
- one or more deterministic rules
- one or more semantic rules, where each rule states one yes/no question at one scope
- one `uncheckable` entry with a reason

A guideline item MAY produce rules of both deterministic and semantic kinds only when the item states more than one requirement.

#### Scenario: Pattern-based guideline
- **WHEN** a guideline item says "Don't overuse em dashes"
- **THEN** the compiled pack contains a deterministic `count` rule at sentence or paragraph scope with an explicit maximum

#### Scenario: Judgment-based guideline
- **WHEN** a guideline item says "Don't use metaphors whose comparisons fail under scrutiny"
- **THEN** the compiled pack contains a semantic rule whose description is one yes/no question about one target

#### Scenario: Guideline that cannot be checked
- **WHEN** a guideline item gives general advice that no check can test, such as "Apply with judgment"
- **THEN** the compiled pack contains an `uncheckable` entry with the item text and a reason, and no rule for that item

### Requirement: Compiled rules are drafts and never run until activated
Every rule that the compiler writes SHALL have `status: draft`. The review engine SHALL treat a draft rule as a candidate and SHALL NOT run it. A rule SHALL run only when its status is `active` or the status field is absent. The compiler SHALL NOT write `status: active`.

#### Scenario: Review with a draft pack present
- **WHEN** a review loads a guideline directory that contains a compiled pack with only draft rules
- **THEN** the review lists the draft rules as candidates and produces no findings from them

#### Scenario: User activates one rule
- **WHEN** a user changes one compiled rule to `status: active`
- **THEN** the next review runs that rule and continues to treat the other draft rules as candidates

### Requirement: Compiled rules carry provenance
Each compiled rule and `uncheckable` entry SHALL record the source file path, the source line range, a SHA-256 hash of the guideline item text, the compiler provider, the compiler model (or `default`), and the compile time.

#### Scenario: Provenance in a finding
- **WHEN** an activated compiled rule produces a finding
- **THEN** the finding's source guideline names the source file and line range of the guideline item

### Requirement: Stale compiled rules are reported
When the system loads a rule with provenance, it SHALL hash the current text at the recorded source location. If the hash differs or the source is missing, the system SHALL report the rule as stale in the review metadata. A stale rule SHALL keep its status, and staleness alone SHALL NOT change the review exit code.

#### Scenario: Source guideline was edited
- **WHEN** a user edits a guideline item after compilation and runs a review
- **THEN** the review report lists each rule compiled from that item as stale, with the rule id and the source location

### Requirement: Compilation sends only guideline text
The compile command SHALL send only guideline text, its heading context, and the rule schema to the provider. It SHALL NOT send manuscript text. It SHALL run the CLI with the same isolation as semantic review: prompt on standard input, an isolated working directory, no session persistence, and no tool access.

#### Scenario: Compile in a manuscript project
- **WHEN** a user runs compile from a directory that contains a manuscript
- **THEN** the prompt contains no manuscript text, and the CLI cannot read or change project files

### Requirement: Compiler repairs rules that fail their tests
After the provider returns a draft pack, the system SHALL validate each rule and run its example tests. For each rule that is invalid or fails a test, the system SHALL send the failure details back to the provider and ask for a corrected rule. The system SHALL stop after a configurable number of repair rounds (default 3). A rule that still fails SHALL be written as an `uncheckable` entry whose reason includes the last failure.

#### Scenario: Regex misses its positive example
- **WHEN** a compiled `count` rule does not fire on its own positive example
- **THEN** the system sends the rule, the example, and the result to the provider in a repair round

#### Scenario: Repair limit is reached
- **WHEN** a rule still fails its tests after the last repair round
- **THEN** the pack contains an `uncheckable` entry for that rule with the failure reason, and the command summary counts it as failed

### Requirement: Compile reports a summary
The compile command SHALL print, for each input file, the number of guideline items, deterministic rules, semantic rules, uncheckable entries, and rules that failed repair. It SHALL exit with code 0 when it writes every pack, even if some items are uncheckable.

#### Scenario: Mixed outcome
- **WHEN** compile produces 4 deterministic rules, 3 semantic rules, and 2 uncheckable entries for one file
- **THEN** the summary shows those counts and the command exits with code 0
