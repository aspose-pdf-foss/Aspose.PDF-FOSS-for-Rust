// PDF /Encrypt application layer for one indirect-object payload.
// Composes the rc4 / aes_cbc / digest::md5 foundation primitives
// to implement the three CFM dispatch paths (V2 / AESV2 / AESV3)
// per PDF 32000-1:2008 §7.6.2 Algorithm 1 + Algorithm 1a and PDF
// 32000-2:2020 §7.6.4.4. Per-object key derivation lives here;
// random-IV generation lives in the caller, so this primitive is
// fully deterministic given inputs.
//
// Cross-module imports use `super::*` so the same source file
// works in both the lib build (where siblings are
// `crate::internal::*`) and the integration-test build (where
// all four primitives are pulled in via `#[path]` at the test
// crate root, making them siblings under `crate::*`).

use std::fmt;

use super::aes_cbc;
use super::digest::md5;
use super::rc4;

/// Crypt Filter Method dispatch tag — same names PDF
/// /Encrypt's CF dictionary uses. /Identity is intentionally
/// absent; consumers must short-circuit it before reaching this
/// primitive.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Method {
    /// RC4 (/V=2). Variable file-key length 5..=16.
    #[allow(dead_code)] // reached by Phase E (encryption coverage) beat
    V2,
    /// AES-128-CBC (/V=4). 16-byte file key. Per-object key
    /// derivation via Algorithm 1a + 'sAlT' salt.
    AesV2,
    /// AES-256-CBC (/V=5). 32-byte file key. Algorithm 1 SKIPPED;
    /// the file key IS the per-object key (PDF 32000-2 §7.6.4.4).
    #[allow(dead_code)] // reached by Phase E (encryption coverage) beat
    AesV3,
}

impl Method {
    fn name(self) -> &'static str {
        match self {
            Method::V2 => "V2",
            Method::AesV2 => "AESV2",
            Method::AesV3 => "AESV3",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    InvalidKeyLength { method: Method, got: usize },
    #[allow(dead_code)] // reached by Phase E (encryption coverage) beat
    InvalidIvLength { method: Method, got: usize },
    CiphertextTooShort { method: Method, got: usize },
    /// Wire-format header (gate-only `encode` / `decode` path):
    /// fewer than 2 bytes (no method+key_len).
    #[allow(dead_code)] // reached by Phase E (encryption coverage) beat
    WireTooShort { got: usize },
    /// Wire-format header method byte not in {1, 2, 3}.
    #[allow(dead_code)] // reached by Phase E (encryption coverage) beat
    WireUnknownMethodId { id: u8 },
    /// Wire-format header truncated — declared key length pushes
    /// the fixed header past EOF.
    #[allow(dead_code)] // reached by Phase E (encryption coverage) beat
    WireHeaderTruncated { needed: usize, got: usize },
    /// Propagated cipher error from rc4::apply or aes_cbc::*.
    Cipher(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::InvalidKeyLength { method, got } => write!(
                f, "crypt_filter: {} fileKey length {} invalid",
                method.name(), got),
            Error::InvalidIvLength { method, got } => write!(
                f, "crypt_filter: {} iv must be 16 bytes, got {}",
                method.name(), got),
            Error::CiphertextTooShort { method, got } => write!(
                f, "crypt_filter: {} ciphertext must be >= 16 bytes (iv), got {}",
                method.name(), got),
            Error::WireTooShort { got } => write!(
                f, "crypt_filter: wire input too short for method+key_len ({} bytes)", got),
            Error::WireUnknownMethodId { id } => write!(
                f, "crypt_filter: unknown wire-format method id {}", id),
            Error::WireHeaderTruncated { needed, got } => write!(
                f, "crypt_filter: wire header truncated — need {} bytes, got {}",
                needed, got),
            Error::Cipher(msg) => write!(f, "crypt_filter: cipher error: {}", msg),
        }
    }
}

impl std::error::Error for Error {}

impl From<aes_cbc::Error> for Error {
    fn from(e: aes_cbc::Error) -> Self {
        Error::Cipher(e.to_string())
    }
}

impl From<rc4::Error> for Error {
    fn from(e: rc4::Error) -> Self {
        Error::Cipher(e.to_string())
    }
}

