// Public-API driver — authoring a document that does not come from a
// file: `Document::new()` + `PageCollection::add` + content.
//
// A page created in-session has no leaf object until the save path emits
// one, so everything authored onto it — text, tables, drawings, widgets,
// images, geometry — is recorded against an id that does not exist yet.
// These tests pin that path end to end: the content must reach the saved
// file, on the page it was authored on, at the size that page reports.

use aspose_pdf_foss::{
    Annotation, Color, Document, DrawingRectangle, Graph, GraphInfo, Position, Shape,
    SquareAnnotation, Rectangle, TextAbsorber, TextBoxField, TextBuilder, TextFragment,
    TextParagraph, Field, HorizontalAlignment,
};

fn contains(haystack: &[u8], needle: &[u8]) -> bool {
    haystack.windows(needle.len()).any(|w| w == needle)
}

fn save(doc: &Document) -> Vec<u8> {
    let mut out = Vec::new();
    doc.save_to_writer(&mut out).expect("save");
    out
}

#[test]
fn new_document_page_carries_its_text() {
    let doc = Document::new();
    let page = doc.pages().add();
    let mut fragment = TextFragment::with_text("FromScratch");
    fragment.text_state_mut().set_font_size(18.0);
    fragment.set_position(Position::new(72.0, 700.0));
    page.paragraphs().add(Box::new(fragment));

    let out = save(&doc);
    assert!(contains(&out, b"(FromScratch)"), "text missing from the saved page");
    assert!(contains(&out, b"/Contents"), "the synthesised leaf carries no /Contents");

    let reopened = Document::from_bytes(out).expect("reopen");
    assert_eq!(reopened.pages().count(), 1);
    let mut absorber = TextAbsorber::new();
    absorber.visit_document(&reopened).expect("visit");
    assert!(absorber.text().contains("FromScratch"), "got: {:?}", absorber.text());
}

#[test]
fn page_added_to_an_opened_document_carries_its_text() {
    let doc = Document::open(fixture()).expect("open");
    let before = doc.pages().count();
    let page = doc.pages().add();
    let mut fragment = TextFragment::with_text("AddedPageText");
    fragment.set_position(Position::new(72.0, 700.0));
    page.paragraphs().add(Box::new(fragment));

    let out = save(&doc);
    assert!(contains(&out, b"(AddedPageText)"), "text missing from the appended page");

    let reopened = Document::from_bytes(out).expect("reopen");
    assert_eq!(reopened.pages().count(), before + 1);
    let mut absorber = TextAbsorber::new();
    absorber.visit_page(&reopened.pages().get(before + 1).unwrap()).expect("visit");
    assert!(absorber.text().contains("AddedPageText"), "got: {:?}", absorber.text());
}

#[test]
fn drawing_on_a_created_page_reaches_the_content_stream() {
    let doc = Document::new();
    let page = doc.pages().add();
    let mut graph = Graph::new(200.0, 100.0);
    graph.set_left(50.0);
    graph.set_top(60.0);
    let mut rect = DrawingRectangle::new(0.0, 0.0, 80.0, 40.0);
    let mut info = GraphInfo::new();
    info.set_fill_color(Color::from_argb_rgb(0, 90, 160));
    rect.set_graph_info(info);
    graph.add_shape(Box::new(rect));
    page.paragraphs().add(Box::new(graph));

    let out = save(&doc);
    for op in [&b" re\n"[..], b"\nf\n", b" rg\n"] {
        assert!(contains(&out, op), "missing operator {:?}", std::str::from_utf8(op).unwrap());
    }
}

#[test]
fn text_builder_on_a_created_page_reaches_the_content_stream() {
    let doc = Document::new();
    let page = doc.pages().add();
    let mut para = TextParagraph::new();
    para.set_rectangle(Rectangle::new(72.0, 500.0, 300.0, 720.0, true));
    para.set_horizontal_alignment(HorizontalAlignment::Left);
    para.append_line("BuilderOnNewPage");
    TextBuilder::new(&page).append_paragraph(&para);

    let out = save(&doc);
    assert!(contains(&out, b"BuilderOnNewPage"), "TextBuilder output missing");
}

#[test]
fn created_pages_keep_their_own_content() {
    let doc = Document::new();
    let first = doc.pages().add();
    let second = doc.pages().add();
    assert_eq!(first.number(), 1);
    assert_eq!(second.number(), 2);

    for (page, text) in [(&first, "PageOneOnly"), (&second, "PageTwoOnly")] {
        let mut fragment = TextFragment::with_text(text);
        fragment.set_position(Position::new(72.0, 700.0));
        page.paragraphs().add(Box::new(fragment));
    }

    let reopened = Document::from_bytes(save(&doc)).expect("reopen");
    assert_eq!(reopened.pages().count(), 2);
    for (number, expected, unexpected) in
        [(1, "PageOneOnly", "PageTwoOnly"), (2, "PageTwoOnly", "PageOneOnly")]
    {
        let mut absorber = TextAbsorber::new();
        absorber.visit_page(&reopened.pages().get(number).unwrap()).expect("visit");
        assert!(absorber.text().contains(expected), "page {number}: {:?}", absorber.text());
        assert!(!absorber.text().contains(unexpected), "page {number} carries its neighbour's text");
    }
}

