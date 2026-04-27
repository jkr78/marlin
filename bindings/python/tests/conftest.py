"""Shared pytest fixtures for the marlin binding tests."""

import os

import pytest


@pytest.fixture(scope="session")
def regenerate_goldens() -> bool:
    """True when MARLIN_REGENERATE_GOLDENS=1; rewrites expected JSONs."""
    return os.environ.get("MARLIN_REGENERATE_GOLDENS") == "1"
