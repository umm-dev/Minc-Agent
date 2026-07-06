# Slash commands

Minc Agent includes the inherited slash-command system from Codex. Commands are entered by starting a message with `/`.

## Common commands

- `/model` choose the active Minc mode
- `/status` inspect current session configuration and token usage
- `/permissions` change what the agent is allowed to do
- `/review` review current changes and look for issues
- `/resume` reopen a saved session
- `/fork` branch the current conversation
- `/mcp` inspect configured MCP tools
- `/plugins` browse plugins
- `/skills` inspect skill-related behavior
- `/quit` or `/exit` leave the app

## Minc-specific note

In this fork, `/model` is the main model-selection surface and exposes:

- `Auto`
- `Instant`
- `Low Reasoning`
- `High Reasoning`

## Tips

- use `/debug-config` when config layering is confusing
- use `/statusline` and `/theme` to tune the TUI
- use `/new` when you want a fresh thread without leaving the app

## Upstream reference

- <https://developers.openai.com/codex/cli/slash-commands>
