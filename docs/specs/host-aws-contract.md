# Host AWS Contract

## Command surface
Primary host wrapper command:
- `aws <subcommand> [args...]`

Shorthand aliases:
- `aw` -> `aws`
- `awa` -> `aws auth`
- `awac` -> `aws auth codex`
- `awah` -> `aws auth github`
- `awag` -> `aws auth gpg`
- `awc` -> `aws create`
- `awl` -> `aws ls`
- `awe` -> `aws exec`
- `awr` -> `aws reset`
- `awrr` -> `aws reset repo`
- `awrw` -> `aws reset work-repos`
- `awro` -> `aws reset opt-repos`
- `awrp` -> `aws reset private-repo`
- `awm` -> `aws rm`
- `awt` -> `aws tunnel`

## Host envs
- `AWS_IMAGE`
- `AWS_DOCKER_ARGS`
- `AWS_AUTH`
- `AWS_E2E*` (tests/e2e only)

## Behavioral notes
- Wrapper always mounts `/var/run/docker.sock`.
- Wrapper forwards `GH_TOKEN`/`GITHUB_TOKEN` when available.
- Wrapper may inject `AGENT_WORKSPACE_GPG_KEY` from host git signing key when needed.

## Hard cutover
- `cws` command is removed.
- `CWS_*` runtime fallback is removed.
