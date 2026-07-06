/// ST 0601 Tag 1 checksum: 16-bit running sum over the packet bytes with an alternating byte
/// shift (even index → high byte, odd index → low byte). Caller passes everything from
/// `UAS_LS_KEY[0]` through the checksum item's length byte inclusive.
pub(crate) fn bcc(bytes: &[u8]) -> u16 {
    let mut sum: u16 = 0;
    for (i, b) in bytes.iter().enumerate() {
        sum = sum.wrapping_add(u16::from(*b) << (8 * ((i + 1) % 2)));
    }
    sum
}
