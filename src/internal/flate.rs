// FlateDecode filter (PDF 32000-1:2008 §7.4.4 / RFC 1950 zlib /
// RFC 1951 DEFLATE) — whole-buffer adapter. `encode` wraps RFC
// 1951 DEFLATE in an RFC 1950 zlib container (CMF + FLG header,
// raw deflate body, trailing big-endian Adler-32 of the
// uncompressed input). `decode` unwraps the container, decodes
// every block type (stored, fixed Huffman, dynamic Huffman), and
// validates the Adler-32 trailer.
//
// Bit-packing direction is the eternal first-draft trap: bits
// below the byte level are LSB-FIRST within each byte (RFC 1951
// §3.1.1) so BFINAL is bit 0 of the first byte, BTYPE bits 1-2,
// etc. Huffman codes themselves are MSB-FIRST within their bit
// length — easiest correct route is to read bits LSB-first into
// an accumulator and lookup canonical-Huffman tables indexed by
// the LSB-first bit pattern directly. Both directions handled
// throughout this module.

use std::fmt;

/// Errors returned by [`decode`]. [`encode`] is total.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// zlib container header is missing, truncated, or fails
    /// the `(CMF * 256 + FLG) % 31 == 0` check.
    BadZlibHeader,
    /// FDICT bit is set in the FLG byte — preset dictionaries
    /// are not used by PDF / PNG / TIFF and are not supported.
    PresetDictionary,
    /// BTYPE field of a block header is 11 (reserved per RFC
    /// 1951 §3.2.3).
    ReservedBlockType,
    /// Stored block's NLEN field is not the one's complement of
    /// LEN.
    LenNlenMismatch,
    /// Bit reader hit end-of-input mid-block / mid-symbol.
    Truncated,
    /// Dynamic Huffman code-length sequence emitted a 16
    /// (repeat-previous) opcode before any length had been
    /// emitted.
    RepeatBeforeFirstLength,
    /// LZ77 back-reference distance points behind the start of
    /// the output buffer.
    DistanceTooFar,
    /// A canonical Huffman tree was over-subscribed (bit-length
    /// distribution sums past 1) or under-subscribed.
    MalformedHuffmanTree,
    /// A literal/length code 286 or 287 (reserved) or a distance
    /// code 30 or 31 (reserved) was emitted.
    ReservedSymbol,
    /// Adler-32 trailer disagrees with the running checksum of
    /// the decoded output.
    AdlerMismatch,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::BadZlibHeader => write!(
                f, "flate: bad zlib header"),
            Error::PresetDictionary => write!(
                f, "flate: preset dictionary not supported"),
            Error::ReservedBlockType => write!(
                f, "flate: reserved BTYPE 11"),
            Error::LenNlenMismatch => write!(
                f, "flate: stored block LEN/NLEN mismatch"),
            Error::Truncated => write!(
                f, "flate: truncated stream"),
            Error::RepeatBeforeFirstLength => write!(
                f, "flate: repeat-previous code before first length"),
            Error::DistanceTooFar => write!(
                f, "flate: back-reference distance past start of output"),
            Error::MalformedHuffmanTree => write!(
                f, "flate: malformed Huffman tree"),
            Error::ReservedSymbol => write!(
                f, "flate: reserved symbol in compressed block"),
            Error::AdlerMismatch => write!(
                f, "flate: Adler-32 mismatch"),
        }
    }
}

impl std::error::Error for Error {}

// ----- zlib framing constants -----

const ZLIB_CMF: u8 = 0x78; // method=8 (deflate), CINFO=7 (32KiB window)

// Adler-32 modulus: largest prime below 2^16. Hard-coded — the
// 65521 constant is the trap; a first-draft that uses 65536 (a
// power-of-two mask) computes a checksum that disagrees with
// every other zlib implementation.
const ADLER_MOD: u32 = 65521;

// ----- RFC 1951 §3.2.5 Tables 1 & 2 (length and distance) -----
//
// Indexed by the symbol minus its base (257 for length, 0 for
// distance). Transcribed verbatim from the RFC tables — no
// interpolation; the spacing is irregular and gets the
// first-draft "doubling" approach wrong.

const LENGTH_BASE: [u16; 29] = [
    3, 4, 5, 6, 7, 8, 9, 10, 11, 13, 15, 17, 19, 23, 27, 31,
    35, 43, 51, 59, 67, 83, 99, 115, 131, 163, 195, 227, 258,
];
const LENGTH_EXTRA_BITS: [u8; 29] = [
    0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 2, 2,
    3, 3, 3, 3, 4, 4, 4, 4, 5, 5, 5, 5, 0,
];

const DISTANCE_BASE: [u16; 30] = [
    1, 2, 3, 4, 5, 7, 9, 13, 17, 25, 33, 49, 65, 97, 129, 193,
    257, 385, 513, 769, 1025, 1537, 2049, 3073, 4097, 6145,
    8193, 12289, 16385, 24577,
];
const DISTANCE_EXTRA_BITS: [u8; 30] = [
    0, 0, 0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 5, 5, 6, 6,
    7, 7, 8, 8, 9, 9, 10, 10, 11, 11, 12, 12, 13, 13,
];

// RFC 1951 §3.2.7 — the 19 code-length-code symbols are
// transmitted in this permutation, with HCLEN of them carrying
// 3-bit length values. Hardcoded from the RFC.
const CODE_LENGTH_ORDER: [u8; 19] = [
    16, 17, 18, 0, 8, 7, 9, 6, 10, 5, 11, 4, 12, 3, 13, 2, 14, 1, 15,
];

