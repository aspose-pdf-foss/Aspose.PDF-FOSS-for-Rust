// RC4 stream cipher (RFC 6229; PDF 32000-1:2008 §7.6.4).
// Symmetric — Apply XORs the RC4 keystream with `data`, so the
// same call performs both encryption and decryption.
//
// PDF's standard security handler V=1 uses a 40-bit (5-byte) key
// and V=2 uses 5..=16 bytes; this module enforces that bound.
// Empty input is valid; any longer non-block-aligned length is
// also valid (RC4 is a stream cipher with no padding).

use std::fmt;

/// Errors returned by [`apply`] on a key whose length is outside
/// PDF's 5..=16 byte range.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// Key length not in `5..=16`. PDF §7.6.4: V=1 uses 5 bytes
    /// (40-bit), V=2 uses 5..=16 bytes (40..=128-bit). Other
    /// lengths are rejected at the API boundary.
    InvalidKeyLength { len: usize },
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::InvalidKeyLength { len } => write!(
                f, "rc4: invalid key length {len} (expected 5..=16)"),
        }
    }
}

impl std::error::Error for Error {}

const MIN_KEY_LEN: usize = 5;
const MAX_KEY_LEN: usize = 16;

/// Fixed test key for the gate wrappers — `b"rc4-gate-key"`
/// (12 bytes, in the PDF 5..=16 range). Only used by the
/// hand-authored cargo gate driver; PDF consumers call [`apply`]
/// directly with per-object derived keys (encrypt_parser's job).
#[allow(dead_code)] // reached by Phase E (encryption coverage) beat
const GATE_KEY: &[u8] = b"rc4-gate-key";

/// XOR `data` with the RC4 keystream derived from `key`. Since
/// RC4 is symmetric, calling this twice with the same key returns
/// the original input. Returns `Err(InvalidKeyLength)` if `key`
/// length is outside `5..=16`.
pub fn apply(key: &[u8], data: &[u8]) -> Result<Vec<u8>, Error> {
    if key.len() < MIN_KEY_LEN || key.len() > MAX_KEY_LEN {
        return Err(Error::InvalidKeyLength { len: key.len() });
    }
    // KSA — initialise S = identity, then key-shuffle once.
    let mut s = [0u8; 256];
    for (i, slot) in s.iter_mut().enumerate() {
        *slot = i as u8;
    }
    let keylen = key.len();
    let mut j: u8 = 0;
    for i in 0..256usize {
        j = j.wrapping_add(s[i]).wrapping_add(key[i % keylen]);
        s.swap(i, j as usize);
    }
    // PRGA — generate a keystream byte per data byte and XOR.
    let mut out = Vec::with_capacity(data.len());
    let mut i: u8 = 0;
    let mut j: u8 = 0;
    for &b in data {
        i = i.wrapping_add(1);
        j = j.wrapping_add(s[i as usize]);
        s.swap(i as usize, j as usize);
        let k = s[(s[i as usize].wrapping_add(s[j as usize])) as usize];
        out.push(b ^ k);
    }
    Ok(out)
}

/// Gate wrapper — `apply(GATE_KEY, data)`. Used by the
/// binding_roundtrip driver; identical to [`decode`] because RC4
/// is symmetric. Never fails (gate key is in-range by
/// construction).
#[allow(dead_code)] // reached by Phase E (encryption coverage) beat
pub fn encode(data: &[u8]) -> Vec<u8> {
    apply(GATE_KEY, data).expect("gate key is valid by construction")
}

