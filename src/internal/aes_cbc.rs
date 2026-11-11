// AES-CBC (FIPS 197 + NIST SP 800-38A §6.2 + RFC 5652 §6.3).
//
// AES-128 / AES-256 in CBC mode with two padding modes:
//
//   * PKCS#7 (encrypt / decrypt) — PDF 32000-1 §7.6.3.2
//     mandates this for AES-CBC payload encryption. One full
//     16-byte pad block is ALWAYS emitted (empty plaintext →
//     16 bytes, 16-byte plaintext → 32 bytes); strip validates
//     every pad byte.
//
//   * No-padding (encrypt_no_pad / decrypt_no_pad) — bare CBC
//     chain over a positive-block-aligned input. Used by the
//     V=5 password-handler Algorithm 2.A (file-key recovery
//     from /UE / /OE) and Algorithm 2.B (R=6 hash cascade) in
//     encrypt_parser; output length always equals input length.
//
// Key length selects the cipher: 16 → AES-128 (Nk=4, Nr=10),
// 32 → AES-256 (Nk=8, Nr=14). PDF doesn't use AES-192. IV is
// always 16 bytes (the block size).

#![allow(clippy::needless_range_loop)] // index-arithmetic numeric kernels

use std::fmt;

/// Errors returned by the AES-CBC entry points.
#[allow(clippy::enum_variant_names)] // descriptive `Invalid*` error names
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// Key length is neither 16 (AES-128) nor 32 (AES-256).
    InvalidKeyLength { len: usize },
    /// IV length is not 16 bytes (the AES block size).
    InvalidIvLength { len: usize },
    /// Buffer length doesn't satisfy the block-alignment
    /// contract. `decrypt`'s ciphertext must be a positive
    /// multiple of 16 (AES-CBC produces nothing shorter than
    /// one full pad block); `encrypt_no_pad` /
    /// `decrypt_no_pad` require a positive multiple of 16
    /// (zero-length raw CBC is meaningless).
    InvalidBlockAlignedLength { len: usize },
    /// PKCS#7 padding validation failed — the trailing pad
    /// byte is out of range, longer than the plaintext, or
    /// not all `p` of the trailing bytes equal `p`.
    InvalidPadding,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::InvalidKeyLength { len } => write!(
                f, "aes_cbc: invalid key length {len} (expected 16 or 32)"),
            Error::InvalidIvLength { len } => write!(
                f, "aes_cbc: invalid iv length {len} (expected 16)"),
            Error::InvalidBlockAlignedLength { len } => write!(
                f, "aes_cbc: invalid length {len} (expected positive multiple of 16)"),
            Error::InvalidPadding => write!(
                f, "aes_cbc: invalid PKCS#7 padding"),
        }
    }
}

impl std::error::Error for Error {}

const BLOCK_SIZE: usize = 16;

/// Fixed 16-byte test key for the gate wrappers. AES-128 is
/// enough to exercise the binding_roundtrip property; PDF
/// /Encrypt consumers call [`encrypt`] / [`decrypt`] directly
/// with their own per-object derived keys.
#[allow(dead_code)] // reached by Phase E (encryption coverage) beat
const GATE_KEY: [u8; 16] = *b"aes-cbc-gate-key";
#[allow(dead_code)] // reached by Phase E (encryption coverage) beat
const GATE_IV: [u8; BLOCK_SIZE] = [0u8; BLOCK_SIZE];

