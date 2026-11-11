// Cryptographic message digests — MD5 (RFC 1321) + SHA-1 /
// SHA-256 / SHA-384 / SHA-512 (FIPS 180-4). PDF needs all five:
// MD5 for V=1/V=2/V=4 password handlers (PDF 32000-1 §7.6.3.3
// Algorithm 2), the SHA-2 family for V=5 password handlers
// (PDF 32000-1 §7.6.3.4 Algorithms 8-13), SHA-1 for digital
// signature digests in older signed documents.
//
// Five infallible one-shot entry points plus a gate-visible
// Bundle / parse / to_canonical triple that the
// two_oracle_agreement driver template feeds:
//
//   parse(input)    -> Bundle (all five digests of `input`)
//   to_canonical(b) -> 400-byte text manifest
//
// Padding-tail width matters: 64-bit for MD5 / SHA-1 / SHA-256,
// 128-bit for SHA-384 / SHA-512 (semantic trap #4). Endianness
// matters: MD5 is little-endian end-to-end, the SHA family is
// big-endian end-to-end (semantic trap #3).

#![allow(clippy::needless_range_loop)] // index-arithmetic numeric kernels

use std::fmt::Write;

// ===================  Public surface  =======================

/// One-shot digest bundle — every byte sequence produces all
/// five digests in one call. Used by [`parse`] / the
/// two_oracle_agreement driver and as a self-contained value
/// for callers that want every PDF-relevant digest at once.
#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(dead_code)] // reached by Phase E (encryption coverage) beat
pub struct Bundle {
    pub md5: [u8; 16],
    pub sha1: [u8; 20],
    pub sha256: [u8; 32],
    pub sha384: [u8; 48],
    pub sha512: [u8; 64],
}

/// MD5 (RFC 1321). Returns the 16-byte digest of `data`.
pub fn md5(data: &[u8]) -> [u8; 16] { md5_hash(data) }

/// SHA-1 (FIPS 180-4 §6.1). Returns the 20-byte digest.
#[allow(dead_code)] // reached by Phase E (encryption coverage) beat
pub fn sha1(data: &[u8]) -> [u8; 20] { sha1_hash(data) }

/// SHA-256 (FIPS 180-4 §6.2). Returns the 32-byte digest.
#[allow(dead_code)] // reached by Phase E (encryption coverage) beat
pub fn sha256(data: &[u8]) -> [u8; 32] { sha256_hash(data) }

/// SHA-384 (FIPS 180-4 §6.5). Same compress as SHA-512 with a
/// different IV; output truncated to 48 bytes.
#[allow(dead_code)] // reached by Phase E (encryption coverage) beat
pub fn sha384(data: &[u8]) -> [u8; 48] { sha384_hash(data) }

/// SHA-512 (FIPS 180-4 §6.4). Returns the 64-byte digest.
#[allow(dead_code)] // reached by Phase E (encryption coverage) beat
pub fn sha512(data: &[u8]) -> [u8; 64] { sha512_hash(data) }

/// Run all five digests on `input` in one call.
#[allow(dead_code)] // reached by Phase E (encryption coverage) beat
pub fn parse(input: &[u8]) -> Bundle {
    Bundle {
        md5:    md5(input),
        sha1:   sha1(input),
        sha256: sha256(input),
        sha384: sha384(input),
        sha512: sha512(input),
    }
}

/// Serialise a [`Bundle`] as the 400-byte text manifest the
/// two_oracle_agreement driver compares byte-for-byte against
/// the `.canonical` sidecar:
///
/// ```text
/// md5: <32 lc-hex>\n
/// sha1: <40 lc-hex>\n
/// sha256: <64 lc-hex>\n
/// sha384: <96 lc-hex>\n
/// sha512: <128 lc-hex>\n
/// ```
///
/// Lowercase hex (matches both `openssl dgst` and Python
/// hashlib.hexdigest()), single space after the colon, `\n`
/// line endings, trailing newline on the final line. Algorithm
/// order: md5 → sha1 → sha256 → sha384 → sha512 (output size
/// ascending) — semantic trap #2.
#[allow(dead_code)] // reached by Phase E (encryption coverage) beat
pub fn to_canonical(bundle: &Bundle) -> String {
    let mut s = String::with_capacity(400);
    let _ = write!(s, "md5: ");    push_hex(&mut s, &bundle.md5);    s.push('\n');
    let _ = write!(s, "sha1: ");   push_hex(&mut s, &bundle.sha1);   s.push('\n');
    let _ = write!(s, "sha256: "); push_hex(&mut s, &bundle.sha256); s.push('\n');
    let _ = write!(s, "sha384: "); push_hex(&mut s, &bundle.sha384); s.push('\n');
    let _ = write!(s, "sha512: "); push_hex(&mut s, &bundle.sha512); s.push('\n');
    s
}

