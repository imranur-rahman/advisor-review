## Context

The OpenRouter adapter shares an OpenAI-compatible request path with other providers. Attribution is specific to OpenRouter and must not alter requests sent to the other adapters.

## Goals / Non-Goals

**Goals:**
- Read attribution values from optional environment variables.
- Attach configured values only to OpenRouter requests.

**Non-Goals:**
- Add command-line flags or persistent configuration for attribution.
- Make attribution mandatory or set application identity defaults.
- Add attribution categories or change provider routing.

## Decisions

- Use `OPENROUTER_HTTP_REFERER` and `OPENROUTER_APP_TITLE` as the configuration names, matching the existing OpenRouter environment-variable convention.
- Set `HTTP-Referer` and `X-OpenRouter-Title` only when their respective values are configured and only in the OpenRouter branch of request construction. This keeps the feature opt-in and prevents provider leakage.
- Preserve the existing request body, endpoint, authentication, and behavior for all other providers.

## Risks / Trade-offs

- OpenRouter may not attribute an app when only a title is configured → document that the referer is the primary app identifier and both values remain independently optional.
