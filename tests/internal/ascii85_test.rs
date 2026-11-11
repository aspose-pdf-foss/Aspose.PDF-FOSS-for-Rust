// Hand-authored gate driver — binding_roundtrip gate.
// Primitive: ascii85
// Freeze gate: binding_roundtrip over `Feedstock/PDF/Fixtures/ascii85/*.bin`.
//
// For each fixture blob X, asserts decode(encode(X)) == X. Mirrors
// the cpp / csharp / typescript peer gates; same property, same
// fixture corpus, hand-authored cargo integration test pending the
// SpecDev rust verifier (`_verify_primitive_rust`).

#[path = "../../src/internal/ascii85.rs"]
mod ascii85;

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
        .join("ascii85")
}

#[test]
fn binding_roundtrip_over_corpus() {
    let dir = corpus_dir();
    assert!(dir.is_dir(),
            "corpus dir not found: {} \
             (set SPECDEV_CORPUS_DIR or vendor tests/fixtures/ascii85/)",
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
        let encoded = ascii85::encode(&input);
        let decoded = ascii85::decode(&encoded)
            .unwrap_or_else(|e| panic!(
                "decode failed for {}: {}", path.display(), e));
        assert_eq!(
            decoded, input,
            "round-trip mismatch for {}", path.display());
        checked += 1;
    }
    eprintln!("ascii85 binding_roundtrip: checked={} failed=0",
              checked);
}
