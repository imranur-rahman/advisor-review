## Purpose

Lets every rule carry example texts that must and must not trigger it, and adds a count-based deterministic check, so that users can find rules that never fire or fire on the wrong text.

## ADDED Requirements

### Requirement: Rules can declare example tests
A rule SHALL accept an optional `tests` field with two lists: `should_fire` and `should_pass`. Each entry SHALL be a text string that the system evaluates as one target at the rule's scope. A compiled rule SHALL have at least one entry in each list. A hand-written rule MAY omit `tests`.

#### Scenario: Rule with tests loads
- **WHEN** a YAML rule has `tests.should_fire` and `tests.should_pass` lists of strings
- **THEN** the rule loads without a validation error

#### Scenario: Compiled rule without tests
- **WHEN** a rule has provenance from the compiler but no `should_fire` or no `should_pass` entry
- **THEN** the loader reports a validation error for that rule and does not make it active

### Requirement: Rules test command runs example tests
The system SHALL provide an `advisor-review rules test --guidelines <path>` command. It SHALL run every rule's example tests, including draft rules. For each rule it SHALL report pass or fail for each example. A deterministic test SHALL need no provider. A semantic test SHALL use the configured provider; without one, the command SHALL report the semantic tests as skipped.

#### Scenario: All deterministic tests pass
- **WHEN** every deterministic rule fires on each `should_fire` text and does not fire on each `should_pass` text
- **THEN** the command reports every test as passed and exits with code 0

#### Scenario: A rule never fires
- **WHEN** a rule does not fire on one of its `should_fire` texts
- **THEN** the command reports that rule, that text, and the result as failed, and exits with code 1

#### Scenario: Semantic tests without a provider
- **WHEN** a semantic rule has tests and no provider is configured
- **THEN** the command reports those tests as skipped and exits with code 1

#### Scenario: Invalid rule file
- **WHEN** the guideline path contains an invalid rule
- **THEN** the command reports the validation error and exits with code 2

### Requirement: Semantic tests use the review path
A semantic test SHALL count as fired when the provider returns at least one finding with status `violation` or `concern` for the example target. Findings with status `suggestion` or `uncertain` SHALL NOT count as fired. The test SHALL use the same prompt and evidence checks as a review.

#### Scenario: Provider returns only an uncertain finding
- **WHEN** the provider returns one finding with status `uncertain` for a `should_fire` text
- **THEN** that test is reported as failed

### Requirement: Count check limits pattern occurrences
The system SHALL support a deterministic check with `type: count`, a regular expression `pattern`, and an integer `max`. The check SHALL fire when the number of non-overlapping matches in the target text is greater than `max`. The finding evidence SHALL list each match. A `count` rule without a valid `pattern` or a non-negative `max` SHALL fail validation.

#### Scenario: Too many em dashes
- **WHEN** a sentence rule has `type: count`, `pattern: "—"`, and `max: 1`, and a sentence contains two em dashes
- **THEN** the review reports one finding for that sentence with both matches as evidence

#### Scenario: Limit not exceeded
- **WHEN** the same rule checks a sentence with one em dash
- **THEN** the review reports no finding for that sentence

#### Scenario: Missing max
- **WHEN** a `count` rule has no `max`
- **THEN** the loader reports a validation error for that rule
