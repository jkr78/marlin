#!/usr/bin/env python3
"""Parse NMEA/AIS bytes from stdin and print framed sentences.

Designed for live capture: `nc <host> <port> | parse_stdin.py`. Frames
both NMEA and AIVDM envelopes (AIS payload not decoded — pipe through
decode_aivdm_log.py for that). Non-PRD addition (user-requested).
"""

import sys
from marlin.envelope import StreamingParser


def main() -> int:
    parser = StreamingParser()
    try:
        while chunk := sys.stdin.buffer.read(4096):
            parser.feed(chunk)
            for sentence in parser:
                talker = sentence.talker.decode() if sentence.talker else ""
                print(
                    talker, sentence.sentence_type,
                    "OK" if sentence.checksum_ok else "BAD",
                    flush=True,
                )
    except KeyboardInterrupt:
        return 130
    return 0


if __name__ == "__main__":
    sys.exit(main())
