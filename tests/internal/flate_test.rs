// Hand-authored gate driver — binding_roundtrip gate.
// Primitive: flate
// Freeze gate: for each `Feedstock/PDF/Fixtures/flate/*.bin`
// fixture X, asserts decode(encode(X)) == X. Flate is lossless,
// so the round-trip must be byte-identical.

#[path = "../../src/internal/flate.rs"]
mod flate;

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
        .join("flate")
}

#[test]
fn binding_roundtrip_over_corpus() {
    let dir = corpus_dir();
    assert!(dir.is_dir(),
            "corpus dir not found: {} \
             (set SPECDEV_CORPUS_DIR or vendor tests/fixtures/flate/)",
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
        let encoded = flate::encode(&input);
        let decoded = flate::decode(&encoded)
            .unwrap_or_else(|e| panic!(
                "decode failed for {}: {}", path.display(), e));
        assert_eq!(
            decoded, input,
            "round-trip mismatch for {} (input len {}, encoded len {}, decoded len {})",
            path.display(), input.len(), encoded.len(), decoded.len());
        checked += 1;
    }
    eprintln!("flate binding_roundtrip: checked={} failed=0", checked);
}
