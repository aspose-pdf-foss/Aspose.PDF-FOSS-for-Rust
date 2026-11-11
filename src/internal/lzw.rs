// LZWDecode filter (PDF 32000-1:2008 §7.4.4.2 / TIFF 6.0 §13)
// — whole-buffer adapter. Encode emits a leading ClearCode
// (256), variable-width codes 9 → 10 → 11 → 12, and a trailing
// EOD (257) padded to a whole byte. Decode mirrors the same
// state machine and stops at EOD.
//
// EarlyChange controls the bit-width transition timing — the
// dialect-defining knob between PDF (default 1) and TIFF
// (always 0). See [`EarlyChange`] for the precise semantics.
// Bit packing is MSB-FIRST: the first bit of the first emitted
// code is the HIGH bit of the first output byte.

use std::collections::HashMap;
use std::fmt;

const CLEAR_CODE: u16 = 256;
const EOD_CODE: u16 = 257;
const FIRST_FREE: u16 = 258;
const MAX_CODE: u16 = 4095; // 12-bit ceiling
const MIN_WIDTH: u8 = 9;
const MAX_WIDTH: u8 = 12;

/// Bit-width transition timing — the byte-stream-defining knob
/// that differs between the PDF and TIFF LZW dialects.
///
/// * [`EarlyChange::Enabled`] — encoder bumps when `next_code`
///   reaches `(1 << width) - 1` (i.e., AFTER installing slot
///   `2^width - 2`). PDF default per ISO 32000-1:2008 §7.4.4.3
///   (LzwDecode `EarlyChange = 1`).
/// * [`EarlyChange::Disabled`] — encoder bumps when `next_code`
///   reaches `(1 << width)` (i.e., AFTER installing slot
///   `2^width - 1`). TIFF 6.0 §13 mandates this; the TIFF
///   spec describes it as "switch the next time `next_code`
///   would exceed `2^width - 1`". The PDF filter parameter
///   `EarlyChange = 0` selects this mode for PDF streams as well.
///
/// The decoder transitions ONE INSERT EARLIER than the encoder
/// in either case — the decoder skips its install on the first
/// non-Clear read (no `prev`), so its `next` lags the encoder's
/// by one; the lag is compensated inside [`decode`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EarlyChange {
    /// PDF default — bump at `next == 2^width - 1`.
    Enabled,
    /// TIFF default — bump at `next == 2^width`.
    Disabled,
}

impl EarlyChange {
    /// Returns the `next_code` threshold that triggers the
    /// encoder's width-up transition. With `width=9`:
    /// `Enabled → 511`, `Disabled → 512`.
    #[inline]
    fn encoder_threshold(self, width: u8) -> u16 {
        let base = 1u16 << width;
        match self {
            EarlyChange::Enabled => base.wrapping_sub(1),
            EarlyChange::Disabled => base,
        }
    }
}

/// Errors returned by [`decode`]. [`encode`] is total.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// The bit reader ran out of input before observing an EOD
    /// (257) code. A correctly-encoded stream always ends with
    /// EOD followed by zero-bit padding to a whole byte.
    Truncated,
    /// Decoder read a code that is neither in the table nor the
    /// `next` slot — malformed stream. KwKwK references `next`
    /// exactly; anything past `next` is invalid.
    CodeOutOfRange { code: u16, next: u16 },
    /// First non-Clear code in a fresh state was equal to
    /// `next` (the KwKwK marker) — but there is no `prev` to
    /// reconstruct from. A conformant encoder never produces
    /// this, but a tampered or truncated stream can.
    KwkwkWithoutPrev,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Truncated => write!(
                f, "lzw: truncated stream (no EOD)"),
            Error::CodeOutOfRange { code, next } => write!(
                f, "lzw: code {code} out of range (next={next})"),
            Error::KwkwkWithoutPrev => write!(
                f, "lzw: KwKwK with no prevString (malformed stream)"),
        }
    }
}

impl std::error::Error for Error {}

/// LZW-encode `input` with PDF's default `EarlyChange = 1`
/// timing. Total — every byte slice has a valid encoding.
///
/// TIFF-flavour LZW streams (mandatory `EarlyChange = 0`) must
/// route through [`encode_with`] with [`EarlyChange::Disabled`]
/// — third-party TIFF readers (libtiff, ImageMagick, PIL) reject
/// the PDF-flavour stream at the first width transition.
#[allow(dead_code)] // test-oracle helper, reached by the matching tests/internal/ gate driver via #[path]
pub fn encode(input: &[u8]) -> Vec<u8> {
    encode_with(input, EarlyChange::Enabled)
}

