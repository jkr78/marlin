//! Bit-level reader for AIS message payloads.
//!
//! Reads fields of arbitrary width (1..=64 bits) from a densely-packed
//! byte buffer, MSB-first within each byte. Each individual byte holds
//! 8 bits of the stream; fields may span byte boundaries.
//!
//! The buffer layout is what [`crate::armor::decode`] produces: 6 bits
//! per source character, concatenated and packed into `Vec<u8>` from
//! the MSB end of the first byte down. `total_bits` tells the reader
//! how many leading bits of the buffer are valid (fill bits at the end
//! are padding the reader must not consume).
//!
//! Reading past `total_bits` yields **saturating zeros** — consistent
//! with this crate's panic-free contract (PRD §Q3). Callers that need
//! to detect underrun inspect [`BitReader::remaining`] before or after
//! a read.

use alloc::string::String;

/// AIS 6-bit character table, ITU-R M.1371-5 Table 47 (§8.2.5).
///
/// Maps 6-bit values 0..=63 to printable ASCII. Downstream string
/// consumers typically trim trailing `@` and spaces — this table
/// returns them verbatim.
const AIS_CHARS: &[u8; 64] = b"@ABCDEFGHIJKLMNOPQRSTUVWXYZ[\\]^_ !\"#$%&'()*+,-./0123456789:;<=>?";

/// Cursor-based reader over a bit-packed AIS payload.
///
/// Construct with [`new`](Self::new); read with [`u`](Self::u),
/// [`i`](Self::i), [`b`](Self::b), or [`string`](Self::string); check
/// [`remaining`](Self::remaining) to detect underrun.
///
/// # Example
///
/// ```
/// use marlin_ais::BitReader;
///
/// // 16 bits: 1111_1111_0000_0001
/// let buf = [0xFF, 0x01];
/// let mut r = BitReader::new(&buf, 16);
/// assert_eq!(r.u(8), 0xFF);
/// assert_eq!(r.u(8), 0x01);
/// assert_eq!(r.remaining(), 0);
/// ```
#[derive(Debug, Clone)]
pub struct BitReader<'a> {
    bits: &'a [u8],
    total_bits: usize,
    cursor: usize,
}

impl<'a> BitReader<'a> {
    /// Create a reader over `bits` with `total_bits` of valid data.
    /// Bits past `total_bits` are padding and will not be returned.
    #[must_use]
    pub fn new(bits: &'a [u8], total_bits: usize) -> Self {
        Self {
            bits,
            total_bits,
            cursor: 0,
        }
    }

    /// Number of bits remaining between the cursor and `total_bits`.
    #[must_use]
    pub fn remaining(&self) -> usize {
        self.total_bits.saturating_sub(self.cursor)
    }

    /// Read `n` unsigned bits. `n` must be ≤ 64 (larger values are
    /// clamped). Reading past `total_bits` returns saturating zeros
    /// for the missing tail.
    #[allow(clippy::indexing_slicing)] // byte_idx derived from bit_pos < total_bits check
    pub fn u(&mut self, n: usize) -> u64 {
        let n = n.min(64);
        if n == 0 {
            return 0;
        }
        let mut value: u64 = 0;
        for i in 0..n {
            let bit_pos = self.cursor.saturating_add(i);
            if bit_pos >= self.total_bits {
                // Saturate — fill remaining bits with 0.
                value <<= n.saturating_sub(i);
                break;
            }
            let byte_idx = bit_pos / 8;
            let bit_in_byte = 7 - (bit_pos % 8);
            // Guard byte access — buffer may be shorter than total_bits
            // implies (callers are responsible for sizing, but don't
            // panic if they get it wrong).
            let Some(&byte) = self.bits.get(byte_idx) else {
                value <<= n.saturating_sub(i);
                break;
            };
            let bit = u64::from((byte >> bit_in_byte) & 1);
            value = (value << 1) | bit;
        }
        self.cursor = self.cursor.saturating_add(n);
        value
    }

    /// Read `n` signed bits as two's-complement at field width.
    /// `n` must be ≥ 1 (0 returns 0) and ≤ 64.
    ///
    /// The AIS spec often uses arbitrary widths (27-bit latitude,
    /// 28-bit longitude, 8-bit rate-of-turn with a signed range) and
    /// sign-extends within that width, not within a byte boundary.
    pub fn i(&mut self, n: usize) -> i64 {
        let n = n.min(64);
        if n == 0 {
            return 0;
        }
        let u_val = self.u(n);
        let sign_bit = 1u64 << (n.saturating_sub(1));
        if u_val & sign_bit == 0 {
            // Non-negative — top bits are already zero.
            #[allow(clippy::cast_possible_wrap)] // top bit clear, fits i64
            {
                u_val as i64
            }
        } else {
            // Negative — two's-complement extension.
            // Full-width negative: u_val has bit (n-1) set; the true
            // value is u_val - 2^n. For n < 64 this fits in i64;
            // for n == 64 the value is already in i64's range when
            // interpreted as wrapping.
            let two_n: u64 = if n == 64 { 0 } else { 1u64 << n };
            // wrapping_sub handles the n==64 case (two_n=0 wraps).
            #[allow(clippy::cast_possible_wrap)] // intentional two's-complement
            {
                (u_val.wrapping_sub(two_n)) as i64
            }
        }
    }

