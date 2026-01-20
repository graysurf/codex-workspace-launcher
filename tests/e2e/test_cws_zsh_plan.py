from __future__ import annotations

import pytest

from .plan import CwsE2EPlanCase, case_id, plan_cases


@pytest.mark.e2e
@pytest.mark.parametrize("plan_case", plan_cases("zsh"), ids=case_id)
def test_cws_zsh_e2e_plan(plan_case: CwsE2EPlanCase) -> None:
    # TODO: Execute and assert using the real docker-backed launcher via `source scripts/cws.zsh`.
    assert plan_case.command_argv
    assert plan_case.command_display
    assert True
