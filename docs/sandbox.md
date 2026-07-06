## Sandbox & approvals

Minc Agent can mediate local tools, but what it is allowed to do depends on your active approval and sandbox settings.

## Approvals

Approval policy controls whether the agent pauses before sensitive actions.

Common behavior in this fork:

- repo inspection often proceeds without friction
- file edits, shell commands, or broader system access may require approval
- MCP tool calls are still mediated locally rather than delegated to MincAPI

## Sandboxing

The inherited runtime supports multiple safety postures, including more restrictive read-oriented setups and broader write-enabled setups.

In practice, sandboxing affects:

- which files can be read
- which files can be written
- whether network access is allowed
- whether extra readable roots need to be granted

## Useful commands

- `/permissions` to inspect or change what the agent is allowed to do
- `/sandbox-add-read-dir <absolute_path>` on supported platforms to grant extra read access
- `/setup-default-sandbox` to configure elevated sandbox support where available

## When to loosen restrictions

Consider broader permissions when you want the agent to:

- make multi-file changes
- run project build or test commands
- use MCP tools that need local or network access

## Upstream reference

- <https://developers.openai.com/codex/security>
