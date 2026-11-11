// Public-API smoke driver — Facades content cluster. Ports the C++
// PdfContentEditor / PdfConverter / PdfExtractor smoke tests into the
// Rust sibling.
//
// Coverage:
//   * PdfContentEditor — document-action name constants, viewer-
//     preference round-trip, the no-op content-edit stubs (replace
//     text / image / stamp / attachment), and inherited Save
//     passthrough.
//   * PdfConverter — real page count, unbound zero, real multi-page
//     TIFF render, compressed TIFF, per-page PNG render via the
//     DoConvert / HasNextImage / GetNextImage cursor, property
//     round-trip.
//   * PdfExtractor — property defaults + round-trip, real text
//     extraction (file + page cursor + start/end range), and the
//     image / attachment extraction stubs.
//
// Where the C++ tests assert behavior the Rust lib does not (yet)
// implement — real text replacement that re-renders the page, and
// image extraction / deletion that reads back embedded images — the
// Rust methods are stubs, so those scenarios assert the actual stub
// behavior (replace_text returns false; delete_* no-op; the image
// extractor surfaces nothing). See the OMITTED notes inline.

use std::path::{Path, PathBuf};

use aspose_pdf_foss::{
    CompressionType, Document, ISaveableFacade, PageCoordinateType, PdfContentEditor, PdfConverter,
    PdfExtractor, Resolution,
};

fn fixture(rel: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests").join("fixtures").join(rel)
}

fn hello_world_pdf() -> PathBuf {
    fixture("text_extractor/hello_world.pdf")
}

fn two_pages_pdf() -> PathBuf {
    fixture("text_extractor/two_pages.pdf")
}

fn temp_path(name: &str) -> PathBuf {
    common::unique_temp(name)
}

fn head(path: &Path, n: usize) -> Vec<u8> {
    let bytes = std::fs::read(path).unwrap();
    bytes[..n.min(bytes.len())].to_vec()
}

// ============================================================
// PdfContentEditor — facades_pdf_content_editor_smoke_test.cpp
// ============================================================

#[test]
fn content_editor_document_action_constants() {
    assert_eq!(PdfContentEditor::DOCUMENT_OPEN, "DO");
    assert_eq!(PdfContentEditor::DOCUMENT_CLOSE, "WC");
    assert_eq!(PdfContentEditor::DOCUMENT_WILL_SAVE, "WS");
    assert_eq!(PdfContentEditor::DOCUMENT_SAVED, "DS");
    assert_eq!(PdfContentEditor::DOCUMENT_WILL_PRINT, "WP");
    assert_eq!(PdfContentEditor::DOCUMENT_PRINTED, "DP");
}

#[test]
fn content_editor_viewer_preference_defaults_to_zero() {
    let doc = Document::open(hello_world_pdf()).unwrap();
    let editor = PdfContentEditor::with_document(doc);
    assert_eq!(editor.get_viewer_preference(), 0);
}

#[test]
#[should_panic(expected = "content_stream_authoring")]
fn content_editor_replace_text_count_is_pending() {
    let doc = Document::open(hello_world_pdf()).unwrap();
    let mut editor = PdfContentEditor::with_document(doc);
    editor.replace_text_count("foo", "bar", 1);
}

#[test]
fn content_editor_wired_operations_run() {
    let doc = Document::open(hello_world_pdf()).unwrap();
    let mut editor = PdfContentEditor::with_document(doc);

    editor.add_document_attachment("file.dat", "desc");
    editor.delete_attachments();
    editor.change_viewer_preference(1);
    assert_eq!(editor.get_viewer_preference(), 1);
    editor.replace_image(1, 1, "img.png");
    editor.delete_image(1, &[1, 2]);
    editor.delete_image_all();
}

#[test]
#[should_panic(expected = "document_additional_actions")]
fn content_editor_additional_action_is_pending() {
    let doc = Document::open(hello_world_pdf()).unwrap();
    let mut editor = PdfContentEditor::with_document(doc);
    editor.add_document_additional_action(PdfContentEditor::DOCUMENT_OPEN, "app.alert('hi')");
}

#[test]
#[should_panic(expected = "stamp_authoring")]
fn content_editor_stamp_operations_are_pending() {
    let doc = Document::open(hello_world_pdf()).unwrap();
    let mut editor = PdfContentEditor::with_document(doc);
    editor.delete_stamp(1, &[1]);
}

