// Public-API smoke driver — DocumentInfo accessors + save
// round-trip.
//
// Exercises:
//   * Typed getters on a populated /Info dict.
//   * Empty-string fallback on a doc without /Info.
//   * set_title → save_to → re-open → re-read round trip
//     (validates the incremental update path through
//      pdf_writer_incremental).
//   * DocumentInfo::add / remove / clear / clear_custom_data
//     in-memory semantics.
//   * NoInfoForUpdate rejection when set_title runs against
//     a doc with no /Info indirect.

use std::path::PathBuf;

use aspose_pdf_foss::{Document, DocumentInfo, Error};

fn fixture(rel: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(rel)
}

#[test]
fn typed_getters_on_with_info() {
    let doc = Document::open(fixture("metadata/with_info.pdf")).unwrap();
    let info = doc.info();
    assert_eq!(info.title(), "LibForge Test Document");
    assert_eq!(info.author(), "LibForge");
    assert_eq!(info.subject(), "DocumentInfo smoke fixture");
    assert_eq!(info.keywords(), "libforge test smoke metadata");
    assert_eq!(info.creator(), "Hand-authored by generate_fixtures.py");
    assert_eq!(info.producer(), "LibForge Fixtures");
    assert_eq!(info.creation_date(), "D:20260424000000+00'00'");
    assert!(info.mod_date().starts_with("D:"));
}

#[test]
fn empty_string_fallback_on_without_info() {
    let doc = Document::open(fixture("metadata/without_info.pdf")).unwrap();
    let info = doc.info();
    assert_eq!(info.title(), "");
    assert_eq!(info.author(), "");
    assert_eq!(info.subject(), "");
    assert_eq!(info.keywords(), "");
    assert_eq!(info.creator(), "");
    assert_eq!(info.producer(), "");
    assert_eq!(info.creation_date(), "");
    assert_eq!(info.mod_date(), "");
}

#[test]
fn set_title_save_round_trip() {
    let mut doc = Document::open(fixture("metadata/with_info.pdf")).unwrap();
    doc.set_title("Updated Title — réfléchi");

    let tmp = common::unique_temp("aspose_pdf_foss_set_title_round_trip.pdf");
    doc.save_to(&tmp)
        .unwrap_or_else(|e| panic!("save_to {}: {e}", tmp.display()));

    let reopened = Document::open(&tmp).expect("reopen saved doc");
    assert_eq!(reopened.info().title(), "Updated Title — réfléchi");
    // Other entries survive the incremental update verbatim.
    assert_eq!(reopened.info().author(), "LibForge");
    assert_eq!(reopened.info().producer(), "LibForge Fixtures");
    assert_eq!(reopened.pages().count(), 1);

    std::fs::remove_file(&tmp).ok();
}

#[test]
fn save_to_writer_clean_doc_is_verbatim() {
    let doc = Document::open(fixture("metadata/with_info.pdf")).unwrap();
    let original = std::fs::read(fixture("metadata/with_info.pdf")).unwrap();
    let mut buf = Vec::new();
    doc.save_to_writer(&mut buf).unwrap();
    assert_eq!(buf, original, "clean save_to_writer must be byte-verbatim");
}

#[test]
fn set_title_on_doc_without_info_rejects_on_save() {
    let mut doc = Document::open(fixture("metadata/without_info.pdf")).unwrap();
    doc.set_title("Should Not Persist");
    let mut buf = Vec::new();
    match doc.save_to_writer(&mut buf) {
        Err(Error::NoInfoForUpdate) => {}
        other => panic!("expected NoInfoForUpdate, got {other:?}"),
    }
}

#[test]
fn add_custom_key_and_remove() {
    let mut info = DocumentInfo::new();
    info.add("MyCustom", "alpha").unwrap();
    assert_eq!(info.entries().len(), 1);

    // Duplicate add errors.
    match info.add("MyCustom", "beta") {
        Err(Error::DuplicateInfoKey(k)) => assert_eq!(k, "MyCustom"),
        other => panic!("expected DuplicateInfoKey, got {other:?}"),
    }

    info.remove("MyCustom");
    assert_eq!(info.entries().len(), 0);
}

#[test]
fn clear_custom_data_keeps_predefined() {
    let mut info = DocumentInfo::new();
    info.set_title("T");
    info.set_author("A");
    info.add("Custom1", "x").unwrap();
    info.add("Custom2", "y").unwrap();
    assert_eq!(info.entries().len(), 4);

    info.clear_custom_data();
    assert_eq!(info.entries().len(), 2);
    assert_eq!(info.title(), "T");
    assert_eq!(info.author(), "A");
}

#[test]
fn is_predefined_key() {
    assert!(DocumentInfo::is_predefined_key("Title"));
    assert!(DocumentInfo::is_predefined_key("CreationDate"));
    assert!(!DocumentInfo::is_predefined_key("Random"));
}

#[test]
fn info_mut_visible_to_reader_but_not_persisted() {
    let mut doc = Document::open(fixture("metadata/with_info.pdf")).unwrap();
    doc.info_mut().set_title("In-memory only");
    assert_eq!(doc.info().title(), "In-memory only");

    // save without set_title → clean → verbatim, ignoring the
    // in-memory mutation.
    let mut buf = Vec::new();
    doc.save_to_writer(&mut buf).unwrap();
    let original = std::fs::read(fixture("metadata/with_info.pdf")).unwrap();
    assert_eq!(
        buf, original,
        "info_mut() mutations must not dirty the document"
    );
}

#[path = "../common/mod.rs"]
mod common;
