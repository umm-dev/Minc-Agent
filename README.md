<p align="center">
  <strong>Minc Agent</strong> is a local coding agent with a terminal UI, Minc branding, and <a href="https://mincapi.space-z.ai/#quickstart">MincAPI</a>-backed inference.
</p>
<p align="center">
  It is a separately maintained fork of OpenAI Codex CLI with a Minc-first default experience, local tool execution, and a full Rust CLI/TUI runtime.
</p>

---

## What Minc Agent Is

Minc Agent is a developer-focused CLI/TUI assistant for working inside a real repository on your machine.

It keeps the core local agent workflow:

- terminal chat and full-screen TUI
- approval-governed shell execution
- local patch application
- MCP tools and tool discovery
- session history, resume, and multi-turn workflows
- slash commands including `/model`

This fork changes the default product surface around Minc:

- `MincAPI` is the default model provider
- `/model` switches between `Auto`, `Instant`, `Low Reasoning`, and `High Reasoning`
- the app uses a strict local text-shim so the model can request local tools even though MincAPI does not provide native structured tool calls

Tool execution stays local and mediated by the app. MincAPI does not get direct access to your machine.

## Quickstart

### One Line install

```shell
git clone https://github.com/umm-dev/Minc-Agent.git
cd Minc-Agent
```
### Come back to it anytime

```shell
cd ~/Minc-Agent
pnpm run dev
```

### Run From The Repo Root

```shell
rustup toolchain install 1.95.0
pnpm run install:rust
pnpm run dev
```

If you prefer `just`:

```shell
just install
just minc
```

Other useful root commands:

```shell
pnpm run dev:tui
pnpm run check
pnpm run test:minc
```

### Run From `codex-rs`

```shell
cd codex-rs
cargo run --bin codex --
```

The executable is still named `codex`, but this fork defaults to the built-in `minc` provider.

## Models And Providers

Minc Agent talks to `https://mincapi.space-z.ai` by default.

The normal path in this fork is:

- provider: `minc`
- mode switching with `/model`
- local tools routed through the app instead of the provider

Advanced provider compatibility still exists in the inherited runtime, but the primary onboarding path is Minc-first.

## Local Tools And Approvals

When the model needs to inspect the repo or make changes, Minc Agent can mediate local tools such as:

- `exec_command`
- `apply_patch`
- MCP tools

Those calls are checked and routed locally. Depending on your configuration, the app may ask for approval before running them.

## Project Layout

- [`codex-rs`](./codex-rs): Rust workspace for the CLI, TUI, provider integration, tools, and tests
- [`codex-cli`](./codex-cli): packaging and top-level CLI wrapper
- [`docs`](./docs): setup, usage, policy, and legal-surface docs
- [`scripts`](./scripts): helper scripts used by the repo

## Docs

- [Installing & building](./docs/install.md)
- [Getting started](./docs/getting-started.md)
- [Configuration](./docs/config.md)
- [Authentication](./docs/authentication.md)
- [Slash commands](./docs/slash_commands.md)
- [Sandbox & approvals](./docs/sandbox.md)
- [Contributing](./docs/contributing.md)
- [License](./docs/license.md)
- [MincAPI Quickstart](https://mincapi.space-z.ai/#quickstart)

## License

Minc Agent is licensed under the [Apache-2.0 License](./LICENSE).

This repository is derived from OpenAI Codex and preserves upstream attribution where required. See [NOTICE](./NOTICE) and [docs/license.md](./docs/license.md) for project-specific attribution context.