// FIPS 197 §5.1.1 Figure 7 — forward S-box.
#[rustfmt::skip]
const SBOX: [u8; 256] = [
    0x63, 0x7c, 0x77, 0x7b, 0xf2, 0x6b, 0x6f, 0xc5, 0x30, 0x01, 0x67, 0x2b, 0xfe, 0xd7, 0xab, 0x76,
    0xca, 0x82, 0xc9, 0x7d, 0xfa, 0x59, 0x47, 0xf0, 0xad, 0xd4, 0xa2, 0xaf, 0x9c, 0xa4, 0x72, 0xc0,
    0xb7, 0xfd, 0x93, 0x26, 0x36, 0x3f, 0xf7, 0xcc, 0x34, 0xa5, 0xe5, 0xf1, 0x71, 0xd8, 0x31, 0x15,
    0x04, 0xc7, 0x23, 0xc3, 0x18, 0x96, 0x05, 0x9a, 0x07, 0x12, 0x80, 0xe2, 0xeb, 0x27, 0xb2, 0x75,
    0x09, 0x83, 0x2c, 0x1a, 0x1b, 0x6e, 0x5a, 0xa0, 0x52, 0x3b, 0xd6, 0xb3, 0x29, 0xe3, 0x2f, 0x84,
    0x53, 0xd1, 0x00, 0xed, 0x20, 0xfc, 0xb1, 0x5b, 0x6a, 0xcb, 0xbe, 0x39, 0x4a, 0x4c, 0x58, 0xcf,
    0xd0, 0xef, 0xaa, 0xfb, 0x43, 0x4d, 0x33, 0x85, 0x45, 0xf9, 0x02, 0x7f, 0x50, 0x3c, 0x9f, 0xa8,
    0x51, 0xa3, 0x40, 0x8f, 0x92, 0x9d, 0x38, 0xf5, 0xbc, 0xb6, 0xda, 0x21, 0x10, 0xff, 0xf3, 0xd2,
    0xcd, 0x0c, 0x13, 0xec, 0x5f, 0x97, 0x44, 0x17, 0xc4, 0xa7, 0x7e, 0x3d, 0x64, 0x5d, 0x19, 0x73,
    0x60, 0x81, 0x4f, 0xdc, 0x22, 0x2a, 0x90, 0x88, 0x46, 0xee, 0xb8, 0x14, 0xde, 0x5e, 0x0b, 0xdb,
    0xe0, 0x32, 0x3a, 0x0a, 0x49, 0x06, 0x24, 0x5c, 0xc2, 0xd3, 0xac, 0x62, 0x91, 0x95, 0xe4, 0x79,
    0xe7, 0xc8, 0x37, 0x6d, 0x8d, 0xd5, 0x4e, 0xa9, 0x6c, 0x56, 0xf4, 0xea, 0x65, 0x7a, 0xae, 0x08,
    0xba, 0x78, 0x25, 0x2e, 0x1c, 0xa6, 0xb4, 0xc6, 0xe8, 0xdd, 0x74, 0x1f, 0x4b, 0xbd, 0x8b, 0x8a,
    0x70, 0x3e, 0xb5, 0x66, 0x48, 0x03, 0xf6, 0x0e, 0x61, 0x35, 0x57, 0xb9, 0x86, 0xc1, 0x1d, 0x9e,
    0xe1, 0xf8, 0x98, 0x11, 0x69, 0xd9, 0x8e, 0x94, 0x9b, 0x1e, 0x87, 0xe9, 0xce, 0x55, 0x28, 0xdf,
    0x8c, 0xa1, 0x89, 0x0d, 0xbf, 0xe6, 0x42, 0x68, 0x41, 0x99, 0x2d, 0x0f, 0xb0, 0x54, 0xbb, 0x16,
];

// FIPS 197 §5.3.2 Figure 14 — inverse S-box.
#[rustfmt::skip]
const INV_SBOX: [u8; 256] = [
    0x52, 0x09, 0x6a, 0xd5, 0x30, 0x36, 0xa5, 0x38, 0xbf, 0x40, 0xa3, 0x9e, 0x81, 0xf3, 0xd7, 0xfb,
    0x7c, 0xe3, 0x39, 0x82, 0x9b, 0x2f, 0xff, 0x87, 0x34, 0x8e, 0x43, 0x44, 0xc4, 0xde, 0xe9, 0xcb,
    0x54, 0x7b, 0x94, 0x32, 0xa6, 0xc2, 0x23, 0x3d, 0xee, 0x4c, 0x95, 0x0b, 0x42, 0xfa, 0xc3, 0x4e,
    0x08, 0x2e, 0xa1, 0x66, 0x28, 0xd9, 0x24, 0xb2, 0x76, 0x5b, 0xa2, 0x49, 0x6d, 0x8b, 0xd1, 0x25,
    0x72, 0xf8, 0xf6, 0x64, 0x86, 0x68, 0x98, 0x16, 0xd4, 0xa4, 0x5c, 0xcc, 0x5d, 0x65, 0xb6, 0x92,
    0x6c, 0x70, 0x48, 0x50, 0xfd, 0xed, 0xb9, 0xda, 0x5e, 0x15, 0x46, 0x57, 0xa7, 0x8d, 0x9d, 0x84,
    0x90, 0xd8, 0xab, 0x00, 0x8c, 0xbc, 0xd3, 0x0a, 0xf7, 0xe4, 0x58, 0x05, 0xb8, 0xb3, 0x45, 0x06,
    0xd0, 0x2c, 0x1e, 0x8f, 0xca, 0x3f, 0x0f, 0x02, 0xc1, 0xaf, 0xbd, 0x03, 0x01, 0x13, 0x8a, 0x6b,
    0x3a, 0x91, 0x11, 0x41, 0x4f, 0x67, 0xdc, 0xea, 0x97, 0xf2, 0xcf, 0xce, 0xf0, 0xb4, 0xe6, 0x73,
    0x96, 0xac, 0x74, 0x22, 0xe7, 0xad, 0x35, 0x85, 0xe2, 0xf9, 0x37, 0xe8, 0x1c, 0x75, 0xdf, 0x6e,
    0x47, 0xf1, 0x1a, 0x71, 0x1d, 0x29, 0xc5, 0x89, 0x6f, 0xb7, 0x62, 0x0e, 0xaa, 0x18, 0xbe, 0x1b,
    0xfc, 0x56, 0x3e, 0x4b, 0xc6, 0xd2, 0x79, 0x20, 0x9a, 0xdb, 0xc0, 0xfe, 0x78, 0xcd, 0x5a, 0xf4,
    0x1f, 0xdd, 0xa8, 0x33, 0x88, 0x07, 0xc7, 0x31, 0xb1, 0x12, 0x10, 0x59, 0x27, 0x80, 0xec, 0x5f,
    0x60, 0x51, 0x7f, 0xa9, 0x19, 0xb5, 0x4a, 0x0d, 0x2d, 0xe5, 0x7a, 0x9f, 0x93, 0xc9, 0x9c, 0xef,
    0xa0, 0xe0, 0x3b, 0x4d, 0xae, 0x2a, 0xf5, 0xb0, 0xc8, 0xeb, 0xbb, 0x3c, 0x83, 0x53, 0x99, 0x61,
    0x17, 0x2b, 0x04, 0x7e, 0xba, 0x77, 0xd6, 0x26, 0xe1, 0x69, 0x14, 0x63, 0x55, 0x21, 0x0c, 0x7d,
];

