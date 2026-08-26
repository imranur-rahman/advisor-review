# advisor-review configuration and usage

This guide covers installation, CLI inputs, guideline files, deterministic rules, semantic providers, outputs, and troubleshooting.

The crate uses Rust Edition 2024 and requires Rust 1.85 or newer.

## Installation

### From crates.io

```bash
cargo install advisor-review
advisor-review --help
```

Install an exact release with `cargo install advisor-review --version 0.1.0`.

### From source

```bash
git clone git@github.com:imranur-rahman/advisor-review.git
cd advisor-review
cargo build --release
./target/release/advisor-review --help
```

## CLI usage

The review command is:

```text
advisor-review review [OPTIONS]
```

Example:

```bash
advisor-review review \
  --project ./paper \
  --guidelines ./guidelines \
  --output ./review
```

| Option | Default | Description |
|---|---|---|
| `--project <PATH>`, `-p` | `.` | LaTeX project directory. |
| `--guidelines <PATH>`, `-g` | `guidelines` | Directory containing guideline files. |
| `--output <PATH>`, `-o` | `review` | Output directory; created when needed. |
| `--main-tex <PATH>` | `<project>/main.tex` | Main LaTeX source. If missing, a `.tex` file is searched for. |
| `--pdf <PATH>` | `<project>/main.pdf` | Compiled PDF. The tool does not compile LaTeX. |
| `--provider <NAME>` | environment | Semantic provider name. |
| `--model <NAME>` | environment | Provider-specific model name. |

Paths for `--guidelines` and `--output` are relative to the current working directory. The default manuscript files are resolved under `--project`.

Show all generated options with:

```bash
advisor-review --help
advisor-review review --help
```

Expected project layout:

```text
paper/
├── main.tex
├── main.pdf
├── sections/
└── figures/

guidelines/
├── advisor.md
├── journal.yaml
└── figures.yaml
```

## Guideline files and rules

All supported files are loaded recursively. Supported extensions are `.yaml`, `.yml`, `.md`, and `.markdown`.

### One structured YAML rule

```yaml
id: prose.avoid-vague-intensifiers
name: Avoid vague intensifiers
scope: paragraph
kind: text
severity: warning
priority: 50
description: Avoid unsupported intensifiers in academic prose.
check:
  type: forbid
  pattern: "very"
  message: The paragraph uses a vague intensifier.
  suggestion: Replace it with a measurable claim.
```

### Multiple structured rules

```yaml
rules:
  - id: prose.avoid-very
    scope: paragraph
    kind: text
    severity: warning
    check:
      type: forbid
      pattern: "very"
  - id: figures.minimum-dpi
    scope: figure
    kind: asset
    severity: error
    check:
      type: min_effective_dpi
      value: 300
      message: The figure has insufficient effective resolution.
      suggestion: Export it at a higher resolution or as vector graphics.
```

Required fields are `id`, `scope`, `kind`, and `check.type`. Optional fields are `name`, `description`, `severity`, `priority`, `check.pattern`, `check.value`, `check.message`, `check.suggestion`, and `requires`.

### Markdown guidelines

Plain Markdown prose becomes a candidate and is not silently activated. Explicit rules can be embedded in `rule` fenced blocks:

````markdown
# Professor Smith

Captions should be self-contained.

```rule
id: captions.self-contained
scope: figure
kind: semantic-text
severity: suggestion
requires:
  - semantic-text
description: Captions should explain the figure without requiring the reader to search the main text.
check:
  type: semantic
```
````

Candidates appear under `Rule Candidates` in the report and are not executed automatically.

### Rule scopes

```text
document, section, paragraph, sentence, figure, table,
table_row, table_cell, equation, code_block, code_line,
citation, reference, pdf_page
```

### Deterministic check types

| Check | Behavior | Fields |
|---|---|---|
| `regex` | Flags text matching a regular expression. | `pattern` |
| `forbid` | Flags text containing a literal phrase. | `pattern` |
| `contains` | Flags text missing a required phrase. | `pattern` |
| `min_pixels` | Flags raster figures below a pixel-area threshold. | `value` |
| `min_effective_dpi` | Flags figures below calculated DPI when `includegraphics` has `width=...in` or `width=...cm`. | `value` |
| `environment_exists` | Flags matching LaTeX environments. | none |

