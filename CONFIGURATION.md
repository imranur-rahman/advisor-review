# advisor-review configuration and usage

This guide covers installation, CLI inputs, guideline files, deterministic rules, semantic providers, outputs, and troubleshooting.

The crate uses Rust Edition 2024 and requires Rust 1.85 or newer.

## Installation

### From crates.io

```bash
cargo install advisor-review
advisor-review --help
```

Install an exact release with `cargo install advisor-review --version 0.1.1`.

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
| `--main-tex <PATH>` | `<project>/main.tex` | Main source. Without an explicit override, a single discovered `.tex` file can be used as fallback; multiple candidates require this flag. |
| `--pdf <PATH>` | `<project>/main.pdf` | Compiled PDF. The tool does not compile LaTeX. |
| `--provider <NAME>` | environment | Semantic provider name. |
| `--model <NAME>` | environment | Required when a provider is selected; no built-in model fallback. |

Paths for `--guidelines` and `--output` are relative to the current working directory. Default manuscript files and relative `--main-tex` / `--pdf` overrides are resolved under `--project`; absolute overrides are used directly. An explicitly missing source is an error and never triggers fallback discovery.

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

Supported kinds are `text`, `asset`, `structure`, `semantic-text`, `semantic-vision`, and `cross-modal`. Severities are `error`, `warning`, `suggestion`, and `info`. Text checks require a nonempty pattern; regex syntax is validated before execution. Asset checks require figure scope and a positive threshold (`min_pixels` requires an integer). Semantic kinds require `check.type: semantic` and a nonempty description. Provider capability requirements (`requires`) apply only to semantic rules. Unknown rule fields and check parameters are errors.

Invalid definitions are reported individually while other valid rules continue. Duplicate IDs are rejected across all loaded files. An empty executable rule set produces an incomplete review.

### Conflicts and priorities

Literal `contains` and `forbid` rules conflict when their scope, kind, and pattern are identical. The highest-priority rule determines which condition executes; opposing rules are suppressed. Opposing conditions tied at the highest priority cause the group to be skipped with an issue. Independent rules are not conflicts merely because their scope is the same. Conflicts and resolutions appear in JSON and Markdown. Arbitrary semantic or regex contradictions are not inferred.

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
document, section, paragraph, figure, table, table_cell,
equation, code_block, citation, reference, environment
```

`code` is an alias for `code_block`. `document` evaluates once against aggregate, comment-stripped LaTeX source, including visited input files; it has no single source span. It is not rendered plain text. Figure targets include their enclosing figure environment, caption, and label when present.

`sentence`, `table_row`, `code_line`, and `pdf_page` are recognized but not extracted yet. Rules using them are explicitly skipped and make the review incomplete. A supported scope with no matching targets produces a `not_applicable` issue. Missing pixel dimensions or effective DPI produce a skipped check, never an implicit pass.

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
CLI flag > environment variable
```

With no provider selected, only deterministic rules run. Selecting a provider requires an explicit model through the CLI or environment. Unknown provider names are rejected before any network request. Provider names are case-insensitive. For a compatible custom service, select one of the supported provider formats and set its endpoint.

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

OpenAI-compatible requests are used for OpenAI, OpenRouter, Ollama, and compatible custom endpoints. Anthropic uses its Messages API format. Provider responses must contain structured JSON with `status`, nonempty `evidence`, and nonempty `explanation`. `suggestion` and `confidence` are optional. Status must be `pass`, `violation`, `concern`, `suggestion`, or `uncertain`; confidence must be between 0 and 1. Invalid responses become review issues. Requests have a 30-second timeout; HTTP failures preserve completed findings.

The current adapters support semantic text only. Vision-language and cross-modal rules are explicitly skipped. Credentials and configured endpoint values are redacted from configuration debug output; transport errors do not echo endpoint URLs.

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
| `1` | Processing/output failure, or an incomplete review: invalid rules, unresolved conflicts, missing evidence, skipped capabilities, or provider errors. |
| `2` | Invalid provider/model configuration, invalid project, ambiguous entry point, or missing source/PDF/guideline directory. |

For rule and provider failures, both reports are still written with completed findings and issues. Input parsing and output failures can prevent report creation. General PDF mapping limitations and `not_applicable` issues alone do not change the exit code. Zero findings is not evidence of a complete review: check the exit status and issues.

Common fixes:

- Missing LaTeX source: use `--main-tex path/to/main.tex` or place a `.tex` file in the project.
- Missing PDF: compile first or use `--pdf path/to/paper.pdf`.
- Semantic checks skipped: verify provider, model, API key, and rule `requires` capabilities.
- Provider failure: verify endpoint, model, API key, network, and provider account permissions.
- Missing rule: confirm the file extension and required rule fields.

## Current analysis limits

The tool parses the PDF catalog and page tree using `lopdf` and rejects unreadable, encrypted, and page-less documents. It does not compile LaTeX or validate every PDF content operator. Every report states that source-to-PDF mapping, rendered layout, and page text checks are unavailable. No approximate or exact PDF anchors are invented.

LaTeX analysis handles common line-oriented commands, comments (including escaped percent signs), balanced environments, and literal listing bodies. Input paths are resolved from the project root; missing includes fail with their source location. Each source file is visited once. Extensionless graphics resolve common PDF/PNG/JPEG assets, but `graphicspath`, TeX macro expansion, conditional compilation, environments split across files, and arbitrary command syntax are not fully interpreted. Full visual analysis, OCR, and vision-language review remain future extensions.

## Privacy

Hosted providers receive semantic-review prompts and manuscript target text when those rules run. Use Ollama or another local endpoint when manuscript content must remain local. Keep API keys in environment variables and never in guideline files.