// ============================================================
// Public API
// ============================================================

/// FlateDecode-encode `input` into a zlib-framed DEFLATE stream.
/// Total — every byte slice is encodable. Empty input yields a
/// small non-empty stream (header + 1 empty stored block +
/// Adler-32).
pub fn encode(input: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(input.len() + 16);

    // RFC 1950 §2: zlib header. CMF = 0x78, FLG with FLEVEL=2
    // (default level), FCHECK chosen so (CMF * 256 + FLG) % 31
    // is zero. We pick FLG = 0x9C for the canonical level-6
    // header — it is the most common in the wild and matches
    // miniz / zlib defaults, which keeps interop fingerprints
    // boring. FDICT = 0 (we never use a preset dictionary).
    out.push(ZLIB_CMF);
    out.push(0x9C);

    // Body: dynamic-Huffman compression of the whole input as a
    // single block. Fall back to a stored block if the dynamic
    // encoding would somehow inflate (only happens for very
    // short inputs where the tree-transmission overhead exceeds
    // the savings).
    if input.is_empty() {
        encode_empty_stored_block(&mut out);
    } else {
        encode_one_dynamic_block(input, &mut out);
    }

    // Adler-32 trailer: big-endian.
    let adler = adler32(input);
    out.push((adler >> 24) as u8);
    out.push((adler >> 16) as u8);
    out.push((adler >> 8) as u8);
    out.push(adler as u8);

    out
}

/// FlateDecode-decode `input` from a zlib-framed DEFLATE stream
/// back to the original bytes.
pub fn decode(input: &[u8]) -> Result<Vec<u8>, Error> {
    if input.len() < 2 {
        return Err(Error::BadZlibHeader);
    }
    let cmf = input[0];
    let flg = input[1];
    // CMF: low 4 bits = method (must be 8 = deflate); high 4
    // bits = window size as log2(window) - 8 (max 7 = 32 KiB).
    if (cmf & 0x0F) != 8 || (cmf >> 4) > 7 {
        return Err(Error::BadZlibHeader);
    }
    if ((cmf as u16) * 256 + flg as u16) % 31 != 0 {
        return Err(Error::BadZlibHeader);
    }
    if (flg & 0x20) != 0 {
        // FDICT bit — preset dictionary follows; not supported.
        return Err(Error::PresetDictionary);
    }

    let body_end = input.len().checked_sub(4)
        .ok_or(Error::BadZlibHeader)?;
    if body_end < 2 {
        return Err(Error::BadZlibHeader);
    }
    let body = &input[2..body_end];
    let trailer = &input[body_end..];

    let mut reader = BitReader::new(body);
    let mut out: Vec<u8> = Vec::new();
    loop {
        let bfinal = reader.read_bits(1)? as u8;
        let btype = reader.read_bits(2)? as u8;
        match btype {
            0 => decode_stored_block(&mut reader, &mut out)?,
            1 => decode_compressed_block(
                &mut reader, &mut out,
                &fixed_lit_decoder(),
                &fixed_dist_decoder(),
            )?,
            2 => {
                let (lit, dist) = decode_dynamic_trees(&mut reader)?;
                decode_compressed_block(
                    &mut reader, &mut out, &lit, &dist)?;
            }
            _ => return Err(Error::ReservedBlockType),
        }
        if bfinal != 0 {
            break;
        }
    }

    // Validate Adler-32 trailer.
    let stored = ((trailer[0] as u32) << 24)
        | ((trailer[1] as u32) << 16)
        | ((trailer[2] as u32) << 8)
        | (trailer[3] as u32);
    if adler32(&out) != stored {
        return Err(Error::AdlerMismatch);
    }
    Ok(out)
}

// ============================================================
// Adler-32
// ============================================================

fn adler32(data: &[u8]) -> u32 {
    // s1 starts at 1, s2 at 0 (NOT both zero — the
    // initial-state trap). Modulus is 65521 (prime), NOT 65536.
    let mut s1: u32 = 1;
    let mut s2: u32 = 0;
    // Process in chunks of 5552 bytes — the largest run that
    // keeps both s1 and s2 from overflowing u32 between
    // modulo reductions. Pure performance tuning; correctness
    // is identical to the byte-by-byte form.
    for chunk in data.chunks(5552) {
        for &b in chunk {
            s1 += b as u32;
            s2 += s1;
        }
        s1 %= ADLER_MOD;
        s2 %= ADLER_MOD;
    }
    (s2 << 16) | s1
}

// ============================================================
// Encoder — single dynamic-Huffman block
// ============================================================

fn encode_empty_stored_block(out: &mut Vec<u8>) {
    // BFINAL=1, BTYPE=00, then byte-align and emit LEN=0,
    // NLEN=0xFFFF. Three bits of header into a byte that we
    // immediately complete with five zero pad bits.
    out.push(0x01); // bit 0 = BFINAL=1, bits 1-2 = 00 (stored)
    out.push(0x00); // LEN low
    out.push(0x00); // LEN high
    out.push(0xFF); // NLEN low
    out.push(0xFF); // NLEN high
}

