//! From-scratch detached PKCS#7 / CMS `SignedData` (RFC 5652) over an
//! external content buffer, using the bundled RSA-2048 test signer. The
//! output is a DER blob suitable for a PDF signature dictionary's
//! `/Contents` (SubFilter `adbe.pkcs7.detached`) and verifies under
//! `openssl smime -verify -noverify`.
//!
//! No signed attributes: the signature is RSA-PKCS#1-v1.5 over
//! `SHA-256(content)` directly, so a verifier recomputes the content
//! digest and checks it against the embedded certificate's public key.

use super::digest::sha256;
use super::pkcs7_keymat::SIGNER_CERT_DER;
use super::rsa;

// ---- DER primitives --------------------------------------------------

fn der_len(n: usize) -> Vec<u8> {
    if n < 0x80 {
        vec![n as u8]
    } else {
        let mut be = Vec::new();
        let mut v = n;
        while v > 0 {
            be.insert(0, (v & 0xff) as u8);
            v >>= 8;
        }
        let mut out = vec![0x80 | be.len() as u8];
        out.extend(be);
        out
    }
}

fn tlv(tag: u8, content: &[u8]) -> Vec<u8> {
    let mut out = vec![tag];
    out.extend(der_len(content.len()));
    out.extend_from_slice(content);
    out
}

fn seq(content: &[u8]) -> Vec<u8> {
    tlv(0x30, content)
}
fn set(content: &[u8]) -> Vec<u8> {
    tlv(0x31, content)
}
fn integer(bytes: &[u8]) -> Vec<u8> {
    // Strip leading zeros, then re-add one if the high bit is set
    // (DER INTEGER is signed).
    let mut b = bytes;
    while b.len() > 1 && b[0] == 0 {
        b = &b[1..];
    }
    if !b.is_empty() && b[0] & 0x80 != 0 {
        let mut v = vec![0u8];
        v.extend_from_slice(b);
        tlv(0x02, &v)
    } else {
        tlv(0x02, b)
    }
}
fn octet_string(content: &[u8]) -> Vec<u8> {
    tlv(0x04, content)
}
fn oid(value: &[u8]) -> Vec<u8> {
    tlv(0x06, value)
}
fn explicit(tag_no: u8, content: &[u8]) -> Vec<u8> {
    tlv(0xA0 | tag_no, content)
}
const NULL: [u8; 2] = [0x05, 0x00];

// Object identifiers (value bytes only).
const OID_SIGNED_DATA: [u8; 9] = [0x2a, 0x86, 0x48, 0x86, 0xf7, 0x0d, 0x01, 0x07, 0x02];
const OID_DATA: [u8; 9] = [0x2a, 0x86, 0x48, 0x86, 0xf7, 0x0d, 0x01, 0x07, 0x01];
const OID_SHA256: [u8; 9] = [0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x04, 0x02, 0x01];
const OID_RSA: [u8; 9] = [0x2a, 0x86, 0x48, 0x86, 0xf7, 0x0d, 0x01, 0x01, 0x01];

fn algid(oid_value: &[u8]) -> Vec<u8> {
    let mut c = oid(oid_value);
    c.extend_from_slice(&NULL);
    seq(&c)
}

// ---- Minimal TLV reader (cert issuer + serial extraction) ------------

/// Read one DER TLV at `pos`; return `(tag, content_start, content_end,
/// next_pos)`.
fn read_tlv(data: &[u8], pos: usize) -> Option<(u8, usize, usize, usize)> {
    if pos + 1 >= data.len() {
        return None;
    }
    let tag = data[pos];
    let l0 = data[pos + 1];
    let (len, hdr) = if l0 < 0x80 {
        (l0 as usize, 2)
    } else {
        let nb = (l0 & 0x7f) as usize;
        if pos + 2 + nb > data.len() {
            return None;
        }
        let mut len = 0usize;
        for i in 0..nb {
            len = (len << 8) | data[pos + 2 + i] as usize;
        }
        (len, 2 + nb)
    };
    let start = pos + hdr;
    let end = start + len;
    if end > data.len() {
        return None;
    }
    Some((tag, start, end, end))
}