#[test]
#[should_panic(expected = "stamp_authoring")]
fn content_editor_move_stamp_is_pending() {
    let doc = Document::open(hello_world_pdf()).unwrap();
    let mut editor = PdfContentEditor::with_document(doc);
    editor.move_stamp(1, 2, 10.0, 20.0);
}

#[test]
#[should_panic(expected = "file_specification_contents")]
fn content_editor_attachment_from_bytes_is_pending() {
    let doc = Document::open(hello_world_pdf()).unwrap();
    let mut editor = PdfContentEditor::with_document(doc);
    editor.add_document_attachment_bytes(b"payload", "file.dat", "desc");
}

/// A document with no links extracts none. The substance — that an
/// extracted link keeps its action — is asserted in
/// `an_extracted_link_keeps_its_action` over in the nav driver, where
/// the editors live.
#[test]
fn content_editor_extracts_no_link_from_a_document_without_one() {
    let doc = Document::open(hello_world_pdf()).unwrap();
    let editor = PdfContentEditor::with_document(doc);
    assert!(editor.extract_link().is_empty());
}

#[test]
fn content_editor_save_passes_through() {
    let doc = Document::open(hello_world_pdf()).unwrap();
    let editor = PdfContentEditor::with_document(doc);
    let out = temp_path("aspose_rust_contenteditor_save.pdf");
    // Inherited SaveableFacade::save — passthrough.
    editor.save(&out).unwrap();
    assert!(out.exists());
    let _ = std::fs::remove_file(&out);
}

// ============================================================
// PdfContentEditor real replace — facades_pdf_content_editor_replace_smoke_test.cpp
//
// OMITTED (stub behavior): the C++ ReplaceAllPages / ReplaceOnSpecificPage
// scenarios assert that the saved document's text actually changes
// (find("REPLACED") succeeds, find("Page one") fails). The Rust
// replace_text path is a stub that returns false and does not re-render
// the content stream, so those positive-replacement assertions cannot
// hold. We keep the ReplaceMissingReturnsFalse scenario (which matches
// real stub behavior) and assert the stub contract for the others.
// ============================================================

#[test]
#[should_panic(expected = "content_stream_authoring")]
fn content_editor_replace_text_is_pending() {
    let doc = Document::open(two_pages_pdf()).unwrap();
    let mut ed = PdfContentEditor::with_document(doc);
    // Text replacement needs content-stream write-back; the marker names
    // the capability rather than reporting "nothing matched".
    ed.replace_text("NoSuchString12345", "X");
}

#[test]
fn content_editor_save_round_trips_every_page() {
    let out = temp_path("aspose_rust_replace_all.pdf");
    {
        let doc = Document::open(two_pages_pdf()).unwrap();
        let ed = PdfContentEditor::with_document(doc);
        ed.save(&out).unwrap();
    }
    assert!(out.exists());

    // The saved document round-trips through extraction with its text
    // intact on every page.
    let mut ex = PdfExtractor::with_document(Document::open(&out).unwrap());
    let mut text = Vec::new();
    ex.get_text_to_writer(&mut text).unwrap();
    let text = String::from_utf8_lossy(&text);
    assert!(text.contains("Page one"));
    assert!(text.contains("Page two"));
    let _ = std::fs::remove_file(&out);
}

#[test]
#[should_panic(expected = "content_stream_authoring")]
fn content_editor_replace_on_page_is_pending() {
    let doc = Document::open(two_pages_pdf()).unwrap();
    let mut ed = PdfContentEditor::with_document(doc);
    ed.replace_text_on_page("Page two", 2, "SECOND");
}

// ============================================================
// PdfContentEditor delete image — facades_content_editor_delete_image_smoke_test.cpp
//
// The embed / count / delete / reload scenarios that exercise real
// image readback live in image_extract_test.rs. Here we keep the
// no-image edge: on a document with no embedded images the delete
// editors and the extractor image walk are well-behaved (no-op /
// empty).
// ============================================================

#[test]
fn content_editor_delete_image_on_imageless_doc_is_noop() {
    let doc = Document::open(hello_world_pdf()).unwrap();
    let mut editor = PdfContentEditor::with_document(doc);
    editor.delete_image(1, &[1]); // no images on the page — no-op
    editor.delete_image_all(); // no images anywhere — no-op
}