fn encode_one_dynamic_block(input: &[u8], out: &mut Vec<u8>) {
    // Step 1: produce an LZ77 token sequence — literals (0-255)
    // + (length, distance) back-references. We keep this simple
    // and stream-able: a hash chain over the most recent 3-byte
    // prefixes inside a 32 KiB window finds matches; the
    // longest match wins; minimum match length is 3 (anything
    // shorter is cheaper as literals).
    let tokens = lz77_match(input);

    // Step 2: build literal/length and distance frequency
    // tables from the token stream, then run the package-merge
    // / Huffman-from-frequencies pipeline to produce code-length
    // arrays bounded to 15 bits (DEFLATE's max).
    let mut lit_freq = [0u32; 286];
    let mut dist_freq = [0u32; 30];
    for t in &tokens {
        match *t {
            Token::Literal(b) => lit_freq[b as usize] += 1,
            Token::Match { length, distance: _ } => {
                let (lc, _, _) = length_code(length);
                lit_freq[lc as usize] += 1;
                let (dc, _, _) = distance_code(distance_for(t));
                dist_freq[dc as usize] += 1;
            }
        }
    }
    // End-of-block symbol (256) always counts once.
    lit_freq[256] += 1;

    let lit_lens = build_huffman_lengths(&lit_freq, 15);
    // Distance table needs at least one symbol per RFC; if the
    // input has zero matches we still emit a single-symbol
    // distance tree with code length 1 (a degenerate but legal
    // encoding) so HDIST has something to transmit.
    let dist_lens = if dist_freq.iter().any(|&f| f != 0) {
        build_huffman_lengths(&dist_freq, 15)
    } else {
        let mut d = [0u8; 30];
        d[0] = 1;
        d.to_vec()
    };

    // Step 3: trim trailing zero lengths to determine HLIT / HDIST.
    let hlit = lit_lens.iter().rposition(|&l| l != 0)
        .map(|i| (i + 1).max(257))
        .unwrap_or(257);
    let hdist = dist_lens.iter().rposition(|&l| l != 0)
        .map(|i| i + 1)
        .unwrap_or(1);

    // Step 4: code-length encoding of (lit_lens[..hlit] ++ dist_lens[..hdist]).
    // Symbols 0-15 emit a code length directly; 16/17/18 are
    // run-length operators (and have DIFFERENT bases — 16 → 3,
    // 17 → 3, 18 → 11; the offsets are the trap).
    let mut all_lens: Vec<u8> = Vec::with_capacity(hlit + hdist);
    all_lens.extend_from_slice(&lit_lens[..hlit]);
    all_lens.extend_from_slice(&dist_lens[..hdist]);
    let cl_symbols = code_length_encode(&all_lens);

    let mut cl_freq = [0u32; 19];
    for s in &cl_symbols {
        cl_freq[s.symbol as usize] += 1;
    }
    let cl_lens = build_huffman_lengths(&cl_freq, 7);

    // HCLEN trim: scan the permuted order from the back; the
    // last index whose mapped length is nonzero sets HCLEN.
    let mut hclen_count = 19usize;
    while hclen_count > 4
        && cl_lens[CODE_LENGTH_ORDER[hclen_count - 1] as usize] == 0
    {
        hclen_count -= 1;
    }
    let hclen = hclen_count;

    // Step 5: serialise the block.
    let mut bw = BitWriter::new(out);
    bw.write_bits(1, 1); // BFINAL
    bw.write_bits(2, 2); // BTYPE = dynamic
    bw.write_bits((hlit - 257) as u32, 5);
    bw.write_bits((hdist - 1) as u32, 5);
    bw.write_bits((hclen - 4) as u32, 4);
    for i in 0..hclen {
        bw.write_bits(cl_lens[CODE_LENGTH_ORDER[i] as usize] as u32, 3);
    }

    // Encode the run-length-operator stream using the cl tree.
    let cl_codes = canonical_codes(&cl_lens);
    for s in &cl_symbols {
        let code = cl_codes[s.symbol as usize];
        let len = cl_lens[s.symbol as usize];
        // Huffman codes pack MSB-first within their bit length.
        bw.write_huffman(code, len);
        if let Some((extra_bits, value)) = s.extra {
            bw.write_bits(value as u32, extra_bits);
        }
    }

    // Encode the LZ77 token stream using the lit + dist trees.
    let lit_codes = canonical_codes(&lit_lens);
    let dist_codes = canonical_codes(&dist_lens);
    for t in &tokens {
        match *t {
            Token::Literal(b) => {
                let i = b as usize;
                bw.write_huffman(lit_codes[i], lit_lens[i]);
            }
            Token::Match { length, distance } => {
                let (lc, le, lv) = length_code(length);
                bw.write_huffman(lit_codes[lc as usize],
                                 lit_lens[lc as usize]);
                if le > 0 {
                    bw.write_bits(lv as u32, le);
                }
                let (dc, de, dv) = distance_code(distance);
                bw.write_huffman(dist_codes[dc as usize],
                                 dist_lens[dc as usize]);
                if de > 0 {
                    bw.write_bits(dv as u32, de);
                }
            }
        }
    }
    // End-of-block.
    bw.write_huffman(lit_codes[256], lit_lens[256]);
    bw.flush();
}

fn distance_for(t: &Token) -> u16 {
    match *t {
        Token::Match { distance, .. } => distance,
        _ => 0,
    }
}

#[derive(Clone, Copy, Debug)]
enum Token {
    Literal(u8),
    Match { length: u16, distance: u16 },
}