// FIPS 197 §5.2 — Rcon[i] = (RC[i], 0, 0, 0). RC[1]=1,
// RC[i]=xtime(RC[i-1]). Indices 1..=10 cover both AES-128
// (max i/Nk = 43/4 = 10) and AES-256 (max i/Nk = 59/8 = 7).
const RCON: [u8; 11] = [
    0x00, 0x01, 0x02, 0x04, 0x08, 0x10, 0x20, 0x40, 0x80, 0x1b, 0x36,
];

// =====================  GF(2^8) helpers  =====================

/// xtime(b) — multiplication by x in GF(2^8) modulo
/// x^8 + x^4 + x^3 + x + 1 (the AES reduction polynomial).
#[inline]
fn xtime(b: u8) -> u8 {
    (b << 1) ^ (((b >> 7) & 1).wrapping_mul(0x1b))
}

#[allow(dead_code)] // reached by Phase E (encryption coverage) beat
#[inline] fn gmul2(b: u8) -> u8 { xtime(b) }
#[allow(dead_code)] // reached by Phase E (encryption coverage) beat
#[inline] fn gmul3(b: u8) -> u8 { xtime(b) ^ b }
#[inline] fn gmul9(b: u8) -> u8 { xtime(xtime(xtime(b))) ^ b }
#[inline] fn gmul11(b: u8) -> u8 { xtime(xtime(xtime(b)) ^ b) ^ b }
#[inline] fn gmul13(b: u8) -> u8 { xtime(xtime(xtime(b) ^ b)) ^ b }
#[inline] fn gmul14(b: u8) -> u8 { xtime(xtime(xtime(b) ^ b) ^ b) }

// =====================  AES core (FIPS 197)  ================
//
// State layout: 16 bytes column-major. state[r + 4*c] holds the
// byte at row r, column c of the 4×4 AES state matrix.

#[allow(dead_code)] // reached by Phase E (encryption coverage) beat
fn sub_bytes(state: &mut [u8; BLOCK_SIZE]) {
    for b in state.iter_mut() { *b = SBOX[*b as usize]; }
}

fn inv_sub_bytes(state: &mut [u8; BLOCK_SIZE]) {
    for b in state.iter_mut() { *b = INV_SBOX[*b as usize]; }
}

// Row r is rotated left by r positions. With column-major
// layout that means new[r + 4*c] = old[r + 4*((c + r) mod 4)].
#[allow(dead_code)] // reached by Phase E (encryption coverage) beat
fn shift_rows(state: &mut [u8; BLOCK_SIZE]) {
    let s = *state;
    // Row 1.
    state[1]  = s[5];  state[5]  = s[9];  state[9]  = s[13]; state[13] = s[1];
    // Row 2.
    state[2]  = s[10]; state[6]  = s[14]; state[10] = s[2];  state[14] = s[6];
    // Row 3.
    state[3]  = s[15]; state[7]  = s[3];  state[11] = s[7];  state[15] = s[11];
}

fn inv_shift_rows(state: &mut [u8; BLOCK_SIZE]) {
    let s = *state;
    // Row 1: shift right by 1.
    state[1]  = s[13]; state[5]  = s[1];  state[9]  = s[5];  state[13] = s[9];
    // Row 2: shift right by 2 (same as left by 2).
    state[2]  = s[10]; state[6]  = s[14]; state[10] = s[2];  state[14] = s[6];
    // Row 3: shift right by 3.
    state[3]  = s[7];  state[7]  = s[11]; state[11] = s[15]; state[15] = s[3];
}

