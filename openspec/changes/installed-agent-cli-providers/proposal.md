## Why

Users who already have Claude Code or Codex installed and authenticated should be able to use those tools for semantic manuscript review without configuring a separate API key. This also provides a practical path to test local CLI integrations while retaining the existing evidence validation.

## What Changes

- Add Claude Code and Codex CLI as semantic review providers.
- Reuse each CLI's existing authentication and default model when no model is supplied.
- Run each CLI noninteractively with constrained tool access, an isolated working directory, and no session persistence.
- Pass review prompts through standard input and validate returned JSON through the existing response and evidence checks.
- Document installation, configuration, data flow, limitations, and mock-based tests.

## Capabilities

### New Capabilities
- `installed-cli-providers`: Semantic review through installed Claude Code and Codex command-line clients.

### Modified Capabilities
- None.

## Impact

Affected areas: provider configuration and dispatch (`src/config.rs`, `src/providers.rs`), provider and CLI tests, README and configuration guide. No new runtime crate is required. The feature relies on the user's installed CLI, authentication, and account entitlements.