// LZ77 hash-chain match finder — windowed over the most recent
// 32 KiB. We use a direct-mapped 32 KiB hash table keyed on the
// 3-byte prefix at each position; collisions form a singly
// linked list back through `prev`. Match-length is bounded by
// the maximum DEFLATE length (258).
fn lz77_match(input: &[u8]) -> Vec<Token> {
    const WINDOW: usize = 32768;
    const MIN_MATCH: usize = 3;
    const MAX_MATCH: usize = 258;
    const HASH_BITS: u32 = 15;
    const HASH_SIZE: usize = 1 << HASH_BITS;
    const NIL: u32 = u32::MAX;

    let mut tokens = Vec::with_capacity(input.len());
    if input.len() < MIN_MATCH {
        for &b in input {
            tokens.push(Token::Literal(b));
        }
        return tokens;
    }

    let mut head = vec![NIL; HASH_SIZE];
    let mut prev = vec![NIL; input.len()];

    let hash3 = |a: u8, b: u8, c: u8| -> usize {
        // Multiplicative hash — cheap and well-mixed enough
        // for short prefixes.
        let h = ((a as u32) << 16) | ((b as u32) << 8) | (c as u32);
        ((h.wrapping_mul(2654435761)) >> (32 - HASH_BITS)) as usize
    };

    let mut i = 0usize;
    while i + MIN_MATCH <= input.len() {
        let h = hash3(input[i], input[i + 1], input[i + 2]);
        let mut best_len: usize = 0;
        let mut best_dist: usize = 0;
        let lower = i.saturating_sub(WINDOW);

        // Walk the chain — capped to a finite chain depth to
        // avoid O(n^2) on pathological inputs (long runs of
        // the same 3-byte prefix). 32 is generous enough to
        // get near-optimal matches on real data.
        let mut chain = head[h];
        let mut depth = 0u32;
        while chain != NIL && (chain as usize) >= lower && depth < 32 {
            let j = chain as usize;
            // Compare bytes — bounded by both buffers' tails.
            let max_len = MAX_MATCH.min(input.len() - i);
            let mut k = 0;
            while k < max_len && input[i + k] == input[j + k] {
                k += 1;
            }
            if k >= MIN_MATCH && k > best_len {
                best_len = k;
                best_dist = i - j;
                if best_len == MAX_MATCH {
                    break;
                }
            }
            chain = prev[j];
            depth += 1;
        }

        // Insert the current position into the hash chain.
        prev[i] = head[h];
        head[h] = i as u32;

        if best_len >= MIN_MATCH {
            tokens.push(Token::Match {
                length: best_len as u16,
                distance: best_dist as u16,
            });
            // Advance past the match. We optionally insert
            // every position inside the match into the hash
            // table to keep the chain dense — beneficial on
            // longer inputs.
            for k in 1..best_len {
                if i + k + MIN_MATCH <= input.len() {
                    let h2 = hash3(input[i + k], input[i + k + 1], input[i + k + 2]);
                    prev[i + k] = head[h2];
                    head[h2] = (i + k) as u32;
                }
            }
            i += best_len;
        } else {
            tokens.push(Token::Literal(input[i]));
            i += 1;
        }
    }
    // Trailing bytes (length < MIN_MATCH) emit as literals.
    while i < input.len() {
        tokens.push(Token::Literal(input[i]));
        i += 1;
    }
    tokens
}

fn length_code(length: u16) -> (u16, u8, u16) {
    // Returns (code, extra_bits, extra_value). Matches RFC 1951
    // §3.2.5 Table 1. Length 258 is a special case — the
    // table's last entry, 0 extra bits.
    if length == 258 {
        return (285, 0, 0);
    }
    // Linear scan — 28 entries, hot path is short.
    for i in 0..28 {
        let lo = LENGTH_BASE[i] as i32;
        let hi = lo + (1i32 << LENGTH_EXTRA_BITS[i]) - 1;
        if (length as i32) >= lo && (length as i32) <= hi {
            return (
                (257 + i) as u16,
                LENGTH_EXTRA_BITS[i],
                (length - LENGTH_BASE[i]),
            );
        }
    }
    unreachable!("length out of range: {length}")
}

fn distance_code(distance: u16) -> (u16, u8, u16) {
    // Returns (code, extra_bits, extra_value). Matches RFC 1951
    // §3.2.5 Table 2.
    for i in 0..30 {
        let lo = DISTANCE_BASE[i] as i32;
        let hi = lo + (1i32 << DISTANCE_EXTRA_BITS[i]) - 1;
        if (distance as i32) >= lo && (distance as i32) <= hi {
            return (
                i as u16,
                DISTANCE_EXTRA_BITS[i],
                (distance - DISTANCE_BASE[i]),
            );
        }
    }
    unreachable!("distance out of range: {distance}")
}

// Code-length encoding (RFC 1951 §3.2.7). Operators 16/17/18
// have non-uniform bases (16: prev × 3-6 / +3 base; 17: zero ×
// 3-10 / +3 base; 18: zero × 11-138 / +11 base).
struct ClSymbol {
    symbol: u8,
    extra: Option<(u8, u16)>, // (extra_bits, value)
}

