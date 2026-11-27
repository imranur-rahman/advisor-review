## Context

- `src/guidelines.rs` loads YAML rules as active and turns Markdown prose into one `RuleCandidate` per file. Every YAML file under `--guidelines` is loaded, so a compiled pack in the same directory loads too.
- `RuleDefinition` uses `deny_unknown_fields`, so the new fields (`status`, `tests`, `provenance`) need explicit schema changes.
- Deterministic text checks are `regex`, `forbid`, and `contains` (`src/review.rs`). None of them counts matches.
- `src/providers.rs` already runs `claude` and `codex` noninteractively with no tools, an isolated temporary directory, and no session persistence.
- No hash crate is in `Cargo.toml`.

See proposal.md for motivation and the two specs for required behavior.

## Goals / Non-Goals

**Goals:**
- Keep one isolation model for every CLI call, so that the compile path adds no new attack surface.
- Make the repair loop deterministic and testable with a mock CLI.
- Keep compiled packs readable and editable as plain YAML, because the user reviews them in a Git diff.

**Non-Goals:**
- A triage classifier or cascade (JEV, gpt-oss-safeguard, logprob scoring). That is a later change.
- Compilation through hosted API providers. The loop is provider-neutral, so a later change can add them.
- Compilation of PDF or plain-text guideline files.
- Automatic activation, or a score that activates rules.

## Decisions

### D1. advisor-review runs the repair loop, not the agent
The host process sends a compile prompt, parses the returned JSON, runs the tests in-process, and sends failures back in a new prompt. The CLI keeps `--tools ""` (Claude Code) and the `read-only` sandbox (Codex).

- Alternative: give the agent shell access, so that it runs `advisor-review rules test` by itself. We rejected this, because it needs write and execute permissions inside the user's project. It also breaks the isolation that the `installed-cli-providers` change promises.
- Consequence: each repair round is a new, stateless CLI call. The repair prompt includes the original item, the failed rule, and the test results, so that no session state is needed.

### D2. Draft status lives in the rule, and drafts load as candidates
A new optional `status` field takes `draft` or `active`. An absent field means `active`, so every existing rule keeps its behavior. The loader converts a draft rule into a `RuleCandidate` before the duplicate-id and conflict checks, so that draft rules never suppress or conflict with active rules.

- Alternative: write drafts to a separate directory that the review does not load. We rejected this, because activation then means moving YAML between files, and provenance paths get harder to follow. One field change in place gives a one-line Git diff.

### D3. Output file is `<source-stem>.compiled.yaml` next to the source
The file uses the existing `rules:` list format plus a top-level `uncheckable:` list. Rule ids follow `<source-stem>.<item-number>.<slug>`, for example `writing-donts.7.metaphor`. The item number keeps ids stable across recompiles when the items do not move.

### D4. Guideline items come from a small Markdown splitter
The splitter walks the lines once. It tracks the current heading, skips fenced blocks, and emits one item per top-level list item (with its continuation lines) or per prose paragraph. It records the 1-based start and end lines. We do not add a Markdown parser crate, because the needed structure is small and the loader already scans lines.

### D5. Provenance hash is SHA-256 over the normalized item text
The normalization trims each line and joins the lines with `\n`. The hash covers the item only, not the heading, so that renaming a heading does not mark rules as stale. We add the `sha2` crate, pinned to a version that builds on Rust 1.85. Staleness reads the source file once per load and compares hashes at the recorded line range. If the lines moved, it searches the file for the same hash before it reports a rule as stale, so that inserting a new item above does not mark everything stale.

### D6. The compile prompt fixes the output schema and the tier order
The prompt contains the rule JSON schema, the list of allowed check types and scopes, and this instruction: "Use a deterministic check when a pattern can decide the item. Else use one yes/no semantic question per rule. Else return `uncheckable` with a reason." It also asks for at least two `should_fire` and two `should_pass` examples per rule. The response must be one JSON object. The system validates it with the same loader code as a hand-written rule.

### D7. Semantic tests reuse `semantic_review`
`rules test` builds a synthetic `DocumentTarget` of the rule's scope from each example and calls the normal review path. "Fired" means at least one `violation` or `concern` finding. Because of this, a semantic test measures the real prompt, not a test-only prompt.

### D8. `count` is a deterministic check in `review.rs`
It compiles `pattern` once (as `regex` does now), counts non-overlapping matches with `find_iter`, and fires when the count is greater than `max`. The evidence lists the matched substrings with their byte offsets, so that source mapping works as it does for other findings.

### D9. CLI surface
- `advisor-review compile --guidelines <file-or-dir> --provider <claude-code|codex-cli> [--model M] [--max-repair-rounds N] [--force]`. A directory input compiles every `.md` file in it that has prose items.
- `advisor-review rules test --guidelines <file-or-dir> [--provider P --model M]`.
- Exit codes follow the review command: 0 success, 1 failed or incomplete, 2 invalid input.

## Risks / Trade-offs

- [The CLI writes examples that match its own rule by construction, so tests pass but the rule is still wrong.] → The rule stays a draft, and the user reviews examples and rule together. The compile summary tells the user to add real sentences from their drafts to `should_fire`.
- [Semantic tests are not deterministic, so a test passes in one run and fails in the next.] → `rules test` reports each result. A later change can add repeated runs and pass rates. Compile counts a semantic rule as repaired only after its tests pass once.
- [Compile cost grows with items × repair rounds.] → The default is 3 rounds. The summary shows the number of CLI calls.
- [Regexes from the model can be slow on large text.] → The `regex` crate guarantees linear time, so this risk is low.
- [An existing hand-written rule pack has the same intent as a compiled rule, and both run after activation.] → Ids differ, so no duplicate error occurs. The compile summary lists existing active rules whose `source` is the same Markdown file, so that the user can see the overlap. For `writing-donts`, the docs explain which pack to keep.

## Migration Plan

No migration is needed. Rules without `status`, `tests`, or `provenance` behave as before. Rollback means deleting the `*.compiled.yaml` files.
