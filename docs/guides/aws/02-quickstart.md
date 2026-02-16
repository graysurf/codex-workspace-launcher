# Quickstart

Goal: create a workspace container, run commands inside it, then clean up.

## 1) Verify Docker is running

```sh
docker info >/dev/null
```

## 2) Check `aws` is working

```sh
aws --help
```

## 3) Create a workspace

```sh
aws create graysurf/agent-kit
```

The output includes:

- `workspace: <container>`
- `path: /work/<owner>/<repo>`

## 4) Run commands in the workspace

```sh
aws exec <name|container> git -C /work/graysurf/agent-kit status
```

Interactive shell:

```sh
aws exec <name|container>
```

## 5) Remove the workspace when done

```sh
aws rm <name|container> --yes
```

List workspaces:

```sh
aws ls
```