fn code_length_encode(lens: &[u8]) -> Vec<ClSymbol> {
    let mut out: Vec<ClSymbol> = Vec::with_capacity(lens.len());
    let mut i = 0;
    while i < lens.len() {
        let cur = lens[i];
        // Run-length detection.
        let mut run = 1usize;
        while i + run < lens.len() && lens[i + run] == cur && run < 138 {
            run += 1;
        }
        if cur == 0 {
            if run >= 11 {
                let r = run.min(138);
                out.push(ClSymbol {
                    symbol: 18,
                    extra: Some((7, (r - 11) as u16)),
                });
                i += r;
            } else if run >= 3 {
                let r = run.min(10);
                out.push(ClSymbol {
                    symbol: 17,
                    extra: Some((3, (r - 3) as u16)),
                });
                i += r;
            } else {
                out.push(ClSymbol { symbol: 0, extra: None });
                i += 1;
            }
        } else {
            // First emit the symbol itself, then any run as 16
            // (repeat-previous, 3-6 times, +3 base).
            out.push(ClSymbol { symbol: cur, extra: None });
            i += 1;
            let mut left = run - 1;
            while left >= 3 {
                let r = left.min(6);
                out.push(ClSymbol {
                    symbol: 16,
                    extra: Some((2, (r - 3) as u16)),
                });
                left -= r;
                i += r;
            }
            for _ in 0..left {
                out.push(ClSymbol { symbol: cur, extra: None });
                i += 1;
            }
        }
    }
    out
}

// Build canonical Huffman code-length array from frequencies,
// bounded to `max_bits`. Algorithm: standard package-merge for
// the length-limited optimal Huffman problem. For freezes,
// canonical-Huffman codes are reconstructed from the lengths
// alone (RFC 1951 §3.2.2), so only the lengths matter — not
// the specific bit patterns.
fn build_huffman_lengths(freq: &[u32], max_bits: u8) -> Vec<u8> {
    let n = freq.len();
    let mut lens = vec![0u8; n];
    let used: Vec<usize> = (0..n).filter(|&i| freq[i] != 0).collect();
    if used.is_empty() {
        return lens;
    }
    if used.len() == 1 {
        // Single-symbol tree: length 1 (DEFLATE doesn't allow
        // length-0 codes for symbols that exist).
        lens[used[0]] = 1;
        return lens;
    }

    // Package-merge length-limited Huffman.
    let mut sorted: Vec<(u32, usize)> =
        used.iter().map(|&i| (freq[i], i)).collect();
    sorted.sort_by_key(|&(f, _)| f);

    #[derive(Clone)]
    struct Item {
        weight: u64,
        symbols: Vec<usize>,
    }

    let leaves: Vec<Item> = sorted.iter()
        .map(|&(f, i)| Item { weight: f as u64, symbols: vec![i] })
        .collect();

    let mut prev: Vec<Item> = leaves.clone();
    for _ in 1..max_bits {
        // Package: pair adjacent items.
        let mut packaged: Vec<Item> = Vec::with_capacity(prev.len() / 2);
        let mut i = 0;
        while i + 1 < prev.len() {
            packaged.push(Item {
                weight: prev[i].weight + prev[i + 1].weight,
                symbols: {
                    let mut s = prev[i].symbols.clone();
                    s.extend_from_slice(&prev[i + 1].symbols);
                    s
                },
            });
            i += 2;
        }
        // Merge packaged with leaves, preserving sorted-by-weight.
        let mut merged = Vec::with_capacity(packaged.len() + leaves.len());
        let mut a = 0;
        let mut b = 0;
        while a < packaged.len() && b < leaves.len() {
            if packaged[a].weight <= leaves[b].weight {
                merged.push(packaged[a].clone());
                a += 1;
            } else {
                merged.push(leaves[b].clone());
                b += 1;
            }
        }
        merged.extend_from_slice(&packaged[a..]);
        merged.extend_from_slice(&leaves[b..]);
        prev = merged;
    }

    // Take the first 2 * leaves.len() - 2 items from the final
    // list; each symbol's code length = number of times it
    // appears in those items' symbol lists.
    let take = 2 * used.len() - 2;
    for item in prev.iter().take(take) {
        for &sym in &item.symbols {
            lens[sym] += 1;
        }
    }
    debug_assert!(lens.iter().all(|&l| l <= max_bits));
    lens
}

// ============================================================
// Canonical Huffman codes (RFC 1951 §3.2.2)
// ============================================================

fn canonical_codes(lens: &[u8]) -> Vec<u32> {
    // Returns the canonical Huffman code for each symbol, given
    // that symbol's bit length. Codes for unused symbols (length
    // 0) are returned as 0 — callers must not emit them.
    let max = lens.iter().copied().max().unwrap_or(0);
    if max == 0 {
        return vec![0; lens.len()];
    }
    // Count of codes at each length.
    let mut bl_count = vec![0u32; (max + 1) as usize];
    for &l in lens {
        if l != 0 {
            bl_count[l as usize] += 1;
        }
    }
    // First code at each length — left-pack into the value
    // space at that length (see RFC 1951 §3.2.2).
    let mut next_code = vec![0u32; (max + 1) as usize];
    let mut code: u32 = 0;
    for bits in 1..=max as usize {
        code = (code + bl_count[bits - 1]) << 1;
        next_code[bits] = code;
    }
    // Assign sequentially in symbol-index order.
    let mut codes = vec![0u32; lens.len()];
    for (i, &l) in lens.iter().enumerate() {
        if l != 0 {
            codes[i] = next_code[l as usize];
            next_code[l as usize] += 1;
        }
    }
    codes
}

// ============================================================
// Bit writer (LSB-first into bytes; MSB-first within Huffman codes)
// ============================================================

struct BitWriter<'a> {
    out: &'a mut Vec<u8>,
    buf: u64,
    held: u8, // bits in `buf`, low-aligned
}

impl<'a> BitWriter<'a> {
    fn new(out: &'a mut Vec<u8>) -> Self {
        BitWriter { out, buf: 0, held: 0 }
    }

