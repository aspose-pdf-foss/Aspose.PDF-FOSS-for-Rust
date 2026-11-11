// Hand-authored gate driver — two_oracle_agreement gate.
// Primitive: objects
// Freeze gate: for each `Feedstock/PDF/Fixtures/objects/*.pdf`
// fixture X with sidecar X.canonical, the driver enumerates
// in-use xref entries, calls objects::parse_object on each
// (id, gen, offset), sorts by (id, gen), and concatenates the
// per-object canonical lines. The result must equal X.canonical.
//
// Composes the frozen lexer + flate + xref + objects.

#[path = "../../src/internal/lexer.rs"]
mod lexer;

#[path = "../../src/internal/flate.rs"]
mod flate;

#[path = "../../src/internal/xref.rs"]
mod xref;

#[path = "../../src/internal/objects.rs"]
mod objects;

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
        .join("objects")
}

#[test]
fn two_oracle_agreement_over_corpus() {
    let dir = corpus_dir();
    assert!(dir.is_dir(),
            "corpus dir not found: {} \
             (set SPECDEV_CORPUS_DIR or vendor tests/fixtures/objects/)",
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
        if !expected.ends_with(b"\n") {
            expected.push(b'\n');
        }

        // Use xref to enumerate in-use entries — fixtures are
        // guaranteed to use type-1 InUse only (no ObjStm).
        let table = xref::parse(&input)
            .unwrap_or_else(|e| panic!("xref parse failed for {}: {}",
                                       path.display(), e));
        let mut entries: Vec<(u32, u16, u64)> = table
            .entries
            .iter()
            .filter_map(|e| match e {
                xref::Entry::InUse { id, gen, offset } => Some((*id, *gen, *offset)),
                _ => None,
            })
            .collect();
        entries.sort_by_key(|(id, gen, _)| (*id, *gen));

        let mut got = Vec::new();
        for (_id, _gen, offset) in entries {
            let obj = objects::parse_object(&input, offset as usize)
                .unwrap_or_else(|e| panic!(
                    "parse_object failed for {} at offset {}: {}",
                    path.display(), offset, e));
            got.extend_from_slice(&objects::to_canonical(&obj));
        }

        if got != expected {
            let got_str = String::from_utf8_lossy(&got);
            let exp_str = String::from_utf8_lossy(&expected);
            panic!(
                "canonical mismatch for {}:\n--- expected ---\n{}\n--- got ---\n{}",
                path.display(), exp_str, got_str);
        }
        checked += 1;
    }
    eprintln!("objects two_oracle_agreement: checked={} failed=0", checked);
}
