# `aws exec`

`exec` enters an existing workspace container (created by `aws create`).

## Interactive shell

```sh
aws exec <name|container>
```

## Run a command

```sh
aws exec <name|container> git -C /work/OWNER/REPO status
```

If your shell script uses `--` to separate arguments, note that upstream behavior may vary by version. If
`--` is treated as a literal argument (it will try to execute `--` in the container), drop it:

```sh
# not supported (it will try to run `--`)
aws exec <name|container> -- git status

# fallback
aws exec <name|container> git status
```

## Root / user

Run as root:

```sh
aws exec --root <name|container>
```

Run as a specific user:

```sh
aws exec --user codex <name|container>
```