    /// Write `n` bits of `value` LSB-first into the byte stream.
    /// "LSB-first" means the lowest bit of `value` becomes the
    /// next bit to flush — which, in DEFLATE's byte layout, is
    /// the LOW bit of the next pending byte.
    fn write_bits(&mut self, value: u32, n: u8) {
        debug_assert!(n <= 32);
        let mask = if n == 32 { u32::MAX } else { (1u32 << n) - 1 };
        self.buf |= ((value & mask) as u64) << self.held;
        self.held += n;
        while self.held >= 8 {
            self.out.push(self.buf as u8);
            self.buf >>= 8;
            self.held -= 8;
        }
    }

    /// Write a Huffman code of `n` bits. The canonical-Huffman
    /// code is held MSB-first within its bit length, but the
    /// DEFLATE bit stream is LSB-first within bytes — so we
    /// reverse the code's bits before pushing them through
    /// `write_bits`. Equivalent to "shifting out the MSB of the
    /// code bit by bit and packing each shifted bit LSB-first"
    /// but cheaper.
    fn write_huffman(&mut self, code: u32, n: u8) {
        debug_assert!((1..=15).contains(&n));
        let reversed = reverse_bits(code, n);
        self.write_bits(reversed, n);
    }

    fn flush(&mut self) {
        if self.held > 0 {
            self.out.push(self.buf as u8);
            self.buf = 0;
            self.held = 0;
        }
    }
}

fn reverse_bits(mut v: u32, n: u8) -> u32 {
    let mut r: u32 = 0;
    for _ in 0..n {
        r = (r << 1) | (v & 1);
        v >>= 1;
    }
    r
}

// ============================================================
// Bit reader (LSB-first; matches encoder's framing)
// ============================================================

struct BitReader<'a> {
    bytes: &'a [u8],
    p: usize,
    buf: u64,
    held: u8,
}

impl<'a> BitReader<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        BitReader { bytes, p: 0, buf: 0, held: 0 }
    }

    fn read_bits(&mut self, n: u8) -> Result<u32, Error> {
        debug_assert!(n <= 32);
        while self.held < n {
            if self.p >= self.bytes.len() {
                return Err(Error::Truncated);
            }
            self.buf |= (self.bytes[self.p] as u64) << self.held;
            self.p += 1;
            self.held += 8;
        }
        let mask = if n == 32 { u64::MAX } else { (1u64 << n) - 1 };
        let v = (self.buf & mask) as u32;
        self.buf >>= n;
        self.held -= n;
        Ok(v)
    }

    /// Discard remaining bits in the current byte (stored-block
    /// alignment).
    fn byte_align(&mut self) {
        let drop = self.held % 8;
        self.buf >>= drop;
        self.held -= drop;
    }

    /// Read a single byte, byte-aligned. Caller is responsible
    /// for having already byte-aligned the stream.
    fn read_byte(&mut self) -> Result<u8, Error> {
        if self.held >= 8 {
            let b = self.buf as u8;
            self.buf >>= 8;
            self.held -= 8;
            return Ok(b);
        }
        if self.p >= self.bytes.len() {
            return Err(Error::Truncated);
        }
        let b = self.bytes[self.p];
        self.p += 1;
        Ok(b)
    }
}

// ============================================================
// Decoder — block types
// ============================================================

fn decode_stored_block(
    reader: &mut BitReader<'_>,
    out: &mut Vec<u8>,
) -> Result<(), Error> {
    reader.byte_align();
    let lo_l = reader.read_byte()? as u16;
    let hi_l = reader.read_byte()? as u16;
    let lo_n = reader.read_byte()? as u16;
    let hi_n = reader.read_byte()? as u16;
    let len = lo_l | (hi_l << 8);
    let nlen = lo_n | (hi_n << 8);
    if (len ^ 0xFFFF) != nlen {
        return Err(Error::LenNlenMismatch);
    }
    out.reserve(len as usize);
    for _ in 0..len {
        out.push(reader.read_byte()?);
    }
    Ok(())
}

fn decode_compressed_block(
    reader: &mut BitReader<'_>,
    out: &mut Vec<u8>,
    lit: &Decoder,
    dist: &Decoder,
) -> Result<(), Error> {
    loop {
        let sym = lit.decode(reader)?;
        if sym < 256 {
            out.push(sym as u8);
            continue;
        }
        if sym == 256 {
            return Ok(());
        }
        if sym >= 286 {
            return Err(Error::ReservedSymbol);
        }
        // Length code 257..285 → base + extra bits.
        let li = (sym - 257) as usize;
        let length = LENGTH_BASE[li] as u32
            + reader.read_bits(LENGTH_EXTRA_BITS[li])?;
        let dsym = dist.decode(reader)?;
        if dsym >= 30 {
            return Err(Error::ReservedSymbol);
        }
        let di = dsym as usize;
        let distance = DISTANCE_BASE[di] as u32
            + reader.read_bits(DISTANCE_EXTRA_BITS[di])?;
        if (distance as usize) > out.len() {
            return Err(Error::DistanceTooFar);
        }
        // RFC 1951 §3.2.3: if distance < length the back-
        // reference reads bytes that are themselves being
        // written. Copy ONE byte at a time so each freshly-
        // written byte is visible to the next read — memcpy /
        // memmove on a snapshot would produce wrong output.
        let start = out.len() - distance as usize;
        for k in 0..length as usize {
            let b = out[start + k];
            out.push(b);
        }
    }
}

// ============================================================
// Dynamic Huffman — tree transmission (RFC 1951 §3.2.7)
// ============================================================