/// LZW-encode `input` with the caller-supplied `early_change`
/// dialect selector. Always emits ClearCode first and EOD last,
/// with the trailing partial byte zero-padded to a whole byte.
pub fn encode_with(input: &[u8], early_change: EarlyChange) -> Vec<u8> {
    // Generous output upper bound: every input byte produces at
    // most one 12-bit code, plus the leading ClearCode + trailing
    // EOD + padding + slack for periodic ClearCodes when the
    // dictionary resets. 2 * input.len() + 32 is comfortable.
    let cap = input.len() * 2 + 32;
    let mut writer = BitWriter::with_capacity(cap);

    // Encoder dictionary: key = prev_code << 8 | byte → new_code.
    // HashMap keeps the encode loop simple; the dict is small
    // (≤ 4096 entries) so a bespoke table is unnecessary.
    let mut dict: HashMap<u32, u16> = HashMap::with_capacity(MAX_CODE as usize);
    let mut next: u16 = FIRST_FREE;
    let mut width: u8 = MIN_WIDTH;

    // Always emit the leading ClearCode.
    writer.write_bits(CLEAR_CODE as u32, width);

    if input.is_empty() {
        writer.write_bits(EOD_CODE as u32, width);
        writer.flush();
        return writer.into_bytes();
    }

    // First byte primes `current` — every byte 0..255 is in the
    // initial table without an explicit map entry.
    let mut current: u16 = input[0] as u16;
    for &k in &input[1..] {
        let key = ((current as u32) << 8) | k as u32;
        if let Some(&found) = dict.get(&key) {
            current = found;
            continue;
        }
        // No match — emit the code for `current`, install the
        // new (prev, k) pair, advance the width if appropriate,
        // and restart with k as the new prefix.
        writer.write_bits(current as u32, width);
        dict.insert(key, next);
        next += 1;

        // Width-up transition fires at the dialect-selected
        // threshold. The transition order matters — we
        // increment width AFTER inserting the just-issued code
        // so the NEXT emitted code uses the wider width.
        if next == early_change.encoder_threshold(width)
            && width < MAX_WIDTH
        {
            width += 1;
        }
        // Dictionary full: emit ClearCode, reset everything.
        if next > MAX_CODE {
            writer.write_bits(CLEAR_CODE as u32, width);
            dict.clear();
            next = FIRST_FREE;
            width = MIN_WIDTH;
        }
        current = k as u16;
    }

    // Tail: emit code for the trailing prefix and the EOD.
    writer.write_bits(current as u32, width);
    writer.write_bits(EOD_CODE as u32, width);
    writer.flush();

    writer.into_bytes()
}

/// LZW-decode `input` assuming PDF's `EarlyChange = 1` timing.
/// TIFF-flavour streams must route through [`decode_with`].
pub fn decode(input: &[u8]) -> Result<Vec<u8>, Error> {
    decode_with(input, EarlyChange::Enabled)
}

/// LZW-decode `input` with the caller-supplied `early_change`
/// dialect selector. Reads codes at the dynamically-tracked
/// width and stops at EOD. Returns [`Error`] on malformed input.
pub fn decode_with(input: &[u8], early_change: EarlyChange) -> Result<Vec<u8>, Error> {
    // String table: `Vec<u8>` per code. Codes 0..255 are their
    // own one-byte expansions; 256 / 257 are markers (no string).
    // Using full byte sequences (vs (prev_code, byte) pairs)
    // makes every emit O(length) without a chain walk.
    let mut table: Vec<Vec<u8>> = Vec::with_capacity((MAX_CODE + 1) as usize);
    for i in 0u16..256 {
        table.push(vec![i as u8]);
    }
    // Pad slots 256 / 257 (markers, never emitted as strings).
    table.push(Vec::new());
    table.push(Vec::new());

    let mut next: u16 = FIRST_FREE;
    let mut width: u8 = MIN_WIDTH;
    let mut reader = BitReader::new(input);
    let mut prev: Option<Vec<u8>> = None;
    let mut out: Vec<u8> = Vec::with_capacity(input.len() * 4);

    loop {
        let c = match reader.read_bits(width) {
            Some(c) => c as u16,
            None => return Err(Error::Truncated),
        };
        if c == EOD_CODE {
            break;
        }
        if c == CLEAR_CODE {
            // Reset table + width; truncate dictionary back to
            // its initial 258 entries (256 literals + 2 marker
            // slots at 256 / 257) so any stale post-258 entries
            // from before the reset can't be referenced.
            table.truncate(FIRST_FREE as usize);
            next = FIRST_FREE;
            width = MIN_WIDTH;
            prev = None;
            continue;
        }
        if c > next {
            return Err(Error::CodeOutOfRange { code: c, next });
        }

        let emit: Vec<u8> = if (c as usize) < table.len() && c < next {
            table[c as usize].clone()
        } else {
            // KwKwK: c == next — encoder referenced the code
            // it's about to install. Reconstruct: emit = prev + prev[0].
            let p = prev.as_ref().ok_or(Error::KwkwkWithoutPrev)?;
            if p.is_empty() {
                return Err(Error::KwkwkWithoutPrev);
            }
            let mut e = Vec::with_capacity(p.len() + 1);
            e.extend_from_slice(p);
            e.push(p[0]);
            e
        };

        out.extend_from_slice(&emit);

        // Install (prev + emit[0]) at `next` — the encoder did
        // the same, so the dictionaries stay in sync.
        if let Some(p) = &prev {
            if next <= MAX_CODE {
                let mut entry = Vec::with_capacity(p.len() + 1);
                entry.extend_from_slice(p);
                entry.push(emit[0]);
                if (next as usize) < table.len() {
                    table[next as usize] = entry;
                } else {
                    table.push(entry);
                }
                next += 1;
                // Width-up transition. The decoder lags the
                // encoder by one install (encoder installs in
                // its first emit-iteration; the decoder skips
                // the install on its first read because there
                // is no `prev`). To stay synchronised, the
                // decoder triggers one entry earlier than the
                // encoder: at `next == encoder_threshold - 1`.
                if next == early_change.encoder_threshold(width).wrapping_sub(1)
                    && width < MAX_WIDTH
                {
                    width += 1;
                }
            }
        }

        prev = Some(emit);
    }

    Ok(out)
}

