// Public-API smoke driver — Facades file family: PdfFileInfo + PdfFileEditor.
//
// Ported from the cpp facades_pdf_file_info / facades_pdf_file_editor /
// facades_pdf_file_editor_ops smoke tests.
//
// PdfFileInfo: PasswordType enum values, unbound defaults, bind-by-file
// metadata (page count + version + page geometry), bind-by-document
// /Info round-trip, generic get/set meta info, ClearInfo, the
// strict-validation flag, and the SaveNewInfo wire-through.
//
// PdfFileEditor: the real file ops backed by the page-tree assembler —
// concatenate (pair + many), extract range, delete, append range, and
// the split forms — each verified by reopening the output and counting
// pages / asserting extracted text.
//
// Scenarios from the cpp tests that the Rust lib does not implement are
// omitted (see the returned notes): the BindPdf/Close binding cycle, the
// PdfFileEditor stub properties + nested types + ConcatenateCorruptedFileAction
// + PdfFormat enum, and SplitToPages / SplitToBulks / MakeBooklet / MakeNUp /
// ResizeContentsPct / AddMargins layout ops.

use aspose_pdf_foss::{Document, PasswordType, PdfFileEditor, PdfFileInfo, TextAbsorber};
use std::path::{Path, PathBuf};

fn hello_world() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/text_extractor/hello_world.pdf")
}
fn two_pages() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/text_extractor/two_pages.pdf")
}
fn tmp(name: &str) -> PathBuf {
    common::unique_temp(name)
}
fn doc_text(path: &Path) -> String {
    let doc = Document::open(path).unwrap();
    let mut abs = TextAbsorber::new();
    abs.visit_document(&doc).unwrap();
    abs.text().to_string()
}

// ---------------------------------------------------------------------------
// PdfFileInfo
// ---------------------------------------------------------------------------

#[test]
fn password_type_enum_values() {
    assert_eq!(PasswordType::None as i32, 0);
    assert_eq!(PasswordType::User as i32, 1);
    assert_eq!(PasswordType::Owner as i32, 2);
    assert_eq!(PasswordType::Inaccessible as i32, 3);
}

#[test]
fn file_info_defaults_when_unbound() {
    let info = PdfFileInfo::new();
    assert!(!info.is_pdf_file());
    assert!(!info.is_encrypted());
    assert_eq!(info.number_of_pages(), 0);
    assert_eq!(info.password_type(), PasswordType::None);
    assert!(!info.has_open_password());
    assert!(!info.has_edit_password());
    assert!(!info.has_collection());
    assert!(!info.use_strict_validation());
}

#[test]
fn file_info_bind_by_file_loads_metadata() {
    let info = PdfFileInfo::with_file(hello_world()).unwrap();
    assert!(info.is_pdf_file());
    assert_eq!(info.input_file(), hello_world().to_string_lossy());
    assert!(info.number_of_pages() > 0);
    assert_eq!(info.get_pdf_version(), "1.4");

    // Page geometry: canonical US-Letter defaults, no rotation.
    assert_eq!(info.get_page_width(1), 612.0);
    assert_eq!(info.get_page_height(1), 792.0);
    assert_eq!(info.get_page_rotation(1), 0.0);
}

#[test]
fn file_info_bind_by_document_info_round_trip() {
    let mut info = PdfFileInfo::with_document(Document::open(hello_world()).unwrap());

    info.set_title("Hello PDF");
    info.set_author("Jane Doe");
    info.set_subject("Greeting");
    info.set_keywords("hello,pdf,test");
    info.set_creator("LibForge rust smoke");
    info.set_creation_date("D:20260530000000Z");
    info.set_mod_date("D:20260530000000Z");

    assert_eq!(info.title(), "Hello PDF");
    assert_eq!(info.author(), "Jane Doe");
    assert_eq!(info.subject(), "Greeting");
    assert_eq!(info.keywords(), "hello,pdf,test");
    assert_eq!(info.creator(), "LibForge rust smoke");
    assert_eq!(info.creation_date(), "D:20260530000000Z");
    assert_eq!(info.mod_date(), "D:20260530000000Z");
}

#[test]
fn file_info_generic_get_set_meta_info() {
    let mut info = PdfFileInfo::with_document(Document::open(hello_world()).unwrap());
    info.set_meta_info("CustomKey", "CustomValue");
    assert_eq!(info.get_meta_info("CustomKey"), "CustomValue");
    assert_eq!(info.get_meta_info("Missing"), "");
}

