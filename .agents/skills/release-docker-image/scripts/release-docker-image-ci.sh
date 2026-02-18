#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'USAGE'
Usage:
  .agents/skills/release-docker-image/scripts/release-docker-image-ci.sh \
    [--version <vX.Y.Z>] \
    [--input-ref <git-ref>] \
    [--workflow-ref <git-ref>] \
    [--repo <owner/name>] \
    [--workflow <file>] \
    [--publish-version-tag|--no-publish-version-tag] \
    [--no-wait]

Dispatches GitHub Actions workflow `release-docker.yml` to build/publish Docker images
in CI (Docker Hub + GHCR based on repo secrets/permissions).
USAGE
}

say() {
  printf 'release-docker-image-ci: %s\n' "$*" >&2
}

die() {
  say "error: $*"
  exit 1
}

to_bool() {
  local value
  value="$(printf '%s' "${1:-}" | tr '[:upper:]' '[:lower:]')"
  case "${value}" in
    1|true|yes|on) echo 1 ;;
    0|false|no|off|'') echo 0 ;;
    *) return 1 ;;
  esac
}

assert_semver_tag() {
  local tag="${1:-}"
  [[ "${tag}" =~ ^v[0-9]+\.[0-9]+\.[0-9]+([.-][0-9A-Za-z]+)*$ ]]
}

version=""
input_ref=""
workflow_ref=""
repo=""
workflow_file="release-docker.yml"
publish_version_raw=""
wait_for_run=1

while [[ $# -gt 0 ]]; do
  case "${1:-}" in
    --version)
      [[ $# -ge 2 ]] || {
        usage >&2
        exit 2
      }
      version="${2:-}"
      shift 2
      ;;
    --input-ref)
      [[ $# -ge 2 ]] || {
        usage >&2
        exit 2
      }
      input_ref="${2:-}"
      shift 2
      ;;
    --workflow-ref)
      [[ $# -ge 2 ]] || {
        usage >&2
        exit 2
      }
      workflow_ref="${2:-}"
      shift 2
      ;;
    --repo)
      [[ $# -ge 2 ]] || {
        usage >&2
        exit 2
      }
      repo="${2:-}"
      shift 2
      ;;
    --workflow)
      [[ $# -ge 2 ]] || {
        usage >&2
        exit 2
      }
      workflow_file="${2:-}"
      shift 2
      ;;
    --publish-version-tag)
      publish_version_raw=1
      shift
      ;;
    --no-publish-version-tag)
      publish_version_raw=0
      shift
      ;;
    --no-wait)
      wait_for_run=0
      shift
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      usage >&2
      exit 2
      ;;
  esac
done

command -v gh >/dev/null 2>&1 || die "gh is required"
gh auth status >/dev/null 2>&1 || die "gh auth is required"

if [[ -n "${version}" ]] && ! assert_semver_tag "${version}"; then
  say "error: invalid --version '${version}' (expected vX.Y.Z)"
  exit 2
fi

if [[ -z "${input_ref}" ]]; then
  if [[ -n "${version}" ]]; then
    input_ref="${version}"
  else
    input_ref="$(git rev-parse HEAD 2>/dev/null || true)"
    [[ -n "${input_ref}" ]] || die "unable to resolve default --input-ref (pass --version or --input-ref)"
  fi
fi

if [[ -z "${workflow_ref}" ]]; then
  workflow_ref="$(git branch --show-current 2>/dev/null || true)"
  [[ -n "${workflow_ref}" ]] || workflow_ref="main"
fi

if [[ -z "${repo}" ]]; then
  repo="$(gh repo view --json nameWithOwner --jq '.nameWithOwner' 2>/dev/null || true)"
  [[ -n "${repo}" ]] || die "unable to resolve default --repo (pass --repo owner/name)"
fi

if [[ -z "${publish_version_raw}" ]]; then
  if assert_semver_tag "${input_ref}"; then
    publish_version_tag=1
  else
    publish_version_tag=0
  fi
else
  publish_version_tag="$(to_bool "${publish_version_raw}" || true)"
  [[ -n "${publish_version_tag}" ]] || {
    usage >&2
    exit 2
  }
fi

if ((publish_version_tag == 1)) && ! assert_semver_tag "${input_ref}"; then
  die "publish version tag requires --input-ref to be a semver tag (vX.Y.Z)"
fi

publish_version_value=false
if ((publish_version_tag == 1)); then
  publish_version_value=true
fi

before_run_id="$(gh run list \
  --repo "${repo}" \
  --workflow "${workflow_file}" \
  --event workflow_dispatch \
  --limit 1 \
  --json databaseId \
  --jq '.[0].databaseId // empty' 2>/dev/null || true)"

say "dispatching ${workflow_file} (repo=${repo}, workflow_ref=${workflow_ref}, input_ref=${input_ref})"
gh workflow run "${workflow_file}" \
  --repo "${repo}" \
  --ref "${workflow_ref}" \
  -f ref="${input_ref}" \
  -f publish_version_tag="${publish_version_value}" >/dev/null

if ((wait_for_run == 0)); then
  say "dispatched"
  say "check status: gh run list --repo ${repo} --workflow ${workflow_file} --limit 5"
  exit 0
fi

run_id=""
for _ in $(seq 1 60); do
  run_id="$(gh run list \
    --repo "${repo}" \
    --workflow "${workflow_file}" \
    --event workflow_dispatch \
    --limit 1 \
    --json databaseId \
    --jq '.[0].databaseId // empty' 2>/dev/null || true)"

  if [[ -n "${run_id}" && "${run_id}" != "${before_run_id}" ]]; then
    break
  fi

  sleep 2
done

if [[ -z "${run_id}" || "${run_id}" == "${before_run_id}" ]]; then
  die "dispatched workflow but could not resolve new run id"
fi

say "watching run ${run_id}"
gh run watch "${run_id}" --repo "${repo}" --exit-status

run_url="$(gh run view "${run_id}" --repo "${repo}" --json url --jq '.url')"
say "run complete: ${run_url}"