#[test]
fn annotations_and_widgets_land_on_their_created_page() {
    let mut doc = Document::new();
    let _first = doc.pages().add();
    let second = doc.pages().add();

    let mut square = SquareAnnotation::on_page(&second, Rectangle::new(72.0, 600.0, 172.0, 700.0, false));
    square.set_color(Color::from_argb_rgb(200, 40, 40));
    second.annotations().add(Box::new(square));

    let mut field = TextBoxField::on_page(&second, Rectangle::new(72.0, 500.0, 272.0, 520.0, false));
    field.set_partial_name("onsecond".into());
    field.set_value("value".into());
    doc.form_mut().add(Box::new(field));

    let reopened = Document::from_bytes(save(&doc)).expect("reopen");
    assert_eq!(reopened.pages().get(1).unwrap().annotations().count(), 0);
    // The square plus the field's widget, both on the page they were
    // authored against.
    assert_eq!(reopened.pages().get(2).unwrap().annotations().count(), 2);
    assert_eq!(reopened.form().count(), 1);
}

#[test]
fn image_is_drawn_into_the_page_content_stream() {
    // Embedding alone leaves the image invisible: a viewer paints an
    // image XObject only where the content stream invokes it. Assert the
    // operators, the placement transform, and the clip.
    let doc = Document::new();
    let page = doc.pages().add();
    page.add_image(
        &jpeg_fixture(),
        Rectangle::new(72.0, 200.0, 272.0, 500.0, false),
        None,
        false,
    )
    .expect("add_image");

    let out = save(&doc);
    assert!(contains(&out, b" Do\n"), "no image-invoking operator");
    // Stretched to fill: width 200, height 300, origin (72, 200).
    assert!(contains(&out, b"200 0 0 300 72 200 cm"), "wrong placement transform");

    let reopened = Document::from_bytes(out).expect("reopen");
    assert_eq!(reopened.pages().get(1).unwrap().resources().images().count(), 1);
}

#[test]
fn auto_adjust_fits_the_image_without_distorting_it() {
    // The fixture is square, so a wide rectangle must yield a square
    // placement centred inside it rather than a stretched one.
    let doc = Document::new();
    let page = doc.pages().add();
    page.add_image(
        &jpeg_fixture(),
        Rectangle::new(100.0, 100.0, 400.0, 200.0, false),
        None,
        true,
    )
    .expect("add_image");

    let out = save(&doc);
    // The fixture is 32x32, so the fit is square.
    let reopened = Document::from_bytes(out.clone()).expect("reopen");
    let saved_page = reopened.pages().get(1).unwrap();
    let images = saved_page.resources().images();
    let img = images.get(0).expect("the embedded image");
    assert_eq!(img.width(), img.height(), "fixture is expected to be square");
    drop(img);
    // Fits the 100-unit height, centred across the 300-unit width.
    assert!(contains(&out, b"100 0 0 100 200 100 cm"), "image was not fitted to the box");
}

#[test]
fn a_bounding_box_clips_the_drawing() {
    let doc = Document::new();
    let page = doc.pages().add();
    page.add_image(
        &jpeg_fixture(),
        Rectangle::new(72.0, 200.0, 272.0, 400.0, false),
        Some(Rectangle::new(100.0, 250.0, 200.0, 350.0, false)),
        false,
    )
    .expect("add_image");

    let out = save(&doc);
    assert!(contains(&out, b"100 250 100 100 re"), "clip rectangle missing");
    assert!(contains(&out, b"W\nn\n"), "clip operators missing");
}

fn jpeg_fixture() -> Vec<u8> {
    std::fs::read(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/dctdecode/gradient_32.jpg"),
    )
    .unwrap()
}

#[test]
fn image_embeds_into_a_created_page() {
    let jpeg = std::fs::read(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/dctdecode/gradient_32.jpg"),
    )
    .unwrap();
    let doc = Document::new();
    let page = doc.pages().add();
    page.add_image(&jpeg, Rectangle::new(72.0, 500.0, 272.0, 700.0, false), None, false)
        .expect("add_image");

    let reopened = Document::from_bytes(save(&doc)).expect("reopen");
    assert_eq!(reopened.pages().get(1).unwrap().resources().images().count(), 1);
}

#[test]
fn a_resized_page_reports_and_places_by_its_new_size() {
    let doc = Document::new();
    let page = doc.pages().add();
    page.set_page_size(792.0, 612.0);
    // The size accessors read the live geometry, not the size the page
    // was synthesised at — drawing placement is computed from them.
    assert_eq!(page.width(), 792.0);
    assert_eq!(page.height(), 612.0);

    let mut graph = Graph::new(100.0, 100.0);
    graph.set_left(20.0);
    graph.set_top(30.0);
    let mut rect = DrawingRectangle::new(0.0, 0.0, 100.0, 100.0);
    let mut info = GraphInfo::new();
    info.set_fill_color(Color::from_argb_rgb(0, 0, 0));
    rect.set_graph_info(info);
    graph.add_shape(Box::new(rect));
    page.paragraphs().add(Box::new(graph));

    let out = save(&doc);
    // Origin y = page height - top - graph height = 612 - 30 - 100.
    assert!(contains(&out, b"1 0 0 1 20 482 cm"), "graph placed against the stale page height");

    let reopened = Document::from_bytes(out).expect("reopen");
    assert_eq!(reopened.pages().get(1).unwrap().height(), 612.0);
}

fn fixture() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/text_extractor/two_pages.pdf")
}
