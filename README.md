# advisor-review

`advisor-review` is a local-first CLI for reviewing LaTeX manuscripts against advisor, journal, conference, and project guidelines. It analyzes LaTeX source and the compiled PDF, then writes anchored findings as JSON and Markdown.

## Install from crates.io

Once released:

```bash
cargo install advisor-review
advisor-review --help
```

## Run a review

```bash
advisor-review review \
  --project ./paper \
  --guidelines ./guidelines \
  --output ./review
```

The project must contain `main.tex` and `main.pdf` by default. Results are written to:

```text
review/findings.json
review/findings.md
```

For complete CLI, rule, provider, output, and troubleshooting documentation, see [CONFIGURATION.md](CONFIGURATION.md).

## Build from source

```bash
git clone git@github.com:imranur-rahman/advisor-review.git
cd advisor-review
cargo run -- review --project ./paper --guidelines ./guidelines --output ./review
```

Run tests with:

```bash
cargo test
```

## Publishing

Release and crates.io instructions are in [PUBLISHING.md](PUBLISHING.md).
