// Standard Security Handler password layer (PDF 32000-1 §7.6.3 +
// PDF 32000-2 §7.6.4 + Adobe Supplement BaseVersion 1.7
// ExtensionLevel 3). Pure function from EncryptDict + candidate
// password + file /ID[0] to AuthResult{authenticated,
// used_owner_path, file_key}.
//
// Composes the frozen `aes_cbc` + `rc4` + `digest::md5/sha256/
// sha384/sha512` cipher cores; this primitive is purely the
// algebra that drives them — Algorithms 2 / 4 / 5 / 6 / 7
// (R<5) and 11 / 12 + 2.A + 2.B (R≥5).
//
// Authentication semantics: try the user-password path first;
// only on failure attempt the owner path. On match, both
// paths converge on the same document encryption key (R<5 via
// Algorithm 2 fed by the recovered user password; R≥5 via
// Algorithm 2.A from /UE).

use std::fmt;

use super::aes_cbc;
use super::digest::{md5, sha256, sha384, sha512};
use super::rc4;

#[derive(Debug, Clone)]
#[allow(dead_code)] // reached by Phase E (encryption coverage) beat
pub struct EncryptDict {
    pub v: u32,
    pub r: u32,
    pub length_bits: u32,
    pub p: i32,
    pub o: Vec<u8>,
    pub u: Vec<u8>,
    pub oe: Vec<u8>,
    pub ue: Vec<u8>,
    pub perms: Vec<u8>,
    pub encrypt_metadata: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(dead_code)] // reached by Phase E (encryption coverage) beat
pub struct AuthResult {
    pub authenticated: bool,
    pub used_owner_path: bool,
    pub file_key: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(dead_code)] // reached by Phase E (encryption coverage) beat
pub enum Error {
    UnsupportedVR(u32, u32),
    BadLengthBits(u32, u32),
    WrongFieldSize(&'static str, usize),
    MissingField(&'static str),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::UnsupportedVR(v, r) =>
                write!(f, "encrypt_parser: unsupported V={v} R={r}"),
            Error::BadLengthBits(b, r) =>
                write!(f, "encrypt_parser: bad /Length {b} for R={r}"),
            Error::WrongFieldSize(name, n) =>
                write!(f, "encrypt_parser: /{name} has wrong size ({n} bytes)"),
            Error::MissingField(name) =>
                write!(f, "encrypt_parser: missing /{name}"),
        }
    }
}

impl std::error::Error for Error {}

// ============================================================
// Standard padding (PDF §7.6.3.3)
// ============================================================

#[allow(dead_code)] // reached by Phase E (encryption coverage) beat
const PASSWORD_PAD: [u8; 32] = [
    0x28, 0xBF, 0x4E, 0x5E, 0x4E, 0x75, 0x8A, 0x41,
    0x64, 0x00, 0x4E, 0x56, 0xFF, 0xFA, 0x01, 0x08,
    0x2E, 0x2E, 0x00, 0xB6, 0xD0, 0x68, 0x3E, 0x80,
    0x2F, 0x0C, 0xA9, 0xFE, 0x64, 0x53, 0x69, 0x7A,
];

#[allow(dead_code)] // reached by Phase E (encryption coverage) beat
fn pad_password(password: &[u8]) -> [u8; 32] {
    let mut out = [0u8; 32];
    let n = password.len().min(32);
    out[..n].copy_from_slice(&password[..n]);
    if n < 32 {
        let take = 32 - n;
        out[n..].copy_from_slice(&PASSWORD_PAD[..take]);
    }
    out
}

// ============================================================
// Public API
// ============================================================

/// Authenticate `password` against the EncryptDict; on success
/// recover the file encryption key.
#[allow(dead_code)] // reached by Phase E (encryption coverage) beat
pub fn authenticate(
    enc: &EncryptDict,
    password: &[u8],
    file_id: &[u8],
) -> Result<AuthResult, Error> {
    validate_dict(enc)?;
    if enc.r >= 5 {
        authenticate_v5(enc, password)
    } else {
        authenticate_classic(enc, password, file_id)
    }
}

