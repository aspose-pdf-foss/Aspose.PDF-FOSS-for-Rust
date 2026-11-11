// Public-API driver — embedded-image enumerate / extract / delete.
//
// Ports the C++ image smoke tests:
//   * image_embed_extract_smoke_test.cpp — embed a JPEG / PNG, reopen,
//     and walk it out with PdfExtractor (DCTDecode passthrough gives the
//     verbatim JPEG bytes).
//   * facades_content_editor_delete_image_smoke_test.cpp — embed images,
//     count them via the PdfExtractor image walk, delete a subset with
//     PdfContentEditor, and confirm the count drops + survives reload.
//
// Rust note: the C++ `Page::AddImage(path, rect)` content-draw helper has
// no Rust surface; images are embedded through the canonical
// `Page.Resources.Images` collection (`add_stream`), which is the same
// /XObject /Image embed path. The C++ tests share one `Document&` between
// the editor and the extractor; the Rust `Document` is moved into a
// facade, so the bound document is reached back through `facade.document()`
// (the collections are interior-mutable Rc handles, so edits are visible).

use aspose_pdf_foss::{Document, PdfContentEditor, PdfExtractor};
use std::path::{Path, PathBuf};

fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}
fn hello_world() -> PathBuf {
    fixtures().join("text_extractor/hello_world.pdf")
}
fn jpeg_32() -> Vec<u8> {
    std::fs::read(fixtures().join("dctdecode/gradient_32.jpg")).unwrap()
}
fn png_rgba_8() -> Vec<u8> {
    std::fs::read(fixtures().join("png_decoder/rgba_filter_none_8.png")).unwrap()
}

/// Count a document's embedded images via the public PdfExtractor image
/// walk (mirrors the C++ `CountImages` helper).
fn count_images(doc: &Document) -> usize {
    // The extractor takes an owned document; serialise + reopen into a
    // fresh one so the caller keeps its handle. (A loaded image survives a
    // save round-trip, so this reflects on-disk state — used after save.)
    let mut bytes = Vec::new();
    doc.save_to_writer(&mut bytes).unwrap();
    let mut ex = PdfExtractor::with_document(Document::from_bytes(bytes).unwrap());
    let sink = common::unique_temp("aspose_rust_imgcount_sink.dat");
    let mut n = 0;
    while ex.has_next_image() && n < 100 {
        if !ex.get_next_image(&sink) {
            break;
        }
        n += 1;
    }
    let _ = std::fs::remove_file(&sink);
    n
}

/// Count the live (in-memory, possibly unsaved) images directly off the
/// page collection — what a shared-document editor + walk observes.
fn count_live_images(doc: &Document) -> usize {
    let pages = doc.pages();
    let mut n = 0;
    for p in 1..=pages.count() {
        if let Ok(page) = pages.get(p) {
            n += page.resources().images().count();
        }
    }
    n
}

// ============================================================
// image_embed_extract_smoke_test.cpp
// ============================================================

#[test]
fn embed_jpeg_extract_round_trip() {
    let original = jpeg_32();
    assert!(!original.is_empty());
    let out = common::unique_temp("aspose_rust_embed_jpg.pdf");
    {
        let doc = Document::open(hello_world()).unwrap();
        doc.pages().get(1).unwrap().resources().images().add_stream(&original);
        doc.save_to(&out).unwrap();
    }

    let re = Document::open(&out).unwrap();
    let mut ex = PdfExtractor::with_document(re);
    ex.extract_image();
    assert!(ex.has_next_image());

    let img_out = common::unique_temp("aspose_rust_extracted.jpg");
    assert!(ex.get_next_image(&img_out));
    assert!(!ex.has_next_image(), "only one image");

    // DCTDecode passthrough → the extracted bytes are the original JPEG.
    let extracted = std::fs::read(&img_out).unwrap();
    assert_eq!(extracted, original);

    let _ = std::fs::remove_file(&out);
    let _ = std::fs::remove_file(&img_out);
}

