---
name: release-docker-image
description: Release agent-workspace-launcher container images in CI (default) with local fallback.
---

# Release Docker Image

## Contract

Prereqs:

- Run in repo root (git work tree) for default behavior.
- `gh` authenticated (`gh auth status`) with access to this repository.
- GitHub workflow `.github/workflows/release-docker.yml` available.
- Repo secrets for Docker Hub publish:
  - `DOCKERHUB_USERNAME`
  - `DOCKERHUB_TOKEN`
- GHCR publish uses GitHub Actions `GITHUB_TOKEN` with `packages: write` (already defined in workflow permissions).
- `VERSIONS.env` must contain `AGENT_KIT_REF`.

Local fallback prereqs (only when forcing local publish):

- `docker` with `buildx` enabled.
- Docker Hub + GHCR credentials via `AWL_DOCKER_RELEASE_*` env vars.

Inputs:

- CI entrypoint CLI:
  - `--version <vX.Y.Z>` (recommended)
  - `--input-ref <git-ref>`
  - `--workflow-ref <git-ref>` (which ref to load workflow file from)
  - `--repo <owner/name>`
  - `--publish-version-tag|--no-publish-version-tag`
  - `--no-wait`
- Local fallback env/CLI: same contract as `release-docker-image.sh` (`AWL_DOCKER_RELEASE_*`).

Outputs:

- CI workflow dispatch + optional wait for completion (`release-docker.yml`).
- Multi-arch image publish to Docker Hub + GHCR (linux/amd64, linux/arm64 by default).
- Tags include `latest`, `sha-<short_sha>`, and optional `vX.Y.Z`.

Exit codes:

- `0`: success
- `1`: failure
- `2`: usage error

Failure modes:

- Missing/invalid release tag/ref or publish policy mismatch.
- Missing repo secrets (Docker Hub publish disabled/failed).
- GHCR permission issues (`packages: write` missing or token scope mismatch).
- Workflow dispatch/watch failure.
- Docker login/buildx/push failure (local fallback mode).

## Scripts (only entrypoints)

- CI-first entrypoint:
  - `<PROJECT_ROOT>/.agents/skills/release-docker-image/scripts/release-docker-image-ci.sh`
- Local fallback entrypoint:
  - `<PROJECT_ROOT>/.agents/skills/release-docker-image/scripts/release-docker-image.sh`

## Workflow

1. CI-first release (recommended):
   - `.agents/skills/release-docker-image/scripts/release-docker-image-ci.sh --version vX.Y.Z`
2. Non-blocking dispatch only (do not wait):
   - `.agents/skills/release-docker-image/scripts/release-docker-image-ci.sh --version vX.Y.Z --no-wait`
3. Local fallback publish (only when CI path is unavailable):
   - `.agents/skills/release-docker-image/scripts/release-docker-image.sh --version vX.Y.Z`
4. Verify pushed manifests:
   - `docker buildx imagetools inspect graysurf/agent-workspace-launcher:vX.Y.Z`
   - `docker buildx imagetools inspect ghcr.io/graysurf/agent-workspace-launcher:vX.Y.Z`
