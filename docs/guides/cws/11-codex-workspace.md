# Without `cws` (direct `docker run`)

`cws` is a wrapper. Everything it does can be expressed as a plain `docker run ...`.

## Basic pattern

```sh
docker run --rm -it \
  -v /var/run/docker.sock:/var/run/docker.sock \
  -e GH_TOKEN \
  -e GITHUB_TOKEN \
  graysurf/codex-workspace-launcher:latest \
  --help
```

Create a workspace:

```sh
docker run --rm -it \
  -v /var/run/docker.sock:/var/run/docker.sock \
  -e GH_TOKEN \
  -e GITHUB_TOKEN \
  graysurf/codex-workspace-launcher:latest \
  create OWNER/REPO
```

Use GHCR instead:

```sh
docker run --rm -it \
  -v /var/run/docker.sock:/var/run/docker.sock \
  ghcr.io/graysurf/codex-workspace-launcher:latest \
  ls
```
