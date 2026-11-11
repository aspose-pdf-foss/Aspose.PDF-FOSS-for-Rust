// Hand-authored gate driver — two_oracle_agreement gate.
// Primitive: digest
// Freeze gate: each `<name>.bin` fixture has a paired
// `<name>.canonical` sidecar containing the 5-line text
// manifest produced by Python hashlib + openssl dgst (the two
// oracles, agreed byte-for-byte at fixture-authoring time).
// The gate parses each fixture, runs all five digests, formats
// via to_canonical, and compares to the sidecar exactly.

#[path = "../../src/internal/digest.rs"]
mod digest;

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
        .join("digest")
}

#[test]
fn two_oracle_agreement_over_corpus() {
    let dir = corpus_dir();
    assert!(dir.is_dir(),
            "corpus dir not found: {} \
             (set SPECDEV_CORPUS_DIR or vendor tests/fixtures/digest/)",
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
        let canon_path = path.with_extension("canonical");
        assert!(canon_path.is_file(),
                "missing canonical sidecar for {}", path.display());
        let input = fs::read(path).expect("read fixture");
        let want = fs::read_to_string(&canon_path)
            .expect("read canonical sidecar");
        let got = digest::to_canonical(&digest::parse(&input));
        assert_eq!(
            got, want,
            "canonical mismatch for {}\n  expected:\n{}  actual:\n{}",
            path.display(), want, got);
        checked += 1;
    }
    eprintln!("digest two_oracle_agreement: checked={} failed=0",
              checked);
}
