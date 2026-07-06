# Execution policy

Execution policy in Minc Agent is the combination of:

- approval policy
- sandbox or permission profile settings
- tool-specific mediation inside the local runtime

## What it controls

Execution policy determines whether the agent can:

- inspect files
- run local shell commands
- apply patches
- call MCP tools
- proceed automatically or pause for user approval

## Practical rule of thumb

- tighter settings are better for exploration and review
- broader settings are useful when you want the agent to implement changes end-to-end
- the provider itself does not execute tools directly in Minc mode; the local runtime stays in control

See [sandbox & approvals](./sandbox.md) for the user-facing behavior.

## Upstream reference

- <https://developers.openai.com/codex/exec-policy>