// MSB-FIRST bit writer. Bits accumulate into a 32-bit buffer
// kept top-aligned: the next bit to flush is the MSB of `buf`.
struct BitWriter {
    out: Vec<u8>,
    buf: u32,
    held: u8,
}

impl BitWriter {
    fn with_capacity(cap: usize) -> Self {
        BitWriter { out: Vec::with_capacity(cap), buf: 0, held: 0 }
    }

    /// Write `n` bits (1..=16) of `code`, MSB first.
    fn write_bits(&mut self, code: u32, n: u8) {
        debug_assert!((1..=16).contains(&n));
        let mask = (1u32 << n) - 1;
        let bits = code & mask;
        self.buf |= bits << (32 - n - self.held);
        self.held += n;
        while self.held >= 8 {
            self.out.push((self.buf >> 24) as u8);
            self.buf <<= 8;
            self.held -= 8;
        }
    }

    /// Flush any partial trailing byte, zero-padding the low
    /// bits. The decoder doesn't read past EOD so the pad's
    /// value is irrelevant to correctness, but the encoder must
    /// produce a whole number of bytes so its output composes
    /// with downstream filter chains.
    fn flush(&mut self) {
        if self.held > 0 {
            self.out.push((self.buf >> 24) as u8);
            self.buf = 0;
            self.held = 0;
        }
    }

    fn into_bytes(self) -> Vec<u8> {
        self.out
    }
}

// MSB-FIRST bit reader. Pulls bytes one at a time, accumulating
// into `buf` top-aligned (MSB of the next byte goes immediately
// after the bits already held).
struct BitReader<'a> {
    bytes: &'a [u8],
    p: usize,
    buf: u32,
    held: u8,
}