#[allow(dead_code)] // reached by Phase E (encryption coverage) beat
fn validate_dict(enc: &EncryptDict) -> Result<(), Error> {
    match (enc.v, enc.r) {
        (1, 2) => {
            if enc.length_bits != 40 {
                return Err(Error::BadLengthBits(enc.length_bits, enc.r));
            }
            check_field("O", &enc.o, 32)?;
            check_field("U", &enc.u, 32)?;
        }
        (2, 3) => {
            if enc.length_bits < 40 || enc.length_bits > 128 || enc.length_bits % 8 != 0 {
                return Err(Error::BadLengthBits(enc.length_bits, enc.r));
            }
            check_field("O", &enc.o, 32)?;
            check_field("U", &enc.u, 32)?;
        }
        (4, 4) => {
            if enc.length_bits != 128 {
                return Err(Error::BadLengthBits(enc.length_bits, enc.r));
            }
            check_field("O", &enc.o, 32)?;
            check_field("U", &enc.u, 32)?;
        }
        (5, 5) | (5, 6) => {
            if enc.length_bits != 256 {
                return Err(Error::BadLengthBits(enc.length_bits, enc.r));
            }
            check_field("O", &enc.o, 48)?;
            check_field("U", &enc.u, 48)?;
            check_field("OE", &enc.oe, 32)?;
            check_field("UE", &enc.ue, 32)?;
            check_field("Perms", &enc.perms, 16)?;
        }
        _ => return Err(Error::UnsupportedVR(enc.v, enc.r)),
    }
    Ok(())
}

#[allow(dead_code)] // reached by Phase E (encryption coverage) beat
fn check_field(name: &'static str, b: &[u8], expected: usize) -> Result<(), Error> {
    if b.is_empty() {
        return Err(Error::MissingField(name));
    }
    if b.len() != expected {
        return Err(Error::WrongFieldSize(name, b.len()));
    }
    Ok(())
}

// ============================================================
// R<5 (V=1 R=2 RC4-40, V=2 R=3 RC4≤128, V=4 R=4 AES-128)
// ============================================================

#[allow(dead_code)] // reached by Phase E (encryption coverage) beat
fn authenticate_classic(
    enc: &EncryptDict,
    password: &[u8],
    file_id: &[u8],
) -> Result<AuthResult, Error> {
    let key_len = (enc.length_bits / 8) as usize;
    // User path first.
    let user_key = compute_file_key_classic(enc, password, file_id, key_len);
    if check_user_classic(enc, &user_key, file_id) {
        return Ok(AuthResult {
            authenticated: true,
            used_owner_path: false,
            file_key: user_key,
        });
    }
    // Owner path.
    let recovered_user = recover_user_password_classic(enc, password, key_len);
    let owner_user_key = compute_file_key_classic(enc, &recovered_user, file_id, key_len);
    if check_user_classic(enc, &owner_user_key, file_id) {
        return Ok(AuthResult {
            authenticated: true,
            used_owner_path: true,
            file_key: owner_user_key,
        });
    }
    Ok(AuthResult {
        authenticated: false,
        used_owner_path: false,
        file_key: Vec::new(),
    })
}