// MixColumns — each column is multiplied by the fixed
// polynomial {03}x^3 + {01}x^2 + {01}x + {02} in GF(2^8)[x] /
// (x^4 + 1). FIPS 197 §5.1.3.
#[allow(dead_code)] // reached by Phase E (encryption coverage) beat
fn mix_columns(state: &mut [u8; BLOCK_SIZE]) {
    for c in 0..4 {
        let s0 = state[4 * c];
        let s1 = state[4 * c + 1];
        let s2 = state[4 * c + 2];
        let s3 = state[4 * c + 3];
        state[4 * c]     = gmul2(s0) ^ gmul3(s1) ^ s2          ^ s3;
        state[4 * c + 1] = s0          ^ gmul2(s1) ^ gmul3(s2) ^ s3;
        state[4 * c + 2] = s0          ^ s1          ^ gmul2(s2) ^ gmul3(s3);
        state[4 * c + 3] = gmul3(s0) ^ s1          ^ s2          ^ gmul2(s3);
    }
}

// InvMixColumns — multiply by {0b}x^3 + {0d}x^2 + {09}x + {0e}.
// FIPS 197 §5.3.3.
fn inv_mix_columns(state: &mut [u8; BLOCK_SIZE]) {
    for c in 0..4 {
        let s0 = state[4 * c];
        let s1 = state[4 * c + 1];
        let s2 = state[4 * c + 2];
        let s3 = state[4 * c + 3];
        state[4 * c]     = gmul14(s0) ^ gmul11(s1) ^ gmul13(s2) ^ gmul9(s3);
        state[4 * c + 1] = gmul9(s0)  ^ gmul14(s1) ^ gmul11(s2) ^ gmul13(s3);
        state[4 * c + 2] = gmul13(s0) ^ gmul9(s1)  ^ gmul14(s2) ^ gmul11(s3);
        state[4 * c + 3] = gmul11(s0) ^ gmul13(s1) ^ gmul9(s2)  ^ gmul14(s3);
    }
}

fn add_round_key(state: &mut [u8; BLOCK_SIZE], rk: &[u8; BLOCK_SIZE]) {
    for i in 0..BLOCK_SIZE { state[i] ^= rk[i]; }
}

// Key expansion — FIPS 197 §5.2. AES-128: 11 round keys
// (Nk=4, Nr=10). AES-256: 15 round keys (Nk=8, Nr=14). The
// AES-256 expansion has a SECOND special case at i mod Nk == 4
// (SubWord only, no RotWord, no Rcon) that AES-128 lacks —
// dropping it produces wrong round keys after every fourth word
// in the second half of each 8-word group.
fn expand_key(key: &[u8]) -> Result<(Vec<[u8; BLOCK_SIZE]>, usize), Error> {
    let (nk, nr) = match key.len() {
        16 => (4_usize, 10_usize),
        32 => (8_usize, 14_usize),
        _ => return Err(Error::InvalidKeyLength { len: key.len() }),
    };
    let total_words = 4 * (nr + 1);
    let mut w: Vec<[u8; 4]> = vec![[0u8; 4]; total_words];
    for i in 0..nk {
        w[i].copy_from_slice(&key[4 * i..4 * (i + 1)]);
    }
    for i in nk..total_words {
        let mut t = w[i - 1];
        if i % nk == 0 {
            t = [t[1], t[2], t[3], t[0]];          // RotWord
            for b in t.iter_mut() { *b = SBOX[*b as usize]; }
            t[0] ^= RCON[i / nk];
        } else if nk > 6 && i % nk == 4 {
            for b in t.iter_mut() { *b = SBOX[*b as usize]; }
        }
        let prev = w[i - nk];
        for j in 0..4 { w[i][j] = prev[j] ^ t[j]; }
    }
    let mut round_keys = Vec::with_capacity(nr + 1);
    for r in 0..=nr {
        let mut block = [0u8; BLOCK_SIZE];
        for c in 0..4 {
            block[4 * c..4 * (c + 1)].copy_from_slice(&w[4 * r + c]);
        }
        round_keys.push(block);
    }
    Ok((round_keys, nr))
}

#[allow(dead_code)] // reached by Phase E (encryption coverage) beat
fn encrypt_block(block: &mut [u8; BLOCK_SIZE], round_keys: &[[u8; BLOCK_SIZE]], nr: usize) {
    add_round_key(block, &round_keys[0]);
    for r in 1..nr {
        sub_bytes(block);
        shift_rows(block);
        mix_columns(block);
        add_round_key(block, &round_keys[r]);
    }
    sub_bytes(block);
    shift_rows(block);
    add_round_key(block, &round_keys[nr]);
}

