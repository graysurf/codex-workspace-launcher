# Integration test checklist

This repo’s “launcher image” is **Docker-outside-of-Docker (DooD)**: the launcher container uses the host Docker
daemon (`/var/run/docker.sock`) to create **workspace containers**.

This checklist is for validating the end-to-end experience after cutting a release tag (`vX.Y.Z`).

## What to verify

- [x] macOS quickstart smoke (zsh wrapper via `aws`; no local build)
- [x] macOS quickstart smoke (bash wrapper via `aws`; no local build)
- [x] Linux exploratory smoke run (captures logs)
- [x] Docker release workflow run URL recorded (`release-docker.yml` for `vX.Y.Z`)
- [x] Docker Hub tags exist (`latest`, `vX.Y.Z`, `sha-<short>`)
- [x] Docker Hub image is multi-arch (`linux/amd64`, `linux/arm64`)
- [x] GHCR tags exist (`latest`, `vX.Y.Z`, `sha-<short>`)
- [x] GHCR image is multi-arch (`linux/amd64`, `linux/arm64`)
- [x] Brew release workflow run URL recorded (`release-brew.yml` for `vX.Y.Z`)
- [x] GitHub Release assets exist for all target tarballs + checksum files
- [x] Downloaded Brew asset checksums verify locally

## macOS quickstart smoke (published images; no local build)

This validates the end-user “Quickstart” path:

- Pull + run `graysurf/agent-workspace-launcher`
- During `create`, the workspace runtime image `graysurf/agent-env:linuxbrew` also gets pulled (so you implicitly
  validate it exists and is runnable on your platform).

Pre-flight:

```sh
docker info >/dev/null
```

zsh wrapper:

```sh
source ./scripts/aws.zsh

# Pulls the launcher image and prints help.
aws --help

# Verifies the launcher container can talk to the host daemon.
aws ls

# End-to-end create (public repo).
aws create graysurf/agent-kit

# Copy the printed workspace name, then:
aws exec <name|container> git -C /work/graysurf/agent-kit status
aws rm <name|container> --yes
```

bash wrapper (run in a separate bash shell to avoid mixing wrappers):

```sh
source ./scripts/aws.bash
aws --help
```

Capture evidence:

- Save the full terminal output to a log file and attach it to the integration testing PR (or paste it in a PR comment).

Evidence (2026-01-20):

- `$CODEX_HOME/out/macos-quickstart-smoke-20260120-080236.log`
- `$CODEX_HOME/out/macos-quickstart-create-20260120-080236.log`

## Automated E2E sanity (AWS naming)

Use the new AWS test/env naming for a minimal real-Docker check:

```sh
AWS_E2E=1 \
  AWS_E2E_CASE=help \
  .venv/bin/python -m pytest -m e2e tests/e2e/test_aws_cli_cases.py
```

Optional full matrix:

```sh
AWS_E2E=1 AWS_E2E_FULL=1 .venv/bin/python -m pytest -m e2e
```

## Linux exploratory smoke (do not claim support yet)

Run on a real Linux host with Docker (rootful):

```sh
# Should print help without talking to the Docker daemon.
docker run --rm -it graysurf/agent-workspace-launcher:latest --help

# Verify Docker daemon connectivity from the launcher container.
docker run --rm -it \
  --user 0:0 \
  -v /var/run/docker.sock:/var/run/docker.sock \
  graysurf/agent-workspace-launcher:latest \
  ls

# End-to-end create (public repo).
docker run --rm -it \
  --user 0:0 \
  -v /var/run/docker.sock:/var/run/docker.sock \
  graysurf/agent-workspace-launcher:latest \
  create graysurf/agent-kit
```

Expected failure mode:

- `permission denied` when accessing `/var/run/docker.sock`. Workaround: run as root (`--user 0:0`) or add the
  docker socket group GID via `--group-add ...`.

Capture evidence:

- Save the full terminal output to a log file and attach it to the integration testing PR (or paste it in a PR comment).

Evidence (2026-01-20; OrbStack on macOS):

- `$CODEX_HOME/out/linux-exploratory-smoke-orbstack-20260120-085812.log`
- `$CODEX_HOME/out/linux-exploratory-create-orbstack-20260120-085812.log`

## Release channel verification

After pushing `vX.Y.Z`, verify:

- GitHub Actions workflow `.github/workflows/release-docker.yml` ran successfully for `vX.Y.Z` (record the run URL).
- Docker Hub has the expected tags:
  - `graysurf/agent-workspace-launcher:latest`
  - `graysurf/agent-workspace-launcher:vX.Y.Z`
  - `graysurf/agent-workspace-launcher:sha-<short>`
- GHCR has the expected tags:
  - `ghcr.io/graysurf/agent-workspace-launcher:latest`
  - `ghcr.io/graysurf/agent-workspace-launcher:vX.Y.Z`
  - `ghcr.io/graysurf/agent-workspace-launcher:sha-<short>`
- The published images are multi-arch:

```sh
docker buildx imagetools inspect graysurf/agent-workspace-launcher:latest
docker buildx imagetools inspect ghcr.io/graysurf/agent-workspace-launcher:latest
```

Expected platforms include `linux/amd64` and `linux/arm64`.

Then verify Brew assets:

- GitHub Actions workflow `.github/workflows/release-brew.yml` ran successfully for `vX.Y.Z` (record the run URL).
- GitHub Release contains per-target files:
  - `agent-workspace-launcher-vX.Y.Z-x86_64-apple-darwin.tar.gz` + `.sha256`
  - `agent-workspace-launcher-vX.Y.Z-aarch64-apple-darwin.tar.gz` + `.sha256`
  - `agent-workspace-launcher-vX.Y.Z-x86_64-unknown-linux-gnu.tar.gz` + `.sha256`
  - `agent-workspace-launcher-vX.Y.Z-aarch64-unknown-linux-gnu.tar.gz` + `.sha256`
- Local checksum verification:

```sh
version="vX.Y.Z"
out_dir="${AGENTS_HOME:-$HOME/.agents}/out/release-${version}"
mkdir -p "$out_dir"

gh release download "$version" \
  --pattern "agent-workspace-launcher-${version}-*.tar.gz" \
  --pattern "agent-workspace-launcher-${version}-*.tar.gz.sha256" \
  --dir "$out_dir"

(
  cd "$out_dir"
  for sum in *.sha256; do
    shasum -a 256 -c "$sum"
  done
)
```

Evidence (2026-01-20):

- Docker Hub verification (before GHCR publish): https://github.com/graysurf/agent-workspace-launcher/actions/runs/21154177325
- Docker Hub inspect log: `$CODEX_HOME/out/ci-publish-verification-20260120-081548.log`
- PR build (no publish): https://github.com/graysurf/agent-workspace-launcher/actions/runs/21155245507
- GHCR verification (push to `main`): https://github.com/graysurf/agent-workspace-launcher/actions/runs/21155498181
- GHCR inspect log: `$CODEX_HOME/out/ghcr-verification-20260120-084948.log`
- GHCR not found (pre-merge check): `$CODEX_HOME/out/ghcr-verification-20260120-083359.log`
