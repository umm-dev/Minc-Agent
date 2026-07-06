# Configuration

Minc Agent keeps the inherited layered configuration system from Codex, but this fork is documented Minc-first.

## Common config locations

- user config: `$CODEX_HOME/config.toml`
- default user config home: typically `~/.codex/config.toml`
- project config: `.codex/config.toml` inside a repository
- repo instructions: `AGENTS.md`

## What configuration is usually for

Common settings include:

- choosing or overriding the active model provider
- changing approval and sandbox behavior
- enabling MCP servers, plugins, or apps
- changing TUI preferences and experimental features
- overriding provider base URLs and auth details for advanced setups

## Minc-first defaults

In this fork, the default provider is `minc`, and the common user-facing model choice happens through `/model`.

Most users should not need to hand-edit provider settings just to use Minc Agent.

## Approval and sandbox settings

Approval and sandbox controls are part of normal configuration. Typical areas to review:

- `approval_policy`
- permission profiles
- sandbox settings and readable roots

See [sandbox & approvals](./sandbox.md) for the behavioral model.

## Managed and advanced config

The runtime still supports deeper inherited features such as:

- feature flags
- managed requirements
- MCP server registration
- custom `model_providers`
- hook configuration

If you are debugging layered config behavior, `/debug-config` in the TUI is often the fastest way to see what is currently active.

## Upstream reference

The upstream Codex config docs can still help explain inherited concepts, but they are reference material, not the source of truth for Minc Agent:

- <https://developers.openai.com/codex/config-basic>
- <https://developers.openai.com/codex/config-advanced>
- <https://developers.openai.com/codex/config-reference>
