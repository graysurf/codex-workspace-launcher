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

Docs:

- `docs/DESIGN.md`
