// Public-API smoke driver — A-5 annotation families.
//
// Sound / Redaction (markup) + Movie / Screen / Watermark (direct).
// Behaviour from the cpp media / specialty family smoke tests, using
// canonical enum ordinals.

use aspose_pdf_foss::{
    Annotation, AnnotationType, Document, MovieAnnotation, Point, RedactionAnnotation,
    Rectangle, ScreenAnnotation, SoundAnnotation, SoundEncoding, SoundIcon, SoundSampleData,
    SoundSampleDataEncodingFormat, TextState, WatermarkAnnotation,
};
use std::path::PathBuf;

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(name)
}

fn first_page_doc() -> Document {
    Document::open(fixture("pages_tree/single.pdf")).unwrap()
}

#[test]
fn movie_construct_and_accessors() {
    let doc = Document::new();
    let mut m = MovieAnnotation::new(&doc, "clip.mp4".into());
    assert_eq!(m.annotation_type(), AnnotationType::Movie);
    assert!(!m.poster());
    assert_eq!(m.rotate(), 0);
    assert_eq!(m.file().name(), "clip.mp4");
    m.set_title("Demo".into());
    m.set_poster(true);
    m.set_rotate(90);
    m.set_aspect(Point::new(16.0, 9.0));
    assert_eq!(m.title(), "Demo");
    assert!(m.poster());
    assert_eq!(m.rotate(), 90);
    assert_eq!(m.aspect().x(), 16.0);
}

#[test]
fn redaction_construct_and_accessors() {
    let doc = Document::new();
    let mut r = RedactionAnnotation::new(&doc);
    assert_eq!(r.annotation_type(), AnnotationType::Redaction);
    assert_eq!(r.fill_color().a(), 1.0); // default opaque
    assert!(!r.repeat());
    assert_eq!(r.font_size(), 0.0);
    r.set_overlay_text("[REDACTED]".into());
    r.set_font_size(10.0);
    r.set_repeat(true);
    assert_eq!(r.overlay_text(), "[REDACTED]");
    assert_eq!(r.font_size(), 10.0);
    assert!(r.repeat());
    r.redact(); // no-op in the in-memory model
}

#[test]
fn sound_construct() {
    let doc = first_page_doc();
    let page = doc.pages().get(1).unwrap();
    let mut s = SoundAnnotation::on_page(&page, Rectangle::new(0.0, 0.0, 20.0, 20.0, false), "a.wav".into());
    assert_eq!(s.annotation_type(), AnnotationType::Sound);
    assert_eq!(s.icon(), SoundIcon::Speaker);
    s.set_icon(SoundIcon::Mic);
    assert_eq!(s.icon(), SoundIcon::Mic);
    // No sample data supplied → default (silent) sound parameters.
    assert_eq!(s.sound_data().rate(), 0);
    assert_eq!(s.sound_data().encoding(), SoundEncoding::Raw);
}

#[test]
fn sound_construct_with_sample_data() {
    let doc = first_page_doc();
    let page = doc.pages().get(1).unwrap();
    let sample =
        SoundSampleData::with_encoding(44_100, 2, 16, SoundSampleDataEncodingFormat::Signed);
    let s = SoundAnnotation::on_page_with_sample_data(
        &page,
        Rectangle::new(0.0, 0.0, 20.0, 20.0, false),
        "a.wav".into(),
        sample,
    );
    assert_eq!(s.annotation_type(), AnnotationType::Sound);
    // SoundData is derived from the supplied sample parameters.
    assert_eq!(s.sound_data().rate(), 44_100);
    assert_eq!(s.sound_data().channels(), 2);
    assert_eq!(s.sound_data().bits(), 16);
    assert_eq!(s.sound_data().encoding(), SoundEncoding::Signed);
}

#[test]
fn screen_construct() {
    let doc = first_page_doc();
    let page = doc.pages().get(1).unwrap();
    let mut sc = ScreenAnnotation::on_page(&page, Rectangle::new(0.0, 0.0, 20.0, 20.0, false), "r.mp4".into());
    assert_eq!(sc.annotation_type(), AnnotationType::Screen);
    assert!(sc.action().is_none());
    sc.set_title("rendition".into());
    assert_eq!(sc.title(), "rendition");
}

#[test]
fn watermark_construct() {
    let doc = first_page_doc();
    let page = doc.pages().get(1).unwrap();
    let mut w = WatermarkAnnotation::on_page(&page, Rectangle::new(0.0, 0.0, 50.0, 50.0, false));
    assert_eq!(w.annotation_type(), AnnotationType::Watermark);
    assert_eq!(w.opacity(), 1.0);
    w.set_opacity(0.3);
    assert_eq!(w.opacity(), 0.3);
    // SetTextAndState records the multi-line watermark text + styling.
    w.set_text_and_state(
        vec!["CONFIDENTIAL".to_string(), "DRAFT".to_string()],
        TextState::default(),
    );
    // FixedPrint defaults to identity + zero translation.
    assert_eq!(w.fixed_print().horizontal_translation(), 0.0);
    assert_eq!(w.fixed_print().vertical_translation(), 0.0);
}

/// §12.5.6.17 marks /Movie required: it is the clip the annotation
/// plays. The module declared the dictionary unwritten, but every entry
/// of it — the file, the aspect, the rotation, the poster — is a plain
/// dictionary value needing no second object, so the declaration was
/// the thing that was wrong.
#[test]
fn a_movie_annotation_carries_its_clip() {
    let doc = Document::open(fixture("pages_tree/single.pdf")).unwrap();
    {
        let page = doc.pages().get(1).unwrap();
        let mut m = MovieAnnotation::on_page(
            &page,
            Rectangle::new(0.0, 0.0, 320.0, 240.0, false),
            "clip.mp4".into(),
        );
        m.set_title("the clip".into());
        m.set_aspect(Point::new(320.0, 240.0));
        m.set_rotate(90);
        m.set_poster(true);
        page.annotations().add(Box::new(m));
    }
    let mut out = Vec::new();
    doc.save_to_writer(&mut out).unwrap();
    let text = String::from_utf8_lossy(&out);

    assert!(text.contains("/T (the clip)"), "the title");
    assert!(text.contains("/Movie <<"), "the required clip dictionary");
    assert!(text.contains("(clip.mp4)"), "naming the media file");
    assert!(text.contains("/Aspect [ 320 240 ]"), "the clip's dimensions");
    assert!(text.contains("/Rotate 90"), "its rotation");
    assert!(text.contains("/Poster true"), "and the poster frame");
}

/// The same reach for a watermark's fixed print: get-only, the five
/// placement setters had no way to be called.
#[test]
fn a_watermarks_fixed_print_can_be_configured() {
    use aspose_pdf_foss::Matrix;

    let doc = Document::open(fixture("pages_tree/single.pdf")).unwrap();
    let page = doc.pages().get(1).unwrap();
    let mut w = WatermarkAnnotation::on_page(&page, Rectangle::new(0.0, 0.0, 50.0, 50.0, false));
    w.fixed_print_mut()
        .set_matrix(Matrix::new(2.0, 0.0, 0.0, 2.0, 0.0, 0.0));
    w.fixed_print_mut().set_horizontal_translation(0.25);
    w.fixed_print_mut().set_vertical_translation(-0.5);

    assert_eq!(w.fixed_print().matrix().a(), 2.0);
    assert_eq!(w.fixed_print().horizontal_translation(), 0.25);
    assert_eq!(w.fixed_print().vertical_translation(), -0.5);
}