fn validate_key(method: Method, file_key: &[u8]) -> Result<(), Error> {
    match method {
        Method::V2 => {
            if file_key.len() < 5 || file_key.len() > 16 {
                return Err(Error::InvalidKeyLength {
                    method, got: file_key.len() });
            }
        }
        Method::AesV2 => {
            if file_key.len() != 16 {
                return Err(Error::InvalidKeyLength {
                    method, got: file_key.len() });
            }
        }
        Method::AesV3 => {
            if file_key.len() != 32 {
                return Err(Error::InvalidKeyLength {
                    method, got: file_key.len() });
            }
        }
    }
    Ok(())
}

/// Algorithm 1 (PDF 32000-1 §7.6.2) per-object key derivation
/// for V2 and AESV2. AESV3 SKIPS this routine — V=5 uses
/// fileKey directly per PDF 32000-2 §7.6.4.4.
fn derive_per_obj_key(
    method: Method,
    file_key: &[u8],
    obj_num: u32,
    gen_num: u32,
) -> Vec<u8> {
    debug_assert!(matches!(method, Method::V2 | Method::AesV2));
    let n = file_key.len();
    let wants_salt = matches!(method, Method::AesV2);
    let mut buf = Vec::with_capacity(n + 5 + if wants_salt { 4 } else { 0 });
    buf.extend_from_slice(file_key);
    // Low 3 bytes of obj_num, little-endian.
    buf.push(obj_num as u8);
    buf.push((obj_num >> 8) as u8);
    buf.push((obj_num >> 16) as u8);
    // Low 2 bytes of gen_num, little-endian.
    buf.push(gen_num as u8);
    buf.push((gen_num >> 8) as u8);
    // sAlT salt for AESV2 — exact spelling 0x73 0x41 0x6C 0x54
    // ('s', 'A', 'l', 'T') per PDF 32000-1 §7.6.2 Algorithm 1a.
    if wants_salt {
        buf.extend_from_slice(&[0x73, 0x41, 0x6C, 0x54]);
    }
    let h = md5(&buf);
    // Cap at 16 bytes (MD5 size) — for fileKey shorter than 11
    // bytes the per-object key is shorter than the MD5 output.
    let key_len = (n + 5).min(16);
    h[..key_len].to_vec()
}

#[allow(dead_code)] // reached by Phase E (encryption coverage) beat
pub fn encrypt(
    method: Method,
    file_key: &[u8],
    obj_num: u32,
    gen_num: u32,
    plaintext: &[u8],
    iv: &[u8],
) -> Result<Vec<u8>, Error> {
    validate_key(method, file_key)?;

    if matches!(method, Method::V2) {
        let per_obj_key = derive_per_obj_key(method, file_key, obj_num, gen_num);
        return Ok(rc4::apply(&per_obj_key, plaintext)?);
    }

    if iv.len() != 16 {
        return Err(Error::InvalidIvLength { method, got: iv.len() });
    }

    let cipher_key: Vec<u8> = match method {
        Method::AesV2 => derive_per_obj_key(method, file_key, obj_num, gen_num),
        Method::AesV3 => file_key.to_vec(),
        Method::V2 => unreachable!(),
    };

    let cipher = aes_cbc::encrypt(&cipher_key, iv, plaintext)?;
    let mut out = Vec::with_capacity(16 + cipher.len());
    out.extend_from_slice(iv);
    out.extend_from_slice(&cipher);
    Ok(out)
}

pub fn decrypt(
    method: Method,
    file_key: &[u8],
    obj_num: u32,
    gen_num: u32,
    ciphertext: &[u8],
) -> Result<Vec<u8>, Error> {
    validate_key(method, file_key)?;

    if matches!(method, Method::V2) {
        let per_obj_key = derive_per_obj_key(method, file_key, obj_num, gen_num);
        return Ok(rc4::apply(&per_obj_key, ciphertext)?);
    }

    if ciphertext.len() < 16 {
        return Err(Error::CiphertextTooShort { method, got: ciphertext.len() });
    }
    let iv = &ciphertext[..16];
    let cipher = &ciphertext[16..];
    let cipher_key: Vec<u8> = match method {
        Method::AesV2 => derive_per_obj_key(method, file_key, obj_num, gen_num),
        Method::AesV3 => file_key.to_vec(),
        Method::V2 => unreachable!(),
    };
    Ok(aes_cbc::decrypt(&cipher_key, iv, cipher)?)
}

