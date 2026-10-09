#!/usr/bin/env python3
"""Check that rustdoc comment lines stay within the crate's prose width.

rustfmt wraps code and leaves comments alone, so a long identifier in a
`///` or `//!` line goes unnoticed until review. This script caps the full
line (indentation included) at WIDTH columns, or at a crate's own entry in
CRATE_WIDTH where the crate was written wider.

Exempt lines, which cannot be wrapped: a line inside a fenced code block,
a markdown table row, a line carrying a URL, and a line whose comment text
is a single token (a long intra-doc link, for instance).

Usage: scripts/check_doc_width.py [root-dir]
"""

import re
import sys
from pathlib import Path

WIDTH = 80
CRATE_WIDTH = {"crates/marlin-klv": 90}
SCAN_DIRS = ("crates", "bindings/python/src", "fuzz/fuzz_targets")
DOC_LINE = re.compile(r"^\s*(///|//!)(.*)$")
FENCE = re.compile(r"^\s*(///|//!)\s*```")


def width_for(rel: str) -> int:
    for prefix, width in CRATE_WIDTH.items():
        if rel.startswith(prefix + "/"):
            return width
    return WIDTH


def exempt(text: str) -> bool:
    body = text.strip()
    return (
        body.startswith("|")
        or "http://" in body
        or "https://" in body
        or " " not in body
    )


def check_file(path: Path, rel: str) -> list[str]:
    cap = width_for(rel)
    problems: list[str] = []
    in_fence = False
    for number, line in enumerate(path.read_text().splitlines(), start=1):
        match = DOC_LINE.match(line)
        if not match:
            continue
        if FENCE.match(line):
            in_fence = not in_fence
            continue
        if in_fence or len(line) <= cap or exempt(match.group(2)):
            continue
        problems.append(f"{rel}:{number}: {len(line)} columns, cap {cap}")
    return problems


def main() -> int:
    root = Path(sys.argv[1]) if len(sys.argv) > 1 else Path(".")
    problems: list[str] = []
    for scan in SCAN_DIRS:
        for path in sorted((root / scan).rglob("*.rs")):
            if "target" in path.parts:
                continue
            problems.extend(check_file(path, path.relative_to(root).as_posix()))
    for problem in problems:
        print(problem)
    if problems:
        print(f"{len(problems)} doc comment line(s) over width", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
