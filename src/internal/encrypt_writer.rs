//! Encrypt-writer foundation primitive — the write side of the PDF
//! Standard Security Handler's password layer.
//!
//! Given user + owner passwords, a `/P` permission bitfield, the `(V, R)`
//! variant, `/Length`, the file `/ID[0]`, `/EncryptMetadata`, and a
//! deterministic salt buffer, [`build_encrypt_dict`] computes the
//! `/Encrypt` dictionary fields (`/O`, `/U`, `/OE`, `/UE`, `/Perms`) and
//! the document encryption key. The strict counterpart to
//! [`encrypt_parser`](super::encrypt_parser): the parser RECOVERS the key
//! from an existing dict + password; the writer PRODUCES the dict + key.
//!
//! Authoring strategy: from_scratch (Tier-2-direct — multi-phase
//! cryptography). The algorithms are transcribed from PDF 32000-1:2008
//! §7.6.3 (Algorithms 2–5), the Adobe Supplement (V5 R5 Algorithms
//! 3.7–3.10), and PDF 32000-2:2020 §7.6.4 (R6 Algorithms 8–10 + 2.B),
//! composing the frozen `digest` / `rc4` / `aes_cbc` cipher cores.

use super::aes_cbc;
use super::digest::{md5, sha256, sha384, sha512};
use super::encrypt_parser::EncryptDict;
use super::rc4;

/// The PDF standard 32-byte password padding string (§7.6.3.3).
const PASSWORD_PAD: [u8; 32] = [
    0x28, 0xBF, 0x4E, 0x5E, 0x4E, 0x75, 0x8A, 0x41, 0x64, 0x00, 0x4E, 0x56, 0xFF, 0xFA, 0x01, 0x08,
    0x2E, 0x2E, 0x00, 0xB6, 0xD0, 0x68, 0x3E, 0x80, 0x2F, 0x0C, 0xA9, 0xFE, 0x64, 0x53, 0x69, 0x7A,
];

/// Output of [`build_encrypt_dict`].
#[derive(Debug, Clone)]
pub(crate) struct EncryptOutput {
    pub encrypt_dict: EncryptDict,
    pub file_key: Vec<u8>,
}

/// Structural errors for malformed writer inputs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Error {
    UnsupportedVR(u32, u32),
    BadLengthBits(u32, u32),
    BadSaltSize { expected: usize, got: usize },
    MissingFileId,
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::UnsupportedVR(v, r) => write!(f, "encrypt_writer: unsupported V={v} R={r}"),
            Error::BadLengthBits(b, r) => write!(f, "encrypt_writer: bad /Length {b} for R={r}"),
            Error::BadSaltSize { expected, got } => {
                write!(f, "encrypt_writer: salt is {got} bytes, expected {expected}")
            }
            Error::MissingFileId => write!(f, "encrypt_writer: /ID[0] required for R<5"),
        }
    }
}

impl std::error::Error for Error {}

/// Build the `/Encrypt` dictionary fields + the document encryption key.
#[allow(clippy::too_many_arguments)]
pub(crate) fn build_encrypt_dict(
    user_password: &[u8],
    owner_password: &[u8],
    p: i32,
    v: u32,
    r: u32,
    length_bits: u32,
    file_id: &[u8],
    encrypt_metadata: bool,
    salt: &[u8],
) -> Result<EncryptOutput, Error> {
    validate(v, r, length_bits)?;
    if r >= 5 {
        build_v5(user_password, owner_password, p, v, r, file_id, encrypt_metadata, salt)
    } else {
        build_classic(user_password, owner_password, p, v, r, length_bits, file_id, encrypt_metadata, salt)
    }
}

fn validate(v: u32, r: u32, length_bits: u32) -> Result<(), Error> {
    match (v, r) {
        (1, 2) if length_bits == 40 => Ok(()),
        (2, 3) if (40..=128).contains(&length_bits) && length_bits % 8 == 0 => Ok(()),
        (4, 4) if length_bits == 128 => Ok(()),
        (5, 5) | (5, 6) if length_bits == 256 => Ok(()),
        (1, 2) | (2, 3) | (4, 4) | (5, 5) | (5, 6) => Err(Error::BadLengthBits(length_bits, r)),
        _ => Err(Error::UnsupportedVR(v, r)),
    }
}

// ============================================================
// Classic handler (R = 2 / 3 / 4)
// ============================================================

