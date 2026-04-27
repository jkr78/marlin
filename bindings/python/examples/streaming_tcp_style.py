#!/usr/bin/env python3
"""Parse a TCP-style byte stream where sentences may straddle feed() boundaries.

Python analogue of PRD §10 deliverable 7, item 3 — Streaming mode demo.
Demonstrates: a real TCP receiver gets bytes in arbitrary chunks; the
parser reassembles cross-chunk sentences correctly.
"""

from marlin.envelope import StreamingParser

PAYLOAD = (
    b"$GPGGA,123519,4807.038,N,01131.000,E,1,08,0.9,545.4,M,46.9,M,,*47\r\n"
    b"$GPVTG,054.7,T,034.4,M,005.5,N,010.2,K*48\r\n"
    b"$GPHDT,123.456,T*32\r\n"
)

# Deliberately uneven chunk sizes so sentence boundaries straddle feeds.
CHUNK_SIZES = [17, 23, 9, 50, 1000]


def chunked(data: bytes, sizes: list[int]) -> list[bytes]:
    out: list[bytes] = []
    i = 0
    for n in sizes:
        if i >= len(data):
            break
        out.append(data[i:i + n])
        i += n
    return out


def main() -> None:
    parser = StreamingParser()
    sentences_seen = 0
    for chunk in chunked(PAYLOAD, CHUNK_SIZES):
        parser.feed(chunk)
        for sentence in parser:
            sentences_seen += 1
            talker = sentence.talker.decode() if sentence.talker else ""
            print(f"chunk-drained {talker}{sentence.sentence_type}")
    print(f"total sentences: {sentences_seen}")


if __name__ == "__main__":
    main()