// ===================  Hex (lowercase)  ======================

#[inline]
#[allow(dead_code)] // reached by Phase E (encryption coverage) beat
fn push_hex(out: &mut String, bytes: &[u8]) {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    for &b in bytes {
        out.push(HEX[(b >> 4) as usize] as char);
        out.push(HEX[(b & 0x0f) as usize] as char);
    }
}

// ===================  Endianness helpers  ===================

#[allow(dead_code)] // reached by Phase E (encryption coverage) beat
#[inline] fn be32(b: &[u8]) -> u32 {
    u32::from_be_bytes([b[0], b[1], b[2], b[3]])
}
#[allow(dead_code)] // reached by Phase E (encryption coverage) beat
#[inline] fn be64(b: &[u8]) -> u64 {
    u64::from_be_bytes([b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7]])
}
#[inline] fn le32(b: &[u8]) -> u32 {
    u32::from_le_bytes([b[0], b[1], b[2], b[3]])
}

// ===================  MD5 (RFC 1321)  =======================

#[rustfmt::skip]
const MD5_K: [u32; 64] = [
    0xd76aa478, 0xe8c7b756, 0x242070db, 0xc1bdceee,
    0xf57c0faf, 0x4787c62a, 0xa8304613, 0xfd469501,
    0x698098d8, 0x8b44f7af, 0xffff5bb1, 0x895cd7be,
    0x6b901122, 0xfd987193, 0xa679438e, 0x49b40821,
    0xf61e2562, 0xc040b340, 0x265e5a51, 0xe9b6c7aa,
    0xd62f105d, 0x02441453, 0xd8a1e681, 0xe7d3fbc8,
    0x21e1cde6, 0xc33707d6, 0xf4d50d87, 0x455a14ed,
    0xa9e3e905, 0xfcefa3f8, 0x676f02d9, 0x8d2a4c8a,
    0xfffa3942, 0x8771f681, 0x6d9d6122, 0xfde5380c,
    0xa4beea44, 0x4bdecfa9, 0xf6bb4b60, 0xbebfbc70,
    0x289b7ec6, 0xeaa127fa, 0xd4ef3085, 0x04881d05,
    0xd9d4d039, 0xe6db99e5, 0x1fa27cf8, 0xc4ac5665,
    0xf4292244, 0x432aff97, 0xab9423a7, 0xfc93a039,
    0x655b59c3, 0x8f0ccc92, 0xffeff47d, 0x85845dd1,
    0x6fa87e4f, 0xfe2ce6e0, 0xa3014314, 0x4e0811a1,
    0xf7537e82, 0xbd3af235, 0x2ad7d2bb, 0xeb86d391,
];

#[rustfmt::skip]
const MD5_S: [u32; 64] = [
    7, 12, 17, 22,  7, 12, 17, 22,  7, 12, 17, 22,  7, 12, 17, 22,
    5,  9, 14, 20,  5,  9, 14, 20,  5,  9, 14, 20,  5,  9, 14, 20,
    4, 11, 16, 23,  4, 11, 16, 23,  4, 11, 16, 23,  4, 11, 16, 23,
    6, 10, 15, 21,  6, 10, 15, 21,  6, 10, 15, 21,  6, 10, 15, 21,
];