#[allow(clippy::too_many_arguments)]
fn build_classic(
    user_password: &[u8],
    owner_password: &[u8],
    p: i32,
    v: u32,
    r: u32,
    length_bits: u32,
    file_id: &[u8],
    encrypt_metadata: bool,
    salt: &[u8],
) -> Result<EncryptOutput, Error> {
    if file_id.is_empty() {
        return Err(Error::MissingFileId);
    }
    if salt.len() != 16 {
        return Err(Error::BadSaltSize { expected: 16, got: salt.len() });
    }
    let key_len = (length_bits / 8) as usize;

    // Algorithm 3 (/O) must run before Algorithm 2 (which folds /O in).
    let o = compute_o_classic(owner_password, user_password, r, key_len);
    let file_key = compute_file_key_classic(user_password, &o, p, file_id, r, key_len, encrypt_metadata);
    let u = if r == 2 {
        compute_u_r2(&file_key)
    } else {
        compute_u_r3plus(&file_key, file_id, salt)
    };

    Ok(EncryptOutput {
        encrypt_dict: EncryptDict {
            v,
            r,
            length_bits,
            p,
            o,
            u,
            oe: Vec::new(),
            ue: Vec::new(),
            perms: Vec::new(),
            encrypt_metadata,
        },
        file_key,
    })
}

fn pad_password(password: &[u8]) -> [u8; 32] {
    let mut out = [0u8; 32];
    let n = password.len().min(32);
    out[..n].copy_from_slice(&password[..n]);
    if n < 32 {
        out[n..].copy_from_slice(&PASSWORD_PAD[..32 - n]);
    }
    out
}

/// Algorithm 3 — compute `/O` for R<5.
fn compute_o_classic(owner_password: &[u8], user_password: &[u8], r: u32, key_len: usize) -> Vec<u8> {
    // No owner password → use the user password.
    let owner = if owner_password.is_empty() { user_password } else { owner_password };
    let padded_owner = pad_password(owner);
    let mut h = md5(&padded_owner);
    if r >= 3 {
        for _ in 0..50 {
            h = md5(&h[..16]);
        }
    }
    let k = h[..key_len].to_vec();

    let padded_user = pad_password(user_password);
    if r == 2 {
        return rc4::apply(&k, &padded_user).expect("rc4 /O");
    }
    // R≥3: 20-round outer loop, key K XOR round-counter byte.
    let mut out = padded_user.to_vec();
    for n in 0u8..20 {
        let round_key: Vec<u8> = k.iter().map(|b| b ^ n).collect();
        out = rc4::apply(&round_key, &out).expect("rc4 /O round");
    }
    out
}

/// Algorithm 2 — compute the file encryption key for R<5.
#[allow(clippy::too_many_arguments)]
fn compute_file_key_classic(
    user_password: &[u8],
    o: &[u8],
    p: i32,
    file_id: &[u8],
    r: u32,
    key_len: usize,
    encrypt_metadata: bool,
) -> Vec<u8> {
    let padded = pad_password(user_password);
    let mut input: Vec<u8> = Vec::new();
    input.extend_from_slice(&padded);
    input.extend_from_slice(o);
    input.extend_from_slice(&(p as u32).to_le_bytes());
    input.extend_from_slice(file_id);
    if r >= 4 && !encrypt_metadata {
        input.extend_from_slice(&[0xFF, 0xFF, 0xFF, 0xFF]);
    }
    let mut h = md5(&input);
    if r >= 3 {
        for _ in 0..50 {
            h = md5(&h[..key_len]);
        }
    }
    h[..key_len].to_vec()
}

/// Algorithm 4 — compute `/U` for R=2.
fn compute_u_r2(file_key: &[u8]) -> Vec<u8> {
    rc4::apply(file_key, &PASSWORD_PAD).expect("rc4 /U R2")
}

/// Algorithm 5 — compute `/U` for R≥3.
fn compute_u_r3plus(file_key: &[u8], file_id: &[u8], salt: &[u8]) -> Vec<u8> {
    let mut input = PASSWORD_PAD.to_vec();
    input.extend_from_slice(file_id);
    let digest = md5(&input);
    let mut out = rc4::apply(file_key, &digest).expect("rc4 /U R3");
    for n in 1u8..=19 {
        let round_key: Vec<u8> = file_key.iter().map(|b| b ^ n).collect();
        out = rc4::apply(&round_key, &out).expect("rc4 /U R3 round");
    }
    // First 16 bytes = Algorithm 5 output; trailing 16 = salt.
    out.truncate(16);
    out.extend_from_slice(&salt[..16]);
    out
}

// ============================================================
// V5 handler (R = 5 / 6)
// ============================================================

