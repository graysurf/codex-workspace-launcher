# Reference

## Commands

| Command | Purpose |
| --- | --- |
| `aws --help` | Show help |
| `aws auth ...` | Update auth inside a workspace |
| `aws ls` | List workspaces |
| `aws create ...` | Create a workspace |
| `aws exec ...` | Exec into a workspace |
| `aws rm ...` | Remove workspace(s) |
| `aws reset ...` | Reset repos inside a workspace |
| `aws tunnel ...` | Start a VS Code tunnel |

## Wrapper environment variables (host-side)

| Env | Default | Purpose |
| --- | --- | --- |
| `AWS_IMAGE` | `graysurf/agent-workspace-launcher:latest` | Launcher image to run |
| `AWS_DOCKER_ARGS` | (empty) | Extra `docker run` args for the launcher container |
| `AWS_AUTH` | `auto` | `auto\|env\|none`: when `auto`, reuse host `gh` keyring token for `create/reset/auth github` if no `GH_TOKEN`/`GITHUB_TOKEN` |

## Launcher environment variables

See the full table in [README.md](../../../README.md) under “Configuration (env vars)”.

Codex auth naming exception:

- `CODEX_SECRET_DIR` and `CODEX_AUTH_FILE` remain unchanged for compatibility.
