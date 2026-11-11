// Hand-authored gate driver — binding_roundtrip gate.
// Primitive: crypt_filter
// Freeze gate: for each `Feedstock/PDF/Fixtures/crypt_filter/*.bin`
// fixture X, asserts decode(encode(X)) == X under the wire-format
// protocol described in crypt_filter.yaml.
//
// crypt_filter composes rc4 / aes_cbc / digest::md5, so the
// integration-test crate pulls in all four sources via `#[path]`
// at the crate root. Inside `crypt_filter.rs` the cipher
// primitives are referenced as `super::rc4` / `super::aes_cbc` /
// `super::digest`, which resolves to either `crate::internal::*`
// (lib build) or `crate::*` (this test crate).

#[path = "../../src/internal/rc4.rs"]
mod rc4;

#[path = "../../src/internal/aes_cbc.rs"]
mod aes_cbc;

#[path = "../../src/internal/digest.rs"]
mod digest;

#[path = "../../src/internal/crypt_filter.rs"]
mod crypt_filter;

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
        .join("crypt_filter")
}

#[test]
fn binding_roundtrip_over_corpus() {
    let dir = corpus_dir();
    assert!(dir.is_dir(),
            "corpus dir not found: {} \
             (set SPECDEV_CORPUS_DIR or vendor tests/fixtures/crypt_filter/)",
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
        eprintln!("TRY {}", path.display());
        let input = fs::read(path).expect("read fixture");
        let encoded = crypt_filter::encode(&input)
            .unwrap_or_else(|e| panic!(
                "encode failed for {}: {}", path.display(), e));
        let decoded = crypt_filter::decode(&encoded)
            .unwrap_or_else(|e| panic!(
                "decode failed for {}: {}", path.display(), e));
        assert_eq!(
            decoded, input,
            "round-trip mismatch for {} (input len {}, encoded len {}, decoded len {})",
            path.display(), input.len(), encoded.len(), decoded.len());
        checked += 1;
    }
    eprintln!("crypt_filter binding_roundtrip: checked={} failed=0",
              checked);
}
