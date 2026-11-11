// Hand-authored gate driver — two_oracle_agreement gate.
// Primitive: outlines
// Freeze gate: for each `Feedstock/PDF/Fixtures/outlines/*.pdf`
// fixture X with sidecar X.canonical, asserts that
// to_canonical(parse(X)) == X.canonical byte-for-byte. The
// .canonical sidecar is the agreed output of qpdf and mutool.
//
// outlines composes the frozen lexer + flate + xref + trailer +
// objects + encoding_tables + agl, so the integration-test
// crate pulls in all eight sources via `#[path]` at the crate
// root. Order matters only insofar as a mount must precede
// consumers.

#[path = "../../src/internal/lexer.rs"]
mod lexer;

#[path = "../../src/internal/flate.rs"]
mod flate;

#[path = "../../src/internal/xref.rs"]
mod xref;

#[path = "../../src/internal/trailer.rs"]
mod trailer;

#[path = "../../src/internal/objects.rs"]
mod objects;

#[path = "../../src/internal/encoding_tables.rs"]
mod encoding_tables;

#[path = "../../src/internal/agl.rs"]
mod agl;

#[path = "../../src/internal/outlines.rs"]
mod outlines;

use std::fs;
use std::path::PathBuf;

fn corpus_dir() -> PathBuf {
    if let Ok(env_dir) = std::env::var("SPECDEV_CORPUS_DIR") {
        if !env_dir.is_empty() {
            return PathBuf::from(env_dir);
        }
    }
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("outlines")
}

#[test]
fn two_oracle_agreement_over_corpus() {
    let dir = corpus_dir();
    assert!(dir.is_dir(),
            "corpus dir not found: {} \
             (set SPECDEV_CORPUS_DIR or vendor tests/fixtures/outlines/)",
            dir.display());

    let mut fixtures: Vec<PathBuf> = fs::read_dir(&dir)
        .expect("read corpus dir")
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.is_file())
        .filter(|p| p.extension().and_then(|s| s.to_str()) == Some("pdf"))
        .collect();
    fixtures.sort();

    assert!(!fixtures.is_empty(),
            "no .pdf fixtures under {}", dir.display());

    let mut checked = 0_usize;
    for path in &fixtures {
        eprintln!("TRY {}", path.display());
        let canonical_path = path.with_extension("canonical");
        assert!(canonical_path.is_file(),
                "missing sidecar: {}", canonical_path.display());

        let input = fs::read(path).expect("read fixture");
        let mut expected = fs::read(&canonical_path).expect("read sidecar");
        // The .canonical sidecars are emitted without a trailing
        // newline; to_canonical always terminates with one.
        // Normalise the expectation the same way pages_tree_test
        // does.
        if !expected.is_empty() && !expected.ends_with(b"\n") {
            expected.push(b'\n');
        }

        let parsed = outlines::parse(&input)
            .unwrap_or_else(|e| panic!("parse failed for {}: {}",
                                       path.display(), e));
        let got = outlines::to_canonical(&parsed);

        if got != expected {
            let got_str = String::from_utf8_lossy(&got);
            let exp_str = String::from_utf8_lossy(&expected);
            panic!(
                "canonical mismatch for {}:\n--- expected ---\n{}\n--- got ---\n{}",
                path.display(), exp_str, got_str);
        }
        checked += 1;
    }
    eprintln!("outlines two_oracle_agreement: checked={} failed=0", checked);
}
