# AGENTS.md

`AGENTS.md` is the repo-local instruction file that tells Minc Agent how to behave inside a project.

## What it does

An `AGENTS.md` file can define:

- coding conventions
- testing expectations
- review rules
- project-specific workflows
- file or directory scope rules

## Scope model

The instructions apply to the directory tree rooted at the folder that contains the file.

That means:

- a repo-root `AGENTS.md` applies broadly
- a deeper nested `AGENTS.md` can specialize behavior for a subdirectory
- more specific instructions win when scopes overlap

## Practical use

- keep instructions concrete and actionable
- prefer project truth over generic process language
- use `/init` if you want the app to help create an `AGENTS.md` file

## Upstream reference

- <https://developers.openai.com/codex/guides/agents-md>
