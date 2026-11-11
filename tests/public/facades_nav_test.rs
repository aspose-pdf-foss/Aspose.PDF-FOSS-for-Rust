// Public-API smoke driver — Facades cluster (navigation / editing /
// metadata / forms + the facade base contract).
//
// Ports the C++ smoke tests:
//   facades_pdf_bookmark_editor_smoke_test
//   facades_pdf_page_editor_smoke_test
//   facades_pdf_annotation_editor_smoke_test
//   facades_pdf_xmp_metadata_smoke_test
//   facades_form_editor_smoke_test
//   facades_form_field_facade_smoke_test
//   facades_foundation_smoke_test
//
// Each scenario is exercised against the REAL Rust facade surface.
// Where the Rust lib ships a v1 stub (e.g. bookmark create/extract is
// in-memory only, annotation deletion is a no-op stub, AcroForm save-
// through is a follow-on) the C++ behavioural assertion is relaxed to
// match real Rust behaviour — see the OMITTED notes in the port report.

use aspose_pdf_foss::{
    AlignmentType, Bookmark, Bookmarks, Document, EncodingType, Facade, FieldType, FontStyle,
    FormEditor, FormFieldFacade, HorizontalAlignment, IFacade, ISaveableFacade, PageSize,
    PdfAnnotationEditor, PdfBookmarkEditor, PdfPageEditor, PdfXmpMetadata,
    SaveableFacade, SubmitFormFlag, VerticalAlignment, VerticalAlignmentType, XmpValue,
};
use std::path::{Path, PathBuf};

fn hello_world() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/text_extractor/hello_world.pdf")
}

fn two_pages() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/text_extractor/two_pages.pdf")
}

fn with_info() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/metadata/with_info.pdf")
}

// =============================================================================
// PdfBookmarkEditor + Bookmark / Bookmarks value types
// =============================================================================

#[test]
fn bookmark_pod_defaults_and_roundtrip() {
    let mut bm = Bookmark::new();
    assert!(bm.title().is_empty());
    assert_eq!(bm.page_number(), 1);
    assert_eq!(bm.level(), 1);
    assert!(!bm.bold_flag());
    assert!(!bm.italic_flag());
    assert!(!bm.open());

    bm.set_title("Chapter 1");
    bm.set_page_number(5);
    bm.set_level(2);
    bm.set_bold_flag(true);
    bm.set_italic_flag(true);
    bm.set_open(true);
    bm.set_action("GoTo");
    bm.set_destination("dest1");
    bm.set_page_display("FitH");
    bm.set_page_display_left(10);
    bm.set_page_display_top(700);
    bm.set_page_display_zoom(100);
    bm.set_remote_file("other.pdf");

    assert_eq!(bm.title(), "Chapter 1");
    assert_eq!(bm.page_number(), 5);
    assert_eq!(bm.level(), 2);
    assert!(bm.bold_flag());
    assert!(bm.italic_flag());
    assert!(bm.open());
    assert_eq!(bm.action(), "GoTo");
    assert_eq!(bm.destination(), "dest1");
    assert_eq!(bm.page_display(), "FitH");
    assert_eq!(bm.page_display_left(), 10);
    assert_eq!(bm.page_display_top(), 700);
    assert_eq!(bm.page_display_zoom(), 100);
    assert_eq!(bm.remote_file(), "other.pdf");
}

#[test]
fn bookmarks_collection_is_a_list() {
    let mut list = Bookmarks::new();
    assert!(list.is_empty());

    let mut a = Bookmark::new();
    a.set_title("A");
    let mut b = Bookmark::new();
    b.set_title("B");
    list.add(a);
    list.add(b);

    assert_eq!(list.count(), 2);
    assert_eq!(list.get(0).unwrap().title(), "A");
    assert_eq!(list.get(1).unwrap().title(), "B");
}

#[test]
fn child_items_roundtrip() {
    let mut parent = Bookmark::new();
    parent.set_title("Parent");

    let mut children = Bookmarks::new();
    let mut child = Bookmark::new();
    child.set_title("Child");
    children.add(child);

    parent.set_child_items(children);
    let back = parent.child_items();
    assert_eq!(back.count(), 1);
    assert_eq!(back.get(0).unwrap().title(), "Child");

    // ChildItem is a canonical alias over the same backing collection.
    assert_eq!(parent.child_item().count(), 1);
}

#[test]
fn editor_stubs_do_not_panic() {
    let mut editor = PdfBookmarkEditor::with_document(Document::open(hello_world()).unwrap());

    let mut bm = Bookmark::new();
    bm.set_title("Intro");
    bm.set_page_number(1);

    editor.create_bookmarks_from(bm);
    editor.create_bookmark_of_page("Intro", 1);
    editor.create_bookmark_of_pages(&["A", "B"], &[1, 2]);
    editor.modify_bookmarks("old", "new");
    editor.delete_bookmarks_named("Intro");
    editor.delete_bookmarks();
}

/// A document with no outline fills the editor with nothing. That it
/// fills the list from an outline the document DOES have is asserted in
/// `the_bookmark_editor_reads_the_documents_outline`, over in the
/// outline driver.
#[test]
fn bookmark_editor_reads_nothing_from_a_document_without_an_outline() {
    let mut editor = PdfBookmarkEditor::with_document(Document::open(hello_world()).unwrap());
    editor.create_bookmarks();
    assert_eq!(editor.extract_bookmarks().count(), 0);
}

#[test]
#[should_panic(expected = "bookmark_serialization")]
fn bookmark_editor_xml_export_is_pending() {
    let editor = PdfBookmarkEditor::with_document(Document::open(hello_world()).unwrap());
    let _ = editor.export_bookmarks_to_xml("out.xml");
}

#[test]
#[should_panic(expected = "bookmark_serialization")]
fn bookmark_editor_html_export_is_pending() {
    let editor = PdfBookmarkEditor::with_document(Document::open(hello_world()).unwrap());
    let _ = editor.export_bookmarks_to_html("dir", "out.html");
}