#[test]
fn extractor_image_walk_on_imageless_doc_is_empty() {
    // hello_world.pdf has no embedded images, so the walk yields nothing.
    let doc = Document::open(hello_world_pdf()).unwrap();
    let mut ex = PdfExtractor::with_document(doc);
    assert!(!ex.has_next_image());
    let sink = temp_path("aspose_rust_delimg_sink.dat");
    assert!(!ex.get_next_image(&sink));
}

// ============================================================
// PdfConverter — facades_pdf_converter_smoke_test.cpp
// ============================================================

fn bound_converter() -> PdfConverter {
    PdfConverter::with_document(Document::open(hello_world_pdf()).unwrap())
}

#[test]
fn converter_page_count_real() {
    let conv = bound_converter();
    let expected = Document::open(hello_world_pdf()).unwrap().pages().count() as i32;
    assert_eq!(conv.page_count(), expected);
    assert!(conv.page_count() > 0);
}

#[test]
fn converter_unbound_page_count_zero() {
    let conv = PdfConverter::new();
    assert_eq!(conv.page_count(), 0);
    assert!(!conv.has_next_image());
}

#[test]
fn converter_save_as_tiff_renders_real_file() {
    let conv = bound_converter();
    let out = temp_path("aspose_rust_pdfconv_all.tiff");
    conv.save_as_tiff(&out).unwrap();
    assert!(out.exists());
    assert!(std::fs::metadata(&out).unwrap().len() > 0);
    // TIFF magic: "II" (little-endian) or "MM" (big-endian).
    let h = head(&out, 2);
    assert!(h == b"II" || h == b"MM", "head={h:?}");
    let _ = std::fs::remove_file(&out);
}

#[test]
fn converter_save_as_tiff_with_compression() {
    let conv = bound_converter();
    let out = temp_path("aspose_rust_pdfconv_lzw.tiff");
    conv.save_as_tiff_with_compression(&out, CompressionType::Lzw).unwrap();
    assert!(out.exists());
    assert!(std::fs::metadata(&out).unwrap().len() > 0);
    let _ = std::fs::remove_file(&out);
}

#[test]
fn converter_get_next_image_renders_png() {
    let mut conv = bound_converter();
    let total = conv.page_count();
    conv.do_convert();

    let mut rendered = 0;
    while conv.has_next_image() {
        let out = temp_path(&format!("aspose_rust_pdfconv_page{rendered}.png"));
        conv.get_next_image(&out).unwrap();
        assert!(out.exists());
        // PNG signature 0x89 'P' 'N' 'G'.
        let h = head(&out, 4);
        assert!(h.len() >= 4);
        assert_eq!(h[0], 0x89);
        assert_eq!(&h[1..4], b"PNG");
        let _ = std::fs::remove_file(&out);
        rendered += 1;
    }
    assert_eq!(rendered, total);
}

#[test]
fn converter_property_roundtrip() {
    let mut conv = PdfConverter::new();
    conv.set_start_page(2);
    conv.set_end_page(5);
    conv.set_show_hidden_areas(true);
    conv.set_coordinate_type(PageCoordinateType::MediaBox);
    conv.set_password("pw");
    conv.set_user_password("upw");
    conv.set_resolution(Resolution::new(300).unwrap());

    assert_eq!(conv.start_page(), 2);
    assert_eq!(conv.end_page(), 5);
    assert!(conv.show_hidden_areas());
    assert_eq!(conv.coordinate_type(), PageCoordinateType::MediaBox);
    assert_eq!(conv.password(), "pw");
    assert_eq!(conv.user_password(), "upw");
}

// ============================================================
// PdfExtractor — facades_pdf_extractor_smoke_test.cpp
// ============================================================

fn read_file(path: &Path) -> String {
    String::from_utf8_lossy(&std::fs::read(path).unwrap()).into_owned()
}

#[test]
fn extractor_property_defaults_and_roundtrip() {
    let mut ex = PdfExtractor::new();
    assert_eq!(ex.start_page(), 1);
    assert_eq!(ex.end_page(), 0);
    assert_eq!(ex.extract_text_mode(), 0);
    assert_eq!(ex.resolution(), 150);
    assert!(ex.password().is_empty());
    assert!(!ex.is_bidi());

    ex.set_start_page(2);
    ex.set_end_page(5);
    ex.set_extract_text_mode(1);
    ex.set_resolution(300);
    ex.set_password("secret");
    assert_eq!(ex.start_page(), 2);
    assert_eq!(ex.end_page(), 5);
    assert_eq!(ex.extract_text_mode(), 1);
    assert_eq!(ex.resolution(), 300);
    assert_eq!(ex.password(), "secret");
}

