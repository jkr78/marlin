#!/usr/bin/env python3
"""Parse a single-sentence UDP-style datagram with no CRLF.

Python analogue of PRD §10 deliverable 7, item 2.
"""

from marlin.envelope import OneShotParser

DATAGRAM = b"$GPGGA,123519,4807.038,N,01131.000,E,1,08,0.9,545.4,M,46.9,M,,*47"


def main() -> None:
    parser = OneShotParser()
    parser.feed(DATAGRAM)
    sentence = parser.next_sentence()
    if sentence is None:
        print("no sentence decoded")
        return
    talker = sentence.talker.decode() if sentence.talker else ""
    print(f"{talker}{sentence.sentence_type}: {len(sentence.fields)} fields")


if __name__ == "__main__":
    main()