fn decrypt_block(block: &mut [u8; BLOCK_SIZE], round_keys: &[[u8; BLOCK_SIZE]], nr: usize) {
    add_round_key(block, &round_keys[nr]);
    for r in (1..nr).rev() {
        inv_shift_rows(block);
        inv_sub_bytes(block);
        add_round_key(block, &round_keys[r]);
        inv_mix_columns(block);
    }
    inv_shift_rows(block);
    inv_sub_bytes(block);
    add_round_key(block, &round_keys[0]);
}

// =====================  CBC mode (NIST SP 800-38A §6.2)  ====

fn check_iv(iv: &[u8]) -> Result<[u8; BLOCK_SIZE], Error> {
    if iv.len() != BLOCK_SIZE {
        return Err(Error::InvalidIvLength { len: iv.len() });
    }
    let mut out = [0u8; BLOCK_SIZE];
    out.copy_from_slice(iv);
    Ok(out)
}

#[allow(dead_code)] // reached by Phase E (encryption coverage) beat
fn cbc_encrypt(
    round_keys: &[[u8; BLOCK_SIZE]], nr: usize,
    iv: [u8; BLOCK_SIZE], plaintext: &[u8],
) -> Vec<u8> {
    debug_assert!(plaintext.len() % BLOCK_SIZE == 0);
    let mut out = Vec::with_capacity(plaintext.len());
    let mut prev = iv;
    for chunk in plaintext.chunks_exact(BLOCK_SIZE) {
        let mut block = [0u8; BLOCK_SIZE];
        for i in 0..BLOCK_SIZE { block[i] = chunk[i] ^ prev[i]; }
        encrypt_block(&mut block, round_keys, nr);
        out.extend_from_slice(&block);
        prev = block;
    }
    out
}

fn cbc_decrypt(
    round_keys: &[[u8; BLOCK_SIZE]], nr: usize,
    iv: [u8; BLOCK_SIZE], ciphertext: &[u8],
) -> Vec<u8> {
    debug_assert!(ciphertext.len() % BLOCK_SIZE == 0);
    let mut out = Vec::with_capacity(ciphertext.len());
    let mut prev = iv;
    for chunk in ciphertext.chunks_exact(BLOCK_SIZE) {
        let mut block = [0u8; BLOCK_SIZE];
        block.copy_from_slice(chunk);
        let saved = block;
        decrypt_block(&mut block, round_keys, nr);
        for i in 0..BLOCK_SIZE { block[i] ^= prev[i]; }
        out.extend_from_slice(&block);
        prev = saved;
    }
    out
}

// =====================  Public surface  =====================

/// AES-CBC + PKCS#7 padding. Empty plaintext produces exactly one
/// 16-byte ciphertext block (the unconditional pad block);
/// block-aligned plaintext produces an extra full pad block.
#[allow(dead_code)] // reached by Phase E (encryption coverage) beat
pub fn encrypt(key: &[u8], iv: &[u8], data: &[u8]) -> Result<Vec<u8>, Error> {
    let (round_keys, nr) = expand_key(key)?;
    let iv = check_iv(iv)?;
    let pad = BLOCK_SIZE - (data.len() % BLOCK_SIZE);   // 1..=16
    let mut padded = Vec::with_capacity(data.len() + pad);
    padded.extend_from_slice(data);
    padded.resize(data.len() + pad, pad as u8);
    Ok(cbc_encrypt(&round_keys, nr, iv, &padded))
}

/// AES-CBC + PKCS#7 strip. Validates every pad byte —
/// `Err(InvalidPadding)` on any mismatch (a corrupted ciphertext
/// or a wrong key surfaces here, not as garbled plaintext).
pub fn decrypt(key: &[u8], iv: &[u8], data: &[u8]) -> Result<Vec<u8>, Error> {
    let (round_keys, nr) = expand_key(key)?;
    let iv = check_iv(iv)?;
    if data.is_empty() || data.len() % BLOCK_SIZE != 0 {
        return Err(Error::InvalidBlockAlignedLength { len: data.len() });
    }
    let mut out = cbc_decrypt(&round_keys, nr, iv, data);
    let pad = *out.last().expect("non-empty per length check");
    if pad == 0 || pad as usize > BLOCK_SIZE {
        return Err(Error::InvalidPadding);
    }
    let pad = pad as usize;
    if pad > out.len() {
        return Err(Error::InvalidPadding);
    }
    let tail_start = out.len() - pad;
    if out[tail_start..].iter().any(|&b| b as usize != pad) {
        return Err(Error::InvalidPadding);
    }
    out.truncate(tail_start);
    Ok(out)
}

