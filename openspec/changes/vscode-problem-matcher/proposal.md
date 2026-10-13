## Why

Reviewers currently need to move from the CLI output to a Markdown or JSON report to locate findings in a manuscript. A VS Code task and problem matcher can show findings in the Problems panel and let authors jump directly to source while keeping edits under their control.

## What Changes

- Add a VS Code diagnostic output format with one parseable location and message per source-anchored finding.
- Document a copyable VS Code task and problem matcher that runs `advisor-review` and presents findings as editor diagnostics.
- Keep JSON and Markdown reports as the detailed review outputs; do not edit manuscript files or generate replacement text.
- Map finding severities to VS Code's supported problem levels and keep findings without source locations in the reports.

## Capabilities

### New Capabilities
- `vscode-diagnostics`: Expose source-anchored manuscript review findings in VS Code through task output and a problem matcher.

### Modified Capabilities

## Impact

- Affected code: CLI argument parsing and terminal output formatting.
- Affected documentation: VS Code setup and task configuration examples.
- No new runtime dependencies, VS Code extension, manuscript edits, or JSON report schema changes.
