## Installing & building

### System requirements

| Requirement                 | Details                                                         |
| --------------------------- | --------------------------------------------------------------- |
| Operating systems           | macOS 12+, Ubuntu 20.04+/Debian 10+, or Windows 11 **via WSL2** |
| Git (recommended)           | Helpful for diff, review, and repo-aware workflows              |
| RAM                         | 4 GB minimum, 8 GB recommended                                  |

### Build from source

```bash
# Clone the fork and move into the Rust workspace.
git clone https://github.com/umm-dev/Minc-Agent.git
cd Minc-Agent/codex-rs

# Install Rust if needed.
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
source "$HOME/.cargo/env"
rustup toolchain install 1.95.0
rustup component add rustfmt
rustup component add clippy

# Install workspace helpers.
cargo install --locked just
cargo install --locked dotslash
cargo install --locked cargo-nextest

# Build and run.
cargo build
cargo run --bin codex --
```

From the repository root, equivalent commands are:

```bash
pnpm run install:rust
pnpm run dev
```

Or with `just`:

```bash
just install
just minc
```

### DotSlash

This repo uses [DotSlash](https://dotslash-cli.com/) for some pinned development tools, such as `buildifier`. If `just fmt` reports that `dotslash` is missing, install it with:

```bash
cargo install --locked dotslash
```

### Common development commands

```bash
# From the repo root
pnpm run check
pnpm run test:minc

# From codex-rs
just fmt
just fix -p <crate>
just test -p codex-tui
```

### Logging

Minc Agent is written in Rust and honors `RUST_LOG`.

To enable a plaintext TUI log for a run:

```bash
codex -c log_dir=./.codex-log
tail -F ./.codex-log/codex-tui.log
```

The non-interactive path (`codex exec`) prints its output inline, so you usually do not need a separate log tail unless you are debugging startup or provider behavior.
