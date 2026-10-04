#!/usr/bin/env python3
"""Check the mechanical rules for cards under "## New tasks" in TODO.md.

Rules (only the mechanical ones; the rest is judged at review):
- a card starts with "- [ ] " or "- [x] "
- the title is at most 72 characters and does not end with a period
- a ticked card ends with **DONE yyyy-mm-dd** or **CANCELLED yyyy-mm-dd**
  (a reason may follow CANCELLED); an open card carries neither
- description lines are indented under their card

Older sections of TODO.md predate these rules and are not checked.
Usage: scripts/check_todo.py [path-to-TODO.md]
"""

import re
import sys
from pathlib import Path

MAX_TITLE = 72
CARD = re.compile(r"^- \[( |x)\] (.*)$")
CLOSED = re.compile(r"\s*\*\*(DONE|CANCELLED) \d{4}-\d{2}-\d{2}\*\*(.*)$")


def new_task_lines(text: str) -> list[tuple[int, str]]:
    """The numbered lines between "## New tasks" and the next "## " heading."""
    lines = []
    inside = False
    for number, line in enumerate(text.splitlines(), start=1):
        if line.startswith("## "):
            inside = line.strip() == "## New tasks"
            continue
        if inside:
            lines.append((number, line))
    return lines


def check(text: str) -> list[str]:
    problems = []
    for number, line in new_task_lines(text):
        if not line.strip() or line.startswith("  ") or line == "---":
            continue
        card = CARD.match(line)
        if card is None:
            problems.append(f"{number}: not a card and not indented under one")
            continue
        ticked, rest = card.group(1) == "x", card.group(2)
        closed = CLOSED.search(rest)
        title = rest[: closed.start()] if closed else rest
        if ticked and closed is None:
            problems.append(f"{number}: ticked without **DONE yyyy-mm-dd**")
        if not ticked and closed is not None:
            problems.append(f"{number}: DONE or CANCELLED on an open card")
        if closed and closed.group(1) == "DONE" and closed.group(2).strip():
            problems.append(f"{number}: text after **DONE yyyy-mm-dd**")
        if len(title) > MAX_TITLE:
            problems.append(f"{number}: title is {len(title)} characters, limit {MAX_TITLE}")
        if title.rstrip().endswith("."):
            problems.append(f"{number}: title ends with a period")
    return problems


def main() -> int:
    path = Path(sys.argv[1] if len(sys.argv) > 1 else "TODO.md")
    problems = check(path.read_text(encoding="utf-8"))
    for problem in problems:
        print(f"{path}:{problem}")
    return 1 if problems else 0


if __name__ == "__main__":
    sys.exit(main())