#[test]
fn extractor_bind_and_extract_text() {
    let doc = Document::open(two_pages_pdf()).unwrap();
    let mut ex = PdfExtractor::with_document(doc);
    ex.extract_text();

    let out = temp_path("aspose_rust_extractor_all.txt");
    ex.get_text(&out).unwrap();
    assert!(out.exists());
    let text = read_file(&out);
    assert!(text.contains("Page one"));
    assert!(text.contains("Page two"));
    let _ = std::fs::remove_file(&out);
}

#[test]
fn extractor_get_text_lazily_extracts() {
    // GetText without a prior ExtractText() still works — extraction
    // runs lazily on first read.
    let doc = Document::open(two_pages_pdf()).unwrap();
    let mut ex = PdfExtractor::with_document(doc);
    let out = temp_path("aspose_rust_extractor_lazy.txt");
    ex.get_text(&out).unwrap();
    let text = read_file(&out);
    assert!(text.contains("Page one"));
    assert!(text.contains("Page two"));
    let _ = std::fs::remove_file(&out);
}

#[test]
fn extractor_page_by_page_cursor() {
    let doc = Document::open(two_pages_pdf()).unwrap();
    let mut ex = PdfExtractor::with_document(doc);

    assert!(ex.has_next_page_text());
    let p1 = temp_path("aspose_rust_extractor_p1.txt");
    ex.get_next_page_text(&p1).unwrap();
    assert!(read_file(&p1).contains("Page one"));

    assert!(ex.has_next_page_text());
    let p2 = temp_path("aspose_rust_extractor_p2.txt");
    ex.get_next_page_text(&p2).unwrap();
    assert!(read_file(&p2).contains("Page two"));

    assert!(!ex.has_next_page_text());
    let _ = std::fs::remove_file(&p1);
    let _ = std::fs::remove_file(&p2);
}

#[test]
fn extractor_start_page_end_page_range() {
    let doc = Document::open(two_pages_pdf()).unwrap();
    let mut ex = PdfExtractor::with_document(doc);
    ex.set_start_page(2); // page 2 only — resets the extraction pass
    ex.extract_text();

    let out = temp_path("aspose_rust_extractor_range.txt");
    ex.get_text(&out).unwrap();
    let text = read_file(&out);
    assert!(!text.contains("Page one"));
    assert!(text.contains("Page two"));
    let _ = std::fs::remove_file(&out);
}

#[test]
fn extractor_image_extraction_surfaces_nothing() {
    let doc = Document::open(two_pages_pdf()).unwrap();
    let mut ex = PdfExtractor::with_document(doc);

    // Image extraction — v1 stubs: no images surfaced.
    assert!(!ex.has_next_image());
    assert!(!ex.get_next_image(temp_path("aspose_rust_extractor_img.bin")));
    ex.extract_image(); // no-op, must not panic
}

/// A document with no attachments answers with empty lists, and asking
/// to write one does nothing — there is nothing to be wrong about.
#[test]
fn extractor_reports_no_attachments_when_there_are_none() {
    let doc = Document::open(two_pages_pdf()).unwrap();
    let mut ex = PdfExtractor::with_document(doc);
    assert!(ex.get_attach_names().is_empty());
    assert!(ex.get_attachment_info().is_empty());
    // No extraction asked for yet, so this writes nothing.
    let unwritten = temp_path("aspose_rust_no_attachment.bin");
    ex.get_attachment(unwritten.to_string_lossy().as_ref());
    assert!(!unwritten.exists());
}

