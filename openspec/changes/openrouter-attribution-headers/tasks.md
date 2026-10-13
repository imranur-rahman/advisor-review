## 1. Provider configuration and request construction

- [x] 1.1 Read the optional `OPENROUTER_HTTP_REFERER` and `OPENROUTER_APP_TITLE` values and attach the corresponding headers only to OpenRouter requests; inspect the request builder against each scenario in the spec.

## 2. User documentation

- [x] 2.1 Document both environment variables, their optional behavior, and the OpenRouter-only scope in `CONFIGURATION.md`; confirm the documented names match the provider configuration.

## 3. Change validation

- [x] 3.1 Validate the change with `openspec validate openrouter-attribution-headers --type change`.
