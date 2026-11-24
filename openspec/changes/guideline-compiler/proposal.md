## Why

Natural-language guidelines (for example `guidelines/writing-donts.md`) load only as rule candidates and never run. A user must hand-write YAML rules for each guideline, so most advisor guidance has no effect on a review. Hand-written rules also have no tests, so a rule that never fires looks the same as a rule that finds nothing.

## What Changes

- Add an `advisor-review compile` command. It sends Markdown guideline items to an installed Claude Code or Codex CLI and writes a draft rule pack next to the source.
- The compiler puts each guideline item into the cheapest tier that can hold it: a deterministic check, one or more atomic yes/no semantic checks, or an "uncheckable" entry with a reason.
- Add a `count` deterministic check type. It fires when a pattern matches more than a set number of times in a target (for example, more than one em dash in a sentence).
- Each compiled rule carries example tests: text that must trigger the rule and text that must not trigger it.
- Add an `advisor-review rules test` command that runs the example tests for any rule pack, compiled or hand-written.
- The compiler runs the tests and sends failures back to the CLI for a bounded number of repair rounds.
- Each compiled rule records provenance: the source file, the line range, a hash of the source text, and the compiler provider and model. A review reports a rule as stale when its source text changes.
- Compiled rules have `status: draft` and do not run in a review. A user activates a rule by changing its status. The compiler never activates a rule.

## Capabilities

### New Capabilities
- `guideline-compilation`: Compilation of natural-language guideline items into draft, tested rule packs through installed agent CLIs, with provenance and staleness detection.
- `rule-example-tests`: Example-based tests for rules, the `rules test` command, and the `count` deterministic check type.

### Modified Capabilities
- None. No main specs exist under `openspec/specs/`. The existing rule loader keeps its behavior for rules without the new fields.

## Impact

- Code: `src/main.rs` (two new subcommands), `src/model.rs` (rule fields `status`, `tests`, `provenance`), `src/guidelines.rs` (draft and stale handling, Markdown item splitting), `src/review.rs` (`count` check), `src/providers.rs` (reuse of CLI invocation for compile prompts), a new compile module.
- Data flow: compile sends guideline text only, never manuscript text, to the selected CLI.
- Docs: `CONFIGURATION.md`, `README.md`, and `guidelines/writing-donts.md`.
- No new runtime crate is expected. `sha2` or an equivalent hash crate may be needed if no hash is available.
