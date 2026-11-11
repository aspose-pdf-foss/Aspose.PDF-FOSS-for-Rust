// Hand-authored gate driver — binding_roundtrip gate.
// Primitive: asciihex
// Freeze gate: binding_roundtrip over
//   `Feedstock/PDF/Fixtures/asciihex/*.bin` (vendored under
//   `tests/fixtures/asciihex/`).
//
// For each fixture blob X, the gate asserts
//     decode(encode(X)) == X
// — the binding-roundtrip property declared by the shared spec
// yaml. The cpp / csharp / typescript peers run this via SpecDev's
// emit-driver-from-template pipeline; the rust track hand-authors
// the same gate as a cargo integration test until SpecDev's rust
// verifier (`_verify_primitive_rust`) lands. Both shapes assert
// identical semantics — known LibForge gap, not a gate gap.
//
// NB: foundation modules are crate-private (`pub(crate)`); this
// integration test reaches them via a `#[path = …]` direct-include
// of the source file — same pattern peer Rust foundation libs use
// to drive crate-private modules from outside the crate boundary.

#[path = "../../src/internal/asciihex.rs"]
mod asciihex;

use std::fs;
use std::path::PathBuf;

const FIXTURE_EXT: &str = "bin";

fn corpus_dir() -> PathBuf {
    // SPECDEV_CORPUS_DIR (set by the verifier) wins; otherwise
    // fall back to the vendored snapshot under tests/fixtures/
    // so the gate runs out of the box for dev clones that don't
    // configure the env. Mirrors cpp / csharp / typescript peers.
    if let Ok(env_dir) = std::env::var("SPECDEV_CORPUS_DIR") {
        if !env_dir.is_empty() {
            return PathBuf::from(env_dir);
        }
    }
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("asciihex")
}

#[test]
fn binding_roundtrip_over_corpus() {
    let dir = corpus_dir();
    assert!(dir.is_dir(),
            "corpus dir not found: {} \
             (set SPECDEV_CORPUS_DIR or vendor tests/fixtures/asciihex/)",
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
        let encoded = asciihex::encode(&input);
        let decoded = asciihex::decode(&encoded)
            .unwrap_or_else(|e| panic!(
                "decode failed for {}: {}", path.display(), e));
        assert_eq!(
            decoded, input,
            "round-trip mismatch for {}", path.display());
        checked += 1;
    }
    eprintln!("asciihex binding_roundtrip: checked={} failed=0",
              checked);
}