    /// Read a single bit.
    pub fn b(&mut self) -> bool {
        self.u(1) != 0
    }

    /// Read `chars` × 6 bits and decode as AIS 6-bit ASCII per
    /// ITU-R M.1371-5 Table 47.
    ///
    /// Trailing `@` padding is **not** trimmed — callers that want
    /// clean ship names or destinations should trim `@` and spaces
    /// themselves.
    #[allow(clippy::indexing_slicing)] // idx < 64 guaranteed by .min(63)
    pub fn string(&mut self, chars: usize) -> String {
        let mut out = String::with_capacity(chars);
        for _ in 0..chars {
            // `u(6)` returns 0..=63; fits losslessly in any usize.
            let idx = (self.u(6) & 0x3F) as usize;
            out.push(AIS_CHARS[idx] as char);
        }
        out
    }
}

// ---------------------------------------------------------------------------
// Tests (PRD §T5)
// ---------------------------------------------------------------------------

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]
mod tests {
    use super::*;

    // -----------------------------------------------------------------
    // Unsigned reads across widths
    // -----------------------------------------------------------------

    #[test]
    fn u1_reads_single_bits_msb_first() {
        // 1010_1010 = 0xAA
        let buf = [0xAA];
        let mut r = BitReader::new(&buf, 8);
        assert_eq!(r.u(1), 1);
        assert_eq!(r.u(1), 0);
        assert_eq!(r.u(1), 1);
        assert_eq!(r.u(1), 0);
    }

    #[test]
    fn u6_reads_within_a_byte() {
        // 1010_1100 = 0xAC — top 6 bits are 101011 = 43.
        let buf = [0xAC];
        let mut r = BitReader::new(&buf, 8);
        assert_eq!(r.u(6), 43);
    }

    #[test]
    fn u8_reads_full_byte() {
        let buf = [0xFF];
        let mut r = BitReader::new(&buf, 8);
        assert_eq!(r.u(8), 0xFF);
    }

    #[test]
    fn u16_spans_byte_boundary() {
        let buf = [0x12, 0x34];
        let mut r = BitReader::new(&buf, 16);
        assert_eq!(r.u(16), 0x1234);
    }

    #[test]
    fn u30_spans_multiple_bytes_with_trailing_bits() {
        // 30 bits starting from MSB of a 4-byte buffer:
        // 0x12345678 = 0001_0010_0011_0100_0101_0110_0111_1000
        // Top 30 bits = 0001_0010_0011_0100_0101_0110_0111_10 = 0x048D159E
        let buf = [0x12, 0x34, 0x56, 0x78];
        let mut r = BitReader::new(&buf, 32);
        assert_eq!(r.u(30), 0x048D_159E);
        // Two bits remain: 00.
        assert_eq!(r.u(2), 0);
    }

    #[test]
    fn u32_reads_exactly_four_bytes() {
        let buf = [0xDE, 0xAD, 0xBE, 0xEF];
        let mut r = BitReader::new(&buf, 32);
        assert_eq!(r.u(32), 0xDEAD_BEEF);
    }

    #[test]
    fn u64_reads_eight_bytes() {
        let buf = [0x01, 0x23, 0x45, 0x67, 0x89, 0xAB, 0xCD, 0xEF];
        let mut r = BitReader::new(&buf, 64);
        assert_eq!(r.u(64), 0x0123_4567_89AB_CDEF);
    }

    // -----------------------------------------------------------------
    // Signed reads — the single most error-prone area (PRD §A2)
    // -----------------------------------------------------------------

    #[test]
    fn i8_positive_zero_and_negative() {
        // 127 = 0x7F, 0 = 0x00, -128 = 0x80
        let buf = [0x7F, 0x00, 0x80];
        let mut r = BitReader::new(&buf, 24);
        assert_eq!(r.i(8), 127);
        assert_eq!(r.i(8), 0);
        assert_eq!(r.i(8), -128);
    }

    #[test]
    fn i27_latitude_sign_handling() {
        // 27-bit latitude: AIS sentinel for "not available" is 0x3412140
        // (decimal 54_600_000 ≡ 91°). For negative values, bit 26 is set.
        //
        // Pick a known negative: -45_000 in 27-bit two's complement.
        // -45_000 mod 2^27 = 2^27 - 45_000 = 134_172_728 = 0x07FF_5038
        // Pack that into 27 MSB-first bits of a 4-byte buffer.
        let val: u32 = 0x07FF_5038;
        // Shift into the top 27 bits of a 32-bit word.
        let packed = val << 5;
        let buf = packed.to_be_bytes();
        let mut r = BitReader::new(&buf, 27);
        assert_eq!(r.i(27), -45_000);
    }