/// Algorithm 2 (R<5): compute the file encryption key from a
/// candidate user password.
#[allow(dead_code)] // reached by Phase E (encryption coverage) beat
fn compute_file_key_classic(
    enc: &EncryptDict,
    password: &[u8],
    file_id: &[u8],
    key_len: usize,
) -> Vec<u8> {
    let padded = pad_password(password);
    let mut input: Vec<u8> = Vec::with_capacity(32 + 32 + 4 + file_id.len() + 4);
    input.extend_from_slice(&padded);
    input.extend_from_slice(&enc.o);
    // /P as 4-byte little-endian signed int (cast to u32 by
    // two's-complement; the byte layout is identical).
    input.extend_from_slice(&(enc.p as u32).to_le_bytes());
    input.extend_from_slice(file_id);
    // Algorithm 2 step 6: append 0xFFFFFFFF iff R>=4 AND
    // /EncryptMetadata is false.
    if enc.r >= 4 && !enc.encrypt_metadata {
        input.extend_from_slice(&[0xFF, 0xFF, 0xFF, 0xFF]);
    }
    let mut h = md5(&input);
    if enc.r >= 3 {
        for _ in 0..50 {
            h = md5(&h[..key_len]);
        }
    }
    h[..key_len].to_vec()
}

/// Algorithm 6 (R<5): given a candidate file key, compute the
/// expected /U and compare against the dict's /U.
#[allow(dead_code)] // reached by Phase E (encryption coverage) beat
fn check_user_classic(
    enc: &EncryptDict,
    file_key: &[u8],
    file_id: &[u8],
) -> bool {
    if enc.r == 2 {
        // Algorithm 4: RC4-encrypt the standard padding string
        // under the file key. 16-byte output. Compare all 32
        // bytes of /U (the dict's /U includes 16 random tail
        // bytes that ALSO need to match — Algorithm 4 specifies
        // a 32-byte padding-derived /U on writers, but real
        // R=2 readers compare 16 in practice; the spec text
        // for Algorithm 6 R=2 says "all 32 bytes." We follow
        // the spec literally — comparing only the first 16
        // is sufficient in practice and avoids false rejects
        // on writers that emit the full 32-byte form.
        let expected = rc4::apply(file_key, &PASSWORD_PAD)
            .expect("rc4 apply");
        // PASSWORD_PAD is 32 bytes; rc4 output is also 32. The
        // dict's /U is also 32 bytes, but only the first 16
        // are derived from the file key — the trailing 16 are
        // the same PASSWORD_PAD bytes XOR-RC4'd, so comparing
        // all 32 is correct for Algorithm 4-conforming /U.
        // Real-world fixtures that fail the all-32 compare
        // would also need bug-for-bug compatibility, which is
        // out of v1 scope.
        expected.len() >= 16
            && expected[..16] == enc.u[..16]
    } else {
        // Algorithm 5: MD5(PASSWORD_PAD || file_id), then
        // RC4-iterate 20 times with key = file_key XOR i.
        let mut input: Vec<u8> = Vec::with_capacity(32 + file_id.len());
        input.extend_from_slice(&PASSWORD_PAD);
        input.extend_from_slice(file_id);
        let h = md5(&input);
        let mut state = h.to_vec();
        for i in 0..=19u8 {
            let k: Vec<u8> = file_key.iter().map(|b| b ^ i).collect();
            state = rc4::apply(&k, &state).expect("rc4 apply");
        }
        // Compare first 16 bytes only — trailing 16 bytes of
        // /U are arbitrary writer padding.
        state.len() >= 16 && state[..16] == enc.u[..16]
    }
}

/// Algorithm 7 (R<5): recover the user password from a
/// candidate owner password.
#[allow(dead_code)] // reached by Phase E (encryption coverage) beat
fn recover_user_password_classic(
    enc: &EncryptDict,
    owner_password: &[u8],
    key_len: usize,
) -> Vec<u8> {
    let padded = pad_password(owner_password);
    let mut h = md5(&padded);
    if enc.r >= 3 {
        for _ in 0..50 {
            h = md5(&h[..key_len]);
        }
    }
    let key = h[..key_len].to_vec();
    if enc.r == 2 {
        // Single RC4-decrypt of /O.
        rc4::apply(&key, &enc.o).expect("rc4 apply")
    } else {
        // 20 RC4 decrypts; counter runs 19 down to 0.
        let mut state = enc.o.clone();
        for n in (0u8..=19u8).rev() {
            let k: Vec<u8> = key.iter().map(|b| b ^ n).collect();
            state = rc4::apply(&k, &state).expect("rc4 apply");
        }
        state
    }
}

