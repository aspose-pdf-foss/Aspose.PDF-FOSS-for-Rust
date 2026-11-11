// Hand-authored gate driver — binding_roundtrip gate.
// Primitive: rc4
// Freeze gate: binding_roundtrip over `Feedstock/PDF/Fixtures/rc4/*.bin`.
//
// RC4 is symmetric — encode and decode call the same Apply
// underneath with the gate test key. The binding_roundtrip
// property `decode(encode(X)) == X` certifies that two
// back-to-back calls produce the same keystream (no leaked S-box
// state between calls).

#[path = "../../src/internal/rc4.rs"]
mod rc4;

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
        .join("rc4")
}

#[test]
fn binding_roundtrip_over_corpus() {
    let dir = corpus_dir();
    assert!(dir.is_dir(),
            "corpus dir not found: {} \
             (set SPECDEV_CORPUS_DIR or vendor tests/fixtures/rc4/)",
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
        let encoded = rc4::encode(&input);
        let decoded = rc4::decode(&encoded);
        assert_eq!(
            decoded, input,
            "round-trip mismatch for {}", path.display());
        checked += 1;
    }
    eprintln!("rc4 binding_roundtrip: checked={} failed=0",
              checked);
}
