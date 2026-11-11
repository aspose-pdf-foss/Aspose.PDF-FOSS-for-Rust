// Corpus-wide round-trip gate.
//
// Every PDF in the fixture tree is opened, saved, and reopened. The
// per-feature tests each prove one path end to end; this one proves that
// no path in the corpus panics and that a save preserves what the parser
// read back.
//
// The corpus is deliberately mixed: alongside well-formed documents it
// carries fixtures built to exercise parser leniency (truncated xrefs,
// wrong `/Type`, junk before the header) and encrypted files whose
// password this gate does not supply. Those are expected to return an
// error, and an error is a pass — the gate's subject is that the library
// answers rather than aborts, on every byte sequence in the tree.

use aspose_pdf_foss::Document;
use std::path::{Path, PathBuf};

#[path = "../common/mod.rs"]
mod common;

fn fixtures_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

/// Every `.pdf` under the fixture tree, in a stable order so a failure
/// names the same file on every run.
fn corpus() -> Vec<PathBuf> {
    fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
        let Ok(entries) = std::fs::read_dir(dir) else { return };
        let mut paths: Vec<PathBuf> = entries.flatten().map(|e| e.path()).collect();
        paths.sort();
        for p in paths {
            if p.is_dir() {
                walk(&p, out);
            } else if p.extension().is_some_and(|e| e == "pdf") {
                out.push(p);
            }
        }
    }
    let mut out = Vec::new();
    walk(&fixtures_root(), &mut out);
    out
}

#[test]
fn corpus_is_discovered() {
    // Guards the walker itself: a gate that silently finds nothing would
    // pass forever.
    let files = corpus();
    assert!(
        files.len() >= 60,
        "expected the fixture corpus, found {} files",
        files.len()
    );
}

#[test]
fn every_fixture_opens_without_panicking() {
    let mut opened = 0usize;
    let mut refused = 0usize;
    for path in corpus() {
        match Document::open(&path) {
            Ok(_) => opened += 1,
            Err(_) => refused += 1,
        }
    }
    // Both outcomes are acceptable; reaching this line at all is the
    // assertion. The counts are asserted loosely so the gate does not
    // become a tripwire for every new fixture.
    assert!(opened > 0, "no fixture opened at all");
    assert_eq!(opened + refused, corpus().len());
}

#[test]
fn saving_and_reopening_preserves_the_page_count() {
    let mut checked = 0usize;
    for path in corpus() {
        let Ok(doc) = Document::open(&path) else { continue };
        let before = doc.pages().count();

        let out = common::unique_temp(format!(
            "aspose_rust_rt_{}",
            path.file_name().unwrap().to_string_lossy()
        ));
        let saved = doc.save_to(&out);
        assert!(saved.is_ok(), "save failed for {}: {saved:?}", path.display());

        let reopened = Document::open(&out);
        assert!(
            reopened.is_ok(),
            "a document this library wrote did not reopen: {} ({reopened:?})",
            path.display()
        );
        assert_eq!(
            reopened.unwrap().pages().count(),
            before,
            "page count changed across save/reopen for {}",
            path.display()
        );
        let _ = std::fs::remove_file(&out);
        checked += 1;
    }
    assert!(checked > 0, "no fixture reached the round trip");
}

#[test]
fn reopened_bytes_round_trip_a_second_time() {
    // A single save can hide a defect that only shows when the library
    // reads its own output and writes it again — an incremental update
    // appended onto an incremental update.
    for path in corpus() {
        let Ok(doc) = Document::open(&path) else { continue };
        let first = common::unique_temp(format!(
            "aspose_rust_rt1_{}",
            path.file_name().unwrap().to_string_lossy()
        ));
        if doc.save_to(&first).is_err() {
            continue;
        }
        let Ok(again) = Document::open(&first) else {
            panic!("first save did not reopen: {}", path.display())
        };
        let second = common::unique_temp(format!(
            "aspose_rust_rt2_{}",
            path.file_name().unwrap().to_string_lossy()
        ));
        assert!(
            again.save_to(&second).is_ok(),
            "second save failed for {}",
            path.display()
        );
        let twice = Document::open(&second);
        assert!(
            twice.is_ok(),
            "second save did not reopen: {} ({twice:?})",
            path.display()
        );
        assert_eq!(
            twice.unwrap().pages().count(),
            again.pages().count(),
            "page count changed on the second round trip for {}",
            path.display()
        );
        let _ = std::fs::remove_file(&first);
        let _ = std::fs::remove_file(&second);
    }
}