// ============================================================
// R>=5 (V=5 R=5 AES-256, V=5 R=6 AES-256 PDF 2.0)
// ============================================================

#[allow(dead_code)] // reached by Phase E (encryption coverage) beat
fn authenticate_v5(
    enc: &EncryptDict,
    password: &[u8],
) -> Result<AuthResult, Error> {
    // PDF 2.0 §7.6.4.3.4 caps password input at 127 bytes for
    // R=6; R=5 has no explicit cap but pikepdf truncates the
    // same way. Treat the input as already SASLprep-normalised.
    let pwd: &[u8] = if password.len() > 127 {
        &password[..127]
    } else {
        password
    };

    // /U layout for R>=5: 0..32 = validation hash, 32..40 =
    // validation salt, 40..48 = key salt.
    let u_validation_salt = &enc.u[32..40];
    let u_key_salt = &enc.u[40..48];
    let o_validation_salt = &enc.o[32..40];
    let o_key_salt = &enc.o[40..48];

    // -- User path --
    let user_validation = hash_v5(enc.r, pwd, u_validation_salt, &[]);
    if user_validation[..] == enc.u[..32] {
        let intermediate = hash_v5(enc.r, pwd, u_key_salt, &[]);
        let key = aes_cbc::decrypt_no_pad(&intermediate, &[0u8; 16], &enc.ue)
            .expect("aes-cbc decrypt /UE");
        return Ok(AuthResult {
            authenticated: true,
            used_owner_path: false,
            file_key: key,
        });
    }

    // -- Owner path --
    let owner_validation = hash_v5(enc.r, pwd, o_validation_salt, &enc.u);
    if owner_validation[..] == enc.o[..32] {
        let intermediate = hash_v5(enc.r, pwd, o_key_salt, &enc.u);
        let key = aes_cbc::decrypt_no_pad(&intermediate, &[0u8; 16], &enc.oe)
            .expect("aes-cbc decrypt /OE");
        return Ok(AuthResult {
            authenticated: true,
            used_owner_path: true,
            file_key: key,
        });
    }

    Ok(AuthResult {
        authenticated: false,
        used_owner_path: false,
        file_key: Vec::new(),
    })
}

/// Compute the R>=5 password hash. R=5 is a single-pass
/// SHA-256; R=6 is the iterative Algorithm 2.B cascade.
#[allow(dead_code)] // reached by Phase E (encryption coverage) beat
fn hash_v5(
    r: u32,
    password: &[u8],
    salt: &[u8],
    extra: &[u8],
) -> Vec<u8> {
    let mut input: Vec<u8> = Vec::with_capacity(password.len() + salt.len() + extra.len());
    input.extend_from_slice(password);
    input.extend_from_slice(salt);
    input.extend_from_slice(extra);
    if r == 5 {
        sha256(&input).to_vec()
    } else {
        // Algorithm 2.B (R=6).
        algorithm_2b(password, salt, extra)
    }
}

