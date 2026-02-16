# Development Guide

## Setup

1. Create virtual env: `python3 -m venv .venv`
2. Install Python dev deps: `.venv/bin/python -m pip install -r requirements-dev.txt`
3. Ensure Rust toolchain is available (`rustup` + `stable` with `rustfmt` and `clippy`).

## Pre-submit checks (required)

Run all checks below before submitting changes:

- Shell syntax
  - bash: `bash -n $(git ls-files 'scripts/*.sh' 'scripts/*.bash')`
  - zsh: `zsh -n $(git ls-files 'scripts/*.zsh')`
- Shell lint (requires `shellcheck`):
  - `shellcheck $(git ls-files 'scripts/*.sh' 'scripts/*.bash')`
- Python format/lint/smoke:
  - `.venv/bin/python -m ruff format --check .`
  - `.venv/bin/python -m ruff check .`
  - `.venv/bin/python -m pytest -m script_smoke`
- Rust format/build/lint/test:
  - `cargo fmt --all -- --check`
  - `cargo check --workspace`
  - `cargo clippy --workspace --all-targets -- -D warnings`
  - `cargo test -p agent-workspace`

## Optional test commands

- Full Python suite: `.venv/bin/python -m pytest`
- Rust-only quick loop:
  - `cargo check --workspace`
  - `cargo test -p agent-workspace`

## E2E tests (real Docker)

E2E is opt-in and not required for every change.

Pre-flight:

- Docker daemon is running: `docker info >/dev/null`
- Build local launcher image: `docker build -t agent-workspace-launcher:local .`
- Enable E2E with `AWS_E2E=1`

Run a single case (example):

```sh
AWS_E2E=1 \
  AWS_E2E_IMAGE=agent-workspace-launcher:local \
  AWS_E2E_CASE=help \
  .venv/bin/python -m pytest -m e2e tests/e2e/test_aws_cli_cases.py
```

Run the full matrix:

```sh
AWS_E2E=1 AWS_E2E_IMAGE=agent-workspace-launcher:local AWS_E2E_FULL=1 .venv/bin/python -m pytest -m e2e
```

### E2E environment variables

| Env | Required? | Purpose / Affects | Notes / Example |
| --- | --- | --- | --- |
| `AWS_E2E` | yes | Enables real Docker e2e | `1` / `true` / `yes` / `on` |
| `AWS_E2E_FULL` | optional | Runs full CLI plan suite | `1` to run everything |
| `AWS_E2E_CASE` | optional | Select specific CLI plan cases | `help` or `help,ls` |
| `AWS_E2E_IMAGE` | optional | Override launcher image tag used by e2e | `agent-workspace-launcher:local` |
| `AWS_E2E_PUBLIC_REPO` | required for repo-backed cases | Enables public repo create/reset/exec cases | `OWNER/REPO` |
| `AWS_E2E_PRIVATE_REPO` | required for private repo cases | Enables private repo coverage | `OWNER/PRIVATE_REPO` |
| `AWS_E2E_GH_TOKEN` | recommended | Token input for GitHub auth/private repo | Keep separate from your default token |
| `AWS_E2E_ENABLE_AUTH` | optional gate | Enables auth cases | `1` to enable |
| `AWS_E2E_ENABLE_CODEX` | optional gate | Enables Codex auth case | Requires `AWS_E2E_AGENT_PROFILE` |
| `AWS_E2E_AGENT_PROFILE` | required for codex auth | Codex secret profile name | e.g. `work` |
| `AWS_E2E_ENABLE_GPG` | optional gate | Enables GPG auth case | Requires `AWS_E2E_GPG_KEY_ID` |
| `AWS_E2E_GPG_KEY_ID` | required for gpg auth | GPG key id / fingerprint | e.g. output of `git config --global user.signingkey` |
| `AWS_E2E_ENABLE_SSH` | optional gate | Enables SSH create cases | Requires usable SSH credentials |
| `AWS_E2E_ENABLE_TUNNEL` | optional gate | Enables tunnel cases | `1` to enable |
| `AWS_E2E_ENABLE_EXEC_SHELL` | optional gate | Enables interactive `exec` case | `1` to enable |
| `AWS_E2E_ALLOW_RM_ALL` | optional (dangerous) | Enables `rm --all --yes` coverage | Use with care |
| `AWS_E2E_KEEP_WORKSPACES` | optional | Keep created workspaces for debugging | `1` to keep |
| `AWS_E2E_USE_HOST_HOME` | optional | Use host HOME/XDG and preserve `AWS_DOCKER_ARGS` | Needed for Codex/GPG/SSH mounts |
| `AWS_DOCKER_ARGS` | optional | Extra `docker run` args for launcher container | Use host paths for DooD |

## Artifacts

- Test summaries and coverage files are written under `out/tests/`.
