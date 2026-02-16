# Release Guide (agent-workspace-launcher)

This repo publishes launcher images from the `docker` branch (see `.github/workflows/publish.yml`).

## Preconditions

- Run at repo root.
- Clean worktree: `git status -sb`
- Target branch checked out (usually `main`): `git branch --show-current`
- Docker available: `docker info >/dev/null`
- GitHub CLI authenticated: `gh auth status`
- E2E env prepared (`direnv`/`.envrc` + `.env`, or equivalent).
  - Repo-backed cases need `AWS_E2E_PUBLIC_REPO`.
  - Auth-heavy cases need additional `AWS_E2E_*` inputs (see `DEVELOPMENT.md`).

## Steps

1. Decide release metadata
   - Version: `vX.Y.Z`
   - Date: `YYYY-MM-DD` (default: `date +%Y-%m-%d`)

2. Run mandatory local gate (real Docker E2E)

   ```sh
   set -euo pipefail
   set -a; source ./VERSIONS.env; set +a

   direnv exec . ./scripts/bump_versions.sh \
     --from-main \
     --image-tag aws-launcher:e2e \
     --run-e2e
   ```

   Stop on failure.

3. Update `CHANGELOG.md`
   - Create release section from `## Unreleased`:
     - `./scripts/release_prepare_changelog.sh --version vX.Y.Z --date YYYY-MM-DD`
   - Clean up placeholder bullets.

4. Run required checks (`DEVELOPMENT.md`)
   - `cargo fmt --all -- --check`
   - `cargo clippy --workspace --all-targets -- -D warnings`
   - `cargo test --workspace`
   - `.venv/bin/python -m ruff format --check .`
   - `.venv/bin/python -m ruff check .`
   - `.venv/bin/python -m pytest -m script_smoke`

5. Commit changelog
   - Suggested message: `chore(release): vX.Y.Z`
   - Use semantic commit helper (per `AGENTS.md`).

6. Audit
   - `./scripts/release_audit.sh --version vX.Y.Z --branch main --strict`

7. Tag + push
   - `git -c tag.gpgSign=false tag vX.Y.Z`
   - `git push origin vX.Y.Z`

8. Publish GitHub Release
   - Extract notes:
     - `./scripts/release_notes_from_changelog.sh --version vX.Y.Z --output "$CODEX_HOME/out/release-notes-vX.Y.Z.md"`
   - Create release:
     - `gh release create vX.Y.Z -F "$CODEX_HOME/out/release-notes-vX.Y.Z.md" --title "vX.Y.Z"`
   - Verify:
     - `gh release view vX.Y.Z`

9. Trigger image publish
   - `git fetch origin`
   - `git checkout docker`
   - `git merge --ff-only origin/main`
   - `git push origin docker`
   - `git checkout main`

10. Verify publish artifacts
    - Follow `docs/runbooks/INTEGRATION_TEST.md`.
    - Record workflow URL + published tags/manifests in release evidence.
