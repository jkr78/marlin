"""AIS test sentences shared by the unit tests.

One copy of every pinned AIVDM sentence the unit tests feed to a parser,
plus the ``aivdm`` builder for ad hoc wrappers. The golden tests read
their vectors from ``tests/fixtures`` instead and do not import this.
"""

from __future__ import annotations


def aivdm(
    frag_count: int,
    frag_num: int,
    seq_id: int | None,
    channel: str | None,
    payload: bytes,
    fill_bits: int,
) -> bytes:
    """Wrap ``payload`` in an ``!AIVDM`` sentence with a valid checksum.

    Mirror of ``marlin_ais::testing::build_aivdm``: XOR over
    ``AIVDM,<fields>`` and ``!...*hh\\r\\n`` framing.
    """
    parts = [b"AIVDM", str(frag_count).encode(), str(frag_num).encode()]
    parts.append(b"" if seq_id is None else str(seq_id).encode())
    parts.append(b"" if channel is None else channel.encode())
    parts.append(payload)
    parts.append(str(fill_bits).encode())
    body = b",".join(parts)
    x = 0
    for b in body:
        x ^= b
    return b"!" + body + b"*%02X\r\n" % x


# Classic Type 1 (position report A) from ITU-R M.1371 Annex 5, also used
# by the Rust crate (crates/marlin-ais/src/parser.rs) and the envelope
# fixture 03_aivdm_encapsulation.nmea. Raw rate of turn is -128.
AIVDM_TYPE1 = b"!AIVDM,1,1,,A,13aGmP0P00PD;88MD5MTDww@2<0L,0*23\r\n"

# Synthetic Type 1 payloads (MMSI 123456789, every other field zero) with
# raw rate of turn +127 and -127. There is no Python-side bit packer; the
# armored strings are pinned by the Rust BitWriter test
# `rate_of_turn_no_indicator_payloads_armor_to_known_strings` in
# crates/marlin-ais/src/position_report_a.rs.
AIVDM_TYPE1_ROT_PLUS_127 = b"!AIVDM,1,1,,A,11mg=5@Oh0000000000000000000,0*73\r\n"
AIVDM_TYPE1_ROT_MINUS_127 = b"!AIVDM,1,1,,A,11mg=5@P@0000000000000000000,0*44\r\n"

# Synthetic Type 1 payload (MMSI 123456789) whose every partitioned field
# carries an undefined or over-range code: navigation status 9, SOG 1022,
# longitude +181° + 1, latitude +91° + 1, COG 3601, heading 360, timestamp
# 61, manoeuvre 3. Pinned by the Rust BitWriter test
# `undefined_code_payload_armors_to_known_string` in
# crates/marlin-ais/src/position_report_a.rs.
AIVDM_TYPE1_INVALID_AND_OVER_RANGE = aivdm(1, 1, None, "A", b"11mg=5I0?v<tSF2l4Q@N4KAsP000", 0)

# Type 5 (StaticAndVoyageA) two-fragment message from the gpsd AIS corpus,
# same as the Rust crate's parser tests (crates/marlin-ais/src/parser.rs).
AIVDM_TYPE5_FRAG1 = (
    b"!AIVDM,2,1,3,A,"
    b"55P5TL01VIaAL@7WKO@mBplU@<PDhh000000001S;AJ::4A80?4i@E53,"
    b"0*3D\r\n"
)
AIVDM_TYPE5_FRAG2 = b"!AIVDM,2,2,3,A,1CQWBDhH888888888880,2*4D\r\n"

# Type 9 SAR aircraft position report: gpsd test/sample.aivdm T9-2
# (BSD-2-Clause). Its .chk file gives mmsi 111232511, altitude 303 m,
# SOG 42 kn, 6.27884°W 58.144°N, COG 154.5°, second 15, DTE 1, radio 0x8270.
AIVDM_TYPE9_GPSD_T9_2 = b"!AIVDM,1,1,,B,91b55wi;hbOS@OdQAC062Ch2089h,0*30\r\n"

# Synthetic Type 24 Part B from auxiliary-craft MMSI 987654321 (ship type
# 37, vendor "VND1234", call sign "CS001", EPFD GPS) whose 30 extent bits
# hold the mother-ship MMSI 211000123. Pinned by the Rust BitWriter test
# `part_b_auxiliary_craft_payload_armors_to_known_string` in
# crates/marlin-ais/src/static_data_b.rs.
AIVDM_TYPE24B_AUXILIARY_CRAFT = b"!AIVDM,1,1,,A,H>eq`dDUF>4ijkl3Chhi00<Tqds4,0*3A\r\n"

# Type 21 aid-to-navigation report: gpsd test/sample.aivdm T21-1
# (BSD-2-Clause), two fragments, 346 bits. Its .chk file gives mmsi
# 123456789, aid type 20 (cardinal mark N), name "CHINA ROSE MURPHY
# EXPRESS ALERT" (20 + 12 characters), 122.698592°W 47.920618°N, dims
# 5/5/5/5, EPFD GPS, second 50, status 165.
AIVDM_TYPE21_GPSD_T21_1_FRAG1 = (
    b"!AIVDM,2,1,5,B,E1mg=5J1T4W0h97aRh6ba84<h2d;W:Te=eLvH50```q,0*46\r\n"
)
AIVDM_TYPE21_GPSD_T21_1_FRAG2 = b"!AIVDM,2,2,5,B,:D44QDlp0C1DU00,2*36\r\n"

# Type 21 gpsd T21-2 (BSD-2-Clause), two fragments, 368 bits: over the
# 360-bit maximum, with a 16-character all-@ extension. mmsi 4000003, aid
# type 30 (special mark); the decoder keeps the embedded @ and yields
# name "IBC G BUOY@?????????" where gpsd stops at the first @.
AIVDM_TYPE21_GPSD_T21_2_FRAG1 = (
    b"!AIVDM,2,1,8,B,"
    b"E03l90w4Q1h3h1:WdPOwwwwwwwwlQdn`:e55020@@@gP0000000000000000,"
    b"0*47\r\n"
)
AIVDM_TYPE21_GPSD_T21_2_FRAG2 = b"!AIVDM,2,2,8,B,00,4*19\r\n"
