// ASCIIHexDecode filter (PDF 32000-1:2008 §7.4.2). Whole-buffer
// codec — input/output are owned/borrowed byte slices.

use std::fmt;

/// Errors returned by [`decode`] on malformed input. [`encode`] is
/// total and never fails.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// Decoder hit a byte that isn't a hex digit, PDF whitespace,
    /// or the '>' EOD marker. Carries the offending byte.
    InvalidChar(u8),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::InvalidChar(b) => write!(
                f, "asciihex: invalid character 0x{:02X}", b),
        }
    }
}

impl std::error::Error for Error {}

#[allow(dead_code)] // test-oracle helper, reached by the matching tests/internal/ gate driver via #[path]
const HEX_UPPER: &[u8; 16] = b"0123456789ABCDEF";

/// Encode `input` as uppercase ASCII hex pairs. Output length is
/// always exactly `2 * input.len()`. Never fails.
#[allow(dead_code)] // test-oracle helper, reached by the matching tests/internal/ gate driver via #[path]
pub fn encode(input: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(input.len() * 2);
    for &b in input {
        out.push(HEX_UPPER[((b >> 4) & 0x0F) as usize]);
        out.push(HEX_UPPER[(b & 0x0F) as usize]);
    }
    out
}

/// Decode ASCII hex bytes back to the original payload. Skips PDF
/// whitespace, accepts both cases of hex, stops on '>' EOD,
/// odd-trailing-digit treated as `<digit>0`. Returns
/// `Err(InvalidChar)` on any other byte.
pub fn decode(input: &[u8]) -> Result<Vec<u8>, Error> {
    let mut out = Vec::with_capacity(input.len() / 2);
    // Pending nibble — None when we're starting a new pair, Some
    // when we have the high nibble and are waiting for the low.
    let mut pending: Option<u8> = None;
    for &b in input {
        match b {
            // PDF whitespace per §7.2.3 — skip silently anywhere.
            0x00 | 0x09 | 0x0A | 0x0C | 0x0D | b' ' => continue,
            // '>' is the EOD marker — stop immediately. An odd
            // pending nibble is treated as followed by '0' per
            // spec.
            b'>' => break,
            _ => {}
        }
        let nibble = match b {
            b'0'..=b'9' => b - b'0',
            b'A'..=b'F' => b - b'A' + 10,
            b'a'..=b'f' => b - b'a' + 10,
            _ => return Err(Error::InvalidChar(b)),
        };
        match pending {
            None => pending = Some(nibble),
            Some(high) => {
                out.push((high << 4) | nibble);
                pending = None;
            }
        }
    }
    // Odd-trailing-digit per §7.4.2: treat pending nibble as if
    // followed by '0'.
    if let Some(high) = pending {
        out.push(high << 4);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encode_empty() {
        assert_eq!(encode(b""), b"");
    }

    #[test]
    fn encode_single_byte() {
        assert_eq!(encode(b"\x00"), b"00");
        assert_eq!(encode(b"\xFF"), b"FF");
        assert_eq!(encode(b"\xAB"), b"AB");
    }

    #[test]
    fn encode_uppercase_only() {
        // Output is always uppercase per spec.
        assert_eq!(encode(b"\xab\xcd\xef"), b"ABCDEF");
    }

    #[test]
    fn decode_empty() {
        assert_eq!(decode(b"").unwrap(), b"");
    }

    #[test]
    fn decode_uppercase_pair() {
        assert_eq!(decode(b"AB").unwrap(), b"\xAB");
    }

    #[test]
    fn decode_lowercase_accepted() {
        assert_eq!(decode(b"ab").unwrap(), b"\xAB");
        assert_eq!(decode(b"aB").unwrap(), b"\xAB");
    }

    #[test]
    fn decode_skips_whitespace() {
        assert_eq!(decode(b"AB CD\n EF\t").unwrap(),
                   b"\xAB\xCD\xEF");
    }

    #[test]
    fn decode_eod_marker() {
        assert_eq!(decode(b"AB>CD").unwrap(), b"\xAB");
    }

    #[test]
    fn decode_odd_trailing_digit() {
        // Per §7.4.2, odd trailing digit treated as <digit>0.
        assert_eq!(decode(b"A").unwrap(), b"\xA0");
        assert_eq!(decode(b"ABC").unwrap(), b"\xAB\xC0");
    }

    #[test]
    fn decode_odd_trailing_digit_before_eod() {
        assert_eq!(decode(b"ABC>EF").unwrap(), b"\xAB\xC0");
    }

    #[test]
    fn decode_rejects_invalid_byte() {
        assert_eq!(decode(b"AG"), Err(Error::InvalidChar(b'G')));
    }

    #[test]
    fn roundtrip_full_byte_range() {
        let input: Vec<u8> = (0..=255).collect();
        let encoded = encode(&input);
        assert_eq!(decode(&encoded).unwrap(), input);
    }

    #[test]
    fn roundtrip_random_pattern() {
        let input = b"The quick brown fox jumps \
                      over the lazy dog. 0123456789!";
        let encoded = encode(input);
        assert_eq!(decode(&encoded).unwrap(), input);
    }
}
