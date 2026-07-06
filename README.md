<p align="center"><strong>Minc Agent</strong> is a local coding agent fork with a terminal UI, light-blue branding, and MincAPI-backed inference.
<p align="center">
  <img src="https://github.com/openai/codex/blob/main/.github/codex-cli-splash.png" alt="Minc Agent splash" width="80%" />
</p>
</br>
Use `/model` to switch between Minc modes: `Auto`, `Instant`, `Low Reasoning`, and `High Reasoning`.
</br>MincAPI docs live at <a href="https://mincapi.space-z.ai/#quickstart">mincapi.space-z.ai/#quickstart</a>.</p>

---

## Quickstart

### Running Minc Agent

This fork keeps the existing `codex` binary name and defaults to the built-in `minc` provider.

Install the pinned Rust toolchain first. The repo already includes [`codex-rs/rust-toolchain.toml`](./codex-rs/rust-toolchain.toml), so `rustup` will pick the right version automatically once it is installed.

```shell
rustup toolchain install 1.95.0
pnpm run install:rust
pnpm run dev
```

If you prefer `just`, the equivalent commands are:

```shell
just install
just minc
```

Helpful shortcuts from the repo root:

```shell
pnpm run dev:tui
pnpm run check
pnpm run test:minc
```

### Using MincAPI

Minc Agent talks to `https://mincapi.space-z.ai` by default.
You can override the provider base URL in config if you need to target another deployment.

The main in-app model picker exposes the four Minc modes directly:

- `Auto`
- `Instant`
- `Low Reasoning`
- `High Reasoning`

## Docs

- [**MincAPI Quickstart**](https://mincapi.space-z.ai/#quickstart)
- [**Contributing**](./docs/contributing.md)
- [**Installing & building**](./docs/install.md)
- [**Open source fund**](./docs/open-source-fund.md)

This repository is licensed under the [Apache-2.0 License](LICENSE).
