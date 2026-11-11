// RunLengthDecode filter (PDF 32000-1:2008 §7.4.5). Whole-buffer
// codec — input/output are owned/borrowed byte slices.
//
// Stream format: a sequence of segments, each starting with a
// length byte L:
//   * L in 0x00..=0x7F: LITERAL — copy next (L + 1) bytes verbatim.
//   * L == 0x80:        END-OF-DATA — stop.
//   * L in 0x81..=0xFF: REPEAT — read next byte, emit (257 - L)
//                       copies.
//
// Encode always terminates output with the 0x80 EOD marker.
// Decode treats end-of-input on a segment boundary as a clean stop
// (boundary clause — see tests/fixtures/runlength/boundary/),
// matching the spec's "Read segments until EOD or end of input."
// Truncated payload (a length byte that demands more bytes than
// remain) is malformed.

use std::fmt;

/// Errors returned by [`decode`] on malformed input. [`encode`] is
/// total and never fails.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// Decode was called on an empty buffer. A complete RunLength
    /// stream is at minimum the one-byte EOD `0x80`; empty input
    /// is ambiguous between "missing EOD" and "empty stream" and
    /// the contract requires the caller to provide at least the
    /// EOD.
    EmptyInput,
    /// Literal segment whose declared payload extends past the
    /// end of input. Carries the offset of the offending length
    /// byte.
    TruncatedLiteral { offset: usize },
    /// Repeat segment whose single payload byte is missing
    /// (end-of-input immediately after the length byte). Carries
    /// the offset of the offending length byte.
    TruncatedRepeat { offset: usize },
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::EmptyInput => write!(
                f, "runlength: empty input (missing EOD marker)"),
            Error::TruncatedLiteral { offset } => write!(
                f, "runlength: truncated literal at offset {offset}"),
            Error::TruncatedRepeat { offset } => write!(
                f, "runlength: truncated repeat at offset {offset}"),
        }
    }
}

impl std::error::Error for Error {}

/// Encode `input` as a RunLength stream. Output always ends with
/// the 0x80 EOD marker (so empty input encodes to a one-byte
/// `[0x80]`). Never fails.
pub fn encode(input: &[u8]) -> Vec<u8> {
    // Worst case: every byte is its own literal segment, so two
    // bytes out per byte in, plus the trailing EOD. Allocate that
    // up front to avoid mid-encode reallocs on small inputs.
    let mut out = Vec::with_capacity(input.len() * 2 + 1);
    let mut literal_start = 0usize;
    let mut i = 0usize;
    let n = input.len();

    while i < n {
        // Probe a run starting at i, capped at 128 (the maximum
        // single-segment repeat length). A run needs at least 2
        // identical bytes to be worth packing — a length-1 repeat
        // would be larger than a length-1 literal.
        let mut run_len = 1usize;
        while run_len < 128
            && i + run_len < n
            && input[i + run_len] == input[i]
        {
            run_len += 1;
        }
        if run_len >= 2 {
            // Flush any pending literal first (1..128 bytes), then
            // emit the repeat segment.
            flush_literal(&mut out, &input[literal_start..i]);
            out.push((257 - run_len) as u8);
            out.push(input[i]);
            i += run_len;
            literal_start = i;
        } else {
            // Single byte; defer to literal flush. If the literal
            // window has reached the per-segment cap, flush early
            // so the next byte starts a fresh segment.
            i += 1;
            if i - literal_start == 128 {
                flush_literal(&mut out, &input[literal_start..i]);
                literal_start = i;
            }
        }
    }
    flush_literal(&mut out, &input[literal_start..]);
    out.push(0x80);
    out
}

/// Append a literal segment (length byte + payload) for `chunk`.
/// `chunk.len()` must be in `0..=128`; an empty slice is a no-op.
fn flush_literal(out: &mut Vec<u8>, chunk: &[u8]) {
    if chunk.is_empty() {
        return;
    }
    debug_assert!(chunk.len() <= 128);
    out.push((chunk.len() - 1) as u8);
    out.extend_from_slice(chunk);
}