#[test]
fn extract_returns_empty_for_fresh_editor() {
    let editor = PdfBookmarkEditor::with_document(Document::open(hello_world()).unwrap());
    assert!(editor.extract_bookmarks().is_empty());
    assert!(editor.extract_bookmarks_with_children(true).is_empty());
    assert!(editor.extract_bookmarks_named("Intro").is_empty());

    let parent = Bookmark::new();
    assert!(editor.extract_bookmarks_under(&parent).is_empty());
}

#[test]
fn create_then_extract_in_memory() {
    // The Rust editor manages an in-memory Bookmarks list; create/extract
    // round-trips through that list (the /Outlines save-through is a
    // follow-on, so we assert against the editor list, not a reload).
    let mut ed = PdfBookmarkEditor::with_document(Document::open(two_pages()).unwrap());
    ed.create_bookmark_of_page("Chapter 1", 1);
    ed.create_bookmark_of_page("Chapter 2", 2);

    let bms = ed.extract_bookmarks();
    assert_eq!(bms.count(), 2);
    assert_eq!(bms.get(0).unwrap().title(), "Chapter 1");
    assert_eq!(bms.get(1).unwrap().title(), "Chapter 2");
}

#[test]
fn nested_bookmarks_round_trip_in_memory() {
    let mut ed = PdfBookmarkEditor::with_document(Document::open(two_pages()).unwrap());
    let mut parent = Bookmark::new();
    parent.set_title("Part I");
    parent.set_page_number(1);

    let mut kids = Bookmarks::new();
    let mut child = Bookmark::new();
    child.set_title("Section 1.1");
    child.set_page_number(2);
    kids.add(child);
    parent.set_child_items(kids);

    ed.create_bookmarks_from(parent);
    let bms = ed.extract_bookmarks();
    assert_eq!(bms.count(), 1);
    let p = bms.get(0).unwrap();
    assert_eq!(p.title(), "Part I");
    assert_eq!(p.child_items().get(0).unwrap().title(), "Section 1.1");
}

#[test]
fn delete_clears_editor_bookmarks() {
    let mut ed = PdfBookmarkEditor::with_document(Document::open(two_pages()).unwrap());
    ed.create_bookmark_of_page("Temp", 1);
    ed.delete_bookmarks();
    assert!(ed.extract_bookmarks().is_empty());
}

// =============================================================================
// PdfPageEditor
// =============================================================================

#[test]
fn page_editor_transition_constants() {
    assert_eq!(PdfPageEditor::SPLITVOUT, 1);
    assert_eq!(PdfPageEditor::SPLITHIN, 4);
    assert_eq!(PdfPageEditor::INBOX, 7);
    assert_eq!(PdfPageEditor::DISSOLVE, 13);
    assert_eq!(PdfPageEditor::DGLITTER, 16);
}

#[test]
fn page_editor_get_pages_real() {
    let doc = Document::open(hello_world()).unwrap();
    let expected = doc.pages().count() as i32;
    let editor = PdfPageEditor::with_document(doc);
    assert_eq!(editor.get_pages(), expected);
    assert!(editor.get_pages() > 0);
}

#[test]
fn page_editor_geometry() {
    let editor = PdfPageEditor::with_document(Document::open(hello_world()).unwrap());
    let sz = editor.get_page_size(1);
    assert_eq!(sz.width(), 612.0);
    assert_eq!(sz.height(), 792.0);
    assert_eq!(editor.get_page_rotation(1), 0);
}

#[test]
#[should_panic(expected = "content_stream_authoring")]
fn page_editor_move_position_is_pending() {
    let mut editor = PdfPageEditor::with_document(Document::open(hello_world()).unwrap());
    editor.move_position(10.0, 20.0);
}

#[test]
fn page_editor_unbound_get_pages_zero() {
    let editor = PdfPageEditor::new();
    assert_eq!(editor.get_pages(), 0);
}

#[test]
fn page_editor_property_roundtrip() {
    let mut editor = PdfPageEditor::new();
    editor.set_transition_duration(5);
    editor.set_transition_type(PdfPageEditor::BLINDV);
    editor.set_display_duration(3);
    editor.set_process_pages(vec![1, 3, 5]);
    editor.set_rotation(90);
    editor.set_zoom(1.5);
    editor.set_page_size(PageSize::new(595.0, 842.0));
    editor.set_horizontal_alignment(HorizontalAlignment::Left);
    editor.set_vertical_alignment_type(VerticalAlignment::Top);

    assert_eq!(editor.transition_duration(), 5);
    assert_eq!(editor.transition_type(), PdfPageEditor::BLINDV);
    assert_eq!(editor.display_duration(), 3);
    assert_eq!(editor.process_pages().len(), 3);
    assert_eq!(editor.process_pages()[1], 3);
    assert_eq!(editor.rotation(), 90);
    assert_eq!(editor.zoom(), 1.5);
    assert_eq!(editor.page_size().width(), 595.0);
    assert_eq!(editor.horizontal_alignment(), HorizontalAlignment::Left);
    assert_eq!(editor.vertical_alignment_type(), VerticalAlignment::Top);
}

// =============================================================================
// PdfAnnotationEditor — the extract / delete / flatten family is real;
// XFDF import-export and the two modify overloads remain stubs
// =============================================================================

/// Deleting on a document whose pages carry no annotations is a no-op
/// rather than a failure, and the annotations a document DOES carry are
/// deleted — see `the_annotation_editor_extracts_and_deletes_by_name_and_type`
/// for that half.
#[test]
fn deleting_annotations_that_are_not_there_is_harmless() {
    let mut editor = PdfAnnotationEditor::with_document(Document::open(hello_world()).unwrap());
    editor.delete_annotations();
    editor.delete_annotation("first");
    assert_eq!(
        editor
            .document()
            .unwrap()
            .pages()
            .get(1)
            .unwrap()
            .annotations()
            .count(),
        0
    );
}

#[test]
#[should_panic(expected = "xfdf_serialization")]
fn annotation_editor_xfdf_import_is_pending() {
    let mut editor = PdfAnnotationEditor::with_document(Document::open(hello_world()).unwrap());
    editor.import_annotations_from_xfdf("a.xfdf");
}

