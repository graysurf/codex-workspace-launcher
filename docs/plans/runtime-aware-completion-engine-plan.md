# Plan: Runtime-aware completion engine via hidden `__complete` subcommand

## Overview
This plan replaces duplicated shell completion logic with a single Rust completion engine exposed through a hidden `__complete` subcommand. The engine will produce runtime-aware suggestions (container/host) so workspace names are completed correctly for `auth`, `rm`, `exec`, `reset`, and `tunnel`. Bash and zsh scripts become thin adapters that forward shell context to the Rust command and render the returned candidates. Existing user-facing command behavior remains unchanged; only completion implementation and related docs/tests are updated.

## Scope
- In scope: Add hidden `__complete` command contract in Rust CLI and keep it out of standard help output.
- In scope: Implement runtime-aware completion candidate generation using the same runtime precedence as normal command execution.
- In scope: Replace tracked bash/zsh completion logic with thin wrappers that call `__complete`.
- In scope: Add unit/integration smoke coverage for completion behavior and wrapper equivalence.
- In scope: Update specs/docs for completion architecture and operational fallback.
- Out of scope: Fish/PowerShell completion support.
- Out of scope: Changes to non-completion command semantics (`create`, `ls`, `rm`, `exec`, `auth`, `reset`, `tunnel`).
- Out of scope: Runtime backend refactors unrelated to completion candidates.

## Assumptions (if any)
1. Hidden subcommands are acceptable for internal integration and release artifacts.
2. Completion calls may invoke the binary repeatedly, so candidate generation must be lightweight and deterministic.
3. Runtime resolution for completion must match runtime resolution for command execution (`--runtime` > `AGENT_WORKSPACE_RUNTIME` > `AWL_RUNTIME` > default `container`).
4. Returning empty candidates on transient runtime/list failures is acceptable if base static completions still work.

## Sprint 1: Completion Contract and CLI Plumbing
**Goal**: Define a stable completion protocol and wire hidden command dispatch without changing user-facing command behavior.
**Demo/Validation**:
- Command(s): `cargo test -p agent-workspace cli::tests`
- Command(s): `cargo test -p agent-workspace runtime::tests`
- Verify: hidden command parses and dispatches, and runtime resolution semantics are unchanged.

### Task 1.1: Define completion protocol and compatibility contract
- **Location**:
  - `docs/specs/agent-workspace-rust-contract.md`
  - `docs/specs/host-awl-contract.md`
  - `docs/DESIGN.md`
- **Description**: Specify hidden command interface (inputs: shell, words/cursor/runtime context; outputs: candidate list + optional descriptions), failure behavior, and compatibility guarantees for wrappers.
- **Dependencies**:
  - none
- **Complexity**: 4
- **Acceptance criteria**:
  - Specs define request/response shape for `__complete` and document runtime-aware workspace completion behavior.
  - Docs state that completion internals are centralized in Rust and shell scripts are adapters only.
- **Validation**:
  - `rg -n "__complete|completion protocol|runtime-aware" docs/specs/agent-workspace-rust-contract.md docs/specs/host-awl-contract.md docs/DESIGN.md`

### Task 1.2: Add hidden `__complete` command to CLI parser and dispatcher
- **Location**:
  - `crates/agent-workspace/src/cli.rs`
  - `crates/agent-workspace/src/lib.rs`
  - `crates/agent-workspace/src/launcher.rs`
- **Description**: Add a hidden clap subcommand routed through existing entrypoints, with explicit passthrough argument handling for shell context.
- **Dependencies**:
  - Task 1.1
- **Complexity**: 6
- **Acceptance criteria**:
  - `agent-workspace-launcher __complete ...` is routable and returns a stable exit code contract.
  - Standard help output remains focused on public commands and does not advertise internal completion command.
- **Validation**:
  - `cargo test -p agent-workspace cli::tests`
  - `bash -lc 'set -euo pipefail; ! cargo run -p agent-workspace -- --help | rg -q "__complete"'`
  - `cargo run -p agent-workspace -- __complete --shell bash --words "agent-workspace-launcher " --cword 1 >/dev/null`
  - `bash -lc 'cargo run -p agent-workspace -- __complete --shell invalid --words "agent-workspace-launcher " --cword 1 >/dev/null 2>&1; test $? -ne 0'`

