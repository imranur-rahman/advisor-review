## 1. Rule model and loader

- [x] 1.1 Add optional `status` (`draft` | `active`), `tests` (`should_fire`, `should_pass`), and `provenance` (file, start_line, end_line, sha256, provider, model, compiled_at) fields to `RuleDefinition`; verify with a `tests/rules.rs` case that an existing rule file without these fields still loads unchanged
- [x] 1.2 Make the loader turn `status: draft` rules into candidates before duplicate and conflict checks; verify with a test that a draft rule produces no finding and does not suppress an active rule with the same pattern
- [x] 1.3 Reject a rule that has `provenance` but lacks a `should_fire` or `should_pass` entry; verify with a test that the loader reports the validation error
- [x] 1.4 Add the `sha2` crate pinned for Rust 1.85; verify `cargo build --locked` and CI toolchain build succeed

## 2. Count check

- [x] 2.1 Implement `type: count` with `pattern` and `max` in `src/review.rs`, with match evidence and byte offsets; verify with tests for over-limit, at-limit, and missing `max`
- [x] 2.2 Document `count` in `CONFIGURATION.md`; verify the doc example loads in a test

## 3. Rules test command

- [x] 3.1 Add `advisor-review rules test --guidelines <path>` that runs deterministic examples in-process and prints per-example results; verify exit codes 0, 1, and 2 with `tests/cli.rs` cases
- [x] 3.2 Run semantic examples through the review path with a configured provider, counting only `violation` or `concern` as fired; verify with the mock HTTP provider in `tests/provider_http.rs`
- [x] 3.3 Report semantic tests as skipped without a provider; verify exit code 1 and the skipped count in a CLI test

## 4. Guideline item splitter and provenance

- [x] 4.1 Implement the Markdown splitter (items per top-level list item or prose paragraph, heading context, fenced blocks skipped, 1-based line ranges); verify with a test over `guidelines/writing-donts.md` that gives the expected item count and line ranges
- [x] 4.2 Implement item hashing over normalized text and the stale check, with a search by hash when lines moved; verify tests for an edited item (stale), an inserted item above (not stale), and a removed file (stale)
- [x] 4.3 Add stale rules to review metadata in JSON and Markdown reports without changing the exit code; verify in `tests/end_to_end.rs`

## 5. Compile command

- [x] 5.1 Build the compile prompt (rule schema, allowed checks and scopes, tier order, example minimums, guideline item with heading); verify with a snapshot-style test that the prompt contains no manuscript text
- [x] 5.2 Reuse the isolated CLI invocation from `src/providers.rs` for compile prompts; verify with a mock `claude` and `codex` script in `tests/provider.rs` that the arguments keep tools disabled and the working directory is temporary
- [x] 5.3 Parse the response, validate rules with the loader, run tests, and run repair rounds up to `--max-repair-rounds`; verify with a mock CLI that returns a failing rule first and a passing rule second
- [x] 5.4 Turn rules that still fail into `uncheckable` entries with the last failure as reason; verify with a mock CLI that always fails
- [x] 5.5 Write `<source-stem>.compiled.yaml` with `status: draft`, ids `<stem>.<item>.<slug>`, and provenance; refuse to overwrite without `--force` (exit 2); write nothing when the CLI fails (exit 1); verify each case in `tests/cli.rs`
- [x] 5.6 Print the per-file summary (items, deterministic, semantic, uncheckable, failed repair, CLI calls, overlapping active rules from the same source); verify the summary text in a CLI test

## 6. Documentation and integration

- [x] 6.1 Document `compile` and `rules test` in `CONFIGURATION.md` and `README.md`, including the data flow (guideline text only) and the activation step; verify the documented commands run against the test fixtures
- [x] 6.2 Update `guidelines/writing-donts.md` to explain how its compiled pack relates to `writing-donts.yaml`; verify a review with `--guidelines ./guidelines` gives the same findings as before when no compiled rule is active
- [x] 6.3 Run `cargo test --locked` and `cargo fmt --all -- --check`; verify both pass