#[test]
#[should_panic(expected = "xfdf_serialization")]
fn annotation_editor_fdf_import_is_pending() {
    let mut editor = PdfAnnotationEditor::with_document(Document::open(hello_world()).unwrap());
    editor.import_annotations_from_fdf("a.fdf");
}

/// Flattening is structural: the annotations stop being interactive
/// objects and the save writes the pages without them. It does not draw
/// their appearances into the content first — that needs an appearance
/// generator this crate does not have, which is what FlattenSettings
/// steers and why those options change nothing.
#[test]
fn flattening_annotations_removes_them_from_the_page() {
    use aspose_pdf_foss::{
        AnnotationType, FlattenSettings, Rectangle, SquareAnnotation, TextAnnotation,
    };

    let out = common::unique_temp("aspose_rust_flatten_annots.pdf")
        .to_string_lossy()
        .into_owned();
    {
        let doc = Document::open(hello_world()).unwrap();
        {
            let page = doc.pages().get(1).unwrap();
            let r = Rectangle::new(10.0, 10.0, 40.0, 40.0, false);
            page.annotations().add(Box::new(TextAnnotation::on_page(&page, r)));
            page.annotations().add(Box::new(SquareAnnotation::on_page(&page, r)));
            assert_eq!(page.annotations().count(), 2);
        }
        let mut editor = PdfAnnotationEditor::with_document(doc);
        // Only the note is asked for, so the square stays.
        editor.flattening_annotations_range(1, 1, &[AnnotationType::Text]);
        {
            let doc = editor.document().unwrap();
            let page = doc.pages().get(1).unwrap();
            assert_eq!(page.annotations().count(), 1, "the note was flattened");
            assert_eq!(
                page.annotations().get(0).unwrap().annotation_type(),
                AnnotationType::Square,
                "and the square was not"
            );
        }
        // Settings are accepted and steer the part that is missing.
        editor.flattening_annotations_with_settings(FlattenSettings::new());
        {
            let doc = editor.document().unwrap();
            assert_eq!(
                doc.pages().get(1).unwrap().annotations().count(),
                0,
                "the rest are flattened too"
            );
            doc.save_to(&out).unwrap();
        }
    }
    let reopened = Document::open(&out).unwrap();
    assert_eq!(
        reopened.pages().get(1).unwrap().annotations().count(),
        0,
        "and the saved file carries none"
    );
    let _ = std::fs::remove_file(&out);
}

#[test]
#[should_panic(expected = "annotation_parse_back")]
fn annotation_editor_modify_author_is_pending() {
    let mut editor = PdfAnnotationEditor::with_document(Document::open(hello_world()).unwrap());
    editor.modify_annotations_author(1, 1, "old", "new");
}

/// An unbound editor has no pages to walk, so every one of these is a
/// quiet no-op — there is no document to be wrong about.
#[test]
fn an_unbound_annotation_editor_does_nothing() {
    let mut editor = PdfAnnotationEditor::new();
    editor.delete_annotations();
    editor.delete_annotation("first");
    editor.flattening_annotations();
    assert!(editor.extract_annotations(1, 1, &["a"]).is_empty());
}

// =============================================================================
// PdfXmpMetadata — wired through the real foundation Metadata
// =============================================================================

#[test]
fn xmp_add_contains_count_through_bound_doc() {
    let mut xmp = PdfXmpMetadata::with_document(Document::open(with_info()).unwrap());
    xmp.add("dc:creator", XmpValue::from_string("LibForge")).unwrap();
    assert!(xmp.contains("dc:creator"));
    assert!(xmp.contains_key("dc:creator"));
    assert!(xmp.count() >= 1);
    assert!(!xmp.is_read_only());
    assert!(!xmp.is_fixed_size());

    // The mutation is visible on the underlying document Metadata.
    assert!(xmp.document().unwrap().metadata().contains_key("dc:creator"));
}

#[test]
fn xmp_try_get_value_and_indexer() {
    let mut xmp = PdfXmpMetadata::with_document(Document::open(with_info()).unwrap());
    xmp.add("dc:title", XmpValue::from_string("Hello XMP")).unwrap();

    let got = xmp.try_get_value("dc:title").unwrap();
    assert_eq!(got.to_string_value(), "Hello XMP");
    assert_eq!(xmp.get("dc:title").unwrap().to_string_value(), "Hello XMP");

    // Indexer assignment routes to the document Metadata.
    xmp.set("dc:title", XmpValue::from_string("Updated"));
    assert_eq!(xmp.get("dc:title").unwrap().to_string_value(), "Updated");
}

#[test]
fn xmp_keys_values_remove_clear() {
    let mut xmp = PdfXmpMetadata::with_document(Document::open(with_info()).unwrap());
    xmp.add("dc:a", XmpValue::from_string("1")).unwrap();
    xmp.add("dc:b", XmpValue::from_string("2")).unwrap();

    assert!(xmp.keys().len() >= 2);
    assert!(xmp.values().len() >= 2);

    assert!(xmp.remove("dc:a"));
    assert!(!xmp.contains("dc:a"));

    xmp.clear();
    assert_eq!(xmp.count(), 0);
}

#[test]
fn xmp_namespace_registry() {
    let mut xmp = PdfXmpMetadata::with_document(Document::open(with_info()).unwrap());
    let uri = "http://purl.org/dc/elements/1.1/";
    xmp.register_namespace_uri("dc", uri);
    assert_eq!(xmp.namespace_uri_by_prefix("dc"), uri);
    assert_eq!(xmp.prefix_by_namespace_uri(uri), "dc");
}

#[test]
fn xmp_save_then_reload_persists() {
    let out = common::unique_temp("aspose_rust_facades_nav_xmp_roundtrip.pdf");
    {
        let mut xmp = PdfXmpMetadata::with_document(Document::open(with_info()).unwrap());
        xmp.register_namespace_uri("dc", "http://purl.org/dc/elements/1.1/");
        xmp.add("dc:title", XmpValue::from_string("Facade Title")).unwrap();
        xmp.save(&out).unwrap();
    }
    assert!(out.exists());

    let reloaded = Document::open(&out).unwrap();
    assert_eq!(
        reloaded.metadata().get("dc:title").unwrap().to_string_value(),
        "Facade Title"
    );
    let _ = std::fs::remove_file(&out);
}

