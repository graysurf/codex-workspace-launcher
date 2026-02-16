# `aws reset`

Resets repos inside an existing workspace container.

The reset behavior is implemented by the launcher runtime (`agent-workspace` + `agent-kit`), while the
user-facing `aws reset` interface remains stable.

If your workspace contains private repos, `aws reset` may need GitHub auth. When `GH_TOKEN`/`GITHUB_TOKEN` are not
set, the wrapper will automatically reuse your host `gh` login token (keyring) when available (also used by
`aws auth github`).

## Reset a specific repo directory

```sh
aws reset repo <name|container> /work/OWNER/REPO --yes
```

## Reset all work repos

```sh
aws reset work-repos <name|container> --yes
```

## Reset optional repos

```sh
aws reset opt-repos <name|container> --yes
```

## Reset private repo

```sh
aws reset private-repo <name|container> --yes
```
