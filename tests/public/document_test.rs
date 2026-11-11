// Public-API smoke driver — Document open + from_bytes.
//
// Exercises Document::open(path) and Document::from_bytes(bytes)
// over the vendored corpus, asserting page count and structural
// errors on malformed input. The full per-page geometry walk is
// covered by page_collection_test.rs; this driver focuses on the
// Document entry points themselves.

use std::path::PathBuf;

use aspose_pdf_foss::{Document, Error};

fn fixture(rel: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(rel)
}

#[test]
fn open_minimal_pdf() {
    let doc = Document::open(fixture("xref/minimal.pdf"))
        .expect("open xref/minimal.pdf");
    assert_eq!(doc.pages().count(), 1);
}

#[test]
fn open_pages_tree_corpus() {
    let cases = [
        ("pages_tree/single.pdf", 1usize),
        ("pages_tree/three_sizes.pdf", 3),
        ("pages_tree/nested_tree.pdf", 4),
    ];
    for (rel, expected_count) in cases {
        let doc = Document::open(fixture(rel))
            .unwrap_or_else(|e| panic!("open {rel}: {e}"));
        assert_eq!(
            doc.pages().count(),
            expected_count,
            "page count mismatch for {rel}"
        );
    }
}

#[test]
fn from_bytes_round_trip() {
    let bytes = std::fs::read(fixture("xref/minimal.pdf"))
        .expect("read xref/minimal.pdf");
    let doc = Document::from_bytes(bytes).expect("from_bytes");
    assert_eq!(doc.pages().count(), 1);
}

#[test]
fn open_missing_file_returns_io_error() {
    let err = Document::open(fixture("does_not_exist.pdf")).unwrap_err();
    match err {
        Error::Io(_) => {}
        other => panic!("expected Io error, got {other:?}"),
    }
}

#[test]
fn from_bytes_junk_returns_invalid_pdf() {
    let err = Document::from_bytes(b"not a pdf".to_vec()).unwrap_err();
    match err {
        Error::InvalidPdf(_) => {}
        other => panic!("expected InvalidPdf error, got {other:?}"),
    }
}

// ============================================================
// Empty-ctor — Document::new() (Phase 4 Beat 16)
// ============================================================

#[test]
fn new_yields_zero_page_document() {
    let doc = Document::new();
    assert_eq!(doc.pages().count(), 0);
}

#[test]
fn default_matches_new() {
    // `Default` is the Rust idiom for a no-arg ctor — must
    // produce a structurally identical document to
    // Document::new().
    let a = Document::new();
    let b = Document::default();
    assert_eq!(a.pages().count(), b.pages().count());
    assert_eq!(a.pages().count(), 0);
}

#[test]
fn new_save_without_path_returns_no_source_filename() {
    // The no-arg ctor synthesises bytes but tracks no source
    // filename — Document::save() (no-arg) must reject that
    // state with NoSourceFilename, matching the canonical
    // behaviour across peer tracks.
    let doc = Document::new();
    match doc.save() {
        Err(Error::NoSourceFilename) => {}
        Err(other) => panic!("expected NoSourceFilename, got {other:?}"),
        Ok(()) => panic!("Document::new().save() must error"),
    }
}

#[test]
fn new_save_to_writer_round_trips_through_from_bytes() {
    // Serialise the empty doc to a buffer and re-parse it —
    // must yield another 0-page document. This is the
    // structural round-trip; byte-identity across libs is not
    // guaranteed (peer trackers' BuildEmptyPdf headers all
    // call this out).
    let doc = Document::new();
    let mut buf: Vec<u8> = Vec::new();
    doc.save_to_writer(&mut buf).expect("save_to_writer");
    assert!(!buf.is_empty());

    let reparsed = Document::from_bytes(buf).expect("re-parse empty doc");
    assert_eq!(reparsed.pages().count(), 0);
}

#[test]
fn new_save_to_path_writes_parsable_empty_pdf() {
    let doc = Document::new();
    let tmp = common::unique_temp("aspose_pdf_foss_empty_doc.pdf");
    if tmp.exists() {
        std::fs::remove_file(&tmp).ok();
    }
    doc.save_to(&tmp).expect("save_to empty");

    let reopened = Document::open(&tmp).expect("re-open empty");
    assert_eq!(reopened.pages().count(), 0);

    std::fs::remove_file(&tmp).ok();
}