#[test]
fn xmp_unbound_reads_safe_mutations_error() {
    let mut xmp = PdfXmpMetadata::new();
    assert_eq!(xmp.count(), 0);
    assert!(!xmp.contains("k"));
    assert!(xmp.keys().is_empty());
    assert!(xmp.add("k", XmpValue::from_string("v")).is_err());
}

// =============================================================================
// FormEditor
// =============================================================================

#[test]
fn form_editor_add_field_real() {
    let mut editor = FormEditor::with_document(Document::open(hello_world()).unwrap());
    assert!(editor.add_field(FieldType::Text, "name1", 1, 100.0, 600.0, 300.0, 620.0));
    assert!(editor.add_field(FieldType::CheckBox, "agree", 1, 100.0, 560.0, 120.0, 580.0));
}

#[test]
fn form_editor_add_field_invalid_type_returns_false() {
    let mut editor = FormEditor::with_document(Document::open(hello_world()).unwrap());
    assert!(!editor.add_field(FieldType::InvalidNameOrType, "x", 1, 0.0, 0.0, 10.0, 10.0));
}

#[test]
fn form_editor_remove_field_real() {
    let mut editor = FormEditor::with_document(Document::open(hello_world()).unwrap());
    editor.add_field(FieldType::Text, "keep", 1, 100.0, 600.0, 300.0, 620.0);
    editor.add_field(FieldType::Text, "drop", 1, 100.0, 560.0, 300.0, 580.0);
    editor.remove_field("drop"); // real removal from the editor's Form
}

#[test]
fn form_editor_file_based_add_field_save() {
    let out = common::unique_temp("aspose_rust_facades_nav_formeditor_persist.pdf")
        .to_string_lossy()
        .into_owned();
    {
        let mut editor = FormEditor::with_files(hello_world(), &out).unwrap();
        assert!(editor.add_field(FieldType::Text, "myfield", 1, 100.0, 600.0, 300.0, 620.0));
        editor.save().unwrap();
    }
    assert!(Path::new(&out).exists());
    let _ = std::fs::remove_file(&out);
}

#[test]
fn form_editor_unbound_add_field_returns_false() {
    let mut editor = FormEditor::new();
    assert!(!editor.add_field(FieldType::Text, "x", 1, 0.0, 0.0, 10.0, 10.0));
    editor.remove_field("x"); // safe no-op
}

#[test]
fn form_editor_adds_a_field_and_resets() {
    let mut editor = FormEditor::with_document(Document::open(hello_world()).unwrap());
    editor.add_field(FieldType::Text, "f", 1, 100.0, 600.0, 300.0, 620.0);
    editor.reset_facade();
}


#[test]
fn form_editor_property_roundtrip() {
    let mut editor = FormEditor::new();
    editor.set_src_file_name("in.pdf");
    editor.set_dest_file_name("out.pdf");
    editor.set_radio_gap(2.5);
    editor.set_radio_horiz(true);
    editor.set_radio_button_item_size(15.0);
    editor.set_submit_flag_value(SubmitFormFlag::Pdf);
    editor.set_items(vec!["x".to_string(), "y".to_string()]);

    assert_eq!(editor.src_file_name(), "in.pdf");
    assert_eq!(editor.dest_file_name(), "out.pdf");
    assert_eq!(editor.radio_gap(), 2.5);
    assert!(editor.radio_horiz());
    assert_eq!(editor.radio_button_item_size(), 15.0);
    assert_eq!(editor.submit_flag(), SubmitFormFlag::Pdf);
    assert_eq!(editor.items().len(), 2);
    assert_eq!(editor.items()[0], "x");
}

// =============================================================================
// FormFieldFacade — appearance descriptor value type
// =============================================================================

#[test]
fn form_field_facade_canonical_constants() {
    assert_eq!(FormFieldFacade::BORDER_WIDTH_UNDEFINED, -1.0);
    assert_eq!(FormFieldFacade::BORDER_WIDTH_UNDIFIED, -1.0);
    assert_eq!(FormFieldFacade::BORDER_WIDTH_THIN, 1.0);
    assert_eq!(FormFieldFacade::BORDER_WIDTH_MEDIUM, 2.0);
    assert_eq!(FormFieldFacade::BORDER_WIDTH_THICK, 3.0);

    assert_eq!(FormFieldFacade::BORDER_STYLE_SOLID, 0);
    assert_eq!(FormFieldFacade::BORDER_STYLE_UNDERLINE, 4);
    assert_eq!(FormFieldFacade::BORDER_STYLE_UNDEFINED, 5);

    assert_eq!(FormFieldFacade::ALIGN_LEFT, 0);
    assert_eq!(FormFieldFacade::ALIGN_JUSTIFIED, 4);
    assert_eq!(FormFieldFacade::ALIGN_TOP, 0);
    assert_eq!(FormFieldFacade::ALIGN_BOTTOM, 2);

    assert_eq!(FormFieldFacade::CHECK_BOX_STYLE_CIRCLE, 108);
    assert_eq!(FormFieldFacade::CHECK_BOX_STYLE_CHECK, 52);
    assert_eq!(FormFieldFacade::CHECK_BOX_STYLE_CROSS, 56);
    assert_eq!(FormFieldFacade::CHECK_BOX_STYLE_DIAMOND, 117);
    assert_eq!(FormFieldFacade::CHECK_BOX_STYLE_STAR, 72);
    assert_eq!(FormFieldFacade::CHECK_BOX_STYLE_SQUARE, 110);
    assert_eq!(FormFieldFacade::CHECK_BOX_STYLE_UNDEFINED, 32);
}