#[allow(clippy::too_many_arguments)]
fn build_v5(
    user_password: &[u8],
    owner_password: &[u8],
    p: i32,
    v: u32,
    r: u32,
    _file_id: &[u8],
    encrypt_metadata: bool,
    salt: &[u8],
) -> Result<EncryptOutput, Error> {
    if salt.len() != 68 {
        return Err(Error::BadSaltSize { expected: 68, got: salt.len() });
    }
    let user = trunc127(user_password);
    let owner = trunc127(owner_password);

    let user_validation_salt = &salt[0..8];
    let user_key_salt = &salt[8..16];
    let owner_validation_salt = &salt[16..24];
    let owner_key_salt = &salt[24..32];
    let file_key = salt[32..64].to_vec();
    let perms_tail = &salt[64..68];

    // Algorithm 3.7 / 8 — /U.
    let mut u = hash_v5(r, user, user_validation_salt, &[]);
    u.extend_from_slice(user_validation_salt);
    u.extend_from_slice(user_key_salt);

    // Algorithm 3.8 / 8 — /UE.
    let u_intermediate = hash_v5(r, user, user_key_salt, &[]);
    let ue = aes_cbc::encrypt_no_pad(&u_intermediate, &[0u8; 16], &file_key).expect("aes /UE");

    // Algorithm 3.9 / 9 — /O (folds the just-built /U in).
    let mut o = hash_v5(r, owner, owner_validation_salt, &u);
    o.extend_from_slice(owner_validation_salt);
    o.extend_from_slice(owner_key_salt);

    // Algorithm 3.10 / 9 — /OE.
    let o_intermediate = hash_v5(r, owner, owner_key_salt, &u);
    let oe = aes_cbc::encrypt_no_pad(&o_intermediate, &[0u8; 16], &file_key).expect("aes /OE");

    // Algorithm 10 — /Perms.
    let perms = compute_perms(p, encrypt_metadata, perms_tail, &file_key);

    Ok(EncryptOutput {
        encrypt_dict: EncryptDict {
            v,
            r,
            length_bits: 256,
            p,
            o,
            u,
            oe,
            ue,
            perms,
            encrypt_metadata,
        },
        file_key,
    })
}

fn trunc127(password: &[u8]) -> &[u8] {
    if password.len() > 127 {
        &password[..127]
    } else {
        password
    }
}

/// Algorithm 10 — compute `/Perms` for R≥5.
fn compute_perms(p: i32, encrypt_metadata: bool, perms_tail: &[u8], file_key: &[u8]) -> Vec<u8> {
    let mut plain = [0u8; 16];
    plain[0..4].copy_from_slice(&(p as u32).to_le_bytes());
    plain[4..8].copy_from_slice(&[0xFF, 0xFF, 0xFF, 0xFF]);
    plain[8] = if encrypt_metadata { b'T' } else { b'F' };
    plain[9] = b'a';
    plain[10] = b'd';
    plain[11] = b'b';
    plain[12..16].copy_from_slice(&perms_tail[..4]);
    // AES-256-ECB == single-block AES-256-CBC with a zero IV, no padding.
    aes_cbc::encrypt_no_pad(file_key, &[0u8; 16], &plain).expect("aes /Perms")
}

/// R≥5 password hash. R=5 is a single-pass SHA-256; R=6 is the
/// iterative Algorithm 2.B cascade.
fn hash_v5(r: u32, password: &[u8], salt: &[u8], extra: &[u8]) -> Vec<u8> {
    if r == 5 {
        let mut input: Vec<u8> = Vec::with_capacity(password.len() + salt.len() + extra.len());
        input.extend_from_slice(password);
        input.extend_from_slice(salt);
        input.extend_from_slice(extra);
        sha256(&input).to_vec()
    } else {
        algorithm_2b(password, salt, extra)
    }
}

/// PDF 32000-2:2020 Algorithm 2.B — iterative SHA-256/384/512 +
/// AES-128-CBC cascade.
fn algorithm_2b(password: &[u8], salt: &[u8], extra: &[u8]) -> Vec<u8> {
    let mut init: Vec<u8> = Vec::with_capacity(password.len() + salt.len() + extra.len());
    init.extend_from_slice(password);
    init.extend_from_slice(salt);
    init.extend_from_slice(extra);
    let mut k: Vec<u8> = sha256(&init).to_vec();

    let mut round: u32 = 0;
    loop {
        round += 1;
        let block_len = password.len() + k.len() + extra.len();
        let mut k1 = Vec::with_capacity(block_len * 64);
        for _ in 0..64 {
            k1.extend_from_slice(password);
            k1.extend_from_slice(&k);
            k1.extend_from_slice(extra);
        }
        let e = aes_cbc::encrypt_no_pad(&k[..16], &k[16..32], &k1).expect("aes-128-cbc 2.B");
        let mut sum_mod3: u32 = 0;
        for &b in &e[..16] {
            sum_mod3 = (sum_mod3 + b as u32) % 3;
        }
        k = match sum_mod3 {
            0 => sha256(&e).to_vec(),
            1 => sha384(&e).to_vec(),
            _ => sha512(&e).to_vec(),
        };
        let last = *e.last().unwrap();
        if round >= 64 && (last as u32) <= round - 32 {
            break;
        }
    }
    k[..32].to_vec()
}