/// Bare AES-CBC over a positive-block-aligned input. Used by the
/// V=5 password-handler Algorithm 2.A (32-byte ciphertext →
/// 32-byte file key, zero IV) and Algorithm 2.B (R=6 hash
/// cascade) in encrypt_parser. Output length always equals input
/// length.
#[allow(dead_code)] // reached by Phase E (encryption coverage) beat
pub fn encrypt_no_pad(key: &[u8], iv: &[u8], data: &[u8]) -> Result<Vec<u8>, Error> {
    if data.is_empty() || data.len() % BLOCK_SIZE != 0 {
        return Err(Error::InvalidBlockAlignedLength { len: data.len() });
    }
    let (round_keys, nr) = expand_key(key)?;
    let iv = check_iv(iv)?;
    Ok(cbc_encrypt(&round_keys, nr, iv, data))
}

/// Inverse of [`encrypt_no_pad`]. No PKCS#7 strip, no validation
/// of any tail byte — the only error comes from bad
/// key/IV/length.
#[allow(dead_code)] // reached by Phase E (encryption coverage) beat
pub fn decrypt_no_pad(key: &[u8], iv: &[u8], data: &[u8]) -> Result<Vec<u8>, Error> {
    if data.is_empty() || data.len() % BLOCK_SIZE != 0 {
        return Err(Error::InvalidBlockAlignedLength { len: data.len() });
    }
    let (round_keys, nr) = expand_key(key)?;
    let iv = check_iv(iv)?;
    Ok(cbc_decrypt(&round_keys, nr, iv, data))
}

/// Gate wrapper — `encrypt(GATE_KEY, GATE_IV, data)`. The fixed
/// AES-128 key + zero IV exists solely to feed the
/// binding_roundtrip driver template; PDF /Encrypt callers use
/// [`encrypt`] directly with their own derived keys.
#[allow(dead_code)] // reached by Phase E (encryption coverage) beat
pub fn encode(data: &[u8]) -> Vec<u8> {
    encrypt(&GATE_KEY, &GATE_IV, data)
        .expect("gate key + iv are valid by construction")
}

/// Gate wrapper — inverse of [`encode`]. Only fails if the input
/// wasn't produced by [`encode`].
#[allow(dead_code)] // reached by Phase E (encryption coverage) beat
pub fn decode(data: &[u8]) -> Vec<u8> {
    decrypt(&GATE_KEY, &GATE_IV, data)
        .expect("gate roundtrip cannot fail on encode output")
}

// =====================  Tests  ==============================

#[cfg(test)]
mod tests {
    use super::*;

    fn unhex(s: &str) -> Vec<u8> {
        let s: String = s.chars().filter(|c| !c.is_whitespace()).collect();
        let bytes = s.as_bytes();
        let mut out = Vec::with_capacity(bytes.len() / 2);
        let mut i = 0;
        while i < bytes.len() {
            let nibble = |c: u8| match c {
                b'0'..=b'9' => c - b'0',
                b'a'..=b'f' => c - b'a' + 10,
                b'A'..=b'F' => c - b'A' + 10,
                _ => panic!("bad hex"),
            };
            out.push((nibble(bytes[i]) << 4) | nibble(bytes[i + 1]));
            i += 2;
        }
        out
    }

    // FIPS 197 Appendix C.1 — AES-128 single-block test vector.
    #[test]
    fn fips197_appendix_c1_aes128() {
        let key = unhex("000102030405060708090a0b0c0d0e0f");
        let pt  = unhex("00112233445566778899aabbccddeeff");
        let want = unhex("69c4e0d86a7b0430d8cdb78070b4c55a");
        let ct = encrypt_no_pad(&key, &[0u8; 16], &pt).unwrap();
        assert_eq!(ct, want);
        let back = decrypt_no_pad(&key, &[0u8; 16], &ct).unwrap();
        assert_eq!(back, pt);
    }

    // FIPS 197 Appendix C.3 — AES-256 single-block test vector.
    #[test]
    fn fips197_appendix_c3_aes256() {
        let key = unhex(
            "000102030405060708090a0b0c0d0e0f\
             101112131415161718191a1b1c1d1e1f");
        let pt  = unhex("00112233445566778899aabbccddeeff");
        let want = unhex("8ea2b7ca516745bfeafc49904b496089");
        let ct = encrypt_no_pad(&key, &[0u8; 16], &pt).unwrap();
        assert_eq!(ct, want);
        let back = decrypt_no_pad(&key, &[0u8; 16], &ct).unwrap();
        assert_eq!(back, pt);
    }

    // NIST SP 800-38A F.2.1/F.2.2 — AES-128-CBC, 4 plaintext
    // blocks. Exercises CBC chaining across multiple blocks.
    #[test]
    fn nist_sp_800_38a_f2_aes128_cbc() {
        let key = unhex("2b7e151628aed2a6abf7158809cf4f3c");
        let iv  = unhex("000102030405060708090a0b0c0d0e0f");
        let pt = unhex(
            "6bc1bee22e409f96e93d7e117393172a\
             ae2d8a571e03ac9c9eb76fac45af8e51\
             30c81c46a35ce411e5fbc1191a0a52ef\
             f69f2445df4f9b17ad2b417be66c3710");
        let want = unhex(
            "7649abac8119b246cee98e9b12e9197d\
             5086cb9b507219ee95db113a917678b2\
             73bed6b8e3c1743b7116e69e22229516\
             3ff1caa1681fac09120eca307586e1a7");
        let ct = encrypt_no_pad(&key, &iv, &pt).unwrap();
        assert_eq!(ct, want);
        assert_eq!(decrypt_no_pad(&key, &iv, &ct).unwrap(), pt);
    }

