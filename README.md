# advisor-review

`advisor-review` is a local-first CLI for reviewing LaTeX manuscripts against advisor, journal, conference, and project guidelines.

## Quick start

```bash
cargo run -- review --project ./paper --guidelines ./guidelines --output ./review
```

The project must contain a `main.tex` and `main.pdf` by default. Use `--main-tex` and `--pdf` to select different files. The command writes `findings.json` and `findings.md`.

Provider settings can be supplied with `--provider` and `--model` or the `ADVISOR_REVIEW_PROVIDER`, `ADVISOR_REVIEW_MODEL`, and `ADVISOR_REVIEW_API_KEY` environment variables. Deterministic checks run without credentials. Semantic checks use the configured hosted or Ollama-compatible provider and are reported as skipped when credentials or required capabilities are unavailable.

For crates.io release instructions, see [PUBLISHING.md](PUBLISHING.md).

## Structured rules

Guideline YAML files can contain one rule or a `rules` list:

```yaml
id: prose.avoid-very
scope: paragraph
kind: text
severity: warning
check:
  type: forbid
  pattern: "very"
  message: Avoid vague intensifiers.
  suggestion: Replace the intensifier with a measurable claim.
```

Markdown guidelines may contain ` ```rule ` YAML blocks. Ordinary Markdown prose is preserved as a candidate rule requiring review before activation.

## Development

```bash
cargo test
cargo run -- --help
```

See `TOOLCHAIN.md` for current parsing and PDF-analysis boundaries.