    #[test]
    fn i28_longitude_sign_handling() {
        // Positive: 11° in 1/10000-minute units = 11 × 600_000 = 6_600_000.
        // 6_600_000 = 0x0064_B540.
        let val: u32 = 0x0064_B540;
        let packed = val << 4; // top 28 bits
        let buf = packed.to_be_bytes();
        let mut r = BitReader::new(&buf, 28);
        assert_eq!(r.i(28), 6_600_000);
    }

    #[test]
    fn i_at_signed_minimum_for_width() {
        // 8-bit minimum: -128.
        // 16-bit minimum: -32768.
        let buf = [0x80, 0x80, 0x00];
        let mut r = BitReader::new(&buf, 24);
        assert_eq!(r.i(8), -128);
        assert_eq!(r.i(16), -32_768);
    }

    #[test]
    fn i_at_signed_maximum_for_width() {
        // 8-bit max: 127. 16-bit max: 32_767.
        let buf = [0x7F, 0x7F, 0xFF];
        let mut r = BitReader::new(&buf, 24);
        assert_eq!(r.i(8), 127);
        assert_eq!(r.i(16), 32_767);
    }

    // -----------------------------------------------------------------
    // Single bit
    // -----------------------------------------------------------------

    #[test]
    fn b_reads_single_bits() {
        let buf = [0b1010_0000];
        let mut r = BitReader::new(&buf, 8);
        assert!(r.b());
        assert!(!r.b());
        assert!(r.b());
        assert!(!r.b());
    }

    // -----------------------------------------------------------------
    // AIS 6-bit ASCII string
    // -----------------------------------------------------------------

    #[test]
    fn string_decodes_six_bit_ais_chars() {
        // Pack 6-bit values [1, 2, 20, 32] which map to ['A', 'B', 'T', ' '].
        // Concatenated MSB-first: 000001 000010 010100 100000
        // Byte 0 (bits 0-7):   0000 0100 = 0x04   (val 1's 6 bits + top 2 of val 2)
        // Byte 1 (bits 8-15):  0010 0101 = 0x25   (low 4 of val 2 + top 4 of val 20)
        // Byte 2 (bits 16-23): 0010 0000 = 0x20   (low 2 of val 20 + val 32)
        let buf = [0x04, 0x25, 0x20];
        let mut r = BitReader::new(&buf, 24);
        let s = r.string(4);
        assert_eq!(s, "ABT ");
    }

    #[test]
    fn string_returns_at_padding_verbatim() {
        // 6-bit value 0 maps to '@' — the canonical AIS padding char.
        let buf = [0x00, 0x00, 0x00];
        let mut r = BitReader::new(&buf, 24);
        assert_eq!(r.string(4), "@@@@");
    }

    // -----------------------------------------------------------------
    // Past-end behavior: saturating zero (PRD §T5 implementer's choice)
    // -----------------------------------------------------------------

    #[test]
    fn reads_past_end_saturate_to_zero() {
        let buf = [0xFF];
        let mut r = BitReader::new(&buf, 8);
        // Consume all 8 bits.
        assert_eq!(r.u(8), 0xFF);
        assert_eq!(r.remaining(), 0);
        // Now read past the end — should be zero.
        assert_eq!(r.u(8), 0);
        assert_eq!(r.i(8), 0);
    }

    #[test]
    fn partial_read_past_end_fills_with_zeros() {
        // 12 bits available, read 16. Last 4 bits should be zero.
        let buf = [0xFF, 0xF0];
        let mut r = BitReader::new(&buf, 12);
        // 0xFFF followed by 4 zero bits = 0xFFF0
        assert_eq!(r.u(16), 0xFFF0);
    }

    // -----------------------------------------------------------------
    // remaining() tracks the cursor
    // -----------------------------------------------------------------

    #[test]
    fn remaining_decreases_with_reads() {
        let buf = [0x00, 0x00];
        let mut r = BitReader::new(&buf, 16);
        assert_eq!(r.remaining(), 16);
        r.u(3);
        assert_eq!(r.remaining(), 13);
        r.u(10);
        assert_eq!(r.remaining(), 3);
        r.u(3);
        assert_eq!(r.remaining(), 0);
    }

    // -----------------------------------------------------------------
    // n=0 returns 0 without advancing
    // -----------------------------------------------------------------

    #[test]
    fn zero_width_read_returns_zero_and_does_not_advance() {
        let buf = [0xFF];
        let mut r = BitReader::new(&buf, 8);
        assert_eq!(r.u(0), 0);
        assert_eq!(r.i(0), 0);
        assert_eq!(r.remaining(), 8);
    }
}