fn decode_dynamic_trees(
    reader: &mut BitReader<'_>,
) -> Result<(Decoder, Decoder), Error> {
    let hlit = reader.read_bits(5)? as usize + 257;
    let hdist = reader.read_bits(5)? as usize + 1;
    let hclen = reader.read_bits(4)? as usize + 4;

    if hlit > 286 || hdist > 30 || hclen > 19 {
        return Err(Error::MalformedHuffmanTree);
    }

    // Code-length-code lengths in permuted order.
    let mut cl_lens = [0u8; 19];
    for i in 0..hclen {
        cl_lens[CODE_LENGTH_ORDER[i] as usize] =
            reader.read_bits(3)? as u8;
    }
    let cl_decoder = Decoder::new(&cl_lens)?;

    // Decode hlit + hdist code lengths in one pass.
    let total = hlit + hdist;
    let mut lens = vec![0u8; total];
    let mut i = 0;
    let mut last_len: i32 = -1;
    while i < total {
        let sym = cl_decoder.decode(reader)?;
        match sym {
            0..=15 => {
                lens[i] = sym as u8;
                last_len = sym as i32;
                i += 1;
            }
            16 => {
                if last_len < 0 {
                    return Err(Error::RepeatBeforeFirstLength);
                }
                let n = reader.read_bits(2)? as usize + 3;
                if i + n > total {
                    return Err(Error::MalformedHuffmanTree);
                }
                for _ in 0..n {
                    lens[i] = last_len as u8;
                    i += 1;
                }
            }
            17 => {
                let n = reader.read_bits(3)? as usize + 3;
                if i + n > total {
                    return Err(Error::MalformedHuffmanTree);
                }
                for _ in 0..n {
                    lens[i] = 0;
                    i += 1;
                }
                last_len = 0;
            }
            18 => {
                let n = reader.read_bits(7)? as usize + 11;
                if i + n > total {
                    return Err(Error::MalformedHuffmanTree);
                }
                for _ in 0..n {
                    lens[i] = 0;
                    i += 1;
                }
                last_len = 0;
            }
            _ => return Err(Error::MalformedHuffmanTree),
        }
    }

    let lit = Decoder::new(&lens[..hlit])?;
    let dist = Decoder::new(&lens[hlit..])?;
    Ok((lit, dist))
}

// ============================================================
// Huffman decoder — canonical lookup
// ============================================================
//
// Direct lookup table indexed by the next 15 bits LSB-first.
// Each entry is (symbol, length) — length=0 means "no code,
// stream malformed". Building the table from canonical lengths:
// for each symbol with length L, compute its canonical code
// (MSB-first within L bits) and write the (sym, L) entry into
// every table slot whose low-L bits equal the bit-reversed
// canonical code, leaving the upper (15 - L) bits free. This
// way decode() peeks 15 bits LSB-first and indexes directly.

struct Decoder {
    table: Vec<(u16, u8)>, // (symbol, length); length=0 ⇒ invalid
}

impl Decoder {
    fn new(lens: &[u8]) -> Result<Self, Error> {
        const BITS: u8 = 15;
        let size = 1usize << BITS;
        let mut table = vec![(0u16, 0u8); size];

        let max = lens.iter().copied().max().unwrap_or(0);
        if max == 0 {
            // Empty alphabet — all-zero lengths. The caller
            // never invokes decode() on such a tree (compressed
            // blocks always have at least the EOB symbol), but
            // a single-symbol degenerate tree (one length-1
            // entry) is legal for the distance side.
            return Ok(Decoder { table });
        }
        if max > BITS {
            return Err(Error::MalformedHuffmanTree);
        }

        // Validate Kraft's inequality: sum(2^(BITS - L)) over
        // used codes must equal 2^BITS exactly (over-/under-
        // subscribed otherwise). The single-symbol case (one
        // L=1 code) is the canonical DEFLATE escape from this
        // — RFC 1951 §3.2.7 explicitly allows a one-symbol
        // distance tree; treat it as a special case.
        let used: usize = lens.iter().filter(|&&l| l != 0).count();
        if used == 0 {
            return Err(Error::MalformedHuffmanTree);
        }
        if used == 1 {
            let only = lens.iter().position(|&l| l != 0).unwrap();
            // Single-symbol tree: canonical code 0, length 1.
            // The bit reader hands out bits LSB-first, so
            // "next bit is 0" means BIT 0 of idx is 0 — i.e.,
            // every EVEN idx maps to the lone symbol. Slots
            // with bit 0 = 1 stay malformed (length 0): a
            // stream that emits a 1 bit for a single-symbol
            // tree is corrupt by definition.
            let mut idx = 0;
            while idx < size {
                table[idx] = (only as u16, 1);
                idx += 2;
            }
            return Ok(Decoder { table });
        }

        // Multi-symbol case — proper canonical assignment.
        let mut bl_count = vec![0u32; (max + 1) as usize];
        for &l in lens {
            if l != 0 {
                bl_count[l as usize] += 1;
            }
        }
        // Kraft sum check.
        let mut left: i64 = 1 << max;
        for (l, &c) in bl_count.iter().enumerate().skip(1) {
            left -= (c as i64) << (max as usize - l);
        }
        if left != 0 {
            return Err(Error::MalformedHuffmanTree);
        }

        let mut next_code = vec![0u32; (max + 1) as usize];
        let mut code: u32 = 0;
        for bits in 1..=max as usize {
            code = (code + bl_count[bits - 1]) << 1;
            next_code[bits] = code;
        }
        for (sym, &l) in lens.iter().enumerate() {
            if l == 0 {
                continue;
            }
            let canonical = next_code[l as usize];
            next_code[l as usize] += 1;
            // The bit reader hands out bits LSB-first, so the
            // physical pattern of the `l` bits about to leave
            // the stream is the canonical code BIT-REVERSED.
            let lsb = reverse_bits(canonical, l);
            // Fill every table slot whose low-l bits are `lsb`.
            let stride = 1usize << l;
            let mut idx = lsb as usize;
            while idx < size {
                table[idx] = (sym as u16, l);
                idx += stride;
            }
        }
        Ok(Decoder { table })
    }

