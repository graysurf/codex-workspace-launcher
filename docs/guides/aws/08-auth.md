# `aws auth`

Updates auth inside an existing workspace container (without recreating it).

Providers:

- `github`: refresh GitHub auth inside the workspace (`gh`/git credentials).
- `codex`: re-apply Codex auth inside the workspace (profile-based, or sync `CODEX_AUTH_FILE`).
- `gpg`: import your signing key so `git commit -S` works in the workspace.

## GitHub auth

Update GitHub auth for an existing workspace (auto-picks the workspace when only one exists):

```sh
aws auth github
```

Update a specific workspace:

```sh
aws auth github <name|container>
```

Use a non-default GitHub hostname:

```sh
aws auth github --host github.com <name|container>
```

Token selection:

- If `GH_TOKEN`/`GITHUB_TOKEN` are set on the host, `aws` forwards them into the launcher container.
- If they are not set and `AWS_AUTH=auto`, `aws` will try to reuse your host `gh` keyring token for `create/reset/auth github`.

## Codex auth

Apply a Codex profile to a workspace:

```sh
aws auth codex --profile work <name|container>
```

Notes:

- Profile-based auth typically requires your host Codex secrets to be available to the launcher container
  (DooD same-path bind). Example:

  ```sh
  AWS_DOCKER_ARGS=(
    -e HOME="$HOME"
    -v "$HOME/.config/AGENT_secrets:$HOME/.config/AGENT_secrets:rw"
  )
  aws auth codex --profile work <name|container>
  ```
- Naming exception: Codex compatibility inputs keep their original names, `CODEX_SECRET_DIR` and
  `CODEX_AUTH_FILE` (they are intentionally not migrated to `AWS_*`).

## GPG auth

Import a GPG signing key into an existing workspace:

```sh
aws auth gpg --key <keyid|fingerprint> <name|container>
```

If you set `AGENT_WORKSPACE_GPG_KEY` on the host, you can omit `--key`:

```sh
export AGENT_WORKSPACE_GPG_KEY="<keyid|fingerprint>"
aws auth gpg <name|container>
```

Notes:

- `auth gpg` exports your secret key and imports it into the workspace container. Treat the workspace container
  as sensitive and remove it when done.
- The launcher container must be able to read your keyring. On macOS hosts, prefer a DooD-safe same-path bind:

  ```sh
  AWS_DOCKER_ARGS=(
    -e HOME="$HOME"
    -v "$HOME/.gnupg:$HOME/.gnupg:ro"
  )
  aws auth gpg --key <keyid|fingerprint> <name|container>
  ```
