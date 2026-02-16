---
name: release-agent-workspace-launcher
description: "Release agent-workspace-launcher: record pinned agent-kit ref in CHANGELOG, run local real-Docker e2e, and publish split Docker/Brew channels from release tags."
---

# Release Workflow (agent-workspace-launcher)

This repo releases from semver tags (`vX.Y.Z`) with split publish channels: Docker images and Brew artifacts.

This project-specific workflow extends the base `release-workflow` with three non-negotiables:

1) Record the active upstream pin (`AGENT_KIT_REF` from `VERSIONS.env`) in the release entry in `CHANGELOG.md`.
2) Run local real-Docker E2E (full matrix) and **abort** if it fails.
3) Treat Docker and Brew as separate publish channels that must both be verified from the same release tag.

## Contract

Prereqs:

- Run in this repo root.
- Working tree is clean before running the E2E gate.
- Docker is available (E2E is real Docker): `docker info` succeeds.
- E2E environment is configured (via `direnv`/`.envrc` + `.env`, or equivalent).
  - At minimum, repo-backed cases require `AWS_E2E_PUBLIC_REPO`.
  - Auth-heavy cases require additional secrets/mounts (see `DEVELOPMENT.md`).
- `git` available on `PATH`.
- Optional (recommended): `gh` available + `gh auth status` succeeds (for tagging / releases / branch ops).

Inputs:

- Release version: `vX.Y.Z`
- Optional: release date (`YYYY-MM-DD`; defaults to today)
- Optional: E2E image tag (defaults to `aws-launcher:e2e`)

Outputs:

- `CHANGELOG.md` updated with a `## vX.Y.Z - YYYY-MM-DD` entry that includes:
  - `### Upstream pins`
    - `- agent-kit: <AGENT_KIT_REF>`
- Local E2E run result (pass required; artifacts under `out/tests/e2e/`)
- Optionally:
  - Git tag `vX.Y.Z` pushed
  - Docker channel publish completed for `vX.Y.Z`
  - Brew channel publish completed for `vX.Y.Z` (assets + checksums)

Stop conditions:

- Local E2E fails: stop immediately; do not publish; report the failure output and ask how to proceed.
- Changelog audit fails (missing pins / bad version heading / placeholders): stop; fix before publishing.
- Docker or Brew publish fails: stop and report channel-specific failure evidence; do not declare release complete.

## Key rule: E2E gate is mandatory

Run E2E before publishing and before any irreversible actions.

Recommended: run via `direnv exec .` so your `.env` is applied:

```sh
set -euo pipefail

direnv exec . ./scripts/bump_versions.sh \
  --from-main \
  --image-tag aws-launcher:e2e \
  --run-e2e
```

Notes:

- `--run-e2e` forces `AWS_E2E=1`, `AWS_E2E_FULL=1`, and sets `AWS_E2E_IMAGE=<image-tag>`.
- Destructive `rm --all --yes` coverage remains gated by `AWS_E2E_ALLOW_RM_ALL=1`.

## Workflow

1. Decide version + date
   - Version: `vX.Y.Z`
   - Date: `YYYY-MM-DD` (default: `date +%Y-%m-%d`)

2. Run mandatory local E2E gate (real Docker)
   - Use the command in “Key rule: E2E gate is mandatory” above.
   - If it fails: stop and report (use `.agents/skills/release-workflow/references/OUTPUT_TEMPLATE_BLOCKED.md`).

3. Prepare the changelog (records upstream pins)
   - Use the helper that moves `## Unreleased` into a new release entry and injects pins from `VERSIONS.env`:
     - `./scripts/release_prepare_changelog.sh --version vX.Y.Z`
   - Review `CHANGELOG.md` (fill out wording; remove `- None` if you added real bullets).

4. Run required repo checks (per `DEVELOPMENT.md`)
   - `bash -n $(git ls-files 'scripts/*.sh' 'scripts/*.bash')`
   - `zsh -n $(git ls-files 'scripts/*.zsh')`
   - `shellcheck $(git ls-files 'scripts/*.sh' 'scripts/*.bash')`
   - `.venv/bin/python -m ruff format --check .`
   - `.venv/bin/python -m ruff check .`
   - `.venv/bin/python -m pytest -m script_smoke`
   - `cargo fmt --all -- --check`
   - `cargo check --workspace`
   - `cargo clippy --workspace --all-targets -- -D warnings`
   - `cargo test -p agent-workspace`

5. Commit the release notes
   - Suggested message: `chore(release): vX.Y.Z`
   - Do not run `git commit` directly; use the repo’s Semantic Commit helper (see `AGENTS.md`).

6. Audit (strict)
   - Run after committing (audit requires a clean working tree):
     - `./scripts/release_audit.sh --version vX.Y.Z --branch main --strict`

7. Tag the release (required for split channel publish)
   - Create: `git tag vX.Y.Z`
   - Push: `git push origin vX.Y.Z`
   - Optional GitHub Release:
     - Extract notes from `CHANGELOG.md` and publish with `gh release create`.

8. Publish Docker channel (tag-driven)
   - Confirm Docker release workflow ran for `vX.Y.Z` (for example: `release-docker.yml`).
   - If Docker uses manual dispatch, run it against the tag and record the run URL.
   - Transition note: if a temporary legacy workflow still exists (for example `publish.yml`), treat it as compatibility only, not the primary release contract.

9. Publish Brew channel (tag-driven)
   - Confirm Brew release workflow ran for `vX.Y.Z` (for example: `release-brew.yml`).
   - Verify release assets are present for each supported target (`*.tar.gz` + matching `*.sha256`).
   - If one channel fails, retry only that channel after fixing the issue.

10. Verify channel outcomes
   - Docker: follow `docs/runbooks/INTEGRATION_TEST.md` and capture run URL + image tag evidence.
   - Brew: verify GitHub Release assets and checksums for `vX.Y.Z` are complete before tap update work.

## Helper scripts (project)

- Prepare changelog + inject upstream pins: `scripts/release_prepare_changelog.sh`
- Audit changelog entry (pins + basic release checks): `scripts/release_audit.sh`
- Build + verify + local E2E (full matrix): `scripts/bump_versions.sh --run-e2e`

## Output templates

- Success: `.agents/skills/release-workflow/references/OUTPUT_TEMPLATE.md`
- Blocked: `.agents/skills/release-workflow/references/OUTPUT_TEMPLATE_BLOCKED.md`