### Task 1.3: Build completion request parser and typed model
- **Location**:
  - `crates/agent-workspace/src/completion/mod.rs` (new)
  - `crates/agent-workspace/src/completion/protocol.rs` (new)
- **Description**: Introduce typed parser for completion input (shell, argv words, cursor index, optional runtime override), plus canonical normalization shared by all candidate providers.
- **Dependencies**:
  - Task 1.2
- **Complexity**: 7
- **Acceptance criteria**:
  - Parser rejects malformed input deterministically with actionable errors.
  - Parser output captures enough state to drive all current subcommand completion branches.
- **Validation**:
  - `cargo test -p agent-workspace completion::protocol_tests`

### Task 1.4: Add completion feature flag for safe rollback path
- **Location**:
  - `crates/agent-workspace/src/completion/mod.rs`
  - `crates/agent-workspace/src/completion/protocol.rs`
  - `docs/specs/agent-workspace-rust-contract.md`
- **Description**: Add an opt-out env gate contract (for example `AGENT_WORKSPACE_COMPLETION_MODE=legacy`) and expose completion mode in typed request/dispatch so shell adapters can consume it safely.
- **Dependencies**:
  - Task 1.2
- **Complexity**: 5
- **Acceptance criteria**:
  - Completion mode is parsed deterministically (`rust` default, `legacy` opt-out).
  - Completion mode contract is documented and available to shell adapters without changing public CLI behavior.
- **Validation**:
  - `cargo test -p agent-workspace completion::mode_tests`
  - `rg -n "AGENT_WORKSPACE_COMPLETION_MODE" docs/specs/agent-workspace-rust-contract.md`

## Sprint 2: Runtime-aware Candidate Engine
**Goal**: Implement one Rust source of truth for candidates across global flags, subcommands, and workspace-aware positions.
**Demo/Validation**:
- Command(s): `cargo test -p agent-workspace completion::engine_tests`
- Verify: candidate sets are correct for both runtime modes and include workspace suggestions only where expected.

### Task 2.1: Implement completion state machine for command surface
- **Location**:
  - `crates/agent-workspace/src/completion/engine.rs` (new)
  - `crates/agent-workspace/src/completion/candidates.rs` (new)
- **Description**: Encode completion routing rules for global options and subcommands (`auth/create/ls/rm/exec/reset/tunnel`) including nested reset/auth subcommands and option positions.
- **Dependencies**:
  - Task 1.3
- **Complexity**: 8
- **Acceptance criteria**:
  - Candidate generation is deterministic for identical input.
  - Option and subcommand candidates match documented CLI contract.
- **Validation**:
  - `cargo test -p agent-workspace completion::engine_tests`

### Task 2.2: Implement runtime-aware workspace provider
- **Location**:
  - `crates/agent-workspace/src/completion/providers/workspaces.rs` (new)
  - `crates/agent-workspace/src/runtime.rs`
  - `crates/agent-workspace/src/launcher/ls.rs`
  - `crates/agent-workspace/src/launcher/container.rs`
- **Description**: Resolve runtime from completion context and fetch workspace candidates from the corresponding backend, preserving runtime precedence and aliases (`docker/native`).
- **Dependencies**:
  - Task 1.3
  - Task 2.1
- **Complexity**: 8
- **Acceptance criteria**:
  - `--runtime host` completion suggests host workspaces only.
  - `--runtime container` completion suggests container workspaces only.
  - Unset runtime follows default/container behavior exactly.
- **Validation**:
  - `cargo test -p agent-workspace completion::workspace_provider_tests`

### Task 2.3: Implement shell-specific output adapters
- **Location**:
  - `crates/agent-workspace/src/completion/output.rs` (new)
  - `crates/agent-workspace/src/completion/mod.rs`
- **Description**: Add output modes required by bash/zsh wrappers (plain candidates and optional descriptions), including safe escaping and newline-delimited output.
- **Dependencies**:
  - Task 2.1
- **Complexity**: 6
- **Acceptance criteria**:
  - Output format is consumable by both tracked bash and zsh completion scripts.
  - Escaping does not break candidates containing separators (`--runtime=host`, `--output=json`).
- **Validation**:
  - `cargo test -p agent-workspace completion::output_tests`

