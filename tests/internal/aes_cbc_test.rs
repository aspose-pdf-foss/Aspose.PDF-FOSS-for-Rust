// Hand-authored gate driver — binding_roundtrip gate.
// Primitive: aes_cbc
// Freeze gate: binding_roundtrip over `Feedstock/PDF/Fixtures/aes_cbc/*.bin`.
//
// The gate property `decode(encode(X)) == X` certifies the AES
// core (FIPS 197), the CBC chain (NIST SP 800-38A §6.2), and
// PKCS#7 padding (RFC 5652 §6.3) together — the corpus covers
// the byte-shape axes those three layers expose: empty input
// (full pad block), 1 byte (15 pad bytes), 15 bytes (1 pad
// byte), 16 bytes (full pad block — semantic trap #1), 17 bytes
// (two output blocks), 256 bytes, ~4 KiB.

#[path = "../../src/internal/aes_cbc.rs"]
mod aes_cbc;

use std::fs;
use std::path::PathBuf;

const FIXTURE_EXT: &str = "bin";

fn corpus_dir() -> PathBuf {
    if let Ok(env_dir) = std::env::var("SPECDEV_CORPUS_DIR") {
        if !env_dir.is_empty() {
            return PathBuf::from(env_dir);
        }
    }
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("aes_cbc")
}

#[test]
fn binding_roundtrip_over_corpus() {
    let dir = corpus_dir();
    assert!(dir.is_dir(),
            "corpus dir not found: {} \
             (set SPECDEV_CORPUS_DIR or vendor tests/fixtures/aes_cbc/)",
            dir.display());

    let mut fixtures: Vec<PathBuf> = fs::read_dir(&dir)
        .expect("read corpus dir")
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.is_file())
        .filter(|p| p.extension()
                     .and_then(|s| s.to_str()) == Some(FIXTURE_EXT))
        .collect();
    fixtures.sort();

    assert!(!fixtures.is_empty(),
            "no .{} fixtures under {}", FIXTURE_EXT, dir.display());

    let mut checked = 0_usize;
    for path in &fixtures {
        let input = fs::read(path).expect("read fixture");
        let encoded = aes_cbc::encode(&input);
        // PKCS#7 always emits at least one full pad block, so
        // the ciphertext is strictly longer than the plaintext.
        assert!(
            encoded.len() > input.len(),
            "ciphertext not longer than plaintext for {} \
             ({} <= {}) — likely a missing pad block",
            path.display(), encoded.len(), input.len());
        let decoded = aes_cbc::decode(&encoded);
        assert_eq!(
            decoded, input,
            "round-trip mismatch for {}", path.display());
        checked += 1;
    }
    eprintln!("aes_cbc binding_roundtrip: checked={} failed=0",
              checked);
}