/// Gate wrapper — bit-for-bit identical to [`encode`]; both call
/// `apply` with the same fixed test key.
#[allow(dead_code)] // reached by Phase E (encryption coverage) beat
pub fn decode(data: &[u8]) -> Vec<u8> {
    apply(GATE_KEY, data).expect("gate key is valid by construction")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn unhex(s: &str) -> Vec<u8> {
        let s = s.replace(' ', "");
        let mut out = Vec::with_capacity(s.len() / 2);
        let bytes = s.as_bytes();
        let mut i = 0;
        while i < bytes.len() {
            let hi = match bytes[i] {
                b'0'..=b'9' => bytes[i] - b'0',
                b'a'..=b'f' => bytes[i] - b'a' + 10,
                b'A'..=b'F' => bytes[i] - b'A' + 10,
                _ => panic!("bad hex"),
            };
            let lo = match bytes[i + 1] {
                b'0'..=b'9' => bytes[i + 1] - b'0',
                b'a'..=b'f' => bytes[i + 1] - b'a' + 10,
                b'A'..=b'F' => bytes[i + 1] - b'A' + 10,
                _ => panic!("bad hex"),
            };
            out.push((hi << 4) | lo);
            i += 2;
        }
        out
    }

    #[test]
    fn rejects_short_key() {
        assert_eq!(
            apply(&[0u8; 4], b"data").unwrap_err(),
            Error::InvalidKeyLength { len: 4 });
    }

    #[test]
    fn rejects_long_key() {
        assert_eq!(
            apply(&[0u8; 17], b"data").unwrap_err(),
            Error::InvalidKeyLength { len: 17 });
    }

    #[test]
    fn accepts_min_max_keys() {
        assert!(apply(&[0u8; 5], b"x").is_ok());
        assert!(apply(&[0u8; 16], b"x").is_ok());
    }

    #[test]
    fn empty_input_returns_empty() {
        assert_eq!(apply(b"AnyKey", b"").unwrap(), b"");
    }

    #[test]
    fn rfc6229_key_0102030405_keystream_first_16() {
        // RFC 6229 §2 — key 0x0102030405, first 16 keystream bytes.
        let key = [0x01, 0x02, 0x03, 0x04, 0x05];
        let zeros = vec![0u8; 16];
        let ks = apply(&key, &zeros).unwrap();
        assert_eq!(ks, unhex("b2 39 63 05 f0 3d c0 27 cc c3 52 4a 0a 11 18 a8"));
    }

    #[test]
    fn rfc6229_key_01020304050607_keystream_first_16() {
        // RFC 6229 §2 — key 0x01020304050607, first 16 keystream bytes.
        let key = [0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07];
        let zeros = vec![0u8; 16];
        let ks = apply(&key, &zeros).unwrap();
        assert_eq!(ks, unhex("29 3f 02 d4 7f 37 c9 b6 33 f2 af 52 85 fe b4 6b"));
    }

    #[test]
    fn rfc6229_key_0102030405060708_keystream_first_16() {
        // RFC 6229 §2 — key 0x0102030405060708, first 16 keystream bytes.
        let key = [0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08];
        let zeros = vec![0u8; 16];
        let ks = apply(&key, &zeros).unwrap();
        assert_eq!(ks, unhex("97 ab 8a 1b f0 af b9 61 32 f2 f6 72 58 da 15 a8"));
    }

    #[test]
    fn rfc6229_key_16_bytes_keystream_first_16() {
        // RFC 6229 §2 — key 0x0102030405060708090a0b0c0d0e0f10,
        // first 16 keystream bytes.
        let key: [u8; 16] = [
            0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08,
            0x09, 0x0a, 0x0b, 0x0c, 0x0d, 0x0e, 0x0f, 0x10,
        ];
        let zeros = vec![0u8; 16];
        let ks = apply(&key, &zeros).unwrap();
        assert_eq!(ks, unhex("9a c7 cc 9a 60 9d 1e f7 b2 93 28 99 cd e4 1b 97"));
    }

    #[test]
    fn symmetric_roundtrip() {
        let key = b"PDFstandardKey16"; // 16 bytes
        let pt = b"Round-trip me through RC4 twice and watch the bytes return.";
        let ct = apply(key, pt).unwrap();
        assert_ne!(ct, pt.to_vec());
        let recovered = apply(key, &ct).unwrap();
        assert_eq!(recovered, pt.to_vec());
    }

    #[test]
    fn back_to_back_calls_dont_leak_state() {
        // Two independent calls with the same key on the same
        // data must produce identical output. If the impl shared
        // state between calls (e.g. a static S box), the second
        // call would diverge.
        let key = b"abcde";
        let data = b"deterministic";
        assert_eq!(apply(key, data).unwrap(), apply(key, data).unwrap());
    }

    #[test]
    fn gate_wrappers_are_symmetric() {
        let pt = b"gate-driver-roundtrip";
        assert_eq!(decode(&encode(pt)), pt);
    }
}