fn md5_compress(state: &mut [u32; 4], block: &[u8; 64]) {
    let mut m = [0u32; 16];
    for i in 0..16 {
        m[i] = le32(&block[4 * i..4 * (i + 1)]);
    }
    let (mut a, mut b, mut c, mut d) = (state[0], state[1], state[2], state[3]);
    for i in 0..64 {
        let (f, g) = if i < 16 {
            ((b & c) | (!b & d), i)
        } else if i < 32 {
            ((b & d) | (c & !d), (5 * i + 1) % 16)
        } else if i < 48 {
            (b ^ c ^ d, (3 * i + 5) % 16)
        } else {
            (c ^ (b | !d), (7 * i) % 16)
        };
        let f = f
            .wrapping_add(a)
            .wrapping_add(MD5_K[i])
            .wrapping_add(m[g]);
        a = d;
        d = c;
        c = b;
        b = b.wrapping_add(f.rotate_left(MD5_S[i]));
    }
    state[0] = state[0].wrapping_add(a);
    state[1] = state[1].wrapping_add(b);
    state[2] = state[2].wrapping_add(c);
    state[3] = state[3].wrapping_add(d);
}

fn md5_hash(data: &[u8]) -> [u8; 16] {
    let mut state = [0x67452301_u32, 0xefcdab89, 0x98badcfe, 0x10325476];
    let bit_len = (data.len() as u64).wrapping_mul(8);
    // Whole 64-byte blocks straight from input.
    let n_full = data.len() / 64;
    for i in 0..n_full {
        let mut block = [0u8; 64];
        block.copy_from_slice(&data[64 * i..64 * (i + 1)]);
        md5_compress(&mut state, &block);
    }
    // Tail block(s): 0x80 marker + zeros + 8-byte LE bit length.
    let rem = &data[64 * n_full..];
    let mut tail = [0u8; 128];
    tail[..rem.len()].copy_from_slice(rem);
    tail[rem.len()] = 0x80;
    let pad_to = if rem.len() < 56 { 64 } else { 128 };
    tail[pad_to - 8..pad_to].copy_from_slice(&bit_len.to_le_bytes());
    let mut block = [0u8; 64];
    block.copy_from_slice(&tail[..64]);
    md5_compress(&mut state, &block);
    if pad_to == 128 {
        block.copy_from_slice(&tail[64..128]);
        md5_compress(&mut state, &block);
    }
    let mut out = [0u8; 16];
    for i in 0..4 {
        out[4 * i..4 * (i + 1)].copy_from_slice(&state[i].to_le_bytes());
    }
    out
}

// ===================  SHA-1 (FIPS 180-4 §6.1)  ==============

#[allow(dead_code)] // reached by Phase E (encryption coverage) beat
fn sha1_compress(state: &mut [u32; 5], block: &[u8; 64]) {
    let mut w = [0u32; 80];
    for i in 0..16 {
        w[i] = be32(&block[4 * i..4 * (i + 1)]);
    }
    for i in 16..80 {
        w[i] = (w[i - 3] ^ w[i - 8] ^ w[i - 14] ^ w[i - 16]).rotate_left(1);
    }
    let (mut a, mut b, mut c, mut d, mut e) =
        (state[0], state[1], state[2], state[3], state[4]);
    for i in 0..80 {
        let (f, k) = if i < 20 {
            ((b & c) | (!b & d), 0x5a827999)
        } else if i < 40 {
            (b ^ c ^ d, 0x6ed9eba1)
        } else if i < 60 {
            ((b & c) | (b & d) | (c & d), 0x8f1bbcdc)
        } else {
            (b ^ c ^ d, 0xca62c1d6)
        };
        let t = a.rotate_left(5)
            .wrapping_add(f)
            .wrapping_add(e)
            .wrapping_add(k)
            .wrapping_add(w[i]);
        e = d;
        d = c;
        c = b.rotate_left(30);
        b = a;
        a = t;
    }
    state[0] = state[0].wrapping_add(a);
    state[1] = state[1].wrapping_add(b);
    state[2] = state[2].wrapping_add(c);
    state[3] = state[3].wrapping_add(d);
    state[4] = state[4].wrapping_add(e);
}