### Task 2.4: Add failure budget and graceful degradation rules
- **Location**:
  - `crates/agent-workspace/src/completion/mod.rs`
  - `crates/agent-workspace/src/completion/providers/workspaces.rs`
- **Description**: Define behavior for transient errors/timeouts (for example Docker unavailable), returning partial/static candidates instead of hard-failing tab completion.
- **Dependencies**:
  - Task 2.2
- **Complexity**: 5
- **Acceptance criteria**:
  - Completion command never panics on backend failures.
  - Workspace candidate failures do not suppress non-workspace candidates.
- **Validation**:
  - `cargo test -p agent-workspace completion::degradation_tests`

## Sprint 3: Shell Adapter Migration (Bash/Zsh)
**Goal**: Replace duplicated shell completion logic with thin wrappers that delegate to Rust completion engine.
**Demo/Validation**:
- Command(s): `bash -n completions/agent-workspace-launcher.bash scripts/awl.bash scripts/awl_docker.bash && zsh -n completions/_agent-workspace-launcher scripts/awl.zsh scripts/awl_docker.zsh`
- Verify: wrappers contain minimal parsing and call Rust completion command for candidates.

### Task 3.1: Add adapter conformance tests for shell wrappers
- **Location**:
  - `tests/test_completion_adapters.py` (new)
  - `tests/test_wrapper_equivalence.py`
  - `tests/script_specs/scripts/awl.bash.json`
  - `tests/script_specs/scripts/awl.zsh.json`
- **Description**: Add deterministic tests for adapter input/output, runtime-aware workspace completion, and legacy mode fallback before migrating shell scripts.
- **Dependencies**:
  - Task 2.3
  - Task 2.4
- **Complexity**: 6
- **Acceptance criteria**:
  - Test suite validates adapter completion candidates for `auth/rm/exec/reset/tunnel` in both runtimes.
  - Test suite validates `AGENT_WORKSPACE_COMPLETION_MODE=legacy` fallback behavior.
- **Validation**:
  - `.venv/bin/python -m pytest tests/test_completion_adapters.py`
  - `.venv/bin/python -m pytest tests/test_wrapper_equivalence.py`

### Task 3.2: Migrate tracked bash completion to Rust-backed adapter
- **Location**:
  - `completions/agent-workspace-launcher.bash`
  - `scripts/awl.bash`
  - `scripts/awl_docker.bash`
- **Description**: Refactor bash completion functions to call hidden Rust completion command, populate `COMPREPLY` from engine output, and honor completion mode fallback.
- **Dependencies**:
  - Task 1.4
  - Task 3.1
- **Complexity**: 7
- **Acceptance criteria**:
  - Bash completion no longer hardcodes per-subcommand workspace logic.
  - Wrapper-level aliases (`awl`, `aw`) still receive valid candidates.
- **Validation**:
  - `bash -n completions/agent-workspace-launcher.bash scripts/awl.bash scripts/awl_docker.bash`
  - `.venv/bin/python -m pytest tests/test_completion_adapters.py -k "bash"`

### Task 3.3: Migrate tracked zsh completion to Rust-backed adapter
- **Location**:
  - `completions/_agent-workspace-launcher`
  - `scripts/awl.zsh`
  - `scripts/awl_docker.zsh`
- **Description**: Refactor zsh completion functions to delegate candidate generation to hidden Rust command, preserve compdef/alias wiring, and honor completion mode fallback.
- **Dependencies**:
  - Task 1.4
  - Task 3.1
- **Complexity**: 7
- **Acceptance criteria**:
  - Zsh completion no longer duplicates full command matrix in shell logic.
  - Alias and primary command completion parity remains intact.
- **Validation**:
  - `zsh -n completions/_agent-workspace-launcher scripts/awl.zsh scripts/awl_docker.zsh`
  - `.venv/bin/python -m pytest tests/test_completion_adapters.py -k "zsh"`

## Sprint 4: Validation Matrix, Docs, and Release Hardening
**Goal**: Ensure completion engine is production-safe, documented, and reversible.
**Demo/Validation**:
- Command(s): `cargo test -p agent-workspace && .venv/bin/python -m pytest -m script_smoke`
- Verify: Rust + shell validation gates pass and docs describe architecture/fallback clearly.

### Task 4.1: Add end-to-end completion matrix tests in Rust
- **Location**:
  - `crates/agent-workspace/src/completion/mod.rs`
  - `crates/agent-workspace/src/completion/fixtures/` (new)
