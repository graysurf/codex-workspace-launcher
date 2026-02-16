# Install `aws` (zsh / bash)

`aws` is optional, but recommended. It saves you from re-typing `docker run ...` and includes shell completion.

## Option A: clone the repo (recommended for contributors)

```sh
git clone https://github.com/graysurf/agent-workspace-launcher.git
cd agent-workspace-launcher

# zsh
source ./scripts/aws.zsh

# bash
# source ./scripts/aws.bash
```

## Option B: without cloning (recommended for users)

zsh:

```sh
mkdir -p "$HOME/.config/agent-workspace-launcher"
curl -fsSL https://raw.githubusercontent.com/graysurf/agent-workspace-launcher/main/scripts/aws.zsh \
  -o "$HOME/.config/agent-workspace-launcher/aws.zsh"
source "$HOME/.config/agent-workspace-launcher/aws.zsh"
```

bash:

```sh
mkdir -p "$HOME/.config/agent-workspace-launcher"
curl -fsSL https://raw.githubusercontent.com/graysurf/agent-workspace-launcher/main/scripts/aws.bash \
  -o "$HOME/.config/agent-workspace-launcher/aws.bash"
source "$HOME/.config/agent-workspace-launcher/aws.bash"
```

## Option C: install as an executable

```sh
cp ./scripts/aws.bash ~/.local/bin/aws
```

## Optional: shell completion

- zsh: completion registers automatically if `compinit` is enabled.
- bash: completion registers when you `source ./scripts/aws.bash`.

## Configuration

### Choose the launcher image

Default:

```sh
export AWS_IMAGE="graysurf/agent-workspace-launcher:latest"
```

Use GHCR:

```sh
export AWS_IMAGE="ghcr.io/graysurf/agent-workspace-launcher:latest"
```

### Add extra `docker run` args (optional)

Examples:

- Pass your host `HOME` into the launcher container
- Bind-mount host `~/.config` into the launcher (same-path bind; DooD-safe)

zsh (array form preserves quoting):

```sh
AWS_DOCKER_ARGS=(
  -e HOME="$HOME"
  -v "$HOME/.config:$HOME/.config:ro"
)
```

bash (string form is simplest):

```sh
export AWS_DOCKER_ARGS="-e HOME=$HOME -v $HOME/.config:$HOME/.config:ro"
```

## Private repos

If you have `gh` logged in on the host, `aws create/reset/auth github` will automatically reuse that token
(keyring) when `GH_TOKEN`/`GITHUB_TOKEN` are not set.

Or set `GH_TOKEN` (or `GITHUB_TOKEN`) on your host — `aws` forwards it into the launcher container:

```sh
export GH_TOKEN=...
aws create OWNER/PRIVATE_REPO
```