## Provider and model configuration

Deterministic checks do not require a provider or API key. Semantic checks use hosted providers or Ollama.

Provider/model precedence is:

```text
CLI flag > environment variable > built-in fallback
```

Credential precedence is:

```text
ADVISOR_REVIEW_API_KEY > provider-specific API key
```

### Generic environment variables

```bash
export ADVISOR_REVIEW_PROVIDER=openai
export ADVISOR_REVIEW_MODEL=gpt-4o-mini
export ADVISOR_REVIEW_API_KEY="your-api-key"
advisor-review review --project ./paper --guidelines ./guidelines
```

Do not pass secrets as CLI arguments because shell history may retain them.

### Provider-specific variables

| Provider value | API-key variable | Example model |
|---|---|---|
| `openai` | `OPENAI_API_KEY` | `gpt-4o-mini` |
| `anthropic` | `ANTHROPIC_API_KEY` | A model supported by your account |
| `openrouter` | `OPENROUTER_API_KEY` | `anthropic/claude-sonnet-4` |
| `ollama` | None by default | `llama3.1` |

Examples:

```bash
export OPENAI_API_KEY="..."
advisor-review review --provider openai --model gpt-4o-mini

export ANTHROPIC_API_KEY="..."
advisor-review review --provider anthropic --model claude-sonnet-4-20250514

export OPENROUTER_API_KEY="..."
advisor-review review --provider openrouter --model anthropic/claude-sonnet-4

ollama serve
advisor-review review --provider ollama --model llama3.1
```

### Custom endpoint

Set `ADVISOR_REVIEW_ENDPOINT` for a compatible endpoint or local proxy:

```bash
export ADVISOR_REVIEW_PROVIDER=ollama
export ADVISOR_REVIEW_MODEL=llama3.1
export ADVISOR_REVIEW_ENDPOINT=http://localhost:11434/v1/chat/completions
advisor-review review
```

OpenAI-compatible requests are used for OpenAI, OpenRouter, Ollama, and custom endpoints. Anthropic uses its Messages API format. Provider responses must contain structured JSON with `status`, `evidence`, `explanation`, `suggestion`, and optionally `confidence`.

Vision-language rules are skipped unless the provider advertises the required capability. API keys are never written to reports.

## Output files

Every completed review creates:

```text
review/
├── findings.json
└── findings.md
```

`findings.json` is the stable integration contract and includes the schema version, project/PDF/provider metadata, rule provenance, findings, typed targets, source anchors, candidates, conflicts, and analysis issues. Markdown is generated from the same JSON model.

Findings can contain manuscript excerpts, file paths, and provider metadata. Treat reports as sensitive.

## Exit codes and troubleshooting

| Exit code | Meaning |
|---:|---|
| `0` | Review completed; findings may still be present. |
| `1` | Processing, parsing, provider, or output failure. |
| `2` | Invalid project or missing `main.tex`, `main.pdf`, or guideline directory. |

Common fixes:

- Missing LaTeX source: use `--main-tex path/to/main.tex` or place a `.tex` file in the project.
- Missing PDF: compile first or use `--pdf path/to/paper.pdf`.
- Semantic checks skipped: verify provider, model, API key, and rule `requires` capabilities.
- Provider failure: verify endpoint, model, API key, network, and provider account permissions.
- Missing rule: confirm the file extension and required rule fields.

## Current analysis limits

The tool validates and inspects basic PDF page structure but does not compile LaTeX. Full visual layout analysis, OCR, exact PDF-to-source mapping, and vision-language figure review are future extensions. Unavailable mappings are reported as limitations rather than exact locations.

## Privacy

Hosted providers receive semantic-review prompts and manuscript target text when those rules run. Use Ollama or another local endpoint when manuscript content must remain local. Keep API keys in environment variables and never in guideline files.