// ---- Wire format for the binding_roundtrip gate ---------------
//
// (described in crypt_filter.yaml). Encode parses a header from
// the input bytes, runs encrypt on the trailing payload, and
// re-emits header || ciphertext. Decode is symmetric. This
// protocol exists ONLY for the gate driver; production callers
// invoke encrypt / decrypt directly with their own (method,
// file_key, obj_num, gen_num, iv) tuples.

#[allow(dead_code)] // reached by Phase E (encryption coverage) beat
const METHOD_V2_ID: u8 = 1;
#[allow(dead_code)] // reached by Phase E (encryption coverage) beat
const METHOD_AESV2_ID: u8 = 2;
#[allow(dead_code)] // reached by Phase E (encryption coverage) beat
const METHOD_AESV3_ID: u8 = 3;

#[allow(dead_code)] // reached by Phase E (encryption coverage) beat
fn method_from_id(id: u8) -> Result<Method, Error> {
    match id {
        METHOD_V2_ID => Ok(Method::V2),
        METHOD_AESV2_ID => Ok(Method::AesV2),
        METHOD_AESV3_ID => Ok(Method::AesV3),
        _ => Err(Error::WireUnknownMethodId { id }),
    }
}

#[allow(dead_code)] // reached by Phase E (encryption coverage) beat
struct WireHeader<'a> {
    method: Method,
    file_key: &'a [u8],
    obj_num: u32,
    gen_num: u32,
    iv: &'a [u8],
    payload_off: usize,
}

#[allow(dead_code)] // reached by Phase E (encryption coverage) beat
fn parse_wire(input: &[u8]) -> Result<WireHeader<'_>, Error> {
    if input.len() < 2 {
        return Err(Error::WireTooShort { got: input.len() });
    }
    let method = method_from_id(input[0])?;
    let key_len = input[1] as usize;
    let header_len = 2 + key_len + 4 + 4 + 16;
    if input.len() < header_len {
        return Err(Error::WireHeaderTruncated {
            needed: header_len, got: input.len() });
    }
    let file_key = &input[2..2 + key_len];
    let off = 2 + key_len;
    let obj_num = u32::from_le_bytes([
        input[off], input[off + 1], input[off + 2], input[off + 3]]);
    let gen_num = u32::from_le_bytes([
        input[off + 4], input[off + 5], input[off + 6], input[off + 7]]);
    let iv = &input[off + 8..off + 24];
    Ok(WireHeader {
        method, file_key, obj_num, gen_num, iv,
        payload_off: header_len,
    })
}

#[allow(dead_code)] // reached by Phase E (encryption coverage) beat
pub fn encode(input: &[u8]) -> Result<Vec<u8>, Error> {
    let h = parse_wire(input)?;
    let plaintext = &input[h.payload_off..];
    let cipher = encrypt(
        h.method, h.file_key, h.obj_num, h.gen_num, plaintext, h.iv)?;
    let mut out = Vec::with_capacity(h.payload_off + cipher.len());
    out.extend_from_slice(&input[..h.payload_off]);
    out.extend_from_slice(&cipher);
    Ok(out)
}

#[allow(dead_code)] // reached by Phase E (encryption coverage) beat
pub fn decode(input: &[u8]) -> Result<Vec<u8>, Error> {
    let h = parse_wire(input)?;
    let cipher = &input[h.payload_off..];
    let plaintext = decrypt(
        h.method, h.file_key, h.obj_num, h.gen_num, cipher)?;
    let mut out = Vec::with_capacity(h.payload_off + plaintext.len());
    out.extend_from_slice(&input[..h.payload_off]);
    out.extend_from_slice(&plaintext);
    Ok(out)
}

#[cfg(test)]
mod tests {
    // Inner unit tests use `super::*` (the crypt_filter module
    // itself) plus the sibling primitives via the same
    // `super::aes_cbc` / `super::rc4` paths the production code
    // uses. This means the inner tests Just Work in both the
    // lib build (super = crate::internal) and the integration
    // test build (super = test crate root with #[path]
    // siblings).
    use super::*;

    fn aes_iv() -> [u8; 16] { [0x55u8; 16] }

    #[test]
    fn validate_key_v2_range() {
        assert!(validate_key(Method::V2, &[0u8; 5]).is_ok());
        assert!(validate_key(Method::V2, &[0u8; 16]).is_ok());
        assert_eq!(
            validate_key(Method::V2, &[0u8; 4]).unwrap_err(),
            Error::InvalidKeyLength { method: Method::V2, got: 4 });
        assert_eq!(
            validate_key(Method::V2, &[0u8; 17]).unwrap_err(),
            Error::InvalidKeyLength { method: Method::V2, got: 17 });
    }