/// The `IssuerAndSerialNumber` SEQUENCE built from the bundled cert's
/// issuer Name + serialNumber.
fn issuer_and_serial() -> Option<Vec<u8>> {
    let cert = &SIGNER_CERT_DER[..];
    // Certificate ::= SEQUENCE { tbsCertificate ::= SEQUENCE { ... } ... }
    let (_, tbs_start, _, _) = read_tlv(cert, 0)?; // outer SEQUENCE content
    let (_, fields_start, fields_end, _) = read_tlv(cert, tbs_start)?; // tbs SEQUENCE content
    let mut pos = fields_start;
    // Optional [0] version.
    let (tag, _, _, after) = read_tlv(cert, pos)?;
    if tag == 0xA0 {
        pos = after;
    }
    // serialNumber INTEGER (full TLV).
    let (_, _, _, after_serial) = read_tlv(cert, pos)?;
    let serial = cert[pos..after_serial].to_vec();
    pos = after_serial;
    // signature AlgorithmIdentifier SEQUENCE.
    let (_, _, _, after_sigalg) = read_tlv(cert, pos)?;
    pos = after_sigalg;
    // issuer Name (full SEQUENCE TLV).
    let (_, _, _, after_issuer) = read_tlv(cert, pos)?;
    if after_issuer > fields_end {
        return None;
    }
    let issuer = cert[pos..after_issuer].to_vec();

    let mut c = issuer;
    c.extend(serial);
    Some(seq(&c))
}

// ---- SignedData assembly --------------------------------------------

/// Build a detached PKCS#7 `SignedData` DER over `content`.
pub(crate) fn sign_detached(content: &[u8]) -> Vec<u8> {
    let digest = sha256(content);
    let signature = rsa::sign_sha256(&digest);
    let ias = issuer_and_serial().unwrap_or_default();

    // SignerInfo ::= SEQUENCE { version(1), sid, digestAlg, sigAlg, sig }
    let mut signer = Vec::new();
    signer.extend(integer(&[1]));
    signer.extend(ias);
    signer.extend(algid(&OID_SHA256));
    signer.extend(algid(&OID_RSA));
    signer.extend(octet_string(&signature));
    let signer_info = seq(&signer);

    // SignedData ::= SEQUENCE { version, digestAlgs SET, encapContentInfo,
    //                           [0] certificates, signerInfos SET }
    let mut sd = Vec::new();
    sd.extend(integer(&[1]));
    sd.extend(set(&algid(&OID_SHA256)));
    sd.extend(seq(&oid(&OID_DATA))); // detached: no eContent
    sd.extend(explicit(0, &SIGNER_CERT_DER)); // certificates [0]
    sd.extend(set(&signer_info));
    let signed_data = seq(&sd);

    // ContentInfo ::= SEQUENCE { OID signedData, [0] EXPLICIT SignedData }
    let mut ci = oid(&OID_SIGNED_DATA);
    ci.extend(explicit(0, &signed_data));
    seq(&ci)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn produces_der_sequence() {
        let der = sign_detached(b"hello world");
        assert_eq!(der[0], 0x30, "CMS starts with a SEQUENCE");
        assert!(der.len() > 800, "carries the embedded certificate");
    }
}

#[cfg(test)]
mod openssl_tests {
    use super::*;
    use std::io::Write;
    use std::process::Command;

    #[test]
    fn openssl_verifies_detached_signature() {
        if Command::new("sh").arg("-c").arg("command -v openssl").output().map(|o| !o.status.success()).unwrap_or(true) {
            eprintln!("openssl not available — skipping");
            return;
        }
        let content = b"The quick brown fox jumps over the lazy dog.";
        let der = sign_detached(content);

        let dir = std::env::temp_dir();
        let cpath = dir.join("aspose_pkcs7_content.bin");
        let dpath = dir.join("aspose_pkcs7_sig.der");
        std::fs::File::create(&cpath).unwrap().write_all(content).unwrap();
        std::fs::File::create(&dpath).unwrap().write_all(&der).unwrap();

        let cmd = format!(
            "openssl smime -verify -inform DER -in '{}' -content '{}' -noverify -out /dev/null 2>/dev/null",
            dpath.display(), cpath.display()
        );
        let status = Command::new("sh").arg("-c").arg(&cmd).status().unwrap();
        let _ = std::fs::remove_file(&cpath);
        let _ = std::fs::remove_file(&dpath);
        assert!(status.success(), "openssl failed to verify the detached PKCS#7");
    }
}
