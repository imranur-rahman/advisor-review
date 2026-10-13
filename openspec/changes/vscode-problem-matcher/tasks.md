## 1. CLI diagnostics

- [x] 1.1 Add a `--format` CLI option with `text` (default) and `vscode` modes.
- [x] 1.2 Format source-anchored findings as one-line VS Code diagnostics, map severities, normalize messages, and omit findings without source locations from diagnostic lines.
- [x] 1.3 Add tests for relative location formatting, severity mapping, message output, and the missing-location fallback while preserving reports.

## 2. VS Code setup and verification

- [x] 2.1 Document a copyable `.vscode/tasks.json` task and problem matcher, including how to run it and where to find complete reports.
- [x] 2.2 Verify the matcher parses representative CLI diagnostic lines and update README/configuration guidance.