#[allow(dead_code)] // reached by Phase E (encryption coverage) beat
fn sha1_hash(data: &[u8]) -> [u8; 20] {
    let mut state = [
        0x67452301_u32, 0xefcdab89, 0x98badcfe, 0x10325476, 0xc3d2e1f0,
    ];
    let bit_len = (data.len() as u64).wrapping_mul(8);
    let n_full = data.len() / 64;
    for i in 0..n_full {
        let mut block = [0u8; 64];
        block.copy_from_slice(&data[64 * i..64 * (i + 1)]);
        sha1_compress(&mut state, &block);
    }
    let rem = &data[64 * n_full..];
    let mut tail = [0u8; 128];
    tail[..rem.len()].copy_from_slice(rem);
    tail[rem.len()] = 0x80;
    let pad_to = if rem.len() < 56 { 64 } else { 128 };
    tail[pad_to - 8..pad_to].copy_from_slice(&bit_len.to_be_bytes());
    let mut block = [0u8; 64];
    block.copy_from_slice(&tail[..64]);
    sha1_compress(&mut state, &block);
    if pad_to == 128 {
        block.copy_from_slice(&tail[64..128]);
        sha1_compress(&mut state, &block);
    }
    let mut out = [0u8; 20];
    for i in 0..5 {
        out[4 * i..4 * (i + 1)].copy_from_slice(&state[i].to_be_bytes());
    }
    out
}

// ===================  SHA-256 (FIPS 180-4 §6.2)  ============

#[rustfmt::skip]
#[allow(dead_code)] // reached by Phase E (encryption coverage) beat
const SHA256_K: [u32; 64] = [
    0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5,
    0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5,
    0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3,
    0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174,
    0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc,
    0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da,
    0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7,
    0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967,
    0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13,
    0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85,
    0xa2bfe8a1, 0xa81a664b, 0xc24b8b70, 0xc76c51a3,
    0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070,
    0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5,
    0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
    0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208,
    0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2,
];

#[allow(dead_code)] // reached by Phase E (encryption coverage) beat
fn sha256_compress(state: &mut [u32; 8], block: &[u8; 64]) {
    let mut w = [0u32; 64];
    for i in 0..16 {
        w[i] = be32(&block[4 * i..4 * (i + 1)]);
    }
    for i in 16..64 {
        let s0 = w[i - 15].rotate_right(7)
            ^ w[i - 15].rotate_right(18)
            ^ (w[i - 15] >> 3);
        let s1 = w[i - 2].rotate_right(17)
            ^ w[i - 2].rotate_right(19)
            ^ (w[i - 2] >> 10);
        w[i] = w[i - 16]
            .wrapping_add(s0)
            .wrapping_add(w[i - 7])
            .wrapping_add(s1);
    }
    let mut a = state[0]; let mut b = state[1];
    let mut c = state[2]; let mut d = state[3];
    let mut e = state[4]; let mut f = state[5];
    let mut g = state[6]; let mut h = state[7];
    for i in 0..64 {
        let big_s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
        let ch = (e & f) ^ (!e & g);
        let t1 = h
            .wrapping_add(big_s1)
            .wrapping_add(ch)
            .wrapping_add(SHA256_K[i])
            .wrapping_add(w[i]);
        let big_s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
        let maj = (a & b) ^ (a & c) ^ (b & c);
        let t2 = big_s0.wrapping_add(maj);
        h = g; g = f; f = e;
        e = d.wrapping_add(t1);
        d = c; c = b; b = a;
        a = t1.wrapping_add(t2);
    }
    state[0] = state[0].wrapping_add(a);
    state[1] = state[1].wrapping_add(b);
    state[2] = state[2].wrapping_add(c);
    state[3] = state[3].wrapping_add(d);
    state[4] = state[4].wrapping_add(e);
    state[5] = state[5].wrapping_add(f);
    state[6] = state[6].wrapping_add(g);
    state[7] = state[7].wrapping_add(h);
}

#[allow(dead_code)] // reached by Phase E (encryption coverage) beat
fn sha256_hash(data: &[u8]) -> [u8; 32] {
    let mut state = [
        0x6a09e667_u32, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a,
        0x510e527f, 0x9b05688c, 0x1f83d9ab, 0x5be0cd19,
    ];
    let bit_len = (data.len() as u64).wrapping_mul(8);
    let n_full = data.len() / 64;
    for i in 0..n_full {
        let mut block = [0u8; 64];
        block.copy_from_slice(&data[64 * i..64 * (i + 1)]);
        sha256_compress(&mut state, &block);
    }
    let rem = &data[64 * n_full..];
    let mut tail = [0u8; 128];
    tail[..rem.len()].copy_from_slice(rem);
    tail[rem.len()] = 0x80;
    let pad_to = if rem.len() < 56 { 64 } else { 128 };
    tail[pad_to - 8..pad_to].copy_from_slice(&bit_len.to_be_bytes());
    let mut block = [0u8; 64];
    block.copy_from_slice(&tail[..64]);
    sha256_compress(&mut state, &block);
    if pad_to == 128 {
        block.copy_from_slice(&tail[64..128]);
        sha256_compress(&mut state, &block);
    }
    let mut out = [0u8; 32];
    for i in 0..8 {
        out[4 * i..4 * (i + 1)].copy_from_slice(&state[i].to_be_bytes());
    }
    out
}