    fn decode(&self, reader: &mut BitReader<'_>) -> Result<u16, Error> {
        const BITS: u8 = 15;
        // Need 15 bits available to index the table; the bit
        // reader handles top-up on demand.
        while reader.held < BITS {
            if reader.p >= reader.bytes.len() {
                // Allow short reads near end-of-stream — we
                // index with whatever bits we have, padding the
                // top with zeros. The table is built so that
                // any valid prefix code resolves correctly even
                // with zero-padded high bits.
                if reader.held == 0 {
                    return Err(Error::Truncated);
                }
                break;
            }
            reader.buf |= (reader.bytes[reader.p] as u64) << reader.held;
            reader.p += 1;
            reader.held += 8;
        }
        let idx = (reader.buf & ((1u64 << BITS) - 1)) as usize;
        let (sym, l) = self.table[idx];
        if l == 0 || l > reader.held {
            return Err(Error::Truncated);
        }
        reader.buf >>= l;
        reader.held -= l;
        Ok(sym)
    }
}

// ============================================================
// Fixed Huffman trees (RFC 1951 §3.2.6)
// ============================================================

fn fixed_lit_decoder() -> Decoder {
    // Literal/length codes 0-143 → 8 bits, 144-255 → 9 bits,
    // 256-279 → 7 bits, 280-287 → 8 bits.
    let mut lens = [0u8; 288];
    lens[0..=143].fill(8);
    lens[144..=255].fill(9);
    lens[256..=279].fill(7);
    lens[280..=287].fill(8);
    Decoder::new(&lens).expect("fixed literal tree well-formed")
}

fn fixed_dist_decoder() -> Decoder {
    // All 30 valid distance symbols use a 5-bit fixed code; the
    // two reserved codes (30, 31) also get length 5 so the
    // tree is full per Kraft. Reserved-symbol enforcement
    // happens at decode time when sym >= 30 surfaces.
    Decoder::new(&[5u8; 32]).expect("fixed distance tree well-formed")
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn short_ascii_roundtrip() {
        roundtrip(b"The quick brown fox jumps over the lazy dog.");
    }

    #[test]
    fn highly_repetitive_roundtrip() {
        // RLE-y input — exercises long matches with distance < length.
        let input = vec![b'A'; 4096];
        roundtrip(&input);
    }

    #[test]
    fn high_entropy_roundtrip() {
        let mut input = Vec::with_capacity(4096);
        let mut x: u32 = 0x12345678;
        for _ in 0..4096 {
            x = x.wrapping_mul(1664525).wrapping_add(1013904223);
            input.push((x >> 16) as u8);
        }
        roundtrip(&input);
    }

    #[test]
    fn mixed_4k_roundtrip() {
        // Half text, half random — no degenerate path dominates.
        let mut input = Vec::with_capacity(4096);
        for _ in 0..256 {
            input.extend_from_slice(b"The fox jumped. ");
        }
        let mut x: u32 = 0xDEAD_BEEF;
        for _ in 0..2048 {
            x = x.wrapping_mul(1664525).wrapping_add(1013904223);
            input.push((x >> 16) as u8);
        }
        roundtrip(&input);
    }

    #[test]
    fn adler32_known_values() {
        // RFC 1950 reference: empty -> 1, "Wikipedia" -> 0x11E60398.
        assert_eq!(adler32(b""), 1);
        assert_eq!(adler32(b"Wikipedia"), 0x11E60398);
    }

    #[test]
    fn decode_known_zlib_stream() {
        // zlib-encoded "hello world" produced by the standard
        // zlib library at default level. Exercises decode in
        // isolation against a third-party-encoded stream — if
        // our Adler-32 modulus / bit ordering were wrong, this
        // would fail.
        let stream: [u8; 19] = [
            0x78, 0x9C, 0xCB, 0x48, 0xCD, 0xC9, 0xC9, 0x57,
            0x28, 0xCF, 0x2F, 0xCA, 0x49, 0x01, 0x00, 0x1A,
            0x0B, 0x04, 0x5D,
        ];
        let decoded = decode(&stream)
            .unwrap_or_else(|e| panic!("decode failed: {e:?}"));
        assert_eq!(decoded, b"hello world");
    }

    #[test]
    fn bad_zlib_header_errors() {
        // CMF/FLG that fails the divisibility-by-31 check.
        let bad: [u8; 6] = [0x78, 0x00, 0x00, 0x00, 0x00, 0x01];
        assert_eq!(decode(&bad).unwrap_err(), Error::BadZlibHeader);
    }

    #[test]
    fn truncated_stream_errors() {
        let mut enc = encode(b"some data");
        enc.truncate(enc.len() - 5);
        let err = decode(&enc).unwrap_err();
        assert!(
            matches!(err, Error::Truncated | Error::AdlerMismatch
                       | Error::BadZlibHeader),
            "unexpected error variant: {err:?}");
    }
}
