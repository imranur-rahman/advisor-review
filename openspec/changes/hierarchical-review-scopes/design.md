## Context

The existing Rust analyzer produces flat targets, section headings, and raw document text. Its specialized figure/table/code extraction is useful and will be retained. See proposal.md for motivation.

## Goals / Non-Goals

Goals: independent scopes, cross-file hierarchy, exact source offsets where available, contextual semantic review, bounded input and explicit coverage.

Non-goals: full TeX expansion, OCR, PDF source mapping, publishing, or live model evaluation.

## Decisions

- Expand includes into an ordered source stream with per-character origins before analysis. Run specialized extraction against this stream and remap its anchors. A shared hierarchy builder normalizes common prose commands while preserving citation/math text and offsets. This avoids using file boundaries as paragraph or section boundaries.
- Use document -> sections/subsections -> paragraphs -> sentences; heading targets are children of sections. Sections include descendants by default, with direct-body views available. Content preceding the first section attaches to document. Stable deterministic IDs identify nodes within a run.
- Retain raw source alongside normalized prose and mapping segments; use byte offsets (half-open) and one-based line/byte-column positions. Multiple source spans represent included files without fabricated contiguous locations.
- Default rules to normalized prose; support text_view: source, context: none/paragraph/neighbors/section, section: exact heading title, and include_subsections. Preserve document name and accept manuscript alias. Explicit heading scope replaces former section-heading behavior.
- Add execution options and detailed results while retaining the simple run wrapper. Track per-scope evaluations and coverage independently of findings. Context is supplied separately and never expands rule ownership.
- Add a contextual multi-finding provider method with a legacy adapter for existing test providers. Structured findings cite supplied target IDs and exact quotations. Locate quotations against supplied mapped text before accepting evidence.
- Enforce a conservative UTF-8 byte input budget (an upper bound proxy rather than claiming model-specific token accuracy). Split at paragraph/section boundaries, fall back to character-safe chunks, and synthesize bounded chunk notes. Budget failures and summary-only global synthesis remain partial; no automatic retries or live requests in tests.

## Risks / Trade-offs

- Scientific sentence segmentation is heuristic -> cover common abbreviations, decimals, citations, math, punctuation, and Unicode; document remaining limits.
- Summaries omit global information -> preserve original evidence, bounded synthesis, and partial coverage rather than claim a global pass.
- Normalization changes matches -> schema 2.0 and explicit source/heading migration documentation.

## Migration Plan

Introduce schema 2.0 while retaining legacy anchor.source where one contiguous span is appropriate. Keep existing CLI invocation valid and add optional scope/budget flags. Document source view and heading scope. No release or push is part of this change.
