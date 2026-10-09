#!/usr/bin/env python3
"""Check the mechanical state of a `release: prepare vX.Y.Z` tree.

The workspace version is the reference. Rules:
- the bindings crate, pyproject.toml and the version test carry it
- the intra-workspace pins in [workspace.dependencies] require it
- every CHANGELOG has a `## [X.Y.Z] - yyyy-mm-dd` section directly under
  an empty `## [Unreleased]`

Usage: scripts/check_release.py  (from the repo root)
"""

import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
CHANGELOGS = [
    ROOT / "crates" / crate / "CHANGELOG.md"
    for crate in (
        "marlin-field",
        "marlin-nmea-envelope",
        "marlin-nmea-0183",
        "marlin-ais",
        "marlin-klv",
    )
] + [ROOT / "bindings" / "python" / "CHANGELOG.md"]


def first_match(path: Path, pattern: str) -> str:
    """The first capture group of `pattern` in `path`, or an empty string."""
    found = re.search(pattern, path.read_text(), re.MULTILINE)
    return found.group(1) if found else ""


def version_fields(version: str) -> list[str]:
    """Problems with the version fields that must equal `version`."""
    root = ROOT / "Cargo.toml"
    checks = {
        "Cargo.toml [workspace.dependencies] marlin-field": first_match(
            root, r'^marlin-field = \{[^}]*version = "([^"]+)"'
        ),
        "Cargo.toml [workspace.dependencies] marlin-nmea-envelope": first_match(
            root, r'^marlin-nmea-envelope = \{[^}]*version = "([^"]+)"'
        ),
        "bindings/python/Cargo.toml version": first_match(
            ROOT / "bindings/python/Cargo.toml", r'^version = "([^"]+)"'
        ),
        "bindings/python/pyproject.toml version": first_match(
            ROOT / "bindings/python/pyproject.toml", r'^version = "([^"]+)"'
        ),
        "bindings/python/tests/test_import.py __version__": first_match(
            ROOT / "bindings/python/tests/test_import.py",
            r'__version__ == "([^"]+)"',
        ),
    }
    return [
        f"{name}: {found or 'not found'}, expected {version}"
        for name, found in checks.items()
        if found != version
    ]


def changelog_sections(version: str) -> list[str]:
    """Problems with the [Unreleased] and [version] sections of each CHANGELOG."""
    problems = []
    for path in CHANGELOGS:
        text = path.read_text()
        shape = re.search(
            r"^## \[Unreleased\]\n\n## \[([^\]]+)\] - (\d{4}-\d{2}-\d{2})$",
            text,
            re.MULTILINE,
        )
        name = path.relative_to(ROOT)
        if shape is None:
            problems.append(
                f"{name}: expected an empty [Unreleased] directly above "
                f"`## [{version}] - yyyy-mm-dd`"
            )
        elif shape.group(1) != version:
            problems.append(f"{name}: first section is [{shape.group(1)}], expected [{version}]")
    return problems


def main() -> int:
    version = first_match(ROOT / "Cargo.toml", r'^\[workspace\.package\]\nversion = "([^"]+)"')
    if not version:
        print("Cargo.toml: no [workspace.package] version", file=sys.stderr)
        return 1
    problems = version_fields(version) + changelog_sections(version)
    for problem in problems:
        print(problem, file=sys.stderr)
    if problems:
        return 1
    print(f"release-check: every field and CHANGELOG agrees on {version}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
