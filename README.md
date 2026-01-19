# codex-workspace-launcher

Portable Docker launcher for `zsh-kit`'s `codex-workspace`.

This project packages the full `codex-workspace` CLI (`create/ls/rm/exec/reset/tunnel`) into an image so you can
use it without checking out `zsh-kit` or `codex-kit` locally. It operates in Docker-outside-of-Docker mode by
connecting to the host Docker daemon via `/var/run/docker.sock`.

Quickstart:

```sh
docker run --rm -it \
  -v /var/run/docker.sock:/var/run/docker.sock \
  graysurf/codex-workspace-launcher:latest \
  create OWNER/REPO
```

Common commands:

```sh
docker run --rm -it -v /var/run/docker.sock:/var/run/docker.sock graysurf/codex-workspace-launcher:latest --help
docker run --rm -it -v /var/run/docker.sock:/var/run/docker.sock graysurf/codex-workspace-launcher:latest ls
docker run --rm -it -v /var/run/docker.sock:/var/run/docker.sock graysurf/codex-workspace-launcher:latest exec <name|container>
docker run --rm -it -v /var/run/docker.sock:/var/run/docker.sock graysurf/codex-workspace-launcher:latest rm <name|container> --yes
docker run --rm -it -v /var/run/docker.sock:/var/run/docker.sock graysurf/codex-workspace-launcher:latest rm --all --yes
```

Docker-outside-of-Docker (DooD) rules:

- The launcher container talks to the host Docker daemon via `-v /var/run/docker.sock:/var/run/docker.sock`.
- Any `-v <src>:<dst>` executed by the launcher resolves `<src>` on the host.
- When `codex-workspace` needs to read host files, the launcher container must also be able to `test -d` those
  paths (so bind-mount them into the launcher using the same absolute path).
- Prefer same-path binds + `HOME` passthrough:

```sh
docker run --rm -it \
  -v /var/run/docker.sock:/var/run/docker.sock \
  -e HOME="$HOME" \
  -v "$HOME/.config:$HOME/.config:ro" \
  graysurf/codex-workspace-launcher:latest \
  create OWNER/REPO
```

Optional host mounts:

- Secrets dir (recommended; enables `codex-use` syncing inside the workspace):

```sh
docker run --rm -it \
  -v /var/run/docker.sock:/var/run/docker.sock \
  -e HOME="$HOME" \
  -v "$HOME/.config/codex_secrets:$HOME/.config/codex_secrets:rw" \
  graysurf/codex-workspace-launcher:latest \
  create OWNER/REPO
```

Configuration (env vars):

`codex-workspace` (zsh layer; user-facing CLI):

| Env | Default | Purpose |
| --- | --- | --- |
| `CODEX_WORKSPACE_PREFIX` | `codex-ws` | Workspace container name prefix |
| `CODEX_WORKSPACE_PRIVATE_REPO` | (empty) | During `create`, clone/pull this repo into workspace `~/.private` |
| `CODEX_WORKSPACE_LAUNCHER` | (in image) | Low-level launcher path (this image sets it to `/opt/codex-kit/docker/codex-env/bin/codex-workspace`) |
| `CODEX_WORKSPACE_LAUNCHER_AUTO_DOWNLOAD` | `true` | Auto-download low-level launcher when missing (not used when `CODEX_WORKSPACE_LAUNCHER` is set) |
| `CODEX_WORKSPACE_AUTH` | `auto` | `auto\|gh\|env\|none`: token source selection (`env` is most practical in the launcher container) |
| `CODEX_WORKSPACE_TUNNEL_NAME` | (empty) | Tunnel name for the `tunnel` subcommand (<= 20 chars) |
| `CODEX_WORKSPACE_OPEN_VSCODE_ENABLED` | (empty/false) | Auto-run `code --new-window` (typically not effective inside the launcher container) |
| `CODEX_WORKSPACE_OPEN_VSCODE` | deprecated | Deprecated flag (use `CODEX_WORKSPACE_OPEN_VSCODE_ENABLED`) |

Additional variables used:

- `GH_TOKEN` / `GITHUB_TOKEN`: clone private repos and configure git auth inside the workspace
- `XDG_CACHE_HOME`: launcher auto-download cache root (only when auto-download is enabled)
- `TMPDIR`: temp files

Low-level launcher (`codex-kit` script; invoked by the zsh layer):

| Env | Default | Purpose |
| --- | --- | --- |
| `CODEX_ENV_IMAGE` | `graysurf/codex-env:linuxbrew` | Workspace runtime image |
| `CODEX_WORKSPACE_PREFIX` | `codex-ws` | Workspace container name prefix |
| `GITHUB_HOST` | `github.com` | Repo host (when using `OWNER/REPO` form) |
| `CODEX_SECRET_DIR_HOST` | `$HOME/.config/zsh/scripts/_features/codex/secrets` | Default secrets dir (host path; requires DooD same-path bind) |
| `CODEX_CONFIG_DIR_HOST` | (empty) | Bind-mount host config into the workspace (`/home/codex/.config:ro`) |
| `CODEX_ZSH_PRIVATE_DIR_HOST` | (empty) | Bind-mount host zsh private into the workspace (`/opt/zsh-kit/.private:ro`) |

Security notes:

- Mounting `docker.sock` is root-equivalent host access.
- Passing `GH_TOKEN` into the launcher can make it visible via `docker inspect` on workspace containers when token
  persistence is enabled.

Local build:

```sh
docker build -t codex-workspace-launcher:dev \
  --build-arg ZSH_KIT_REF=main \
  --build-arg CODEX_KIT_REF=main \
  .
```

Publishing (CI):

- Workflow: `.github/workflows/publish.yml`
- Triggers: PRs build only; pushes to `main` publish images
- Registry: Docker Hub (`graysurf/codex-workspace-launcher`)
- Tags: `latest`, `sha-<short>`
- Secrets (GitHub Actions): `DOCKERHUB_USERNAME`, `DOCKERHUB_TOKEN`
- Ref pinning: the workflow resolves `graysurf/zsh-kit@main` and `graysurf/codex-kit@main` to commit SHAs and
  builds with those SHAs (reproducible published images).

Private repo:

```sh
docker run --rm -it \
  -v /var/run/docker.sock:/var/run/docker.sock \
  -e GH_TOKEN="$GH_TOKEN" \
  graysurf/codex-workspace-launcher:latest \
  create OWNER/PRIVATE_REPO
```

Docs:

- `docs/DESIGN.md`
