# Skills

Skills are reusable instruction bundles that help Minc Agent handle specific tasks more effectively.

They usually package workflow guidance, tool-routing hints, or reference material for a particular domain.

## What they are for

Skills are useful when you want the agent to have consistent behavior for:

- documentation work
- plugin creation
- code review flows
- issue triage
- specialized external tool usage

## How they fit into Minc Agent

Minc Agent inherits the skills system from Codex, but the runtime remains local:

- the app decides which skills are available
- skill instructions are read from disk
- the active model uses those instructions as part of the prompt context

## Practical use

- ask the agent to use a named skill when one exists
- keep skills focused and task-specific
- prefer local project truth over generic guidance when the two conflict

## Upstream reference

- <https://developers.openai.com/codex/skills>
