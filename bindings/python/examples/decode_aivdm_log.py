#!/usr/bin/env python3
"""Decode a captured AIVDM log into AIS messages.

Python analogue of PRD §10 deliverable 7, item 4. Demonstrates:
 - single-fragment Type 1 (position report A)
 - multi-fragment Type 5 (static + voyage data) reassembly
"""

from marlin.ais import AisParser, AisMessage, Other, PositionReportA, StaticAndVoyageA

# Type 1: position report A, single fragment.
# Type 5: static + voyage data, split across two !AIVDM fragments
# sharing sequence-id "3".
LOG = (
    b"!AIVDM,1,1,,A,13aGmP0P00PD;88MD5MTDww@2<0L,0*23\r\n"
    b"!AIVDM,2,1,3,A,55?MbV02;H;s<HtKR20EHE:0@T4@Dn2222222216L961O5Gf0NSQEp6ClRp888,0*1E\r\n"
    b"!AIVDM,2,2,3,A,88888888880,2*27\r\n"
)


def describe(msg: AisMessage) -> str:
    """Return a human-readable one-liner for a decoded AIS message."""
    body = msg.body
    if isinstance(body, Other):
        return f"type={msg.type_tag} msg_type={body.msg_type}"
    mmsi = body.mmsi
    if isinstance(body, PositionReportA):
        lat = body.latitude_deg
        lon = body.longitude_deg
        return f"mmsi={mmsi} type={msg.type_tag} lat={lat} lon={lon}"
    if isinstance(body, StaticAndVoyageA):
        name = body.vessel_name
        return f"mmsi={mmsi} type={msg.type_tag} name={name!r}"
    return f"mmsi={mmsi} type={msg.type_tag}"


def main() -> None:
    parser = AisParser.streaming()
    parser.feed(LOG)
    for msg in parser:
        print(describe(msg))


if __name__ == "__main__":
    main()