#[test]
fn embed_png_produces_an_image() {
    let out = common::unique_temp("aspose_rust_embed_png.pdf");
    {
        let doc = Document::open(hello_world()).unwrap();
        doc.pages().get(1).unwrap().resources().images().add_stream(&png_rgba_8());
        doc.save_to(&out).unwrap();
    }
    let re = Document::open(&out).unwrap();
    let mut ex = PdfExtractor::with_document(re);
    ex.extract_image();
    assert!(ex.has_next_image());
    let _ = std::fs::remove_file(&out);
}

#[test]
fn no_images_in_unstamped_doc() {
    let doc = Document::open(hello_world()).unwrap();
    let mut ex = PdfExtractor::with_document(doc);
    ex.extract_image();
    assert!(!ex.has_next_image());
}

#[test]
fn extracted_jpeg_has_soi_marker() {
    // The raw embedded image bytes start with the JPEG SOI marker FF D8.
    let out = common::unique_temp("aspose_rust_embed_soi.pdf");
    {
        let doc = Document::open(hello_world()).unwrap();
        doc.pages().get(1).unwrap().resources().images().add_stream(&jpeg_32());
        doc.save_to(&out).unwrap();
    }
    let re = Document::open(&out).unwrap();
    let __pg0 = re.pages().get(1).unwrap();
    let images = __pg0.resources().images();
    assert_eq!(images.count(), 1);
    let img = images.get(0).unwrap();
    assert!(img.width() > 0);
    assert!(img.height() > 0);
    assert!(!img.name().is_empty());
    let raw = img.get_raw_image_data();
    assert!(raw.len() >= 2);
    assert_eq!(raw[0], 0xFF);
    assert_eq!(raw[1], 0xD8);
    let _ = std::fs::remove_file(&out);
}

// ============================================================
// facades_content_editor_delete_image_smoke_test.cpp
// ============================================================

#[test]
fn delete_one_reduces_count() {
    let doc = Document::open(hello_world()).unwrap();
    let png = png_rgba_8();
    doc.pages().get(1).unwrap().resources().images().add_stream(&png);
    doc.pages().get(1).unwrap().resources().images().add_stream(&png);
    assert_eq!(count_live_images(&doc), 2);

    let mut editor = PdfContentEditor::with_document(doc);
    editor.delete_image(1, &[1]); // remove the first image
    assert_eq!(count_live_images(editor.document().unwrap()), 1);
}

#[test]
fn delete_all_no_arg() {
    let doc = Document::open(hello_world()).unwrap();
    let png = png_rgba_8();
    doc.pages().get(1).unwrap().resources().images().add_stream(&png);
    doc.pages().get(1).unwrap().resources().images().add_stream(&png);
    assert_eq!(count_live_images(&doc), 2);

    let mut editor = PdfContentEditor::with_document(doc);
    editor.delete_image_all(); // every image on every page
    assert_eq!(count_live_images(editor.document().unwrap()), 0);
}

#[test]
fn delete_persists_across_reload() {
    let out = common::unique_temp("aspose_rust_delimg_out.pdf");
    {
        let doc = Document::open(hello_world()).unwrap();
        let png = png_rgba_8();
        doc.pages().get(1).unwrap().resources().images().add_stream(&png);
        doc.pages().get(1).unwrap().resources().images().add_stream(&png);

        let mut editor = PdfContentEditor::with_document(doc);
        editor.delete_image(1, &[2]); // remove the second image
        editor.document().unwrap().save_to(&out).unwrap();
    }

    let re = Document::open(&out).unwrap();
    assert_eq!(count_images(&re), 1);
    assert_eq!(count_live_images(&re), 1);
    let _ = std::fs::remove_file(&out);
}

#[test]
fn extractor_exposes_text_search_options() {
    let ex = PdfExtractor::new();
    assert!(!ex.text_search_options().is_regular_expression_used());
}

#[path = "../common/mod.rs"]
mod common;
