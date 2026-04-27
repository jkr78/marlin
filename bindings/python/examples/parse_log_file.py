#!/usr/bin/env python3
"""Parse a captured NMEA log from disk using StreamingParser.

Python analogue of the example in PRD §10 deliverable 7, item 1.
Rust counterpart: (none yet — examples are a pre-release deliverable.)
"""

import sys
from marlin.envelope import StreamingParser


def main(path: str) -> None:
    parser = StreamingParser()
    with open(path, "rb") as f:
        while chunk := f.read(4096):
            parser.feed(chunk)
            for sentence in parser:
                talker = sentence.talker.decode() if sentence.talker else ""
                print(
                    talker, sentence.sentence_type,
                    "OK" if sentence.checksum_ok else "BAD",
                )


if __name__ == "__main__":
    if len(sys.argv) != 2:
        sys.exit(f"usage: {sys.argv[0]} <log_file>")
    main(sys.argv[1])
