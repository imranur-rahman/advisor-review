## Purpose

Allows users to associate OpenRouter requests with this application in OpenRouter's analytics and app rankings using optional request attribution headers.

## ADDED Requirements

### Requirement: Configure OpenRouter attribution headers
The system SHALL allow users to configure the `HTTP-Referer` and `X-OpenRouter-Title` headers through `OPENROUTER_HTTP_REFERER` and `OPENROUTER_APP_TITLE` environment variables. When configured, the system SHALL send each header on OpenRouter requests. When unset, the corresponding header SHALL be omitted. The system SHALL NOT send these headers to other providers.

#### Scenario: OpenRouter request includes configured attribution
- **WHEN** `OPENROUTER_HTTP_REFERER` and `OPENROUTER_APP_TITLE` are set and the selected provider is OpenRouter
- **THEN** the request includes `HTTP-Referer` and `X-OpenRouter-Title` with the configured values

#### Scenario: OpenRouter request omits unset attribution
- **WHEN** one or both attribution environment variables are unset
- **THEN** the corresponding header or headers are omitted from the OpenRouter request

#### Scenario: Other providers do not receive OpenRouter attribution
- **WHEN** either attribution environment variable is set and a provider other than OpenRouter is selected
- **THEN** the request does not include either OpenRouter attribution header
