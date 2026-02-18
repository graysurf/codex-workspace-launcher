# agent-workspace-launcher (Docker Compatibility Image)

`graysurf/agent-workspace-launcher` is the Docker compatibility channel for the
host-native `agent-workspace-launcher` CLI.

The primary project path is host-native install (Homebrew or source build). This
image is for optional Docker-outside-of-Docker (DooD) workflows where the
launcher runs in a container and controls your host Docker daemon.

## What this image does

- Runs workspace lifecycle commands: `create`, `ls`, `exec`, `rm`, `reset`,
  `auth`, `tunnel`.
- Talks to host Docker via `/var/run/docker.sock`.
- Works together with `graysurf/agent-env` (the workspace runtime image).

## Quick start

```sh
docker pull graysurf/agent-workspace-launcher:latest
docker pull graysurf/agent-env:latest
```

Run directly with Docker:

```sh
docker run --rm -it \
  -v /var/run/docker.sock:/var/run/docker.sock \
  -e AGENT_ENV_IMAGE=graysurf/agent-env:latest \
  graysurf/agent-workspace-launcher:latest \
  create OWNER/REPO
```

For day-to-day usage, prefer the repo-provided wrapper scripts:

- `scripts/awl_docker.zsh`
- `scripts/awl_docker.bash`

These wrappers provide `awl_docker` and convenient defaults.

## Tags

- `latest`
- `sha-<short_sha>`
- `vX.Y.Z` (release tags when enabled by release policy)

## Security note

Mounting `/var/run/docker.sock` gives root-equivalent control of the host Docker
daemon. Use only on trusted hosts and with trusted images/configuration.

## Canonical docs

- Repository: <https://github.com/graysurf/agent-workspace-launcher>
- Install guide: `docs/guides/01-install.md`
- Full user guide: `docs/guides/README.md`

