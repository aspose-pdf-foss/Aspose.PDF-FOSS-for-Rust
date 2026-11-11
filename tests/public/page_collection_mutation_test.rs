// Public-API smoke driver — PageCollection mutation (page-tree
// save-through). Ports the C++ page_collection_mutation_smoke_test.
//
// Each mutator rewrites the in-memory page list (and flags the
// collection dirty); Count() reflects the change immediately. On
// Save, the /Pages /Kids + /Count are rewritten and the new / copied
// leaf objects emitted as an incremental update, so the change
// persists across reopen.
//
// Coverage:
//   * AddBlankSaveReload — Add() appends a blank 612×792 page
//     (returned); count 1→2; save→reopen→count 2.
//   * InsertBlankAtFront — Insert(1) on a 2-page doc → count 3; the
//     original "Page one" is now page 2.
//   * DeletePageSaveReload — Delete(1) drops page 1; the remaining
//     page renders "Page two"; persists across save/reopen.
//   * CopyPageSharesContent — Add(src) appends a copy whose text
//     matches the source ("Page one").
//   * DeleteMultipleAndAll — Delete(&[1, 3]) removes those (1-based);
//     Delete() removes all → count 0.

use std::path::PathBuf;

use aspose_pdf_foss::{Document, TextAbsorber};

fn fixture(rel: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(rel)
}

fn hello_world() -> Document {
    Document::open(fixture("text_extractor/hello_world.pdf")).expect("open hello_world")
}

fn two_pages() -> Document {
    Document::open(fixture("text_extractor/two_pages.pdf")).expect("open two_pages")
}

fn save_reload(doc: &Document) -> Document {
    let mut buf = Vec::new();
    doc.save_to_writer(&mut buf).expect("save_to_writer");
    Document::from_bytes(buf).expect("reload")
}

fn page_text(doc: &Document, index: usize) -> String {
    let page = doc.pages().get(index).expect("page");
    let mut abs = TextAbsorber::new();
    abs.visit_page(&page).expect("visit_page");
    abs.text().to_string()
}

#[test]
fn add_blank_save_reload() {
    let doc = hello_world();
    assert_eq!(doc.pages().count(), 1);
    let added = doc.pages().add();
    assert_eq!(doc.pages().count(), 2);
    assert!((added.rect().width() - 612.0).abs() < 1e-6);
    assert!((added.rect().height() - 792.0).abs() < 1e-6);

    let re = save_reload(&doc);
    assert_eq!(re.pages().count(), 2);
}

#[test]
fn insert_blank_at_front() {
    let doc = two_pages();
    assert_eq!(doc.pages().count(), 2);
    doc.pages().insert(1);
    assert_eq!(doc.pages().count(), 3);
    // The new blank page is page 1; original page 1 ("Page one") is now
    // page 2 (its content still resolves through the source bytes).
    assert!(page_text(&doc, 2).contains("Page one"));
}

#[test]
fn delete_page_save_reload() {
    let doc = two_pages();
    assert_eq!(doc.pages().count(), 2);
    doc.pages().delete(1); // drop "Page one"
    assert_eq!(doc.pages().count(), 1);
    assert!(page_text(&doc, 1).contains("Page two"));

    let re = save_reload(&doc);
    assert_eq!(re.pages().count(), 1);
    assert!(page_text(&re, 1).contains("Page two"));
}

#[test]
fn copy_page_shares_content() {
    let doc = two_pages();
    let src = doc.pages().get(1).expect("page 1");
    let copy = doc.pages().add_page(&src); // append a copy of page 1
    assert_eq!(doc.pages().count(), 3);
    // The appended copy renders the same text as the source page.
    let mut abs = TextAbsorber::new();
    abs.visit_page(&copy).expect("visit_page");
    assert!(abs.text().contains("Page one"));
}

#[test]
fn delete_multiple_and_all() {
    let doc = two_pages();
    doc.pages().add();
    doc.pages().add();
    assert_eq!(doc.pages().count(), 4);
    doc.pages().delete_indexes(&[1, 3]);
    assert_eq!(doc.pages().count(), 2);
    doc.pages().delete_all(); // all
    assert_eq!(doc.pages().count(), 0);
}
