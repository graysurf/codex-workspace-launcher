# `aws create`

Creates a workspace container on the host and clones one or more repos into `/work/...`.

## Basic usage

Public repo:

```sh
aws create OWNER/REPO
```

Multiple repos (cloned in order):

```sh
aws create OWNER/REPO OTHER/REPO
```

## Skip extras

`--no-extras` disables cloning `~/.private` and additional repos:

```sh
aws create --no-extras OWNER/REPO
```

## Seed `~/.private` from a repo

```sh
aws create --private-repo OWNER/PRIVATE_REPO OWNER/REPO
```

This requires auth (see “Private repos” below).

## Create a workspace without cloning repos

Useful when you want an empty workspace container (no `/work/...` repos). This requires an explicit name:

```sh
aws create --no-work-repos --name ws-foo
```

## Private repos

If you have `gh` logged in on the host, `aws create/reset/auth github` will automatically reuse that token
(keyring) when `GH_TOKEN`/`GITHUB_TOKEN` are not set:

```sh
gh auth login
aws create OWNER/PRIVATE_REPO
```

Or pass a token on the host:

```sh
export GH_TOKEN=...
aws create OWNER/PRIVATE_REPO
```

Security note: `create` persists `GH_TOKEN`/`GITHUB_TOKEN` into the workspace container environment (visible via
`docker inspect`). Treat the workspace container as sensitive and remove it when done.
