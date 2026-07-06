# Authentication

Minc Agent is Minc-first by default.

For the built-in `minc` provider, the primary happy path does not require an OpenAI login flow. The app talks to MincAPI directly and uses the provider configuration baked into the fork.

## Default behavior

- Default provider: `minc`
- Default base URL: `https://mincapi.space-z.ai`
- Default UX: no OpenAI sign-in flow as the primary setup path

## Advanced provider compatibility

The inherited runtime still supports additional provider styles. Depending on the provider you configure, authentication may come from:

- an environment variable such as `env_key`
- a command-backed auth flow under `model_providers.<id>.auth`
- a provider that explicitly sets `requires_openai_auth = true`

Those advanced paths are compatibility features, not the main onboarding story for this fork.

## MCP and app authentication

Some MCP servers or connected apps may still require their own tokens, browser sign-in, or environment variables. Those flows are separate from the Minc provider itself.

## Upstream reference

If you need background on inherited provider concepts from upstream Codex, OpenAI’s auth docs can still be useful as reference material, but they do not describe the primary Minc Agent experience:

- <https://developers.openai.com/codex/auth>