#[test]
fn file_info_save_new_info_writes_document() {
    let info = PdfFileInfo::with_file(hello_world()).unwrap();
    let out = tmp("aspose_rust_facade_fileinfo_save.pdf");
    assert!(info.save_new_info(&out).is_ok());
    assert!(out.exists());
    let _ = std::fs::remove_file(&out);
}

#[test]
fn file_info_clear_info_resets_title() {
    let mut info = PdfFileInfo::with_document(Document::open(hello_world()).unwrap());
    info.set_title("Will be cleared");
    assert_eq!(info.title(), "Will be cleared");
    info.clear_info();
    assert!(info.title().is_empty());
}

#[test]
fn file_info_use_strict_validation_flag() {
    let mut info = PdfFileInfo::new();
    assert!(!info.use_strict_validation());
    info.set_use_strict_validation(true);
    assert!(info.use_strict_validation());
}

// ---------------------------------------------------------------------------
// PdfFileEditor — real file ops
// ---------------------------------------------------------------------------

#[test]
fn editor_concatenate_two_files() {
    let out = tmp("aspose_rust_fileeditor_concat.pdf");
    let ed = PdfFileEditor::new();
    assert!(ed.concatenate(hello_world(), two_pages(), &out));

    let re = Document::open(&out).unwrap();
    assert_eq!(re.pages().count(), 3); // 1 + 2

    let text = doc_text(&out);
    assert!(text.contains("Hello World"));
    assert!(text.contains("Page one"));
    assert!(text.contains("Page two"));
    let _ = std::fs::remove_file(&out);
}

#[test]
fn editor_concatenate_many_files() {
    let out = tmp("aspose_rust_fileeditor_concat_many.pdf");
    let ed = PdfFileEditor::new();
    let hello = hello_world();
    let two = two_pages();
    let inputs: Vec<&Path> = vec![hello.as_path(), two.as_path(), hello.as_path()];
    assert!(ed.concatenate_many(&inputs, &out));

    let re = Document::open(&out).unwrap();
    assert_eq!(re.pages().count(), 4); // 1 + 2 + 1
    let _ = std::fs::remove_file(&out);
}

#[test]
fn editor_extract_page_range() {
    let out = tmp("aspose_rust_fileeditor_extract.pdf");
    let ed = PdfFileEditor::new();
    // Keep only page 2 of two_pages ("Page two").
    assert!(ed.extract(two_pages(), 2, 2, &out));

    let re = Document::open(&out).unwrap();
    assert_eq!(re.pages().count(), 1);

    let text = doc_text(&out);
    assert!(text.contains("Page two"));
    assert!(!text.contains("Page one"));
    let _ = std::fs::remove_file(&out);
}

#[test]
fn editor_delete_pages() {
    let out = tmp("aspose_rust_fileeditor_delete.pdf");
    let ed = PdfFileEditor::new();
    assert!(ed.delete(two_pages(), &[1], &out));

    let re = Document::open(&out).unwrap();
    assert_eq!(re.pages().count(), 1);
    assert!(doc_text(&out).contains("Page two"));
    let _ = std::fs::remove_file(&out);
}

#[test]
fn editor_append_range() {
    let out = tmp("aspose_rust_fileeditor_append.pdf");
    let ed = PdfFileEditor::new();
    // Append page 1 ("Page one") of two_pages onto hello_world.
    assert!(ed.append(hello_world(), two_pages(), 1, 1, &out));

    let re = Document::open(&out).unwrap();
    assert_eq!(re.pages().count(), 2); // 1 + 1

    let text = doc_text(&out);
    assert!(text.contains("Hello World"));
    assert!(text.contains("Page one"));
    let _ = std::fs::remove_file(&out);
}

#[test]
fn editor_split_from_first_and_to_end() {
    let ed = PdfFileEditor::new();
    let first = tmp("aspose_rust_fileeditor_first.pdf");
    let rest = tmp("aspose_rust_fileeditor_rest.pdf");

    assert!(ed.split_from_first(two_pages(), 1, &first));
    assert_eq!(Document::open(&first).unwrap().pages().count(), 1);

    assert!(ed.split_to_end(two_pages(), 2, &rest));
    assert_eq!(Document::open(&rest).unwrap().pages().count(), 1);

    let _ = std::fs::remove_file(&first);
    let _ = std::fs::remove_file(&rest);
}

#[test]
fn editor_ops_fail_on_missing_input() {
    // The real ops return false when the source can't be read / parsed.
    let ed = PdfFileEditor::new();
    let out = tmp("aspose_rust_fileeditor_missing.pdf");
    assert!(!ed.concatenate("does_not_exist_a.pdf", "does_not_exist_b.pdf", &out));
    let _ = std::fs::remove_file(&out);
}

#[path = "../common/mod.rs"]
mod common;
