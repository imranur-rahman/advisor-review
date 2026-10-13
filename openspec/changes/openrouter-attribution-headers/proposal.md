## Why

OpenRouter users need to identify this application in OpenRouter usage analytics and rankings. The OpenAI-compatible request path currently sends no app attribution headers.

## What Changes

- Allow users to configure OpenRouter's `HTTP-Referer` and `X-OpenRouter-Title` attribution headers through environment variables.
- Send configured attribution headers only on OpenRouter requests; leave them out when unset and for all other providers.

## Capabilities

### New Capabilities
- `openrouter-attribution`: Optional app attribution for requests sent through OpenRouter.

### Modified Capabilities

## Impact

- Provider configuration and OpenRouter request construction in `src/config.rs` and `src/providers.rs`.
- Provider configuration guidance in `CONFIGURATION.md`.