#[test]
fn form_field_facade_defaults() {
    let f = FormFieldFacade::new();
    assert_eq!(f.border_style(), FormFieldFacade::BORDER_STYLE_UNDEFINED);
    assert_eq!(f.border_width(), FormFieldFacade::BORDER_WIDTH_UNDEFINED);
    assert_eq!(f.font(), FontStyle::Courier);
    assert!(f.custom_font().is_empty());
    assert_eq!(f.font_size(), 0.0);
    assert_eq!(f.text_encoding(), EncodingType::Identity_h);
    assert_eq!(f.alignment(), FormFieldFacade::ALIGN_UNDEFINED);
    assert_eq!(f.rotation(), 0);
    assert!(f.caption().is_empty());
    assert_eq!(f.button_style(), FormFieldFacade::CHECK_BOX_STYLE_UNDEFINED);
    assert!(f.position().is_empty());
    assert_eq!(f.page_number(), 1);
    assert!(f.items().is_empty());
    assert!(f.export_items().is_empty());
}

#[test]
fn form_field_facade_property_roundtrip() {
    let mut f = FormFieldFacade::new();
    f.set_border_style(FormFieldFacade::BORDER_STYLE_BEVELED);
    f.set_border_width(FormFieldFacade::BORDER_WIDTH_THICK);
    f.set_font(FontStyle::CourierBold);
    f.set_custom_font("MyFont");
    f.set_font_size(12.5);
    f.set_text_encoding(EncodingType::Cp1250);
    f.set_alignment(FormFieldFacade::ALIGN_CENTER);
    f.set_rotation(90);
    f.set_caption("Submit");
    f.set_button_style(FormFieldFacade::CHECK_BOX_STYLE_STAR);
    f.set_position(vec![1.0, 2.0, 3.0, 4.0]);
    f.set_page_number(3);
    f.set_items(vec!["A".to_string(), "B".to_string()]);
    f.set_export_items(vec![
        vec!["x".to_string(), "1".to_string()],
        vec!["y".to_string(), "2".to_string()],
    ]);

    assert_eq!(f.border_style(), FormFieldFacade::BORDER_STYLE_BEVELED);
    assert_eq!(f.border_width(), 3.0);
    assert_eq!(f.font(), FontStyle::CourierBold);
    assert_eq!(f.custom_font(), "MyFont");
    assert_eq!(f.font_size(), 12.5);
    assert_eq!(f.text_encoding(), EncodingType::Cp1250);
    assert_eq!(f.alignment(), FormFieldFacade::ALIGN_CENTER);
    assert_eq!(f.rotation(), 90);
    assert_eq!(f.caption(), "Submit");
    assert_eq!(f.button_style(), FormFieldFacade::CHECK_BOX_STYLE_STAR);
    assert_eq!(f.position().len(), 4);
    assert_eq!(f.position()[2], 3.0);
    assert_eq!(f.page_number(), 3);
    assert_eq!(f.items().len(), 2);
    assert_eq!(f.items()[1], "B");
    assert_eq!(f.export_items().len(), 2);
    assert_eq!(f.export_items()[0][0], "x");
}

#[test]
fn form_field_facade_reset_restores_defaults() {
    let mut f = FormFieldFacade::new();
    f.set_font_size(20.0);
    f.set_caption("temp");
    f.set_page_number(9);
    f.set_items(vec!["z".to_string()]);

    f.reset();
    assert_eq!(f.font_size(), 0.0);
    assert!(f.caption().is_empty());
    assert_eq!(f.page_number(), 1);
    assert!(f.items().is_empty());
}

// =============================================================================
// Foundation — facade base contract + the type-safe alignment enum classes
// =============================================================================

#[test]
fn alignment_type_named_instances() {
    assert_eq!(AlignmentType::left().to_string(), "Left");
    assert_eq!(AlignmentType::center().to_string(), "Center");
    assert_eq!(AlignmentType::right().to_string(), "Right");

    assert_eq!(AlignmentType::left(), AlignmentType::left());
    assert_ne!(AlignmentType::left(), AlignmentType::center());

    let custom = AlignmentType::new("Justify");
    assert_eq!(custom.to_string(), "Justify");
    assert_ne!(custom, AlignmentType::left());
}

#[test]
fn vertical_alignment_type_named_instances() {
    assert_eq!(VerticalAlignmentType::top().to_string(), "Top");
    assert_eq!(VerticalAlignmentType::center().to_string(), "Center");
    assert_eq!(VerticalAlignmentType::bottom().to_string(), "Bottom");

    assert_ne!(VerticalAlignmentType::top(), VerticalAlignmentType::bottom());
}

#[test]
fn facade_bind_by_document() {
    let mut f = Facade::new();
    assert!(f.document().is_none());
    f.bind_pdf_document(Document::new());
    assert!(f.document().is_some());
    f.close();
    assert!(f.document().is_none());
}

#[test]
fn facade_bind_by_file() {
    let mut f = Facade::new();
    f.bind_pdf(hello_world()).unwrap();
    assert!(f.document().is_some());
    assert!(f.document().unwrap().pages().count() > 0);
    f.dispose(); // alias for close
    assert!(f.document().is_none());
}

#[test]
fn saveable_facade_roundtrip() {
    let mut sf = SaveableFacade::new();
    sf.bind_pdf(hello_world()).unwrap();

    let out = common::unique_temp("aspose_rust_facades_nav_facade_save.pdf");
    sf.save(&out).unwrap();

    // Reload the saved file through a fresh Document to confirm it parses.
    let reloaded = Document::open(&out).unwrap();
    assert!(reloaded.pages().count() > 0);
    let _ = std::fs::remove_file(&out);
}

#[test]
fn saveable_facade_without_bind_errors() {
    let sf = SaveableFacade::new();
    assert!(sf.save(common::unique_temp("never.pdf")).is_err());
}

