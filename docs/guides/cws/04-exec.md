# `cws exec`

`exec` enters an existing workspace container (created by `cws create`).

## Interactive shell

```sh
cws exec <name|container>
```

## Run a command

```sh
cws exec <name|container> git -C /work/OWNER/REPO status
```

If your shell script uses `--` to separate arguments, note that upstream behavior may vary by version. If
`--` is treated as a literal argument (it will try to execute `--` in the container), drop it:

```sh
# not supported (it will try to run `--`)
cws exec <name|container> -- git status

# fallback
cws exec <name|container> git status
```

## Root / user

Run as root:

```sh
cws exec --root <name|container>
```

Run as a specific user:

```sh
cws exec --user codex <name|container>
```
