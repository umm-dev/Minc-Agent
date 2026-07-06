# Sample configuration

Here is a small Minc-first example:

```toml
model_provider = "minc"
approval_policy = "on-request"

[features]
code_mode = true

[mcp_servers.github]
bearer_token_env_var = "CODEX_GITHUB_PERSONAL_ACCESS_TOKEN"
```

## Notes

- `model_provider = "minc"` is already the intended default in this fork.
- `approval_policy = "on-request"` keeps local actions approval-governed.
- MCP configuration is optional and depends on the servers you want to enable.

For the broader configuration model, see [Configuration](./config.md).
