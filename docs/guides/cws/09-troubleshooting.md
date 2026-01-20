# Troubleshooting

## Docker daemon not running

```sh
docker info
```

## Linux `permission denied` on `/var/run/docker.sock`

Some Linux setups require running the launcher as root or adding the socket group:

```sh
docker run --rm -it \
  --user 0:0 \
  -v /var/run/docker.sock:/var/run/docker.sock \
  graysurf/codex-workspace-launcher:latest \
  ls
```

## Find workspace containers manually

Workspaces are labeled:

```sh
docker ps -a --filter label=codex-kit.workspace=1
```

## If `cws` completion is not working (zsh)

Make sure `compinit` is enabled in your shell config:

```sh
autoload -Uz compinit && compinit
```

## Auth issues (private repos)

Prefer passing `GH_TOKEN`/`GITHUB_TOKEN` on the host:

```sh
export GH_TOKEN=...
cws create OWNER/PRIVATE_REPO
```

If org SSO blocks access, refresh your GitHub CLI auth (host-side):

```sh
gh auth refresh -h github.com -s repo -s read:org
```