// ============================================================
// Viewer preferences — /PageLayout, /PageMode,
// /ViewerPreferences /Direction + /NonFullScreenPageMode
// ============================================================

#[test]
fn viewer_prefs_default_when_absent() {
    use aspose_pdf_foss::{Direction, PageLayout, PageMode, PrintDuplex, PrintScaling};
    let bytes = std::fs::read(fixture("xref/minimal.pdf")).expect("read minimal.pdf");
    let doc = Document::from_bytes(bytes).expect("from_bytes");
    assert_eq!(doc.direction(), Direction::L2R);
    assert_eq!(doc.page_mode(), PageMode::UseNone);
    assert_eq!(doc.non_full_screen_page_mode(), PageMode::UseNone);
    assert_eq!(doc.page_layout(), PageLayout::Default);
    assert_eq!(doc.duplex(), PrintDuplex::Simplex);
    assert_eq!(doc.print_scaling(), PrintScaling::AppDefault);
}

#[test]
fn viewer_prefs_round_trip_through_save() {
    use aspose_pdf_foss::{Direction, PageLayout, PageMode, PrintDuplex, PrintScaling};
    let bytes = std::fs::read(fixture("xref/minimal.pdf")).expect("read minimal.pdf");
    let mut doc = Document::from_bytes(bytes).expect("from_bytes");

    doc.set_direction(Direction::R2L);
    doc.set_page_mode(PageMode::UseThumbs);
    doc.set_non_full_screen_page_mode(PageMode::UseOutlines);
    doc.set_page_layout(PageLayout::TwoPageLeft);
    doc.set_duplex(PrintDuplex::DuplexFlipLongEdge);
    doc.set_print_scaling(PrintScaling::None);

    let mut out: Vec<u8> = Vec::new();
    doc.save_to_writer(&mut out).expect("save_to_writer");

    let reopened = Document::from_bytes(out).expect("reopen");
    assert_eq!(reopened.direction(), Direction::R2L);
    assert_eq!(reopened.page_mode(), PageMode::UseThumbs);
    assert_eq!(reopened.non_full_screen_page_mode(), PageMode::UseOutlines);
    assert_eq!(reopened.page_layout(), PageLayout::TwoPageLeft);
    assert_eq!(reopened.duplex(), PrintDuplex::DuplexFlipLongEdge);
    assert_eq!(reopened.print_scaling(), PrintScaling::None);
}

// ============================================================
// Document /ID file identifier
// ============================================================

#[test]
fn id_absent_yields_empty_identifiers() {
    let bytes = std::fs::read(fixture("xref/minimal.pdf")).expect("read minimal.pdf");
    let doc = Document::from_bytes(bytes).expect("from_bytes");
    let id = doc.id();
    assert_eq!(id.original(), "");
    assert_eq!(id.modified(), "");
}

#[test]
fn id_parses_trailer_id_array_as_hex() {
    // Splice an /ID array into the minimal PDF's trailer dict. The
    // trailer sits after the xref table, so object offsets and the
    // startxref value are unaffected.
    let bytes = std::fs::read(fixture("xref/minimal.pdf")).expect("read minimal.pdf");
    let needle: &[u8] = b"/Root 1 0 R >>";
    let pos = bytes
        .windows(needle.len())
        .position(|w| w == needle)
        .expect("trailer dict");
    let mut spliced = Vec::new();
    spliced.extend_from_slice(&bytes[..pos]);
    spliced.extend_from_slice(b"/Root 1 0 R /ID [<0011AB> <FFCD>] >>");
    spliced.extend_from_slice(&bytes[pos + needle.len()..]);
    let doc = Document::from_bytes(spliced).expect("from_bytes spliced");
    let id = doc.id();
    assert_eq!(id.original(), "0011AB");
    assert_eq!(id.modified(), "FFCD");
}

// ============================================================
// Document default page setup (PageInfo)
// ============================================================

#[test]
fn page_info_returns_a4_default() {
    let doc = Document::new();
    let pi = doc.page_info();
    assert_eq!(pi.width(), 595.0);
    assert_eq!(pi.height(), 842.0);
    assert!(!pi.is_landscape());
    assert_eq!(pi.pure_height(), 698.0);
}

#[path = "../common/mod.rs"]
mod common;