impl<'a> BitReader<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        BitReader { bytes, p: 0, buf: 0, held: 0 }
    }

    /// Read `n` bits (1..=16) MSB first. Returns `None` if the
    /// underlying byte stream is exhausted before `n` bits are
    /// available.
    fn read_bits(&mut self, n: u8) -> Option<u32> {
        debug_assert!((1..=16).contains(&n));
        while self.held < n {
            if self.p >= self.bytes.len() {
                return None;
            }
            self.buf |= (self.bytes[self.p] as u32) << (24 - self.held);
            self.p += 1;
            self.held += 8;
        }
        let code = (self.buf >> (32 - n)) & ((1u32 << n) - 1);
        self.buf <<= n;
        self.held -= n;
        Some(code)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn roundtrip_with(input: &[u8], early_change: EarlyChange) {
        let encoded = encode_with(input, early_change);
        let decoded = decode_with(&encoded, early_change)
            .unwrap_or_else(|e| panic!("decode failed: {:?}", e));
        assert_eq!(decoded, input);
    }

    fn roundtrip(input: &[u8]) {
        let encoded = encode(input);
        let decoded = decode(&encoded)
            .unwrap_or_else(|e| panic!("decode failed: {e:?}"));
        assert_eq!(decoded, input,
            "round-trip mismatch (input len {})", input.len());
    }

    #[test]
    fn empty_roundtrip() {
        roundtrip(&[]);
    }

    #[test]
    fn single_byte_roundtrip() {
        roundtrip(&[0xAB]);
    }

    #[test]
    fn short_repeat_kwkwk() {
        // 128 copies of 'A' — exercises the KwKwK path early.
        let input = vec![b'A'; 128];
        roundtrip(&input);
    }

    #[test]
    fn all_byte_values_roundtrip() {
        let input: Vec<u8> = (0..=255).collect();
        roundtrip(&input);
    }

    #[test]
    fn highly_compressible_crosses_width_boundaries() {
        // Repeating short pattern fills the dictionary quickly and
        // exercises the 9 → 10, 10 → 11, 11 → 12 transitions.
        let mut input = Vec::with_capacity(4096);
        for _ in 0..512 {
            input.extend_from_slice(b"abcdefgh");
        }
        roundtrip(&input);
    }

    #[test]
    fn high_entropy_input_grows_then_resets() {
        // 8 KiB pseudo-random — table grows past 4095 and the
        // encoder emits a mid-stream ClearCode.
        let mut input = Vec::with_capacity(8192);
        let mut x: u32 = 0x12345678;
        for _ in 0..8192 {
            x = x.wrapping_mul(1664525).wrapping_add(1013904223);
            input.push((x >> 16) as u8);
        }
        roundtrip(&input);
    }

    #[test]
    fn tiff_dialect_roundtrips_across_width_boundaries() {
        // EarlyChange::Disabled (TIFF mandate) — the byte stream
        // differs from the PDF dialect's at every 9→10 / 10→11
        // / 11→12 boundary. Internal round-trip locks in
        // self-consistency under the TIFF dialect.
        let mut input = Vec::with_capacity(4096);
        for _ in 0..512 {
            input.extend_from_slice(b"abcdefgh");
        }
        roundtrip_with(&input, EarlyChange::Disabled);

        let mut hi = Vec::with_capacity(8192);
        let mut x: u32 = 0x12345678;
        for _ in 0..8192 {
            x = x.wrapping_mul(1664525).wrapping_add(1013904223);
            hi.push((x >> 16) as u8);
        }
        roundtrip_with(&hi, EarlyChange::Disabled);
    }

    #[test]
    fn pdf_and_tiff_dialects_diverge_after_first_width_bump() {
        // The two dialects must produce different byte streams
        // when the input is large enough to trigger the 9→10
        // transition — verifies the dialect knob actually wires
        // through, not just selects equivalent code paths.
        let input: Vec<u8> = (0u32..600).map(|i| (i & 0xff) as u8).collect();
        let pdf = encode_with(&input, EarlyChange::Enabled);
        let tiff = encode_with(&input, EarlyChange::Disabled);
        assert_ne!(
            pdf, tiff,
            "PDF/TIFF dialects must diverge once a width \
             transition fires"
        );
    }

    #[test]
    fn empty_encoded_starts_with_clear() {
        let enc = encode(&[]);
        // ClearCode = 256 = 0b100000000, written as 9 bits MSB
        // first. Followed by EOD = 257 = 0b100000001 (9 bits).
        // Total = 18 bits → 3 bytes (last with 6 pad bits = 0).
        assert_eq!(enc.len(), 3);
        // First 9 bits = 256 = 0b1_0000_0000.
        // Byte 0 = 0b1000_0000 = 0x80.
        // Byte 1 high 1 bit = 0 (last bit of 256), then EOD = 0b1_0000_0001:
        //   Byte 1 next 7 bits = 1_000_0000 → byte 1 low 7 = 0b1000000.
        //   Byte 1 = 0 | 0b1000000 = 0x40.
        // Byte 2 first 2 bits = 0b01, padded with 6 zeros = 0b0100_0000 = 0x40.
        assert_eq!(enc[0], 0x80);
        assert_eq!(enc[1], 0x40);
        assert_eq!(enc[2], 0x40);
    }

    #[test]
    fn truncated_stream_errors() {
        // ClearCode + literal A but no EOD → truncated.
        let mut enc = encode(b"A");
        enc.truncate(enc.len() - 1);
        assert_eq!(decode(&enc).unwrap_err(), Error::Truncated);
    }

    #[test]
    fn out_of_range_code_errors() {
        // ClearCode (256) followed by a 9-bit code 300 (not yet
        // in the table — we haven't installed any new code, so
        // next == 258 and 300 > 258).
        let mut bw = BitWriter::with_capacity(8);
        bw.write_bits(256, 9);
        bw.write_bits(300, 9);
        bw.write_bits(EOD_CODE as u32, 9);
        bw.flush();
        let enc = bw.into_bytes();
        match decode(&enc).unwrap_err() {
            Error::CodeOutOfRange { code, next } => {
                assert_eq!(code, 300);
                assert_eq!(next, FIRST_FREE);
            }
            other => panic!("unexpected error {other:?}"),
        }
    }
}