/// Decode a RunLength stream back to the original payload. Stops
/// on the 0x80 EOD marker or a clean end-of-input on a segment
/// boundary. Returns `Err` on empty input or a truncated segment.
pub fn decode(input: &[u8]) -> Result<Vec<u8>, Error> {
    if input.is_empty() {
        return Err(Error::EmptyInput);
    }
    let mut out = Vec::with_capacity(input.len());
    let mut i = 0usize;
    let n = input.len();
    while i < n {
        let header = i;
        let len_byte = input[i];
        i += 1;
        if len_byte == 0x80 {
            return Ok(out);
        }
        if len_byte <= 0x7F {
            let count = len_byte as usize + 1;
            if i + count > n {
                return Err(Error::TruncatedLiteral { offset: header });
            }
            out.extend_from_slice(&input[i..i + count]);
            i += count;
        } else {
            if i >= n {
                return Err(Error::TruncatedRepeat { offset: header });
            }
            let b = input[i];
            i += 1;
            let count = 257 - len_byte as usize;
            out.resize(out.len() + count, b);
        }
    }
    // Spec-permissive boundary clause: stream ended cleanly on a
    // segment boundary, no 0x80 EOD seen. Emit what we have.
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encode_empty_emits_only_eod() {
        assert_eq!(encode(b""), b"\x80");
    }

    #[test]
    fn encode_single_byte_literal() {
        // length byte 0x00 (literal of 1) + payload + EOD.
        assert_eq!(encode(b"A"), b"\x00A\x80");
    }

    #[test]
    fn encode_run_of_two() {
        // 257 - 2 = 255 (0xFF), payload 'A', EOD.
        assert_eq!(encode(b"AA"), b"\xFFA\x80");
    }

    #[test]
    fn encode_run_of_128() {
        // 257 - 128 = 129 (0x81), payload 'A', EOD.
        let input = vec![b'A'; 128];
        assert_eq!(encode(&input), b"\x81A\x80");
    }

    #[test]
    fn encode_run_of_129_splits() {
        // 128-run then 1-byte tail. 1-byte tail can't be a repeat
        // (repeat needs >=2), so it falls into a literal segment.
        let input = vec![b'A'; 129];
        assert_eq!(encode(&input), b"\x81A\x00A\x80");
    }

    #[test]
    fn encode_literal_of_128() {
        // 128-byte literal: length byte 0x7F + 128 distinct bytes
        // + EOD.
        let input: Vec<u8> = (0u8..128).collect();
        let encoded = encode(&input);
        assert_eq!(encoded[0], 0x7F);
        assert_eq!(&encoded[1..129], &input[..]);
        assert_eq!(encoded[129], 0x80);
        assert_eq!(encoded.len(), 130);
    }

    #[test]
    fn encode_literal_of_129_splits() {
        // 128-byte literal segment + 1-byte literal segment + EOD.
        let input: Vec<u8> = (0u8..=128).collect();
        let encoded = encode(&input);
        assert_eq!(encoded[0], 0x7F);
        assert_eq!(&encoded[1..129], &input[..128]);
        assert_eq!(encoded[129], 0x00);
        assert_eq!(encoded[130], 128);
        assert_eq!(encoded[131], 0x80);
    }

    #[test]
    fn decode_empty_input_throws() {
        assert_eq!(decode(b""), Err(Error::EmptyInput));
    }

    #[test]
    fn decode_eod_only_returns_empty() {
        assert_eq!(decode(b"\x80").unwrap(), b"");
    }

    #[test]
    fn decode_literal() {
        assert_eq!(decode(b"\x02ABC\x80").unwrap(), b"ABC");
    }

    #[test]
    fn decode_repeat() {
        // 257 - 0xFE = 3 copies of 'A'.
        assert_eq!(decode(b"\xFEA\x80").unwrap(), b"AAA");
    }

    #[test]
    fn decode_truncated_literal() {
        // length byte 0x05 (6 bytes follow) but only 3 remain.
        assert_eq!(
            decode(b"\x05ABC"),
            Err(Error::TruncatedLiteral { offset: 0 }));
    }

    #[test]
    fn decode_truncated_repeat() {
        // length byte 0xFE (repeat) but no payload byte.
        assert_eq!(
            decode(b"\xFE"),
            Err(Error::TruncatedRepeat { offset: 0 }));
    }

    #[test]
    fn decode_no_eod_boundary_repeat() {
        // Spec-permissive: complete repeat segment, no EOD.
        assert_eq!(
            decode(&[0x82, b'A']).unwrap(),
            vec![b'A'; 257 - 130]);
    }

    #[test]
    fn decode_no_eod_boundary_literal() {
        assert_eq!(decode(b"\x03TIFF").unwrap(), b"TIFF");
    }

    #[test]
    fn decode_stops_at_eod_ignores_trailing() {
        // Bytes after EOD are not part of the output.
        assert_eq!(decode(b"\x00A\x80\x00B").unwrap(), b"A");
    }

    #[test]
    fn roundtrip_empty() {
        assert_eq!(decode(&encode(b"")).unwrap(), b"");
    }

    #[test]
    fn roundtrip_high_entropy() {
        let input: Vec<u8> = (0..=255).collect();
        let encoded = encode(&input);
        assert_eq!(decode(&encoded).unwrap(), input);
    }

    #[test]
    fn roundtrip_alternating() {
        // Pattern designed to defeat run packing.
        let input = b"ABABABABABAB".repeat(50);
        let encoded = encode(&input);
        assert_eq!(decode(&encoded).unwrap(), input);
    }

    #[test]
    fn roundtrip_long_run() {
        let input = vec![0xAAu8; 1000];
        let encoded = encode(&input);
        assert_eq!(decode(&encoded).unwrap(), input);
    }

    #[test]
    fn roundtrip_mixed() {
        let mut input = Vec::new();
        input.extend(b"prefix-literal");
        input.extend(vec![b'X'; 50]);
        input.extend(b"middle-literal");
        input.extend(vec![b'Y'; 200]);
        input.extend(b"suffix");
        let encoded = encode(&input);
        assert_eq!(decode(&encoded).unwrap(), input);
    }
}