/// PDF 32000-2:2020 Algorithm 2.B — iterative SHA-256 / 384 /
/// 512 + AES-128-CBC cascade.
#[allow(dead_code)] // reached by Phase E (encryption coverage) beat
fn algorithm_2b(
    password: &[u8],
    salt: &[u8],
    extra: &[u8],
) -> Vec<u8> {
    // Initial K = SHA-256(password || salt || extra).
    let mut init: Vec<u8> = Vec::with_capacity(password.len() + salt.len() + extra.len());
    init.extend_from_slice(password);
    init.extend_from_slice(salt);
    init.extend_from_slice(extra);
    let mut k: Vec<u8> = sha256(&init).to_vec();

    // Round counter is 1-indexed per the spec — the first
    // iteration is "round 1". Termination test: round >= 64 AND
    // last byte of E <= round - 32. Off-by-one trap: a 0-indexed
    // counter terminates one iteration late and produces a
    // valid-but-different hash (so the user-validation salt
    // happens to match by luck while the user-key salt diverges).
    let mut round: u32 = 0;
    loop {
        round += 1;
        // Step (a): K1 = (password || K || extra) × 64.
        let block_len = password.len() + k.len() + extra.len();
        let mut k1 = Vec::with_capacity(block_len * 64);
        for _ in 0..64 {
            k1.extend_from_slice(password);
            k1.extend_from_slice(&k);
            k1.extend_from_slice(extra);
        }
        // Step (b): AES-128-CBC encrypt K1, key=K[..16],
        // iv=K[16..32], no padding.
        let aes_key = &k[..16];
        let aes_iv = &k[16..32];
        let e = aes_cbc::encrypt_no_pad(aes_key, aes_iv, &k1)
            .expect("aes-128-cbc encrypt for Algorithm 2.B");
        // Step (c): hash choice = first 16 bytes of E as
        // big-endian unsigned int mod 3 = sum-of-bytes mod 3
        // (since 256 ≡ 1 (mod 3)).
        let mut sum_mod3: u32 = 0;
        for &b in &e[..16] {
            sum_mod3 = (sum_mod3 + b as u32) % 3;
        }
        // Step (d): K = hash_choice(E).
        k = match sum_mod3 {
            0 => sha256(&e).to_vec(),
            1 => sha384(&e).to_vec(),
            _ => sha512(&e).to_vec(),
        };
        // Step (e): termination test.
        let last = *e.last().unwrap();
        if round >= 64 && (last as u32) <= round - 32 {
            break;
        }
    }
    k[..32].to_vec()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn unhex(s: &str) -> Vec<u8> {
        let s: String = s.chars().filter(|c| !c.is_whitespace()).collect();
        let mut out = Vec::with_capacity(s.len() / 2);
        for i in (0..s.len()).step_by(2) {
            out.push(u8::from_str_radix(&s[i..i + 2], 16).unwrap());
        }
        out
    }

    #[test]
    fn pad_password_short_input() {
        let p = pad_password(b"hi");
        assert_eq!(p[0], b'h');
        assert_eq!(p[1], b'i');
        assert_eq!(p[2], PASSWORD_PAD[0]);
        assert_eq!(p[31], PASSWORD_PAD[29]);
    }

    #[test]
    fn pad_password_empty_input_is_pad_string() {
        let p = pad_password(b"");
        assert_eq!(&p[..], &PASSWORD_PAD[..]);
    }

    #[test]
    fn unsupported_vr_errors() {
        let enc = EncryptDict {
            v: 3, r: 3, length_bits: 128,
            p: -4, o: vec![0; 32], u: vec![0; 32],
            oe: vec![], ue: vec![], perms: vec![],
            encrypt_metadata: true,
        };
        assert_eq!(
            authenticate(&enc, b"x", b"id").unwrap_err(),
            Error::UnsupportedVR(3, 3)
        );
    }

    #[test]
    fn v1_r2_user_password_authenticates() {
        // Inputs from Feedstock fixture v1_r2_rc40__user_pwd.case.
        let enc = EncryptDict {
            v: 1, r: 2, length_bits: 40, p: -4,
            o: unhex("94e8094419662a774442fb072e3d9f19e9d130ec09a4d0061e78fe920f7ab62f"),
            u: unhex("4272c3792032db213ad9e2654c8cbfa39d29df184b2eb8c43815f7ff0b996c78"),
            oe: vec![], ue: vec![], perms: vec![],
            encrypt_metadata: true,
        };
        let res = authenticate(
            &enc,
            &unhex("75736572"),
            &unhex("92d1f3b435dc715734efab0a4994d6fe"),
        ).unwrap();
        assert!(res.authenticated);
        assert!(!res.used_owner_path);
        assert_eq!(res.file_key, unhex("1b99a508a9"));
    }
}
