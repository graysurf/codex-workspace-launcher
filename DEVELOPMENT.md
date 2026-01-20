# Development Guide

## Testing

### Setup

- Create the virtual environment: `python3 -m venv .venv`
- Install dev deps: `.venv/bin/python -m pip install -r requirements-dev.txt`

### Run all tests

- `.venv/bin/python -m pytest`

### Smoke tests (no real Docker)

- `.venv/bin/python -m pytest -m script_smoke`
- These tests stub `docker` via `tests/stubs/bin` and validate the `cws` wrapper output.

### E2E tests (real Docker)

- `.venv/bin/python -m pytest -m e2e`
- Requires opt-in: set `CWS_E2E=1`.
- For create cases, set `CWS_E2E_PUBLIC_REPO=OWNER/REPO` (and `CWS_E2E_PRIVATE_REPO` for private repo coverage).
- Default flow is intentionally small (avoid spinning many containers); set `CWS_E2E_FULL=1` for the full CLI plan suite.
- Run a single CLI plan case (recommended while iterating):
  - `CWS_E2E=1 CWS_E2E_CASE=help .venv/bin/python -m pytest -m e2e tests/e2e/test_cws_cli_cases.py`
- Some cases are gated behind flags (auth, codex, gpg, tunnel, ssh, rm --all); see `tests/e2e/plan.py`.
  - Extra required inputs:
    - `CWS_E2E_CODEX_PROFILE` for `auth_codex_profile`
    - `CWS_E2E_GPG_KEY_ID` for `auth_gpg_key`
- E2E runs serialize via a lock under `out/tests/e2e/` to avoid concurrent Docker runs.

### Artifacts

- Smoke/test summaries and coverage are written to `out/tests/`.
