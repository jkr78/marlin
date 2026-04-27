#!/usr/bin/env python3
"""Live AIS dashboard — pipe a feed via stdin to track ships in real time.

Usage:
    nc 153.44.253.27 5631 | live_ais_dashboard.py

Demonstrates the dual-parser pattern from GUIDE.md §3: an envelope
StreamingParser counts frames and checksum failures; an AisParser
extracts typed AIS messages and pulls positions and names off them.

Refreshes every 2 seconds. Press Ctrl-C to stop. On EOF, prints a final
snapshot — the script also runs cleanly on a captured fixture without
needing a live network feed.
"""

from __future__ import annotations

import sys
import time
from collections import Counter

from marlin.ais import (
    AisParser,
    ExtendedPositionReportB,
    PositionReportA,
    PositionReportB,
    StaticAndVoyageA,
    StaticDataB24A,
)
from marlin.envelope import StreamingParser

CLEAR = "\x1b[2J\x1b[H"
REFRESH_S = 2.0
TOP_N = 15


def render(
    env_frames: int,
    env_bad: int,
    type_counts: Counter[str],
    positions: dict[int, tuple[float, float]],
    names: dict[int, str],
) -> None:
    sys.stdout.write(CLEAR)
    print("=== marlin live AIS dashboard ===")
    print(f"frames: {env_frames}   bad: {env_bad}   types: {dict(type_counts)}")
    print(f"ships tracked: {len(positions)}   named: {len(names)}")
    print()
    print(f"{'MMSI':<12}{'lat':>10}{'lon':>10}   name")
    print("-" * 60)
    for mmsi, (lat, lon) in list(positions.items())[-TOP_N:]:
        name = names.get(mmsi, "")
        print(f"{mmsi:<12}{lat:>10.4f}{lon:>10.4f}   {name}")
    sys.stdout.flush()


def main() -> int:
    env = StreamingParser()
    ais = AisParser.streaming()

    env_frames = 0
    env_bad = 0
    type_counts: Counter[str] = Counter()
    positions: dict[int, tuple[float, float]] = {}
    names: dict[int, str] = {}

    last_render = time.monotonic()

    try:
        while chunk := sys.stdin.buffer.read(4096):
            env.feed(chunk)
            ais.feed(chunk)
            for sentence in env:
                env_frames += 1
                if not sentence.checksum_ok:
                    env_bad += 1
            for msg in ais:
                type_counts[msg.type_tag] += 1
                body = msg.body
                if isinstance(
                    body, (PositionReportA, PositionReportB, ExtendedPositionReportB)
                ):
                    if body.latitude_deg is not None and body.longitude_deg is not None:
                        positions[body.mmsi] = (body.latitude_deg, body.longitude_deg)
                if isinstance(
                    body, (StaticAndVoyageA, StaticDataB24A, ExtendedPositionReportB)
                ):
                    if body.vessel_name:
                        names[body.mmsi] = body.vessel_name
            if time.monotonic() - last_render >= REFRESH_S:
                render(env_frames, env_bad, type_counts, positions, names)
                last_render = time.monotonic()
    except KeyboardInterrupt:
        return 130

    render(env_frames, env_bad, type_counts, positions, names)
    return 0


if __name__ == "__main__":
    sys.exit(main())
