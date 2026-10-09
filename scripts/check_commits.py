#!/usr/bin/env python3
"""Check the mechanical shape of commit subjects in a revision range.

Rules (the body is judged at review):
- `[scope] subject`, the scope from SCOPES, the subject starting with a
  lowercase letter; a subject may first name the document or item it edits
  (`[docs] ADR-0010: ...`, `[docs] GLOSSARY: ...`, `[docs] ADR-0008 and
  ADR-0009: ...`)
- no trailing period
- the release commit is exactly `release: prepare vX.Y.Z`
- merge commits are not checked

Usage: scripts/check_commits.py [range]   (default: main..HEAD)
A range whose start ref does not exist is reported and skipped, so the
check passes on a checkout without a local `main`.
"""

import re
import subprocess
import sys

SCOPES = (
    "ais",
    "ci",
    "crates",
    "docs",
    "envelope",
    "field",
    "klv",
    "nmea-0183",
    "py",
    "todo",
)
NAMED = r"[A-Z][A-Za-z0-9-]*"
SUBJECT = re.compile(
    r"^\[(" + "|".join(re.escape(s) for s in SCOPES) + r")\] "
    r"(?:" + NAMED + r"(?: and " + NAMED + r")*: )?"
    r"[a-z].*[^.]$"
)
RELEASE = re.compile(r"^release: prepare v\d+\.\d+\.\d+$")


def subjects(rev_range: str) -> list[str] | None:
    result = subprocess.run(
        ["git", "log", "--no-merges", "--format=%s", rev_range],
        capture_output=True,
        text=True,
        check=False,
    )
    if result.returncode != 0:
        return None
    return [line for line in result.stdout.splitlines() if line]


def main() -> int:
    rev_range = sys.argv[1] if len(sys.argv) > 1 else "main..HEAD"
    found = subjects(rev_range)
    if found is None:
        print(f"check_commits: range {rev_range} does not resolve, skipped")
        return 0
    bad = [s for s in found if not (SUBJECT.match(s) or RELEASE.match(s))]
    for subject in bad:
        print(f"commit subject off shape: {subject!r}")
    if bad:
        print(
            f"{len(bad)} of {len(found)} subject(s) in {rev_range} off shape; "
            f"expected `[scope] lowercase subject` with scope in {', '.join(SCOPES)}",
            file=sys.stderr,
        )
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
