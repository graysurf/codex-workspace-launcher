#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
skill_root="$(cd "${script_dir}/.." && pwd)"

if [[ ! -f "${skill_root}/SKILL.md" ]]; then
  echo "error: missing SKILL.md" >&2
  exit 1
fi

ci_entrypoint="${skill_root}/scripts/release-docker-image-ci.sh"
local_entrypoint="${skill_root}/scripts/release-docker-image.sh"

if [[ ! -x "${ci_entrypoint}" ]]; then
  echo "error: missing executable ${ci_entrypoint}" >&2
  exit 1
fi

if [[ ! -x "${local_entrypoint}" ]]; then
  echo "error: missing executable ${local_entrypoint}" >&2
  exit 1
fi

"${ci_entrypoint}" --help >/dev/null
"${local_entrypoint}" --help >/dev/null

echo "ok: release-docker-image skill smoke checks passed"
