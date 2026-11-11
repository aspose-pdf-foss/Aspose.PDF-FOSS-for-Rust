//! Public-API smoke driver — `RedactionAnnotation::redact`.
//!
//! Ports the cpp redaction_apply_smoke test: a redaction must genuinely
//! REMOVE the underlying text (not merely cover it). After `redact` +
//! save the redacted text is no longer extractable; text outside the
//! region survives; a `Document`-bound annotation (no page) is a safe
//! no-op.

use aspose_pdf_foss::{Document, Rectangle, RedactionAnnotation, TextAbsorber, TextFragmentAbsorber};
use std::path::{Path, PathBuf};

fn hello_world() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/text_extractor/hello_world.pdf")
}

/// The rectangle of the first text fragment on page 1 (match-all).
fn first_text_box(doc: &Document) -> Rectangle {
    let mut finder = TextFragmentAbsorber::new(); // match-all
    let page = doc.pages().get(1).unwrap();
    finder.visit_page(&page).expect("visit page");
    finder.text_fragments().get(1).unwrap().rectangle()
}

/// Extracted text of a saved PDF (reopened from bytes).
fn extracted_text(bytes: Vec<u8>) -> String {
    let doc = Document::from_bytes(bytes).expect("reopen");
    let mut a = TextAbsorber::new();
    a.visit_document(&doc).expect("visit");
    a.text().to_string()
}

#[test]
fn removes_text_in_region() {
    let mut out = Vec::new();
    {
        let doc = Document::open(hello_world()).unwrap();
        let box_ = first_text_box(&doc);
        let page = doc.pages().get(1).unwrap();
        let mut r = RedactionAnnotation::on_page(&page, box_);
        r.set_overlay_text("REDACTED".into());
        r.redact();
        doc.save_to_writer(&mut out).unwrap();
    }
    // The redacted text is no longer extractable.
    assert!(!extracted_text(out).contains("Hello"));
}

#[test]
fn keeps_text_outside_region() {
    let mut out = Vec::new();
    {
        let doc = Document::open(hello_world()).unwrap();
        let box_ = first_text_box(&doc);
        let page = doc.pages().get(1).unwrap();
        // A region well to the right of the text — no overlap.
        let away = Rectangle::new(box_.urx() + 100.0, box_.lly(), box_.urx() + 150.0, box_.ury(), true);
        let mut r = RedactionAnnotation::on_page(&page, away);
        r.redact();
        doc.save_to_writer(&mut out).unwrap();
    }
    assert!(extracted_text(out).contains("Hello"));
}

#[test]
fn document_bound_no_op() {
    let doc = Document::open(hello_world()).unwrap();
    let mut r = RedactionAnnotation::new(&doc);
    // No page binding — redact must not panic and must not change bytes.
    r.redact();
    let mut out = Vec::new();
    doc.save_to_writer(&mut out).unwrap();
    let src = std::fs::read(hello_world()).unwrap();
    assert_eq!(out, src);
}