    #[test]
    fn v2_roundtrip_n10_with_nonzero_obj_gen() {
        let file_key = [0x01u8; 10];
        let pt = b"PDF 32000 V2 round-trip with non-zero obj/gen";
        let ct = encrypt(Method::V2, &file_key, 7, 3, pt, &[]).unwrap();
        let pt2 = decrypt(Method::V2, &file_key, 7, 3, &ct).unwrap();
        assert_eq!(pt2, pt);
    }

    #[test]
    fn aesv2_roundtrip_with_iv_prepend() {
        let file_key = [0x02u8; 16];
        let pt = b"AESV2 PDF /Encrypt object payload bytes";
        let iv = aes_iv();
        let ct = encrypt(Method::AesV2, &file_key, 12, 0, pt, &iv).unwrap();
        // Output begins with the IV.
        assert_eq!(&ct[..16], &iv);
        let pt2 = decrypt(Method::AesV2, &file_key, 12, 0, &ct).unwrap();
        assert_eq!(pt2, pt);
    }

    #[test]
    fn aesv3_skips_per_object_derivation() {
        let file_key = [0x03u8; 32];
        let pt = b"AESV3 - file key IS per-object key (PDF 32000-2)";
        let iv = aes_iv();
        // obj/gen are intentionally non-trivial; AESV3 ignores them.
        let ct = encrypt(Method::AesV3, &file_key, 99, 5, pt, &iv).unwrap();
        let pt2 = decrypt(Method::AesV3, &file_key, 99, 5, &ct).unwrap();
        assert_eq!(pt2, pt);
        // Sanity: changing obj_num between encrypt/decrypt must
        // not corrupt AESV3 (proves the file key is used directly).
        let pt3 = decrypt(Method::AesV3, &file_key, 1, 0, &ct).unwrap();
        assert_eq!(pt3, pt);
    }

    #[test]
    fn aesv2_obj_gen_matters_for_per_object_key() {
        let file_key = [0x04u8; 16];
        let pt = b"obj/gen mismatch must fail AESV2 decrypt";
        let iv = aes_iv();
        let ct = encrypt(Method::AesV2, &file_key, 5, 0, pt, &iv).unwrap();
        // Changing obj_num desyncs Algorithm 1a; padding fails.
        assert!(decrypt(Method::AesV2, &file_key, 6, 0, &ct).is_err());
    }

    #[test]
    fn v2_obj_gen_matters_for_per_object_key() {
        let file_key = [0x05u8; 16];
        let pt = b"obj/gen desync produces RC4 garbage";
        let ct = encrypt(Method::V2, &file_key, 5, 0, pt, &[]).unwrap();
        let bad = decrypt(Method::V2, &file_key, 5, 1, &ct).unwrap();
        assert_ne!(bad, pt);
        // RC4 is symmetric so the right gen recovers plaintext.
        let good = decrypt(Method::V2, &file_key, 5, 0, &ct).unwrap();
        assert_eq!(good, pt);
    }

    #[test]
    fn wire_encode_decode_roundtrip_v2() {
        // Build a wire input: method=1, key_len=10, 10 key bytes,
        // obj_num=7 LE, gen_num=3 LE, 16 zero IV bytes, payload.
        let mut wire = Vec::new();
        wire.push(METHOD_V2_ID);
        wire.push(10);
        wire.extend_from_slice(&[0xAAu8; 10]);
        wire.extend_from_slice(&7u32.to_le_bytes());
        wire.extend_from_slice(&3u32.to_le_bytes());
        wire.extend_from_slice(&[0u8; 16]);
        wire.extend_from_slice(b"hello world");
        let enc = encode(&wire).unwrap();
        let dec = decode(&enc).unwrap();
        assert_eq!(dec, wire);
    }

    #[test]
    fn wire_too_short_errors() {
        assert_eq!(encode(&[1]).unwrap_err(),
                   Error::WireTooShort { got: 1 });
    }

    #[test]
    fn wire_unknown_method_errors() {
        // Wire id=99, key_len=0 — header layout still parses, but
        // the method byte is rejected.
        let wire = vec![99u8, 0];
        assert_eq!(encode(&wire).unwrap_err(),
                   Error::WireUnknownMethodId { id: 99 });
    }
}