/// A field added through the editor used to go into a form of the
/// editor's own, while Save wrote the bound document — so the field
/// never reached the file. It now goes into the document's form, which
/// is the one the save path walks.
#[test]
fn a_field_added_through_the_editor_reaches_the_saved_file() {
    let out = common::unique_temp("aspose_rust_form_editor.pdf").to_string_lossy().into_owned();
    {
        let mut editor = FormEditor::with_document_and_dest(
            Document::open(hello_world()).unwrap(),
            out.clone(),
        );
        assert!(editor.add_field(FieldType::Text, "surname", 1, 100.0, 600.0, 300.0, 620.0));
        assert!(editor.add_field(FieldType::CheckBox, "agree", 1, 100.0, 560.0, 120.0, 580.0));
        editor.remove_field("agree");
        editor.save().unwrap();
    }
    let saved = std::fs::read(&out).unwrap();
    let text = String::from_utf8_lossy(&saved);
    assert!(text.contains("/AcroForm"), "the form reaches the file");
    assert!(text.contains("(surname)"), "with the field that was added");
    assert!(!text.contains("(agree)"), "and without the one removed");

    // And it is a field, not just a string: reopening finds it.
    let reopened = Document::open(&out).unwrap();
    assert!(reopened.form().has_field_by_name("surname"));
    assert_eq!(reopened.form().count(), 1);
    let _ = std::fs::remove_file(&out);
}

/// The facade is the editor's appearance carrier, and nothing read it:
/// the border, the font and the check-box mark were set on the editor
/// and the field came out with none of them. The three that reach the
/// file are now applied to the field the editor adds.
#[test]
fn the_editors_facade_reaches_the_field_it_adds() {
    use aspose_pdf_foss::{BorderStyle, FontStyle};

    let out = common::unique_temp("aspose_rust_facade_applied.pdf")
        .to_string_lossy()
        .into_owned();
    {
        let mut editor = FormEditor::with_document_and_dest(
            Document::open(hello_world()).unwrap(),
            out.clone(),
        );
        let mut facade = FormFieldFacade::new();
        facade.set_border_width(FormFieldFacade::BORDER_WIDTH_THICK);
        facade.set_border_style(FormFieldFacade::BORDER_STYLE_DASHED);
        facade.set_font(FontStyle::TimesBold);
        facade.set_font_size(14.0);
        facade.set_button_style(FormFieldFacade::CHECK_BOX_STYLE_CROSS);
        editor.set_facade(facade);

        assert!(editor.add_field(FieldType::Text, "styled", 1, 10.0, 10.0, 200.0, 40.0));
        assert!(editor.add_field(FieldType::CheckBox, "ticked", 1, 10.0, 50.0, 30.0, 70.0));
        editor.save().unwrap();
    }
    let text = String::from_utf8_lossy(&std::fs::read(&out).unwrap()).into_owned();

    // The border reaches the file as the §12.5.4 style dictionary.
    assert!(text.contains("/W 3"), "the facade's border width");
    assert!(text.contains("/S /D"), "and its dashed style");
    // The font and size reach it as the field's /DA.
    assert!(text.contains("/DA (/Times-Bold 14 Tf 0 g)"), "the facade's font");
    // The check-box mark is the /MK caption, a ZapfDingbats character.
    assert!(text.contains("/CA (8)"), "a cross is the character 8");

    // Sanity: the enum the facade's constant maps onto.
    assert_eq!(BorderStyle::Dashed as i32, FormFieldFacade::BORDER_STYLE_DASHED);
    let _ = std::fs::remove_file(&out);
}

/// Eleven of the editor's configuration methods panicked on
/// `form_field_configuration`. They are implementable now because the
/// field writer learned what they need: all three Table 226 bits, the
/// comb flag, /MaxLen, /Opt and the annotation /F. Each answers whether
/// it was applied, which is what the canonical `bool` means.
#[test]
fn the_editor_configures_a_field_it_can_find() {
    use aspose_pdf_foss::{AnnotationFlags, PropertyFlag};

    let out = common::unique_temp("aspose_rust_editor_config.pdf")
        .to_string_lossy()
        .into_owned();
    {
        let mut editor = FormEditor::with_document_and_dest(
            Document::open(hello_world()).unwrap(),
            out.clone(),
        );
        editor.add_field(FieldType::Text, "code", 1, 10.0, 10.0, 200.0, 40.0);
        editor.add_field(FieldType::ComboBox, "country", 1, 10.0, 50.0, 200.0, 70.0);

        // Flags, limits and geometry on the text field.
        assert!(editor.set_field_attribute("code", PropertyFlag::Required));
        assert!(editor.set_field_attribute("code", PropertyFlag::NoExport));
        assert!(editor.set_field_comb_number("code", 6));
        assert!(editor.move_field("code", 20.0, 25.0, 220.0, 55.0));
        assert!(editor.set_field_appearance("code", AnnotationFlags::NoZoom));
        assert_eq!(editor.get_field_appearance("code"), AnnotationFlags::NoZoom);

        // Options on the choice field.
        editor.add_list_items("country", vec!["Ukraine".into(), "Poland".into()]);
        editor.add_list_item("country", "Georgia");
        editor.del_list_item("country", "Poland");

        editor.rename_field("code", "pin");
        editor.save().unwrap();
    }
    let text = String::from_utf8_lossy(&std::fs::read(&out).unwrap()).into_owned();

    // Required (1 << 1) | NoExport (1 << 2) | Comb (1 << 24) = 16777222.
    assert!(text.contains("/Ff 16777222"), "the three bits and the comb flag");
    assert!(text.contains("/MaxLen 6"), "the comb cell count is the limit");
    assert!(text.contains("/Rect [ 20.0 25.0 220.0 55.0 ]"), "the new rectangle");
    assert!(text.contains("/F 8"), "the appearance flags, not the hard-coded 4");
    assert!(text.contains("(pin)"), "the new name");
    assert!(!text.contains("(code)"), "and not the old one");
    assert!(text.contains("(Ukraine)") && text.contains("(Georgia)"), "the options kept");
    assert!(!text.contains("(Poland)"), "and the one deleted is gone");
    let _ = std::fs::remove_file(&out);
}

