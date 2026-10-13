## Context

See proposal.md for motivation. The CLI already writes JSON and Markdown reports with finding severity, explanation, suggestion, and source anchors. Source paths in anchors are relative to the project; findings may have multiple spans or no source span. VS Code task problem matchers consume line-oriented task output and support error, warning, and info severities.

## Goals / Non-Goals

**Goals:**
- Add an opt-in terminal format that VS Code can parse as diagnostics.
- Provide a ready-to-copy task and matcher configuration.
- Preserve the current reports and keep review non-mutating.

**Non-Goals:**
- Build or publish a VS Code extension.
- Edit manuscript files, generate replacement text, or create Git diffs.
- Emit diagnostics for findings that have no source location.

## Decisions

- Add `--format text|vscode`, defaulting to the current text output for compatibility.
- Emit each located finding as `path(line,column): severity: message`, using a relative source path and the first target source span (falling back to the legacy single source anchor). This matches a custom VS Code problem matcher and opens the source location.
- Map `error` and `warning` directly; map `suggestion`, `info`, and unrecognized severities to `info`, which is the closest supported VS Code problem level.
- Include the rule ID, finding status, explanation, evidence quote, and existing suggestion in a whitespace-normalized single-line diagnostic message. Keep findings with no source anchor in the reports only.
- Keep the regular completion summary after the diagnostics. The matcher ignores this summary, while users still get a useful terminal result.
- Document a task configured with `advisor-review review --project ${workspaceFolder}` and relative source paths rooted at `${workspaceFolder}`. Users copy it into the manuscript workspace's `.vscode/tasks.json`.

Alternatives considered: SARIF would provide a portable interchange format but requires a viewer extension for this workflow. A VS Code extension could provide richer UI but adds maintenance and release work. A task matcher uses built-in VS Code behavior and fits the selected scope.

## Risks / Trade-offs

- Findings with no source anchor cannot appear in Problems → retain them in reports and keep them in the summary counts.
- A task matcher displays one primary location per finding → use the target's first mapped source span and leave any additional evidence locations in the detailed reports.
- Suggestion severities have no direct VS Code equivalent → present them as informational diagnostics.
- Long explanation/evidence may clutter the Problems list → normalize whitespace and include the existing concise finding fields; full context remains in Markdown/JSON.
