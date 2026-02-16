# Build guide (custom image tags)

This guide shows how to build the launcher image locally, tag it, and run it via `aws`.

## Requirements

- Docker is installed and running (`docker info`)
- Git installed

## Build (simple)

```sh
git clone https://github.com/graysurf/agent-workspace-launcher.git
cd agent-workspace-launcher

docker build -t agent-workspace-launcher:local .
```

Verify the image:

```sh
docker run --rm -it agent-workspace-launcher:local --help
```

## Use local tag with `aws`

zsh:

```sh
source ./scripts/aws.zsh
export AWS_IMAGE="agent-workspace-launcher:local"

aws --help
aws create OWNER/REPO
```

bash:

```sh
source ./scripts/aws.bash
export AWS_IMAGE="agent-workspace-launcher:local"

aws --help
aws create OWNER/REPO
```

Executable mode (no completion):

```sh
export AWS_IMAGE="agent-workspace-launcher:local"
./scripts/aws.bash --help
./scripts/aws.bash create OWNER/REPO
```

One-off image override:

```sh
AWS_IMAGE="agent-workspace-launcher:local" aws ls
```

## Reproducible pinning

`VERSIONS.env` is the source of truth for pinned upstream refs used by release automation.

Build from the pinned values:

```sh
set -euo pipefail
set -a
source ./VERSIONS.env
set +a

docker build -t agent-workspace-launcher:local \
  --build-arg AGENT_KIT_REF="$AGENT_KIT_REF" \
  .
```

Override refs manually (branch/tag/SHA):

```sh
docker build -t agent-workspace-launcher:local \
  --build-arg AGENT_KIT_REF="main" \
  .
```

Build from an alternate `agent-kit` source:

```sh
docker build -t agent-workspace-launcher:local \
  --build-arg AGENT_KIT_REPO="https://github.com/<you>/agent-kit.git" \
  --build-arg AGENT_KIT_REF="main" \
  .
```

## Architecture note (Rust cutover)

The active launcher entrypoint is a Rust `agent-workspace` CLI.
Legacy zsh bundle-generation paths are not part of the primary build flow.

## Cross-platform builds (buildx)

Build amd64 on Apple Silicon and load locally:

```sh
docker buildx build \
  --platform linux/amd64 \
  -t agent-workspace-launcher:local-amd64 \
  --load \
  .
```

Build amd64+arm64 and push:

```sh
docker buildx build \
  --platform linux/amd64,linux/arm64 \
  -t <your-registry>/agent-workspace-launcher:local \
  --push \
  .
```

## Linux / WSL notes

- Linux may hit `permission denied` on `/var/run/docker.sock`.
  - Quick workaround: `AWS_DOCKER_ARGS=(--user 0:0)`
  - Alternative: add the socket group via `--group-add ...`
- WSL2: enable Docker Desktop WSL integration.

zsh/bash function style:

```sh
AWS_DOCKER_ARGS=(--user 0:0)
```

Executable style:

```sh
export AWS_DOCKER_ARGS="--user 0:0"
```
