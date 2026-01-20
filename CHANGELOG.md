# Changelog

All notable changes to this project will be documented in this file.

## v1.0.0 - 2026-01-20

### Added
- Portable Docker launcher image for `codex-workspace` (Docker-outside-of-Docker / DooD)
- Optional `cws` wrappers (zsh + bash + completion) to run the launcher image
- Documentation: quickstart, build notes, troubleshooting, integration test runbook, and a multi-page user guide
- CI publishing to Docker Hub and GitHub Container Registry (GHCR)

### Changed
- README and docs tuned for end users (Codex-ready workspace, copy/paste-first)

### Fixed
- Documentation fixes: correct relative links and note the `exec --` gotcha

### Known
- `codex-workspace exec <name> -- <cmd>` is not supported in the current launcher image (it tries to run `--`)