// ===================  SHA-512 / SHA-384 (FIPS 180-4 §6.4-§6.5) =

#[rustfmt::skip]
#[allow(dead_code)] // reached by Phase E (encryption coverage) beat
const SHA512_K: [u64; 80] = [
    0x428a2f98d728ae22, 0x7137449123ef65cd, 0xb5c0fbcfec4d3b2f, 0xe9b5dba58189dbbc,
    0x3956c25bf348b538, 0x59f111f1b605d019, 0x923f82a4af194f9b, 0xab1c5ed5da6d8118,
    0xd807aa98a3030242, 0x12835b0145706fbe, 0x243185be4ee4b28c, 0x550c7dc3d5ffb4e2,
    0x72be5d74f27b896f, 0x80deb1fe3b1696b1, 0x9bdc06a725c71235, 0xc19bf174cf692694,
    0xe49b69c19ef14ad2, 0xefbe4786384f25e3, 0x0fc19dc68b8cd5b5, 0x240ca1cc77ac9c65,
    0x2de92c6f592b0275, 0x4a7484aa6ea6e483, 0x5cb0a9dcbd41fbd4, 0x76f988da831153b5,
    0x983e5152ee66dfab, 0xa831c66d2db43210, 0xb00327c898fb213f, 0xbf597fc7beef0ee4,
    0xc6e00bf33da88fc2, 0xd5a79147930aa725, 0x06ca6351e003826f, 0x142929670a0e6e70,
    0x27b70a8546d22ffc, 0x2e1b21385c26c926, 0x4d2c6dfc5ac42aed, 0x53380d139d95b3df,
    0x650a73548baf63de, 0x766a0abb3c77b2a8, 0x81c2c92e47edaee6, 0x92722c851482353b,
    0xa2bfe8a14cf10364, 0xa81a664bbc423001, 0xc24b8b70d0f89791, 0xc76c51a30654be30,
    0xd192e819d6ef5218, 0xd69906245565a910, 0xf40e35855771202a, 0x106aa07032bbd1b8,
    0x19a4c116b8d2d0c8, 0x1e376c085141ab53, 0x2748774cdf8eeb99, 0x34b0bcb5e19b48a8,
    0x391c0cb3c5c95a63, 0x4ed8aa4ae3418acb, 0x5b9cca4f7763e373, 0x682e6ff3d6b2b8a3,
    0x748f82ee5defb2fc, 0x78a5636f43172f60, 0x84c87814a1f0ab72, 0x8cc702081a6439ec,
    0x90befffa23631e28, 0xa4506cebde82bde9, 0xbef9a3f7b2c67915, 0xc67178f2e372532b,
    0xca273eceea26619c, 0xd186b8c721c0c207, 0xeada7dd6cde0eb1e, 0xf57d4f7fee6ed178,
    0x06f067aa72176fba, 0x0a637dc5a2c898a6, 0x113f9804bef90dae, 0x1b710b35131c471b,
    0x28db77f523047d84, 0x32caab7b40c72493, 0x3c9ebe0a15c9bebc, 0x431d67c49c100d4c,
    0x4cc5d4becb3e42b6, 0x597f299cfc657e2a, 0x5fcb6fab3ad6faec, 0x6c44198c4a475817,
];

