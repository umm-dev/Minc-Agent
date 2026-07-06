# Getting started with Minc Agent

Minc Agent is designed for local repository work. A typical session looks like this:

1. Start the app from the repo root with `pnpm run dev` or `just minc`.
2. Ask a coding question or request a change in plain language.
3. Let the agent inspect the repository, run approved local tools, and continue the task across multiple turns.
4. Use slash commands to change models, inspect status, or manage the session.

## First things to try

- Ask for a repo summary: `Read and summarize this repository`
- Ask for a code review: `Review my current changes`
- Ask for an implementation: `Add model switching to the settings flow`
- Ask for a patch explanation: `Explain the changes in this diff`

## Model switching

Use `/model` in the TUI to switch between the four Minc modes:

- `Auto`
- `Instant`
- `Low Reasoning`
- `High Reasoning`

## Approvals and local actions

Minc Agent can inspect files, run commands, and apply patches locally. Depending on your approval and sandbox settings, it may ask before taking actions that change files or access broader parts of the system.

See [sandbox & approvals](./sandbox.md) for the policy model.

## Sessions

Useful built-in flows include:

- `/new` to start a fresh chat
- `/resume` to reopen a previous session
- `/fork` to branch a conversation
- `/status` to inspect current session settings

## Project instructions

If you want the agent to follow repo-specific guidance, add an `AGENTS.md` file to the project. Minc Agent reads those instructions and applies them to the matching directory tree.

See [AGENTS.md](./agents_md.md) for details.
