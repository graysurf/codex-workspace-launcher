from __future__ import annotations

import pytest

from .plan import run_aws_e2e_flow


@pytest.mark.e2e
def test_aws_zsh_e2e_flow() -> None:
    run_aws_e2e_flow("zsh")