- **Description**: Add table-driven cases that cover global options, auth/reset nested commands, runtime switches, and workspace lookup positions.
- **Dependencies**:
  - Task 2.4
- **Complexity**: 7
- **Acceptance criteria**:
  - Matrix includes both host/container runtime contexts.
  - Regressions in candidate sets fail with clear diffs.
- **Validation**:
  - `cargo test -p agent-workspace completion::matrix_tests`

### Task 4.2: Update completion and runtime docs
- **Location**:
  - `README.md`
  - `docs/guides/11-reference.md`
  - `docs/DESIGN.md`
  - `docs/specs/agent-workspace-rust-contract.md`
- **Description**: Document hidden completion architecture, runtime-aware behavior, and operator fallback mode for emergency rollback.
- **Dependencies**:
  - Task 1.4
  - Task 3.2
  - Task 3.3
- **Complexity**: 4
- **Acceptance criteria**:
  - Docs explain that bash/zsh adapters delegate to Rust completion engine.
  - Runtime-aware completion behavior and fallback env are discoverable.
- **Validation**:
  - `rg -n "__complete|runtime-aware|AGENT_WORKSPACE_COMPLETION_MODE" README.md docs/guides/11-reference.md docs/DESIGN.md docs/specs/agent-workspace-rust-contract.md`

### Task 4.3: Release/checklist integration for completion changes
- **Location**:
  - `DEVELOPMENT.md`
  - `docs/runbooks/INTEGRATION_TEST.md`
- **Description**: Ensure pre-submit and release runbooks include completion-specific verification commands and expected signals.
- **Dependencies**:
  - Task 4.2
- **Complexity**: 3
- **Acceptance criteria**:
  - Development and release docs include explicit completion validation steps.
  - Operators have a rollback toggle documented and testable.
- **Validation**:
  - `rg -n "completion|__complete|AGENT_WORKSPACE_COMPLETION_MODE" DEVELOPMENT.md docs/runbooks/INTEGRATION_TEST.md`
  - `AGENT_WORKSPACE_COMPLETION_MODE=legacy .venv/bin/python -m pytest tests/test_completion_adapters.py -k legacy`

## Dependency & Parallelization Map
- Critical path: `1.1 -> 1.2 -> 1.3 -> 2.1 -> 2.2 -> 2.4 -> 3.1 -> 3.2/3.3 -> 4.2 -> 4.3`.
- Parallelizable after Task 2.1:
  - Task 2.3 can run in parallel with Task 2.2 (shared dependency: Task 2.1).
- Parallelizable after Task 3.1:
  - Task 3.2 and Task 3.3 can execute in parallel (different shell targets, same protocol contract).
- Parallelizable in Sprint 4:
  - Task 4.1 can run in parallel with Task 4.2 once Sprint 3 is stable.

## Testing Strategy
- Unit:
  - Rust parser/engine/provider/output tests under `crates/agent-workspace/src/completion/*`.
  - CLI parser tests ensuring hidden command does not alter public command parsing.
- Integration:
  - Wrapper equivalence and script smoke tests to ensure shell adapters still forward commands correctly.
  - Runtime-aware completion tests with stubbed workspace lists for host/container contexts.
- E2E/manual:
  - Manual tab-completion spot checks in bash/zsh for:
    - `agent-workspace-launcher --runtime host exec <TAB>`
    - `agent-workspace-launcher --runtime container rm <TAB>`
    - `awl reset repo <TAB>`

## Risks & gotchas
- Completion performance regressions from calling runtime workspace listing on each tab press.
- Shell quoting/escaping differences can cause candidate truncation in one shell only.
- Hidden command contract drift can break wrapper scripts if output format changes without versioning.
- Runtime resolution mismatch between completion and execution would produce confusing candidates.

## Rollback plan
- Keep static shell candidate paths behind `AGENT_WORKSPACE_COMPLETION_MODE=legacy` and document immediate rollback steps.
- If post-release regression appears, set `AGENT_WORKSPACE_COMPLETION_MODE=legacy` in shell profile and reload shell.
- Ship a hotfix reverting wrappers to legacy static mode while preserving hidden command internals for debugging.
- If needed, revert the completion-engine commits and republish completion assets from the previous release tag.
