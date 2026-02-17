#!/usr/bin/env zsh

awl_docker() {
  local state_root image agent_env_image
  local -a docker_args extra_args

  state_root="${AWL_DOCKER_STATE:-$HOME/.awl-docker}"
  image="${AWL_DOCKER_IMAGE:-graysurf/agent-workspace-launcher:latest}"
  agent_env_image="${AWL_DOCKER_AGENT_ENV_IMAGE:-graysurf/agent-env:latest}"

  mkdir -p "${state_root}/home" "${state_root}/xdg-state"

  docker_args=(
    --rm
    -it
    -v /var/run/docker.sock:/var/run/docker.sock
    -v "${state_root}:/state"
    -e HOME=/state/home
    -e XDG_STATE_HOME=/state/xdg-state
    -e AGENT_ENV_IMAGE="${agent_env_image}"
  )

  if (( ${+AWL_DOCKER_ARGS} )); then
    if [[ ${(t)AWL_DOCKER_ARGS} == *array* ]]; then
      extra_args=("${AWL_DOCKER_ARGS[@]}")
    elif [[ -n "${AWL_DOCKER_ARGS}" ]]; then
      extra_args=(${=AWL_DOCKER_ARGS})
    fi
  fi
  docker_args+=("${extra_args[@]}")

  docker run "${docker_args[@]}" "${image}" "$@"
}

_awl_docker_workspace_names() {
  awl_docker ls 2>/dev/null | awk '{print $1}'
}

_awl_docker_completion() {
  local state
  local -a subcommands auth_commands reset_commands workspace_names rm_targets

  subcommands=(
    "auth"
    "create"
    "ls"
    "rm"
    "exec"
    "reset"
    "tunnel"
    "--help"
    "--version"
    "-h"
    "-V"
  )
  auth_commands=("github" "codex" "gpg" "--help" "-h")
  reset_commands=("repo" "work-repos" "opt-repos" "private-repo" "--help" "-h")
  workspace_names=(${(f)"$(_awl_docker_workspace_names)"})
  rm_targets=("${workspace_names[@]}" "--all" "--yes")

  _arguments -C \
    "1:subcommand:->subcommand" \
    "*::arg:->args"

  case "${state}" in
    subcommand)
      _describe -t awl-docker-subcommands "awl_docker subcommand" subcommands
      return 0
      ;;
    args)
      case "${words[2]}" in
        auth)
          if (( CURRENT == 3 )); then
            _describe -t awl-docker-auth-commands "auth command" auth_commands
          elif (( CURRENT >= 4 )); then
            _describe -t awl-docker-workspaces "workspace" workspace_names
          fi
          ;;
        reset)
          if (( CURRENT == 3 )); then
            _describe -t awl-docker-reset-commands "reset command" reset_commands
          elif (( CURRENT == 4 )); then
            _describe -t awl-docker-workspaces "workspace" workspace_names
          fi
          ;;
        rm)
          if (( CURRENT == 3 )); then
            _describe -t awl-docker-rm-targets "rm target" rm_targets
          fi
          ;;
        exec|tunnel)
          if (( CURRENT == 3 )); then
            _describe -t awl-docker-workspaces "workspace" workspace_names
          fi
          ;;
      esac
      return 0
      ;;
  esac
}

if (( ${+functions[compdef]} )); then
  compdef _awl_docker_completion awl_docker
fi
