## Contributing

Minc Agent accepts community contributions.

This repository is a separately maintained fork, so the best contributions are the ones that are:

- scoped to a clear problem
- easy to review
- tested where appropriate
- documented when they change user-visible behavior

## How to contribute

1. Open or find an issue that describes the bug, feature, or change in behavior.
2. Create a focused branch from `main`.
3. Keep the change small enough to review comfortably.
4. Run the relevant checks before opening a pull request.
5. Explain the change clearly in the PR body.

## AI-assisted contributions

AI-assisted code is welcome, including code written with agents, as long as the final pull request is reviewed by a human who understands the change and verifies quality before merge.

Before opening a PR, make sure you can explain:

- what changed
- why it changed
- what you tested
- any risks, tradeoffs, or known limitations

## Recommended workflow

- start with an issue or discussion when the change is large or behavioral
- add or update tests when fixing bugs or changing runtime logic
- update docs if the change affects setup, UX, or configuration
- keep commits and PRs focused

## Useful local commands

From the repo root:

```bash
pnpm run check
pnpm run test:minc
```

From `codex-rs`:

```bash
just fmt
just fix -p <crate>
just test -p <crate>
```

## Pull request expectations

- link the issue or motivation
- describe the user-facing impact
- include tests or explain why none were needed
- avoid unrelated cleanup in the same PR

Maintainers may still request changes, narrower scope, or follow-up work before merging.

## No separate CLA

Minc Agent does not require a separate contributor license agreement. Contributions are handled through the repository’s normal open-source workflow under the project license.

See [CLA](./CLA.md) and [License](./license.md) for the legal-surface summary used by this fork.

## Security reporting

For now, use the GitHub issue tracker for vulnerability reports and security-related concerns.
