# Design: agent-workspace-launcher

## Goal

Ship a Docker image that exposes `agent-workspace` commands (`auth/create/ls/rm/exec/reset/tunnel`) and can create
Codex-ready workspace containers on the host through Docker-outside-of-Docker (DooD).

## Architecture (Rust cutover)

The runtime has two layers:

1. Host entry: `aws` wrapper (`scripts/aws.zsh` / `scripts/aws.bash`) for ergonomic local usage.
2. Container entry: Rust `agent-workspace` CLI inside `graysurf/agent-workspace-launcher`.

The launcher container talks to the host daemon via `/var/run/docker.sock` and creates/manages workspace containers
(running `graysurf/agent-env:linuxbrew` by default).

Legacy zsh bundle-generation paths are treated as historical context, not active architecture.

## Command surface

Primary user command:

```sh
aws <subcommand> [...args]
```

Equivalent direct container command:

```sh
docker run --rm -it \
  -v /var/run/docker.sock:/var/run/docker.sock \
  graysurf/agent-workspace-launcher:latest \
  <subcommand> [...args]
```

Supported subcommands:

- `auth`
- `create`
- `ls`
- `exec`
- `rm`
- `reset`
- `tunnel`

## Env model

### Host wrapper env (`aws`)

- `AWS_IMAGE`: launcher image tag
- `AWS_DOCKER_ARGS`: extra launcher `docker run` args
- `AWS_AUTH`: auth source mode (`auto|env|none`)

### Launcher/runtime env (`agent-workspace` + agent runtime)

- `AGENT_WORKSPACE_*` variables configure workspace behavior inside the launcher
- `GH_TOKEN` / `GITHUB_TOKEN` drive private GitHub auth flows
- `AGENT_ENV_IMAGE` controls runtime image selection

### Naming exception (intentional)

Codex auth compatibility inputs remain unchanged:

- `CODEX_SECRET_DIR`
- `CODEX_AUTH_FILE`

These are not migrated to `AWS_*`.

## DooD invariants

- Mounting `docker.sock` is root-equivalent host access.
- Any `-v <src>:<dst>` executed by the launcher resolves `<src>` on the host.
- For host-dependent auth/materials, use same-path binds and pass `HOME` through.

Example:

```sh
AWS_DOCKER_ARGS=(
  -e HOME="$HOME"
  -v "$HOME/.config:$HOME/.config:ro"
)
```

## Build and packaging

- Build output: `graysurf/agent-workspace-launcher` image
- CI publish target branch: `docker`
- Multi-arch target: `linux/amd64`, `linux/arm64`
- Upstream pin source: `VERSIONS.env`

## Verification focus

- `aws --help` and `docker run ... --help` parity
- `create` -> workspace provision on host
- `auth` / `reset` / `tunnel` behavior parity
- Published image tag + manifest correctness

See:

- [README.md](../README.md)
- [docs/BUILD.md](BUILD.md)
- [docs/runbooks/INTEGRATION_TEST.md](runbooks/INTEGRATION_TEST.md)