/// Each answers `false` rather than failing when there is nothing to
/// apply it to: an absent name, or a family the request means nothing
/// to. A button has no character limit and no options.
#[test]
fn a_configuration_request_with_no_target_is_declined() {
    use aspose_pdf_foss::PropertyFlag;

    let mut editor = FormEditor::with_document(Document::open(hello_world()).unwrap());
    editor.add_field(FieldType::PushButton, "go", 1, 10.0, 10.0, 60.0, 30.0);

    // No such field.
    assert!(!editor.set_field_attribute("absent", PropertyFlag::ReadOnly));
    assert!(!editor.set_field_limit("absent", 5));
    assert!(!editor.move_field("absent", 0.0, 0.0, 1.0, 1.0));
    // The flag that names no property.
    assert!(!editor.set_field_attribute("go", PropertyFlag::InvalidFlag));
    // A button understands neither a limit nor multiline.
    assert!(!editor.set_field_limit("go", 5));
    assert!(!editor.single_to_multiple("go"));
    // And an unbound editor reports the default appearance.
    let bare = FormEditor::new();
    assert_eq!(
        bare.get_field_appearance("anything"),
        aspose_pdf_foss::AnnotationFlags::Default
    );
}

/// A field with no appearance flags of its own is still written as
/// Print, which is what a form field is for — the hard-coded 4 was
/// right for the default case and wrong for every other.
#[test]
fn a_field_with_no_flags_is_still_printed() {
    use aspose_pdf_foss::{Field, TextBoxField};

    let mut doc = Document::open(hello_world()).unwrap();
    let mut tf = TextBoxField::new(&doc);
    tf.set_partial_name("plain".into());
    doc.form_mut().add_at(Box::new(tf), 1);
    let mut out = Vec::new();
    doc.save_to_writer(&mut out).unwrap();
    assert!(
        String::from_utf8_lossy(&out).contains("/F 4"),
        "the default stays Print"
    );
}

/// The action group: a field's script, a button's submit target and
/// format, and removing an action again. All six panicked before; the
/// widget `/A` they need has been written since 2 Oct.
#[test]
fn the_editor_sets_a_fields_script_and_submit_action() {
    use aspose_pdf_foss::SubmitFormFlag;

    let out = common::unique_temp("aspose_rust_editor_actions.pdf")
        .to_string_lossy()
        .into_owned();
    {
        let mut editor = FormEditor::with_document_and_dest(
            Document::open(hello_world()).unwrap(),
            out.clone(),
        );
        editor.add_field(FieldType::Text, "total", 1, 10.0, 10.0, 200.0, 40.0);
        assert!(editor.set_field_script("total", "app.alert('one')"));
        // /A holds one action, so a second script joins the first.
        assert!(editor.add_field_script("total", "app.alert('two')"));

        editor.set_submit_flag_value(SubmitFormFlag::Xfdf);
        editor.add_submit_btn("send", 1, "Send it", "https://example.test/post", 10.0, 50.0, 90.0, 70.0);

        // A field whose action is dropped keeps the field.
        editor.add_field(FieldType::Text, "quiet", 1, 10.0, 80.0, 200.0, 100.0);
        editor.set_field_script("quiet", "app.alert('gone')");
        editor.remove_field_action("quiet");
        editor.save().unwrap();
    }
    let text = String::from_utf8_lossy(&std::fs::read(&out).unwrap()).into_owned();

    assert!(text.contains("/S /JavaScript"), "the script action");
    // The parentheses come out escaped inside a literal string
    // (§7.3.4.2), so the two scripts are matched by their arguments
    // and by their order rather than by the whole text.
    let first = text.find("'one'").expect("the first script");
    let second = text.find("'two'").expect("the second script");
    assert!(first < second, "both scripts, in the order they were added");
    assert!(text.contains("/S /SubmitForm"), "the submit action");
    assert!(text.contains("(https://example.test/post)"), "its target");
    assert!(text.contains("/Flags 32"), "XFDF is bit 6 of Table 237");
    assert!(text.contains("/CA (Send it)"), "the button's caption");
    assert!(text.contains("(quiet)"), "the field whose action was removed stays");
    assert_eq!(
        text.matches("/S /JavaScript").count(),
        1,
        "and its script is gone, leaving one script in the file"
    );
    let _ = std::fs::remove_file(&out);
}

/// Changing the format alone keeps the URL the button already submits
/// to — the editor reads it back off the action rather than losing it.
#[test]
fn changing_the_submit_format_keeps_the_url() {
    use aspose_pdf_foss::SubmitFormFlag;

    let out = common::unique_temp("aspose_rust_editor_submit_fmt.pdf")
        .to_string_lossy()
        .into_owned();
    {
        let mut editor = FormEditor::with_document_and_dest(
            Document::open(hello_world()).unwrap(),
            out.clone(),
        );
        editor.add_field(FieldType::PushButton, "send", 1, 10.0, 10.0, 90.0, 30.0);
        assert!(editor.set_submit_url("send", "https://example.test/keep"));
        assert!(editor.set_submit_flag("send", SubmitFormFlag::Pdf));
        editor.save().unwrap();
    }
    let text = String::from_utf8_lossy(&std::fs::read(&out).unwrap()).into_owned();
    assert!(text.contains("(https://example.test/keep)"), "the URL survives");
    assert!(text.contains("/Flags 256"), "and the new format applies");
    let _ = std::fs::remove_file(&out);
}

