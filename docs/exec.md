# Non-interactive mode

Minc Agent keeps the inherited `codex exec` path for non-interactive runs.

Use it when you want to run a prompt against the current repo without opening the full TUI.

## Example

```bash
codex exec "summarize the architecture of this repository"
```

## Typical uses

- quick repo summaries
- scripted review or analysis tasks
- CI or automation experiments
- one-shot implementation prompts

## Important behavior

- it uses the configured model provider, which is `minc` by default in this fork
- it still respects config, provider settings, and approval policy behavior
- output is printed inline rather than through the full-screen TUI

## Upstream reference

For extra background on the inherited non-interactive model, see:

- <https://developers.openai.com/codex/noninteractive>