    // NIST SP 800-38A F.2.5/F.2.6 — AES-256-CBC, 4 plaintext
    // blocks.
    #[test]
    fn nist_sp_800_38a_f2_aes256_cbc() {
        let key = unhex(
            "603deb1015ca71be2b73aef0857d7781\
             1f352c073b6108d72d9810a30914dff4");
        let iv  = unhex("000102030405060708090a0b0c0d0e0f");
        let pt = unhex(
            "6bc1bee22e409f96e93d7e117393172a\
             ae2d8a571e03ac9c9eb76fac45af8e51\
             30c81c46a35ce411e5fbc1191a0a52ef\
             f69f2445df4f9b17ad2b417be66c3710");
        let want = unhex(
            "f58c4c04d6e5f1ba779eabfb5f7bfbd6\
             9cfc4e967edb808d679f777bc6702c7d\
             39f23369a9d9bacfa530e26304231461\
             b2eb05e2c39be9fcda6c19078c6a9d1b");
        let ct = encrypt_no_pad(&key, &iv, &pt).unwrap();
        assert_eq!(ct, want);
        assert_eq!(decrypt_no_pad(&key, &iv, &ct).unwrap(), pt);
    }

    // Semantic trap #1: empty plaintext produces exactly ONE
    // 16-byte ciphertext block (the unconditional pad block).
    #[test]
    fn pkcs7_empty_emits_full_pad_block() {
        let key = [0u8; 16];
        let iv  = [0u8; 16];
        let ct = encrypt(&key, &iv, b"").unwrap();
        assert_eq!(ct.len(), BLOCK_SIZE);
        assert_eq!(decrypt(&key, &iv, &ct).unwrap(), b"");
    }

    // Semantic trap #1: block-aligned plaintext produces a
    // strictly-longer ciphertext (input + full pad block).
    #[test]
    fn pkcs7_block_aligned_appends_full_pad_block() {
        let key = [0u8; 16];
        let iv  = [0u8; 16];
        let pt  = [0xAA_u8; BLOCK_SIZE];
        let ct = encrypt(&key, &iv, &pt).unwrap();
        assert_eq!(ct.len(), 2 * BLOCK_SIZE);
        assert_eq!(decrypt(&key, &iv, &ct).unwrap(), pt);
    }

    // Semantic trap #2: PKCS#7 strip validates EVERY pad byte,
    // not just the last one. With a 14-byte input the pad is
    // two 0x02 bytes at offsets 14, 15 of the only plaintext
    // block. Flipping IV[14] propagates exactly into plaintext
    // byte 14 (CBC: P[0] = AES-decrypt(C[0]) XOR IV), so the
    // trailing pad byte stays 0x02 (correct length) while the
    // inner pad byte becomes 0x03. A strip that only consults
    // the last byte would silently accept; a correct strip
    // catches the inner mismatch.
    #[test]
    fn pkcs7_strip_validates_every_pad_byte() {
        let key = [0u8; 16];
        let iv  = [0u8; 16];
        let pt  = b"fourteen-bytes";
        let ct = encrypt(&key, &iv, pt).unwrap();
        assert_eq!(decrypt(&key, &iv, &ct).unwrap(), pt);
        let mut bad_iv = iv;
        bad_iv[14] ^= 0x01;
        assert_eq!(
            decrypt(&key, &bad_iv, &ct).unwrap_err(),
            Error::InvalidPadding);
    }

    #[test]
    fn pkcs7_strip_rejects_zero_pad_byte() {
        // Zero pad byte is never valid — every PKCS#7 padding
        // is 1..=16. Construct a ciphertext whose decrypted
        // last byte is 0x00 and confirm InvalidPadding.
        let key = [0u8; 16];
        let iv  = [0u8; 16];
        // Encrypt 16 zero bytes WITHOUT padding to get a
        // ciphertext that decrypts to all-zeros.
        let raw_ct = encrypt_no_pad(&key, &iv, &[0u8; 16]).unwrap();
        let err = decrypt(&key, &iv, &raw_ct).unwrap_err();
        assert_eq!(err, Error::InvalidPadding);
    }

