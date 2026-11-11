// Hand-authored gate driver — two_oracle_agreement gate.
// Primitive: xmp
// Freeze gate: for each `Feedstock/PDF/Fixtures/xmp/*.xmp`
// fixture X with sidecar X.canonical, asserts that
// to_canonical(parse(X)) == X.canonical byte-for-byte. The
// .canonical sidecar is the agreed output of python_xmp_toolkit
// and lxml (computed at fixture-authoring time); the gate only
// enforces equality against that sidecar so it stays
// reproducible per-target without invoking the oracles.

#[path = "../../src/internal/xmp.rs"]
mod xmp;

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
        .join("xmp")
}

#[test]
fn two_oracle_agreement_over_corpus() {
    let dir = corpus_dir();
    assert!(dir.is_dir(),
            "corpus dir not found: {} \
             (set SPECDEV_CORPUS_DIR or vendor tests/fixtures/xmp/)",
            dir.display());

    let mut fixtures: Vec<PathBuf> = fs::read_dir(&dir)
        .expect("read corpus dir")
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.is_file())
        .filter(|p| p.extension().and_then(|s| s.to_str()) == Some("xmp"))
        .collect();
    fixtures.sort();

    assert!(!fixtures.is_empty(),
            "no .xmp fixtures under {}", dir.display());

    let mut checked = 0_usize;
    for path in &fixtures {
        eprintln!("TRY {}", path.display());
        let canonical_path = path.with_extension("canonical");
        assert!(canonical_path.is_file(),
                "missing sidecar: {}", canonical_path.display());

        let input = fs::read(path).expect("read fixture");
        let expected = fs::read(&canonical_path).expect("read sidecar");

        let parsed = xmp::parse(&input)
            .unwrap_or_else(|e| panic!("parse failed for {}: {}",
                                       path.display(), e));
        let got = xmp::to_canonical(&parsed.properties);

        if got != expected {
            let got_str = String::from_utf8_lossy(&got);
            let exp_str = String::from_utf8_lossy(&expected);
            panic!(
                "canonical mismatch for {}:\n--- expected ---\n{}\n--- got ---\n{}",
                path.display(), exp_str, got_str);
        }
        checked += 1;
    }
    eprintln!("xmp two_oracle_agreement: checked={} failed=0", checked);
}
