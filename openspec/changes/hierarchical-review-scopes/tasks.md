## 1. Document model and extraction

- [x] 1.1 Add schema 2.0 hierarchy, text views, mapping, and coverage models; verify serialization and compilation.
- [x] 1.2 Expand included sources and build sections, paragraphs, sentences, and headings with specialized targets; verify cross-file, nested-section, scientific-boundary, and anchor tests.

## 2. Rule execution and providers

- [x] 2.1 Add independent scope filtering, rule selectors/context/views, and per-scope execution accounting; verify isolation and failure-continuation tests.
- [x] 2.2 Implement contextual multi-finding requests and evidence validation; verify mocked request content, evidence mapping, and malformed response tests.
- [x] 2.3 Add bounded chunk review and synthesis with partial global coverage; verify budget, evidence preservation, and incomplete-run tests without live providers.

## 3. Integration and documentation

- [x] 3.1 Integrate CLI flags, hierarchy/coverage JSON, and grouped Markdown; verify end-to-end four-scope reports and exit codes.
- [x] 3.2 Document rule examples, migration, budgets, and limitations; run formatting, full test suite, and strict OpenSpec validation.
