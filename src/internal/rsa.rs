//! RSA-2048 PKCS#1 v1.5 signature over a SHA-256 digest, using the
//! bundled [`pkcs7_keymat`](super::pkcs7_keymat) test key. From-scratch
//! (composes [`bignum`](super::bignum)); test key material only.

use super::bignum::{self, Big};
use super::pkcs7_keymat::{RSA_D, RSA_N};

/// The DER-encoded `DigestInfo` prefix for SHA-256 (RFC 8017 §9.2).
const SHA256_DIGEST_INFO_PREFIX: [u8; 19] = [
    0x30, 0x31, 0x30, 0x0d, 0x06, 0x09, 0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x04, 0x02, 0x01, 0x05,
    0x00, 0x04, 0x20,
];

/// RSA modulus length in bytes (2048-bit key).
const K: usize = 256;

/// Sign `digest` (a SHA-256 hash) with the bundled key, producing a
/// 256-byte PKCS#1 v1.5 signature.
pub(crate) fn sign_sha256(digest: &[u8; 32]) -> Vec<u8> {
    // EMSA-PKCS1-v1_5: 0x00 0x01 PS(0xFF…) 0x00 DigestInfo, total K bytes.
    let t_len = SHA256_DIGEST_INFO_PREFIX.len() + digest.len();
    let ps_len = K - 3 - t_len;
    let mut em = Vec::with_capacity(K);
    em.push(0x00);
    em.push(0x01);
    em.extend(std::iter::repeat_n(0xFF, ps_len));
    em.push(0x00);
    em.extend_from_slice(&SHA256_DIGEST_INFO_PREFIX);
    em.extend_from_slice(digest);

    let m = Big::from_be_bytes(&em);
    let n = Big::from_be_bytes(&RSA_N);
    let sig = bignum::modpow(&m, &RSA_D, &n);
    sig.to_be_bytes(K)
}

#[cfg(test)]
mod tests {
    use super::super::pkcs7_keymat::RSA_E;
    use super::*;

    #[test]
    fn sign_then_verify_with_public_key() {
        let digest = [0x42u8; 32];
        let sig = sign_sha256(&digest);
        assert_eq!(sig.len(), K);

        // Verify: sig^e mod n must reproduce the EMSA encoding.
        let s = Big::from_be_bytes(&sig);
        let n = Big::from_be_bytes(&RSA_N);
        let recovered = bignum::modpow(&s, &RSA_E.to_be_bytes(), &n).to_be_bytes(K);

        // Reconstruct the expected EMSA and compare.
        let t_len = SHA256_DIGEST_INFO_PREFIX.len() + digest.len();
        let ps_len = K - 3 - t_len;
        let mut em = vec![0x00, 0x01];
        em.extend(std::iter::repeat_n(0xFF, ps_len));
        em.push(0x00);
        em.extend_from_slice(&SHA256_DIGEST_INFO_PREFIX);
        em.extend_from_slice(&digest);
        assert_eq!(recovered, em);
    }
}