#[allow(dead_code)] // reached by Phase E (encryption coverage) beat
fn sha512_compress(state: &mut [u64; 8], block: &[u8; 128]) {
    let mut w = [0u64; 80];
    for i in 0..16 {
        w[i] = be64(&block[8 * i..8 * (i + 1)]);
    }
    for i in 16..80 {
        let s0 = w[i - 15].rotate_right(1)
            ^ w[i - 15].rotate_right(8)
            ^ (w[i - 15] >> 7);
        let s1 = w[i - 2].rotate_right(19)
            ^ w[i - 2].rotate_right(61)
            ^ (w[i - 2] >> 6);
        w[i] = w[i - 16]
            .wrapping_add(s0)
            .wrapping_add(w[i - 7])
            .wrapping_add(s1);
    }
    let mut a = state[0]; let mut b = state[1];
    let mut c = state[2]; let mut d = state[3];
    let mut e = state[4]; let mut f = state[5];
    let mut g = state[6]; let mut h = state[7];
    for i in 0..80 {
        let big_s1 = e.rotate_right(14) ^ e.rotate_right(18) ^ e.rotate_right(41);
        let ch = (e & f) ^ (!e & g);
        let t1 = h
            .wrapping_add(big_s1)
            .wrapping_add(ch)
            .wrapping_add(SHA512_K[i])
            .wrapping_add(w[i]);
        let big_s0 = a.rotate_right(28) ^ a.rotate_right(34) ^ a.rotate_right(39);
        let maj = (a & b) ^ (a & c) ^ (b & c);
        let t2 = big_s0.wrapping_add(maj);
        h = g; g = f; f = e;
        e = d.wrapping_add(t1);
        d = c; c = b; b = a;
        a = t1.wrapping_add(t2);
    }
    state[0] = state[0].wrapping_add(a);
    state[1] = state[1].wrapping_add(b);
    state[2] = state[2].wrapping_add(c);
    state[3] = state[3].wrapping_add(d);
    state[4] = state[4].wrapping_add(e);
    state[5] = state[5].wrapping_add(f);
    state[6] = state[6].wrapping_add(g);
    state[7] = state[7].wrapping_add(h);
}

// Shared driver for SHA-512 and SHA-384 — every step is
// identical except the IV. SHA-384 truncates the output.
#[allow(dead_code)] // reached by Phase E (encryption coverage) beat
fn sha512_core(data: &[u8], iv: [u64; 8]) -> [u64; 8] {
    let mut state = iv;
    // 128-bit BE bit length tail (semantic trap #4). Practical
    // PDF inputs fit in u64 bits so the high half is always 0.
    let bit_len = (data.len() as u128).wrapping_mul(8);
    let n_full = data.len() / 128;
    for i in 0..n_full {
        let mut block = [0u8; 128];
        block.copy_from_slice(&data[128 * i..128 * (i + 1)]);
        sha512_compress(&mut state, &block);
    }
    let rem = &data[128 * n_full..];
    let mut tail = [0u8; 256];
    tail[..rem.len()].copy_from_slice(rem);
    tail[rem.len()] = 0x80;
    // Threshold is 112 (semantic trap #4): the 16-byte length
    // pad needs 16 bytes of room, not 8.
    let pad_to = if rem.len() < 112 { 128 } else { 256 };
    tail[pad_to - 16..pad_to].copy_from_slice(&bit_len.to_be_bytes());
    let mut block = [0u8; 128];
    block.copy_from_slice(&tail[..128]);
    sha512_compress(&mut state, &block);
    if pad_to == 256 {
        block.copy_from_slice(&tail[128..256]);
        sha512_compress(&mut state, &block);
    }
    state
}

#[allow(dead_code)] // reached by Phase E (encryption coverage) beat
fn sha512_hash(data: &[u8]) -> [u8; 64] {
    let state = sha512_core(data, [
        0x6a09e667f3bcc908, 0xbb67ae8584caa73b,
        0x3c6ef372fe94f82b, 0xa54ff53a5f1d36f1,
        0x510e527fade682d1, 0x9b05688c2b3e6c1f,
        0x1f83d9abfb41bd6b, 0x5be0cd19137e2179,
    ]);
    let mut out = [0u8; 64];
    for i in 0..8 {
        out[8 * i..8 * (i + 1)].copy_from_slice(&state[i].to_be_bytes());
    }
    out
}

#[allow(dead_code)] // reached by Phase E (encryption coverage) beat
fn sha384_hash(data: &[u8]) -> [u8; 48] {
    let state = sha512_core(data, [
        0xcbbb9d5dc1059ed8, 0x629a292a367cd507,
        0x9159015a3070dd17, 0x152fecd8f70e5939,
        0x67332667ffc00b31, 0x8eb44a8768581511,
        0xdb0c2e0d64f98fa7, 0x47b5481dbefa4fa4,
    ]);
    let mut out = [0u8; 48];
    for i in 0..6 {
        out[8 * i..8 * (i + 1)].copy_from_slice(&state[i].to_be_bytes());
    }
    out
}