    #[test]
    fn invalid_key_lengths_rejected() {
        let iv = [0u8; 16];
        assert!(matches!(
            encrypt(&[0u8; 15], &iv, b""),
            Err(Error::InvalidKeyLength { len: 15 })));
        assert!(matches!(
            encrypt(&[0u8; 24], &iv, b""),     // AES-192 unsupported
            Err(Error::InvalidKeyLength { len: 24 })));
        assert!(matches!(
            encrypt(&[0u8; 33], &iv, b""),
            Err(Error::InvalidKeyLength { len: 33 })));
    }

    #[test]
    fn invalid_iv_length_rejected() {
        let key = [0u8; 16];
        assert!(matches!(
            encrypt(&key, &[0u8; 15], b""),
            Err(Error::InvalidIvLength { len: 15 })));
        assert!(matches!(
            encrypt(&key, &[0u8; 17], b""),
            Err(Error::InvalidIvLength { len: 17 })));
    }

    #[test]
    fn decrypt_rejects_non_aligned_or_empty() {
        let key = [0u8; 16];
        let iv  = [0u8; 16];
        assert!(matches!(
            decrypt(&key, &iv, &[]),
            Err(Error::InvalidBlockAlignedLength { len: 0 })));
        assert!(matches!(
            decrypt(&key, &iv, &[0u8; 15]),
            Err(Error::InvalidBlockAlignedLength { len: 15 })));
    }

    #[test]
    fn no_pad_rejects_empty_or_misaligned() {
        let key = [0u8; 16];
        let iv  = [0u8; 16];
        assert!(matches!(
            encrypt_no_pad(&key, &iv, &[]),
            Err(Error::InvalidBlockAlignedLength { len: 0 })));
        assert!(matches!(
            encrypt_no_pad(&key, &iv, &[0u8; 15]),
            Err(Error::InvalidBlockAlignedLength { len: 15 })));
        assert!(matches!(
            decrypt_no_pad(&key, &iv, &[]),
            Err(Error::InvalidBlockAlignedLength { len: 0 })));
        assert!(matches!(
            decrypt_no_pad(&key, &iv, &[0u8; 17]),
            Err(Error::InvalidBlockAlignedLength { len: 17 })));
    }

    #[test]
    fn round_trip_pkcs7_various_lengths_aes128() {
        let key = *b"PDFstandardKey16";
        let iv  = *b"sixteen-byte-iv!";
        for len in [0, 1, 15, 16, 17, 31, 32, 33, 100, 1024] {
            let pt: Vec<u8> = (0..len).map(|i| (i * 31 + 7) as u8).collect();
            let ct = encrypt(&key, &iv, &pt).unwrap();
            assert_eq!(ct.len(), pt.len() + (BLOCK_SIZE - pt.len() % BLOCK_SIZE));
            assert_eq!(decrypt(&key, &iv, &ct).unwrap(), pt);
        }
    }

    #[test]
    fn round_trip_pkcs7_various_lengths_aes256() {
        let key: [u8; 32] = std::array::from_fn(|i| (i * 13 + 5) as u8);
        let iv  = [0xCC_u8; 16];
        for len in [0, 16, 17, 100, 1024] {
            let pt: Vec<u8> = (0..len).map(|i| (i * 47 + 11) as u8).collect();
            let ct = encrypt(&key, &iv, &pt).unwrap();
            assert_eq!(decrypt(&key, &iv, &ct).unwrap(), pt);
        }
    }

    #[test]
    fn no_pad_round_trip_aes256() {
        // Mirrors the encrypt_parser Algorithm 2.A shape — 32-byte
        // input, AES-256, zero IV, 1:1 length.
        let key: [u8; 32] = std::array::from_fn(|i| (i * 19 + 3) as u8);
        let iv  = [0u8; 16];
        let pt: [u8; 32] = std::array::from_fn(|i| (i * 7 + 1) as u8);
        let ct = encrypt_no_pad(&key, &iv, &pt).unwrap();
        assert_eq!(ct.len(), pt.len());
        assert_eq!(decrypt_no_pad(&key, &iv, &ct).unwrap(), pt.to_vec());
    }

    #[test]
    fn gate_wrappers_round_trip() {
        for input in [
            b"" as &[u8],
            b"x",
            b"sixteen bytes!!!",                    // block-aligned
            b"seventeen bytes!!",
            &[0xFF_u8; 256],
        ] {
            let ct = encode(input);
            assert_eq!(decode(&ct), input);
        }
    }

    #[test]
    fn back_to_back_calls_dont_leak_state() {
        // Two independent encrypts on the same plaintext with the
        // same key+iv must produce identical ciphertext. A round-
        // key cache with leaked mutation between calls would
        // diverge on the second call.
        let key = [0xA0_u8; 16];
        let iv  = [0x55_u8; 16];
        let pt  = b"deterministic encryption please";
        assert_eq!(
            encrypt(&key, &iv, pt).unwrap(),
            encrypt(&key, &iv, pt).unwrap());
    }
}
