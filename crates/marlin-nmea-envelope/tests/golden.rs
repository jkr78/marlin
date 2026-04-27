//! Golden-file integration tests for the envelope crate.
//!
//! Each `#[test]` loads a fixture via `include_bytes!` and asserts the
//! parsed result matches hand-verified expectations. Fixtures are
//! documented in `tests/fixtures/README.md`.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use marlin_nmea_envelope::{OneShot, SentenceSource, Streaming};

// ---------------------------------------------------------------------------
// 01: Classic $GPGGA with CRLF terminator
// ---------------------------------------------------------------------------
const GGA_BASIC: &[u8] = include_bytes!("fixtures/01_gga_basic.nmea");

#[test]
fn golden_gga_basic_via_one_shot() {
    let mut parser = OneShot::new();
    parser.feed(GGA_BASIC);
    let s = parser.next_sentence().unwrap().unwrap();

    assert_eq!(s.start_delimiter, b'$');
    assert_eq!(s.talker, Some(*b"GP"));
    assert_eq!(s.sentence_type, "GGA");
    assert!(s.checksum_ok);
    assert!(s.tag_block.is_none());
    // 14 fields — the trailing ",," preserves two empty fields at the end.
    assert_eq!(s.fields.len(), 14);
    assert_eq!(s.fields[0], b"123519");
    assert_eq!(s.fields[1], b"4807.038");
    assert_eq!(s.fields[2], b"N");
    assert_eq!(s.fields[12], b"");
    assert_eq!(s.fields[13], b"");
    // Raw excludes the trailing CRLF.
    assert!(!s.raw.ends_with(b"\r\n"));
}

#[test]
fn golden_gga_basic_via_streaming() {
    let mut parser = Streaming::new();
    parser.feed(GGA_BASIC);
    let s = parser.next_sentence().unwrap().unwrap();
    assert_eq!(s.sentence_type, "GGA");
    assert!(parser.next_sentence().is_none());
}

// ---------------------------------------------------------------------------
// 02: $INHDT with no terminator (UDP-datagram case)
// ---------------------------------------------------------------------------
const HDT_NO_TERM: &[u8] = include_bytes!("fixtures/02_hdt_no_terminator.nmea");

#[test]
fn golden_hdt_no_terminator_via_one_shot() {
    let mut parser = OneShot::new();
    parser.feed(HDT_NO_TERM);
    let s = parser.next_sentence().unwrap().unwrap();
    assert_eq!(s.talker, Some(*b"IN"));
    assert_eq!(s.sentence_type, "HDT");
    assert_eq!(s.fields.len(), 2);
    assert_eq!(s.fields[0], b"123.4");
    assert_eq!(s.fields[1], b"T");
    // No terminator in the input, so raw == the whole input.
    assert_eq!(s.raw, HDT_NO_TERM);
}

// ---------------------------------------------------------------------------
// 03: !AIVDM encapsulation sentence
// ---------------------------------------------------------------------------
const AIVDM: &[u8] = include_bytes!("fixtures/03_aivdm_encapsulation.nmea");

#[test]
fn golden_aivdm_encapsulation() {
    let mut parser = OneShot::new();
    parser.feed(AIVDM);
    let s = parser.next_sentence().unwrap().unwrap();
    assert_eq!(s.start_delimiter, b'!');
    assert_eq!(s.talker, Some(*b"AI"));
    assert_eq!(s.sentence_type, "VDM");
    assert_eq!(s.fields.len(), 6);
    assert_eq!(s.fields[0], b"1"); // fragment count
    assert_eq!(s.fields[1], b"1"); // fragment number
    assert_eq!(s.fields[2], b""); // sequential message ID (empty for single-frag)
    assert_eq!(s.fields[3], b"A"); // channel
    assert_eq!(s.fields[4], b"13aGmP0P00PD;88MD5MTDww@2<0L"); // armored payload
    assert_eq!(s.fields[5], b"0"); // fill-bit count
}

// ---------------------------------------------------------------------------
// 04: $GPRMC with lowercase hex checksum (PRD §E2)
// ---------------------------------------------------------------------------
const RMC_LOWERCASE: &[u8] = include_bytes!("fixtures/04_rmc_lowercase_checksum.nmea");

#[test]
fn golden_rmc_lowercase_checksum_accepted() {
    let mut parser = OneShot::new();
    parser.feed(RMC_LOWERCASE);
    let s = parser.next_sentence().unwrap().unwrap();
    assert_eq!(s.sentence_type, "RMC");
    assert!(s.checksum_ok);
    // Confirm the raw byte sequence in the fixture really is lowercase.
    let raw = s.raw;
    let hex_slice = &raw[raw.len() - 2..];
    assert!(
        hex_slice
            .iter()
            .all(|b| b.is_ascii_digit() || (*b >= b'a' && *b <= b'f')),
        "fixture should contain lowercase hex digits; got {hex_slice:?}"
    );
}

// ---------------------------------------------------------------------------
// 05: Three back-to-back sentences with CRLF / LF / CR terminators
// ---------------------------------------------------------------------------
const STREAM_MIXED: &[u8] = include_bytes!("fixtures/05_stream_mixed_terminators.nmea");

#[test]
fn golden_stream_mixed_terminators() {
    let mut parser = Streaming::new();
    parser.feed(STREAM_MIXED);

    assert_eq!(
        parser.next_sentence().unwrap().unwrap().sentence_type,
        "GGA",
        "first sentence (CRLF-terminated)"
    );
    assert_eq!(
        parser.next_sentence().unwrap().unwrap().sentence_type,
        "HDT",
        "second sentence (LF-only terminated)"
    );
    assert_eq!(
        parser.next_sentence().unwrap().unwrap().sentence_type,
        "VTG",
        "third sentence (CR-only terminated)"
    );
    assert!(parser.next_sentence().is_none());
}

// ---------------------------------------------------------------------------
// 06: TAG block + sentence
// ---------------------------------------------------------------------------
const TAGGED: &[u8] = include_bytes!("fixtures/06_tagged_sentence.nmea");

#[test]
fn golden_tagged_sentence_via_one_shot() {
    let mut parser = OneShot::new();
    parser.feed(TAGGED);
    let s = parser.next_sentence().unwrap().unwrap();
    assert_eq!(s.tag_block, Some(b"c:1577836800".as_slice()));
    assert_eq!(s.sentence_type, "GGA");
    // Raw must NOT include the TAG prefix.
    assert!(s.raw.starts_with(b"$GPGGA"));
}

#[test]
fn golden_tagged_sentence_via_streaming() {
    let mut parser = Streaming::new();
    parser.feed(TAGGED);
    let s = parser.next_sentence().unwrap().unwrap();
    assert_eq!(s.tag_block, Some(b"c:1577836800".as_slice()));
    assert_eq!(s.sentence_type, "GGA");
    assert!(parser.next_sentence().is_none());
}

// ---------------------------------------------------------------------------
// 07: Streaming garbage-between-sentences recovery
// ---------------------------------------------------------------------------
const STREAM_GARBAGE: &[u8] = include_bytes!("fixtures/07_streaming_with_garbage.nmea");

#[test]
fn golden_streaming_recovers_from_garbage() {
    let mut parser = Streaming::new();
    parser.feed(STREAM_GARBAGE);

    let first = parser.next_sentence().unwrap().unwrap();
    assert_eq!(first.sentence_type, "GGA");
    let second = parser.next_sentence().unwrap().unwrap();
    assert_eq!(second.sentence_type, "HDT");
    assert!(parser.next_sentence().is_none());
}
