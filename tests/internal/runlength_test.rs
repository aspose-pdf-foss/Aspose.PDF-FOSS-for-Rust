// Hand-authored gate driver — binding_roundtrip gate.
// Primitive: runlength
// Freeze gate: binding_roundtrip over
//   `Feedstock/PDF/Fixtures/runlength/*.bin` plus the
//   `boundary/` sub-corpus pinning spec-permissive Decode-only
//   inputs (streams ending without a 0x80 EOD marker — the
//   csharp/cpp divergence trigger that motivated the boundary
//   fixture convention; see feedback_self_roundtrip_blind_spot).
//
// Two test functions:
//   * `binding_roundtrip_over_corpus` — for each top-level fixture
//     X, asserts decode(encode(X)) == X.
//   * `decode_boundary_corpus` — for each pair (X.bin, X.expected)
//     under boundary/, asserts decode(X.bin) == X.expected.

#[path = "../../src/internal/runlength.rs"]
mod runlength;

use std::fs;
use std::path::{Path, PathBuf};

const FIXTURE_EXT: &str = "bin";

fn corpus_root() -> PathBuf {
    if let Ok(env_dir) = std::env::var("SPECDEV_CORPUS_DIR") {
        if !env_dir.is_empty() {
            return PathBuf::from(env_dir);
        }
    }
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("runlength")
}

fn collect_bin_fixtures(dir: &Path) -> Vec<PathBuf> {
    let mut fixtures: Vec<PathBuf> = fs::read_dir(dir)
        .expect("read corpus dir")
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.is_file())
        .filter(|p| p.extension()
                     .and_then(|s| s.to_str()) == Some(FIXTURE_EXT))
        .collect();
    fixtures.sort();
    fixtures
}

#[test]
fn binding_roundtrip_over_corpus() {
    let dir = corpus_root();
    assert!(dir.is_dir(),
            "corpus dir not found: {} \
             (set SPECDEV_CORPUS_DIR or vendor tests/fixtures/runlength/)",
            dir.display());

    let fixtures = collect_bin_fixtures(&dir);
    assert!(!fixtures.is_empty(),
            "no .{} fixtures under {}", FIXTURE_EXT, dir.display());

    let mut checked = 0_usize;
    for path in &fixtures {
        let input = fs::read(path).expect("read fixture");
        let encoded = runlength::encode(&input);
        let decoded = runlength::decode(&encoded)
            .unwrap_or_else(|e| panic!(
                "decode failed for {}: {}", path.display(), e));
        assert_eq!(
            decoded, input,
            "round-trip mismatch for {}", path.display());
        checked += 1;
    }
    eprintln!("runlength binding_roundtrip: checked={} failed=0",
              checked);
}

#[test]
fn decode_boundary_corpus() {
    // Boundary corpus pins spec-permissive Decode-only inputs:
    // complete segments terminated by end-of-input (no 0x80 EOD).
    // The encoder never emits these, so the main roundtrip gate
    // doesn't cover them. Without this gate cpp/csharp/rust could
    // silently diverge on the no-EOD case (cpp lenient, csharp
    // strict — the original 2026-05-02 incident).
    let dir = corpus_root().join("boundary");
    if !dir.is_dir() {
        eprintln!("runlength boundary corpus absent — skipping");
        return;
    }
    let fixtures = collect_bin_fixtures(&dir);
    assert!(!fixtures.is_empty(),
            "boundary corpus empty: {}", dir.display());

    let mut checked = 0_usize;
    for bin_path in &fixtures {
        let expected_path = bin_path.with_extension("expected");
        assert!(expected_path.is_file(),
                "missing .expected sidecar for {}", bin_path.display());
        let input = fs::read(bin_path).expect("read boundary input");
        let expected = fs::read(&expected_path)
            .expect("read boundary expected");
        let decoded = runlength::decode(&input)
            .unwrap_or_else(|e| panic!(
                "boundary decode failed for {}: {}",
                bin_path.display(), e));
        assert_eq!(
            decoded, expected,
            "boundary mismatch for {}", bin_path.display());
        checked += 1;
    }
    eprintln!("runlength decode_boundary: checked={} failed=0",
              checked);
}
