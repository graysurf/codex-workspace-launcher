# Release Guide (agent-workspace-launcher)

Release source-of-truth is a semver tag (`vX.Y.Z`) created from `main`.
Each tag fans out into two independent channels:

- Docker channel: publish OCI images to Docker Hub + GHCR.
- Brew channel: publish Homebrew assets (`.tar.gz` + `.sha256`) on the same GitHub Release.

Temporary transition note: `.github/workflows/publish.yml` (`docker` branch push) is Docker-only compatibility fallback while split workflows roll out. Do not use it as the primary trigger.

## Release contract (tag-based split channels)

- Trigger event: push `vX.Y.Z` tag to `origin`.
- Docker workflow: `.github/workflows/release-docker.yml`.
- Brew workflow: `.github/workflows/release-brew.yml`.
- Both channels must ship outputs mapped to the same release tag.

## Brew asset contract

Supported target matrix:

- `x86_64-apple-darwin`
- `aarch64-apple-darwin`
- `x86_64-unknown-linux-gnu`
- `aarch64-unknown-linux-gnu`

Asset naming (deterministic):

- `agent-workspace-launcher-vX.Y.Z-<target>.tar.gz`
- `agent-workspace-launcher-vX.Y.Z-<target>.tar.gz.sha256`

Archive payload contract (minimum):

- `scripts/aws.bash`
- `scripts/aws.zsh`
- `README.md`
- `LICENSE`

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

5. Commit changelog
   - Suggested message: `chore(release): vX.Y.Z`
   - Use semantic commit helper (per `AGENTS.md`).

6. Audit
   - `./scripts/release_audit.sh --version vX.Y.Z --branch main --strict`

7. Tag + push (primary publish trigger)
   - `git -c tag.gpgSign=false tag vX.Y.Z`
   - `git push origin vX.Y.Z`

8. (Optional) Update GitHub Release notes from changelog
   - `./scripts/release_notes_from_changelog.sh --version vX.Y.Z --output "$AGENTS_HOME/out/release-notes-vX.Y.Z.md"`
   - `gh release create vX.Y.Z -F "$AGENTS_HOME/out/release-notes-vX.Y.Z.md" --title "vX.Y.Z" || gh release edit vX.Y.Z --notes-file "$AGENTS_HOME/out/release-notes-vX.Y.Z.md"`

9. Verify Docker channel
   - Confirm workflow run:
     - `gh run list --workflow release-docker.yml --limit 5`
   - Confirm tags exist (`latest`, `vX.Y.Z`, `sha-<short>`) in Docker Hub + GHCR.
   - Verify multi-arch manifests:
     - `docker buildx imagetools inspect docker.io/graysurf/agent-workspace-launcher:vX.Y.Z`
     - `docker buildx imagetools inspect ghcr.io/graysurf/agent-workspace-launcher:vX.Y.Z`

10. Verify Brew channel + extract checksums
    - Confirm workflow run:
      - `gh run list --workflow release-brew.yml --limit 5`
    - Confirm release assets:
      - `gh release view vX.Y.Z --json assets --jq '.assets[].name'`
    - Download and verify checksums:

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

## Release-to-tap checklist

Run this sequence after Step 10 passes.

1. Update formula in `~/Project/graysurf/homebrew-tap/Formula/agent-workspace-launcher.rb` with the new tag URLs and sha256 values.
2. Validate formula in tap repo:
   - `ruby -c Formula/agent-workspace-launcher.rb`
   - `HOMEBREW_NO_AUTO_UPDATE=1 brew style Formula/agent-workspace-launcher.rb`
   - `brew tap graysurf/tap "$(pwd)" --custom-remote`
   - `brew update-reset "$(brew --repo graysurf/tap)"`
   - `HOMEBREW_NO_AUTO_UPDATE=1 brew reinstall agent-workspace-launcher || HOMEBREW_NO_AUTO_UPDATE=1 brew install agent-workspace-launcher`
   - `HOMEBREW_NO_AUTO_UPDATE=1 brew test agent-workspace-launcher`
3. Open and merge tap PR.
4. Verify end-user install:
   - `brew upgrade agent-workspace-launcher || brew install agent-workspace-launcher`
   - `aws --help`
5. If tap update fails, rollback by reverting the tap formula commit and keep the previous stable formula version active.

## Transition fallback (compatibility only)

Use this only for Docker hotfix publishing when split tag workflows are temporarily unavailable:

```sh
git fetch origin
git checkout docker
git merge --ff-only origin/main
git push origin docker
git checkout main
```