/// The other half of embedding: the names, the specifications and the
/// decoded bytes of an embedded file. The attachment is embedded
/// through the public collection, saved, reopened and read back — which
/// is what a caller does.
#[test]
fn extractor_reads_an_embedded_file_back_out() {
    use aspose_pdf_foss::FileSpecification;

    let payload = b"attached bytes, verbatim\n";
    let source = common::unique_temp("aspose_rust_attach_src.txt");
    std::fs::write(&source, payload).unwrap();

    let pdf = common::unique_temp("aspose_rust_attach.pdf");
    {
        let mut doc = Document::open(two_pages_pdf()).unwrap();
        let mut spec = FileSpecification::with_file_and_description(
            source.to_string_lossy().into_owned(),
            "the payload".into(),
        );
        spec.set_name("payload.txt".into());
        doc.embedded_files_mut().add_with_key("payload".into(), spec);
        doc.save_to(&pdf).unwrap();
    }

    let doc = Document::open(&pdf).unwrap();
    let mut ex = PdfExtractor::with_document(doc);
    assert_eq!(ex.get_attach_names(), vec!["payload.txt".to_string()]);
    let info = ex.get_attachment_info();
    assert_eq!(info.len(), 1);
    assert_eq!(info[0].name(), "payload.txt");
    assert_eq!(info[0].description(), "the payload");

    // Named selection writes that one attachment to the given path.
    let out = common::unique_temp("aspose_rust_attach_out.txt");
    ex.extract_attachment_named("payload.txt");
    ex.get_attachment(out.to_string_lossy().as_ref());
    assert_eq!(
        std::fs::read(&out).expect("the attachment was written"),
        payload,
        "byte for byte"
    );

    // Selecting everything treats the path as a directory.
    let dir = common::unique_temp("aspose_rust_attach_dir");
    ex.extract_attachment();
    ex.get_attachment(dir.to_string_lossy().as_ref());
    assert_eq!(
        std::fs::read(dir.join("payload.txt")).expect("written under its own name"),
        payload
    );

    let _ = std::fs::remove_file(&source);
    let _ = std::fs::remove_file(&pdf);
    let _ = std::fs::remove_file(&out);
    let _ = std::fs::remove_dir_all(&dir);
}

#[path = "../common/mod.rs"]
mod common;

// ============================================================
// Wired PdfContentEditor operations. These three reach real
// machinery — the page's live XImageCollection and the document's
// EmbeddedFileCollection — rather than standing in as no-ops.
// ============================================================

#[test]
fn content_editor_replace_image_swaps_the_page_image() {
    // image_xobject.pdf draws one 4x4 image; the replacement is 16x16.
    let doc = Document::open(fixture("page_renderer/image_xobject.pdf")).unwrap();
    let page = doc.pages().get(1).unwrap();
    assert_eq!(page.resources().images().count(), 1);
    assert_eq!(page.resources().images().get(0).unwrap().width(), 4);
    drop(page);

    let mut editor = PdfContentEditor::with_document(doc);
    editor.replace_image(
        1,
        1,
        fixture("png_decoder/rgb_checker_16.png").to_str().unwrap(),
    );

    {
        let doc = editor.document().unwrap();
        let page = doc.pages().get(1).unwrap();
        let images = page.resources().images();
        // Replaced in place: same slot, new pixels.
        assert_eq!(images.count(), 1);
        assert_eq!(images.get(0).unwrap().width(), 16);
        assert_eq!(images.get(0).unwrap().height(), 16);
    }

    // The replacement survives the save, which is what the caller
    // actually wants from it.
    let out = temp_path("aspose_rust_contenteditor_replace_image.pdf");
    editor.save(&out).unwrap();
    let saved = Document::open(&out).unwrap();
    let page = saved.pages().get(1).unwrap();
    assert_eq!(page.resources().images().get(0).unwrap().width(), 16);
    let _ = std::fs::remove_file(&out);
}

#[test]
fn content_editor_replace_image_leaves_an_unreadable_path_alone() {
    let doc = Document::open(fixture("page_renderer/image_xobject.pdf")).unwrap();
    let mut editor = PdfContentEditor::with_document(doc);
    editor.replace_image(1, 1, "no/such/file.png");

    let doc = editor.document().unwrap();
    let page = doc.pages().get(1).unwrap();
    assert_eq!(page.resources().images().get(0).unwrap().width(), 4);
}

#[test]
fn content_editor_attachments_reach_the_embedded_file_collection() {
    let doc = Document::open(hello_world_pdf()).unwrap();
    let mut editor = PdfContentEditor::with_document(doc);
    assert_eq!(editor.document().unwrap().embedded_files().count(), 0);

    editor.add_document_attachment("notes.txt", "release notes");
    editor.add_document_attachment("data.bin", "payload");
    assert_eq!(editor.document().unwrap().embedded_files().count(), 2);

    editor.delete_attachments();
    assert_eq!(editor.document().unwrap().embedded_files().count(), 0);
}