// ===================  Tests  ================================

#[cfg(test)]
mod tests {
    use super::*;

    fn hex(b: &[u8]) -> String {
        let mut s = String::with_capacity(b.len() * 2);
        push_hex(&mut s, b);
        s
    }

    // ----- Empty-input vectors (semantic trap: don't short-
    // circuit on empty — every digest has a known empty value).
    #[test]
    fn md5_empty() {
        assert_eq!(hex(&md5(b"")), "d41d8cd98f00b204e9800998ecf8427e");
    }

    #[test]
    fn sha1_empty() {
        assert_eq!(hex(&sha1(b"")), "da39a3ee5e6b4b0d3255bfef95601890afd80709");
    }

    #[test]
    fn sha256_empty() {
        assert_eq!(hex(&sha256(b"")),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855");
    }

    #[test]
    fn sha384_empty() {
        assert_eq!(hex(&sha384(b"")),
            "38b060a751ac96384cd9327eb1b1e36a21fdb71114be07434c0cc7bf63f6e1da274edebfe76f65fbd51ad2f14898b95b");
    }

    #[test]
    fn sha512_empty() {
        assert_eq!(hex(&sha512(b"")),
            "cf83e1357eefb8bdf1542850d66d8007d620e4050b5715dc83f4a921d36ce9ce47d0d13c5d85f2b0ff8318d2877eec2f63b931bd47417a81a538327af927da3e");
    }

    // ----- RFC 1321 §A.5 MD5 test suite.
    #[test]
    fn md5_rfc1321_a5() {
        assert_eq!(hex(&md5(b"a")), "0cc175b9c0f1b6a831c399e269772661");
        assert_eq!(hex(&md5(b"abc")), "900150983cd24fb0d6963f7d28e17f72");
        assert_eq!(hex(&md5(b"message digest")),
            "f96b697d7cb7938d525a2f31aaf161d0");
        assert_eq!(hex(&md5(b"abcdefghijklmnopqrstuvwxyz")),
            "c3fcd3d76192e4007dfb496cca67e13b");
        assert_eq!(hex(&md5(
            b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789")),
            "d174ab98d277d9f5a5611c2c9f419d9f");
        assert_eq!(hex(&md5(
            b"12345678901234567890123456789012345678901234567890123456789012345678901234567890")),
            "57edf4a22be3c955ac49da2e2107b67a");
    }

    // ----- FIPS 180-4 SHA-1 examples.
    #[test]
    fn sha1_fips180_examples() {
        assert_eq!(hex(&sha1(b"abc")),
            "a9993e364706816aba3e25717850c26c9cd0d89d");
        assert_eq!(hex(&sha1(
            b"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq")),
            "84983e441c3bd26ebaae4aa1f95129e5e54670f1");
    }

    // ----- FIPS 180-4 SHA-256 examples.
    #[test]
    fn sha256_fips180_examples() {
        assert_eq!(hex(&sha256(b"abc")),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad");
        assert_eq!(hex(&sha256(
            b"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq")),
            "248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1");
    }

    // ----- FIPS 180-4 SHA-384 / SHA-512 short vectors. The
    // longer two-block FIPS examples are exercised exhaustively
    // by the corpus integration gate (sha512_block_111.bin etc.)
    // and by the boundary unit tests below; the "abc" cases
    // pin the published one-block value here.
    #[test]
    fn sha384_fips180_abc() {
        assert_eq!(hex(&sha384(b"abc")),
            "cb00753f45a35e8bb5a03d699ac65007272c32ab0eded1631a8b605a43ff5bed8086072ba1e7cc2358baeca134c825a7");
    }

    #[test]
    fn sha512_fips180_abc() {
        assert_eq!(hex(&sha512(b"abc")),
            "ddaf35a193617abacc417349ae20413112e6fa4e89a97ea20a9eeee64b55d39a2192992a274fc1a836ba3c23a3feebbd454d4423643ce80e2a9ac94fa54ca49f");
    }

    // ----- Block-boundary edges (semantic trap #4 — length tail
    // width). Pinned values come from the .canonical sidecars
    // vendored next to the fixtures (oracles: Python hashlib +
    // openssl dgst, byte-for-byte agreement at authoring time).
    // The corpus integration gate exercises the same bytes;
    // these unit cases surface the failures inline so a wrong
    // length-tail width fails the unit suite, not only the gate.
    #[test]
    fn md5_55_byte_a_run() {
        // 55 'A's — padding fits in same 64-byte block.
        assert_eq!(hex(&md5(&[b'A'; 55])),
            "e38a93ffe074a99b3fed47dfbe37db21");
    }

    #[test]
    fn md5_56_byte_a_run() {
        // 56 'A's — padding overflows to second 64-byte block.
        assert_eq!(hex(&md5(&[b'A'; 56])),
            "a2f3e2024931bd470555002aa5ccc010");
    }

    #[test]
    fn sha512_111_byte_a_run() {
        // 111 'A's — 16-byte length pad fits in same 128-byte
        // block. From sha512_block_111.canonical sidecar.
        assert_eq!(hex(&sha512(&[b'A'; 111])),
            "ec9c8ac688af410e33264ba659dbc7bb652971831680ea2062576d394d298f58b3ccd9a1f39a3b40bc6c73609c976f71524efcd45abbf1a98b3f15dccf0b0b9f");
    }

    #[test]
    fn sha512_112_byte_a_run() {
        // 112 'A's — 16-byte length pad forces a second 128-byte
        // block. An impl that uses SHA-256's 8-byte threshold
        // (< 120 instead of < 112) emits a wrong digest here.
        // From sha512_block_112.canonical sidecar.
        assert_eq!(hex(&sha512(&[b'A'; 112])),
            "1a008b0480a4eb64d292db671d4f43f46fc57e077b72ad3ec0a3b0b63b320357a11418ea916038e9b659ccf39ae574ef8a8f683f1eff954788591c13022fcd81");
    }

    // ----- Determinism: repeated calls produce identical output.
    #[test]
    fn back_to_back_calls_are_deterministic() {
        let data = b"deterministic-digest-test-payload";
        assert_eq!(md5(data), md5(data));
        assert_eq!(sha1(data), sha1(data));
        assert_eq!(sha256(data), sha256(data));
        assert_eq!(sha384(data), sha384(data));
        assert_eq!(sha512(data), sha512(data));
    }

    // ----- Bundle / parse / to_canonical end-to-end.
    #[test]
    fn parse_runs_all_five_digests() {
        let b = parse(b"");
        assert_eq!(hex(&b.md5),    "d41d8cd98f00b204e9800998ecf8427e");
        assert_eq!(hex(&b.sha1),   "da39a3ee5e6b4b0d3255bfef95601890afd80709");
        assert_eq!(hex(&b.sha256), "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855");
        assert_eq!(hex(&b.sha384), "38b060a751ac96384cd9327eb1b1e36a21fdb71114be07434c0cc7bf63f6e1da274edebfe76f65fbd51ad2f14898b95b");
        assert_eq!(hex(&b.sha512), "cf83e1357eefb8bdf1542850d66d8007d620e4050b5715dc83f4a921d36ce9ce47d0d13c5d85f2b0ff8318d2877eec2f63b931bd47417a81a538327af927da3e");
    }

    // Semantic trap #1 + #2: lowercase hex, exact spacing,
    // trailing newline on every line, fixed algorithm order.
    // Pinned against the empty.canonical sidecar.
    #[test]
    fn to_canonical_format_matches_empty_sidecar() {
        let want = "md5: d41d8cd98f00b204e9800998ecf8427e\n\
                    sha1: da39a3ee5e6b4b0d3255bfef95601890afd80709\n\
                    sha256: e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855\n\
                    sha384: 38b060a751ac96384cd9327eb1b1e36a21fdb71114be07434c0cc7bf63f6e1da274edebfe76f65fbd51ad2f14898b95b\n\
                    sha512: cf83e1357eefb8bdf1542850d66d8007d620e4050b5715dc83f4a921d36ce9ce47d0d13c5d85f2b0ff8318d2877eec2f63b931bd47417a81a538327af927da3e\n";
        assert_eq!(to_canonical(&parse(b"")), want);
        assert_eq!(want.len(), 400);
    }
}