/// Alignment and the three decorate overloads. /Q is a field entry
/// (§12.7.4.1) with three values, so justified and undefined are
/// declined, and vertical alignment is declined outright — the format
/// has no field entry for it.
#[test]
fn the_editor_aligns_and_decorates_existing_fields() {
    use aspose_pdf_foss::{FontStyle, PropertyFlag};

    let out = common::unique_temp("aspose_rust_editor_decorate.pdf")
        .to_string_lossy()
        .into_owned();
    {
        let mut editor = FormEditor::with_document_and_dest(
            Document::open(hello_world()).unwrap(),
            out.clone(),
        );
        editor.add_field(FieldType::Text, "centred", 1, 10.0, 10.0, 200.0, 40.0);
        editor.add_field(FieldType::ComboBox, "choice", 1, 10.0, 50.0, 200.0, 70.0);

        assert!(editor.set_field_alignment("centred", FormFieldFacade::ALIGN_CENTER));
        assert!(!editor.set_field_alignment("centred", FormFieldFacade::ALIGN_JUSTIFIED));
        assert!(!editor.set_field_alignment("centred", FormFieldFacade::ALIGN_UNDEFINED));
        assert!(!editor.set_field_alignment_v("centred", FormFieldFacade::ALIGN_MIDDLE));

        // The facade is applied to the text field only: asking for the
        // Text type reaches every /Tx field, and the combo box is /Ch.
        let mut facade = FormFieldFacade::new();
        facade.set_border_width(FormFieldFacade::BORDER_WIDTH_THICK);
        facade.set_font(FontStyle::Courier);
        facade.set_font_size(11.0);
        editor.set_facade(facade);
        editor.decorate_field_of_type(FieldType::Text);
        editor.save().unwrap();
    }
    let text = String::from_utf8_lossy(&std::fs::read(&out).unwrap()).into_owned();

    assert!(text.contains("/Q 1"), "centred quadding");
    assert_eq!(text.matches("/W 3").count(), 1, "one field was decorated");
    assert_eq!(
        text.matches("/DA (/Courier 11 Tf 0 g)").count(),
        1,
        "and it is the /Tx one"
    );

    // Decorating everything reaches the choice field too.
    let out2 = common::unique_temp("aspose_rust_editor_decorate_all.pdf")
        .to_string_lossy()
        .into_owned();
    {
        let mut editor = FormEditor::with_document_and_dest(
            Document::open(hello_world()).unwrap(),
            out2.clone(),
        );
        editor.add_field(FieldType::Text, "a", 1, 10.0, 10.0, 200.0, 40.0);
        editor.add_field(FieldType::ComboBox, "b", 1, 10.0, 50.0, 200.0, 70.0);
        let mut facade = FormFieldFacade::new();
        facade.set_border_width(FormFieldFacade::BORDER_WIDTH_MEDIUM);
        editor.set_facade(facade);
        editor.decorate_facade();
        // A named field can be decorated on its own as well.
        editor.set_field_attribute("a", PropertyFlag::ReadOnly);
        editor.decorate_field("a");
        editor.save().unwrap();
    }
    let all = String::from_utf8_lossy(&std::fs::read(&out2).unwrap()).into_owned();
    assert_eq!(all.matches("/W 2").count(), 2, "both fields were decorated");

    let _ = std::fs::remove_file(&out);
    let _ = std::fs::remove_file(&out2);
}

/// The extract / delete family. An extracted annotation is a
/// standalone copy carrying every entry its source would be written
/// with, which is what makes it writable into another document; it is
/// not of the source's concrete type, because nothing can build one of
/// those from a `&dyn Annotation`.
#[test]
fn the_annotation_editor_extracts_and_deletes_by_name_and_type() {
    use aspose_pdf_foss::{
        Annotation, AnnotationType, CaretAnnotation, CommonFigureAnnotation, LinkAnnotation,
        Rectangle, SquareAnnotation,
    };

    let doc = Document::open(hello_world()).unwrap();
    {
        let page = doc.pages().get(1).unwrap();
        let r = Rectangle::new(10.0, 10.0, 40.0, 40.0, false);
        let mut caret = CaretAnnotation::on_page(&page, r);
        caret.set_name("c1".into());
        page.annotations().add(Box::new(caret));
        let mut square = SquareAnnotation::on_page(&page, r);
        square.set_name("s1".into());
        square.set_interior_color(aspose_pdf_foss::Color::from_rgb(1.0, 0.0, 0.0));
        page.annotations().add(Box::new(square));
        page.annotations().add(Box::new(LinkAnnotation::on_page(&page, r)));
        assert_eq!(page.annotations().count(), 3);
    }

    let mut editor = PdfAnnotationEditor::with_document(doc);

    // By type, and the copy carries the source's own entries.
    let squares = editor.extract_annotations_with_types(1, 1, &[AnnotationType::Square]);
    assert_eq!(squares.len(), 1);
    assert_eq!(squares[0].annotation_type(), AnnotationType::Square);
    assert_eq!(squares[0].name(), "s1");

    // By name, and an empty list extracts nothing.
    assert_eq!(editor.extract_annotations(1, 1, &["c1"]).len(), 1);
    assert_eq!(editor.extract_annotations(1, 1, &[]).len(), 0);
    // A page range outside the document yields nothing rather than failing.
    assert_eq!(editor.extract_annotations(5, 9, &["c1"]).len(), 0);

    // Deleting by name takes one; deleting everything takes the rest.
    editor.delete_annotation("c1");
    {
        let doc = editor.document().unwrap();
        assert_eq!(doc.pages().get(1).unwrap().annotations().count(), 2);
    }
    editor.delete_annotations();
    {
        let doc = editor.document().unwrap();
        assert_eq!(doc.pages().get(1).unwrap().annotations().count(), 0);
    }
}

/// A link annotation extracted through the content editor keeps its
/// action, which is the entry that makes a link a link.
#[test]
fn an_extracted_link_keeps_its_action() {
    use aspose_pdf_foss::{
        AnnotationType, GoToURIAction, LinkAnnotation, PdfContentEditor, Rectangle,
    };

    let out = common::unique_temp("aspose_rust_extract_link.pdf")
        .to_string_lossy()
        .into_owned();
    {
        let doc = Document::open(hello_world()).unwrap();
        {
            let page = doc.pages().get(1).unwrap();
            let mut link =
                LinkAnnotation::on_page(&page, Rectangle::new(10.0, 10.0, 80.0, 30.0, false));
            link.set_action(Box::new(GoToURIAction::new("https://example.test/go".into())));
            page.annotations().add(Box::new(link));
        }
        let editor = PdfContentEditor::with_document(doc);
        let links = editor.extract_link();
        assert_eq!(links.len(), 1, "the one link");
        assert_eq!(links[0].annotation_type(), AnnotationType::Link);

        // Writing the copy into another document reproduces the action.
        let other = Document::open(hello_world()).unwrap();
        {
            let page = other.pages().get(1).unwrap();
            for link in links {
                page.annotations().add(link);
            }
        }
        other.save_to(&out).unwrap();
    }
    let text = String::from_utf8_lossy(&std::fs::read(&out).unwrap()).into_owned();
    assert!(text.contains("/S /URI"), "the copied link still acts");
    assert!(text.contains("https://example.test/go"), "on the same target");
    let _ = std::fs::remove_file(&out);
}

#[path = "../common/mod.rs"]
mod common;
