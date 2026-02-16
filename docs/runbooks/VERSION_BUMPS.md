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

## Publish

Publish workflow triggers on pushes to branch `docker` (see `.github/workflows/publish.yml`).

Recommended pattern:

1. Land changes on `main` via PR.
2. Fast-forward `docker` to `main` when ready to publish.
3. Verify `latest` and `sha-<short>` tags in Docker Hub / GHCR.
