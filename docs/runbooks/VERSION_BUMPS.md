# Version bumps (upstream pin)

This repo now uses a single pinned upstream ref in `VERSIONS.env`:

- `AGENT_KIT_REF`: used at image build time to vendor `/opt/agent-kit` (contains the low-level launcher).

Goal: bumps are **reviewable**, **reproducible**, and validated with Rust + Python checks.

## When to bump

- You merged launcher contract changes in `agent-kit` that must ship in this image.
- You need to roll forward/backward to a known-good upstream commit.

## Automated bump (recommended)

Update pin, run checks, build image, and verify embedded ref:

```sh
./scripts/bump_versions.sh --from-main
```

Run real-Docker E2E (full matrix) against the built image:

```sh
./scripts/bump_versions.sh --from-main --run-e2e
```

Pin explicitly (always resolves to a full commit SHA):

```sh
./scripts/bump_versions.sh --agent-kit-ref <ref|sha>
```

Tip: use `--skip-docker` when you only want file updates + local checks.

## Choose a new pin

Recommended: pin by commit SHA (most deterministic).

Example:

```sh
git ls-remote https://github.com/graysurf/agent-kit.git refs/heads/main | awk '{print $1}' | head -n 1
```

## Update `VERSIONS.env`

Set:

- `AGENT_KIT_REF=<sha>`

Legacy `ZSH_KIT_REF` is removed and must not be reintroduced.

## Build a local image with the pinned ref

```sh
set -euo pipefail
set -a
source ./VERSIONS.env
set +a

docker build -t agent-workspace-launcher:local \
  --build-arg AGENT_KIT_REF="$AGENT_KIT_REF" \
  .
```

## Verify the built image pin

```sh
docker run --rm --entrypoint cat agent-workspace-launcher:local /opt/agent-kit/.ref
```

## Required checks before PR

See `DEVELOPMENT.md`. At minimum:

```sh
python3 -m venv .venv
.venv/bin/python -m pip install -r requirements-dev.txt

.venv/bin/python -m ruff format --check .
.venv/bin/python -m ruff check .
.venv/bin/python -m pytest -m script_smoke

cargo fmt --all -- --check
cargo check --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo test -p agent-workspace
```

## Real-Docker E2E (optional)

Minimal case:

```sh
AWS_E2E=1 \
  AWS_E2E_IMAGE=agent-workspace-launcher:local \
  AWS_E2E_CASE=help \
  .venv/bin/python -m pytest -m e2e tests/e2e/test_aws_cli_cases.py
```

Full matrix:

```sh
AWS_E2E=1 AWS_E2E_FULL=1 .venv/bin/python -m pytest -m e2e
```

Notes:

- Destructive coverage (`rm --all --yes`) is gated by `AWS_E2E_ALLOW_RM_ALL=1`.
- Artifacts are written under `out/tests/e2e/`.

## Publish (tag-based split channels)

Release source-of-truth is a semver tag (`vX.Y.Z`) created from `main`.

1. Land version bump changes on `main` via PR.
2. Create and push release tag:

```sh
git -c tag.gpgSign=false tag vX.Y.Z
git push origin vX.Y.Z
```

3. Ensure a GitHub Release exists for the same tag:

```sh
gh release view vX.Y.Z || gh release create vX.Y.Z --title "vX.Y.Z" --notes ""
```

4. Channel trigger rules:
   - Docker channel: `.github/workflows/release-docker.yml` runs from `push.tags: v*` (and optional `workflow_dispatch` reruns).
   - Brew channel: `.github/workflows/release-brew.yml` runs from `push.tags: v*` (and optional `workflow_dispatch` reruns).
   - Both channels must publish artifacts for the same `vX.Y.Z` tag.

Temporary transition note: `.github/workflows/publish.yml` (push to branch `docker`) may remain as a Docker-only compatibility path while migration is in progress. Treat it as fallback only, not the primary release trigger.

## Channel verification checkpoints

Run both checkpoints before closing the release task.

### Docker channel

- Confirm workflow success for the tag:
  - `gh run list --workflow release-docker.yml --limit 5`
- Confirm expected image tags are visible:
  - `docker buildx imagetools inspect docker.io/graysurf/agent-workspace-launcher:vX.Y.Z`
  - `docker buildx imagetools inspect ghcr.io/graysurf/agent-workspace-launcher:vX.Y.Z`
- Verify `latest` and `sha-<short>` tags in Docker Hub + GHCR.

### Brew channel

- Confirm workflow success for the tag:
  - `gh run list --workflow release-brew.yml --limit 5`
- Confirm release assets include matching `*.tar.gz` + `*.sha256` files:
  - `gh release view vX.Y.Z --json assets --jq '.assets[].name'`
- Verify checksums locally:
  - `mkdir -p "${AGENTS_HOME:-$HOME/.agents}/out/release-vX.Y.Z"`
  - `gh release download vX.Y.Z --pattern '*.tar.gz' --pattern '*.sha256' --dir "${AGENTS_HOME:-$HOME/.agents}/out/release-vX.Y.Z"`
  - `cd "${AGENTS_HOME:-$HOME/.agents}/out/release-vX.Y.Z" && for f in *.sha256; do shasum -a 256 -c "$f"; done`

## Update `homebrew-tap` formula

After the Brew channel checks pass, update the tap formula with exact URL + checksum pairs.

```sh
version="vX.Y.Z"
asset_dir="${AGENTS_HOME:-$HOME/.agents}/out/release-${version}"
tap_dir="$HOME/Project/graysurf/homebrew-tap"

mkdir -p "$asset_dir"
gh release download "$version" \
  --pattern "agent-workspace-launcher-${version}-*.tar.gz" \
  --pattern "agent-workspace-launcher-${version}-*.tar.gz.sha256" \
  --dir "$asset_dir"

(
  cd "$asset_dir"
  for sum in *.sha256; do
    shasum -a 256 -c "$sum"
  done
)

cd "$tap_dir"
# Update Formula/agent-workspace-launcher.rb URLs + sha256 values from downloaded checksums.
```

Validate in `homebrew-tap`:

```sh
ruby -c Formula/agent-workspace-launcher.rb
HOMEBREW_NO_AUTO_UPDATE=1 brew style Formula/agent-workspace-launcher.rb
brew tap graysurf/tap "$(pwd)" --custom-remote
brew update-reset "$(brew --repo graysurf/tap)"
HOMEBREW_NO_AUTO_UPDATE=1 brew reinstall agent-workspace-launcher || HOMEBREW_NO_AUTO_UPDATE=1 brew install agent-workspace-launcher
HOMEBREW_NO_AUTO_UPDATE=1 brew test agent-workspace-launcher
```
