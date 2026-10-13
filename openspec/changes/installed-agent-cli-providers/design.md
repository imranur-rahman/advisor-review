## Context

The existing semantic pipeline builds a bounded prompt in `ConfiguredProvider::review_context`, calls provider-specific transport, then validates a shared JSON structure and exact source quotes. Installed Claude Code and Codex clients can provide semantic review using the user's existing CLI authentication.

## Decisions

- Use provider identifiers `claude-code` and `codex-cli`; model is optional for these identifiers because the CLI may have a configured default.
- Allow an executable override via `ADVISOR_REVIEW_CLAUDE_CODE_BIN` and `ADVISOR_REVIEW_CODEX_BIN`, useful when editor-launched processes have a limited PATH.
- Feed prompts through stdin so manuscript text does not appear in process arguments.
- Run from a fresh temporary directory. Invoke Claude Code in print mode with restricted execution, no tools, strict MCP config, and no session persistence. Invoke Codex with `exec`, read-only sandbox, ephemeral mode, and repository checks disabled.
- Extract Claude's JSON `result` and Codex's last-message output, then pass either into the existing common parser and evidence validation.
- Treat missing binaries, nonzero exits, malformed envelopes, and timeouts as provider issues; never change provider implicitly.
- Do not invoke live CLI models in tests. Use fake executables to verify arguments, stdin, extraction, and error behavior.

## Risks / Trade-offs

- These CLIs evolve independently; their command-line flags and response envelopes can change. Keep invocation focused and document tested assumptions.
- CLI accounts may have different usage limits or privacy terms from direct API credentials. Explain that review prompts are sent to the selected CLI/model service.
- No model is required, so report metadata may not identify the effective CLI default model.
- A process timeout must terminate the child and avoid hanging the review. Implement a bounded wait and collect output safely.
