// ASCII85Decode filter (PDF 32000-1:2008 §7.4.3). Whole-buffer
// codec. 4-byte groups expand to 5 base-85 chars '!'..'u', with
// the 'z' shortcut for all-zero full groups. Caller strips the
// '~>' EOD before invoking decode.

use std::fmt;

/// Errors returned by [`decode`] on malformed input. [`encode`] is
/// total and never fails.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// Decoder hit a byte that isn't whitespace, 'z' at a group
    /// boundary, or '!'..'u'. Carries the offending byte.
    InvalidChar(u8),
    /// 'z' shortcut appeared mid-group (only legal at a group
    /// start).
    ZeroShortcutInPartial,
    /// Trailing partial group has exactly 1 character — needs at
    /// least 2 for a valid output byte.
    OrphanPartialGroup,
    /// Decoded 32-bit value exceeded 0xFFFFFFFF (only possible
    /// when input chars push the base-85 sum past that bound).
    OverflowValue,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::InvalidChar(b) => write!(
                f, "ascii85: invalid character 0x{:02X}", b),
            Error::ZeroShortcutInPartial => write!(
                f, "ascii85: 'z' shortcut inside partial group"),
            Error::OrphanPartialGroup => write!(
                f, "ascii85: partial group of length 1 is malformed"),
            Error::OverflowValue => write!(
                f, "ascii85: decoded group value exceeds 0xFFFFFFFF"),
        }
    }
}

impl std::error::Error for Error {}

/// Encode `input` as ASCII85. Output uses 'z' shortcut for
/// full-zero 4-byte groups; trailing partial group emits
/// (count + 1) characters with no 'z' shortcut. Never fails.
#[allow(dead_code)] // test-oracle helper, reached by the matching tests/internal/ gate driver via #[path]
pub fn encode(input: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(input.len() * 5 / 4 + 5);
    let chunks = input.chunks_exact(4);
    let remainder = chunks.remainder();
    for chunk in chunks {
        let v = u32::from_be_bytes(
            [chunk[0], chunk[1], chunk[2], chunk[3]]);
        if v == 0 {
            out.push(b'z');
        } else {
            emit_group(&mut out, v, 5);
        }
    }
    if !remainder.is_empty() {
        // Pad with zero bytes to four; emit (n+1) chars; no 'z'
        // shortcut on partial group even when padded value is 0.
        let mut buf = [0u8; 4];
        buf[..remainder.len()].copy_from_slice(remainder);
        let v = u32::from_be_bytes(buf);
        emit_group(&mut out, v, remainder.len() + 1);
    }
    out
}

#[allow(dead_code)] // test-oracle helper, reached by the matching tests/internal/ gate driver via #[path]
fn emit_group(out: &mut Vec<u8>, v: u32, count: usize) {
    let mut tmp = [0u8; 5];
    let mut x = v;
    for i in (0..5).rev() {
        tmp[i] = b'!' + (x % 85) as u8;
        x /= 85;
    }
    out.extend_from_slice(&tmp[..count]);
}

/// Decode ASCII85 bytes back to the original payload. Skips PDF
/// whitespace. 'z' at group boundary expands to four zero bytes.
/// Trailing 2..5-char partial group decodes (count - 1) bytes
/// using implicit 'u' padding.
pub fn decode(input: &[u8]) -> Result<Vec<u8>, Error> {
    let mut out = Vec::with_capacity(input.len() * 4 / 5);
    // Group buffer — collects up to 5 base-85 chars before flush.
    let mut group = [0u32; 5];
    let mut count = 0_usize;
    for &b in input {
        match b {
            // PDF whitespace per §7.2.3.
            0x00 | 0x09 | 0x0A | 0x0C | 0x0D | b' ' => continue,
            b'z' => {
                if count != 0 {
                    return Err(Error::ZeroShortcutInPartial);
                }
                out.extend_from_slice(&[0u8; 4]);
            }
            b'!'..=b'u' => {
                group[count] = (b - b'!') as u32;
                count += 1;
                if count == 5 {
                    let v = combine_group(&group, 5)?;
                    let bytes = v.to_be_bytes();
                    out.extend_from_slice(&bytes);
                    count = 0;
                }
            }
            _ => return Err(Error::InvalidChar(b)),
        }
    }
    if count == 1 {
        return Err(Error::OrphanPartialGroup);
    }
    if count >= 2 {
        // Pad with implicit 'u' (84) chars.
        for slot in &mut group[count..5] {
            *slot = 84;
        }
        let v = combine_group(&group, 5)?;
        let bytes = v.to_be_bytes();
        // Emit (count - 1) high bytes.
        out.extend_from_slice(&bytes[..count - 1]);
    }
    Ok(out)
}

fn combine_group(group: &[u32; 5], count: usize) -> Result<u32, Error> {
    let mut v: u64 = 0;
    for &digit in &group[..count] {
        v = v * 85 + digit as u64;
    }
    if v > u32::MAX as u64 {
        return Err(Error::OverflowValue);
    }
    Ok(v as u32)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encode_empty() {
        assert_eq!(encode(b""), b"");
    }

    #[test]
    fn encode_zero_shortcut() {
        // 4 zero bytes -> 'z'
        assert_eq!(encode(b"\x00\x00\x00\x00"), b"z");
    }

    #[test]
    fn encode_full_group() {
        // PostScript reference example.
        // "Man " (0x4D616E20) -> "9jqo^"
        assert_eq!(encode(b"Man "), b"9jqo^");
    }

    #[test]
    fn encode_partial_group_no_z_shortcut() {
        // 3 zero bytes — partial group, must NOT use 'z'.
        let out = encode(b"\x00\x00\x00");
        assert_eq!(out.len(), 4); // count + 1
        assert_ne!(out, b"z"[..]);
    }

    #[test]
    fn decode_empty() {
        assert_eq!(decode(b"").unwrap(), b"");
    }

    #[test]
    fn decode_zero_shortcut() {
        assert_eq!(decode(b"z").unwrap(), b"\x00\x00\x00\x00");
    }

    #[test]
    fn decode_skips_whitespace() {
        assert_eq!(decode(b"9jqo^").unwrap(), b"Man ");
        assert_eq!(decode(b"9j\nqo^\t").unwrap(), b"Man ");
    }

    #[test]
    fn decode_rejects_orphan_partial() {
        assert_eq!(decode(b"9"), Err(Error::OrphanPartialGroup));
    }

    #[test]
    fn decode_rejects_z_in_partial() {
        // 'z' inside a partial group is malformed.
        assert_eq!(
            decode(b"9z"), Err(Error::ZeroShortcutInPartial));
    }

    #[test]
    fn decode_rejects_invalid_char() {
        // 'v' (118) is just above 'u' (117).
        assert_eq!(
            decode(b"9jqv^"), Err(Error::InvalidChar(b'v')));
    }

    #[test]
    fn roundtrip_max_value_group() {
        // 0xFFFFFFFF -> "s8W-!"
        let input = b"\xFF\xFF\xFF\xFF";
        let encoded = encode(input);
        assert_eq!(decode(&encoded).unwrap(), input);
    }

    #[test]
    fn roundtrip_full_byte_range() {
        let input: Vec<u8> = (0..=255).collect();
        let encoded = encode(&input);
        assert_eq!(decode(&encoded).unwrap(), input);
    }

    #[test]
    fn roundtrip_all_partial_lengths() {
        for len in 1..=8 {
            let input: Vec<u8> = (0..len as u8).collect();
            let encoded = encode(&input);
            assert_eq!(decode(&encoded).unwrap(), input,
                       "round-trip failed for len={}", len);
        }
    }
}
