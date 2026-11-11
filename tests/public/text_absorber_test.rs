// Public-API smoke driver — TextAbsorber.
//
// Coverage:
//   * Default ctor produces an absorber with empty text and
//     has_errors=false.
//   * visit_document extracts the full document text matching the
//     canonical text_extractor oracle (encoding_winansi fixture).
//   * visit_document on hello_world / two_pages fixtures round-
//     trips against the per-page canonical files (joined without
//     separator — matches cpp + csharp + typescript peer bodies).
//   * visit_page extracts only the visited page; iterating pages
//     aggregates the same multi-page text as visit_document on a
//     two-page corpus.
//   * Successive visit calls aggregate (text appends).
//   * has_errors stays false on every successful visit.

use std::path::PathBuf;

use aspose_pdf_foss::{Document, TextAbsorber};

fn fixture(rel: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(rel)
}

fn load_canonical(rel: &str) -> String {
    let raw = std::fs::read_to_string(fixture(rel)).expect("read canonical");
    // Canonical sidecars terminate every multi-page join with a
    // single trailing newline (so text editors don't choke). Strip
    // it for absorber comparison.
    raw.trim_end_matches('\n').to_string()
}

// Absorber.visit_document concatenates per-page text WITHOUT a
// separator (matches cpp + csharp + typescript peer bodies). The
// canonical fixture format joins pages with \u{000C} (form feed)
// for grep-friendliness; strip those separators when comparing to
// absorber output.
fn absorber_expected(canonical: &str) -> String {
    canonical.replace('\u{000C}', "")
}

// ============================================================
// Surface
// ============================================================

#[test]
fn default_ctor_empty_text_and_no_errors() {
    let a = TextAbsorber::new();
    assert_eq!(a.text(), "");
    assert!(!a.has_errors());
}

#[test]
fn default_trait_matches_explicit_new() {
    let a: TextAbsorber = Default::default();
    assert_eq!(a.text(), "");
    assert!(!a.has_errors());
}

// ============================================================
// visit_document — full-corpus oracle parity
// ============================================================

#[test]
fn visit_document_hello_world_round_trips() {
    let doc = Document::open(fixture("text_extractor/hello_world.pdf"))
        .expect("open hello_world.pdf");
    let expected = load_canonical("text_extractor/hello_world.canonical");

    let mut a = TextAbsorber::new();
    a.visit_document(&doc).expect("visit");

    assert_eq!(a.text(), expected);
    assert!(!a.has_errors());
}

#[test]
fn visit_document_encoding_winansi_matches_oracle() {
    let doc = Document::open(fixture("text_extractor/encoding_winansi.pdf"))
        .expect("open encoding_winansi.pdf");
    let expected = load_canonical("text_extractor/encoding_winansi.canonical");

    let mut a = TextAbsorber::new();
    a.visit_document(&doc).expect("visit");

    assert_eq!(a.text(), expected);
    assert!(!a.has_errors());
}

#[test]
fn visit_document_two_pages_concatenates_in_order() {
    let doc = Document::open(fixture("text_extractor/two_pages.pdf"))
        .expect("open two_pages.pdf");
    let canonical = load_canonical("text_extractor/two_pages.canonical");

    let mut a = TextAbsorber::new();
    a.visit_document(&doc).expect("visit");

    assert_eq!(a.text(), absorber_expected(&canonical));
}

// ============================================================
// visit_page — single-page extraction
// ============================================================

#[test]
fn visit_page_iterating_pages_matches_visit_document() {
    let doc = Document::open(fixture("text_extractor/two_pages.pdf"))
        .expect("open two_pages.pdf");
    let canonical = load_canonical("text_extractor/two_pages.canonical");

    let mut a = TextAbsorber::new();
    for i in 1..=doc.pages().count() {
        let page = doc.pages().get(i).expect("page");
        a.visit_page(&page).expect("visit_page");
    }

    assert_eq!(a.text(), absorber_expected(&canonical));
}

#[test]
fn visit_page_single_page_is_prefix_of_full_document() {
    let doc = Document::open(fixture("text_extractor/two_pages.pdf"))
        .expect("open two_pages.pdf");

    let mut first = TextAbsorber::new();
    let p1 = doc.pages().get(1).expect("page 1");
    first.visit_page(&p1).expect("visit_page");
    let first_only = first.text().to_string();

    let mut full = TextAbsorber::new();
    full.visit_document(&doc).expect("visit_document");

    assert!(full.text().starts_with(&first_only));
    assert!(first_only.len() < full.text().len());
}

// ============================================================
// Aggregation — successive visits append
// ============================================================

#[test]
fn successive_visits_append_to_text() {
    let doc = Document::open(fixture("text_extractor/hello_world.pdf"))
        .expect("open hello_world.pdf");
    let expected = load_canonical("text_extractor/hello_world.canonical");

    let mut a = TextAbsorber::new();
    a.visit_document(&doc).expect("first visit");
    a.visit_document(&doc).expect("second visit");

    let doubled = format!("{expected}{expected}");
    assert_eq!(a.text(), doubled);
}
