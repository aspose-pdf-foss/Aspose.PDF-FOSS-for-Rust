// Public-API smoke driver — A-4 annotation families.
//
// Ink / Stamp / Caret (markup) + Popup (direct) + FileAttachment
// (markup). Behaviour from the cpp stamp_caret_ink family smoke
// test, using canonical enum ordinals (the cpp test asserts a
// drifted CapStyle::Butt=0 — canonical has Rectangular=1/Rounded=2,
// so the default here is Rectangular).

use aspose_pdf_foss::{
    Annotation, AnnotationType, CapStyle, CaretAnnotation, CaretSymbol, Document,
    FileAttachmentAnnotation, FileIcon, FileSpecification, InkAnnotation, MarkupAnnotation, Point,
    PopupAnnotation, Rectangle, StampAnnotation, StampIcon,
};
use std::path::PathBuf;

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(name)
}

#[test]
fn stamp_construct_and_accessors() {
    let doc = Document::new();
    let mut s = StampAnnotation::new(&doc);
    assert_eq!(s.annotation_type(), AnnotationType::Stamp);
    assert_eq!(s.icon(), StampIcon::Draft);
    s.set_icon(StampIcon::Approved);
    assert_eq!(s.icon(), StampIcon::Approved);
    assert!(s.image().is_empty());
    s.set_image(&[1, 2, 3]);
    assert_eq!(s.image(), vec![1, 2, 3]);
}

#[test]
fn caret_construct_and_accessors() {
    let doc = Document::new();
    let mut c = CaretAnnotation::new(&doc);
    assert_eq!(c.annotation_type(), AnnotationType::Caret);
    assert_eq!(c.symbol(), CaretSymbol::None);
    c.set_symbol(CaretSymbol::Paragraph);
    assert_eq!(c.symbol(), CaretSymbol::Paragraph);
    c.set_frame(Rectangle::new(1.0, 2.0, 10.0, 20.0, false));
    assert_eq!(c.frame().llx(), 1.0);
    assert_eq!(c.frame().ury(), 20.0);
}

#[test]
fn ink_construct_and_accessors() {
    let doc = Document::new();
    let strokes = vec![
        vec![Point::new(0.0, 0.0), Point::new(5.0, 5.0), Point::new(8.0, 2.0)],
        vec![Point::new(10.0, 10.0), Point::new(12.0, 14.0)],
    ];
    let mut k = InkAnnotation::new(&doc, strokes);
    assert_eq!(k.annotation_type(), AnnotationType::Ink);
    assert_eq!(k.cap_style(), CapStyle::Rectangular); // canonical default
    assert_eq!(k.ink_list().len(), 2);
    assert_eq!(k.ink_list()[0].len(), 3);
    assert_eq!(k.ink_list()[1].len(), 2);
    assert_eq!(k.ink_list()[1][0].x(), 10.0);
    k.set_cap_style(CapStyle::Rounded);
    assert_eq!(k.cap_style(), CapStyle::Rounded);
}

#[test]
fn popup_construct_and_accessors() {
    let doc = Document::new();
    let mut p = PopupAnnotation::new(&doc);
    assert_eq!(p.annotation_type(), AnnotationType::Popup);
    assert!(!p.open());
    p.set_open(true);
    assert!(p.open());
    assert!(p.parent().is_none());
    // parent is any annotation
    let mut note = StampAnnotation::new(&doc);
    note.set_name("owner".into());
    p.set_parent(Box::new(note));
    assert_eq!(p.parent().unwrap().name(), "owner");
}

#[test]
fn file_attachment_construct_and_accessors() {
    let doc = Document::open(fixture("pages_tree/single.pdf")).unwrap();
    let page = doc.pages().get(1).unwrap();
    let spec = FileSpecification::with_file("/tmp/data.bin".into());
    let mut fa = FileAttachmentAnnotation::on_page(
        &page,
        Rectangle::new(0.0, 0.0, 20.0, 20.0, false),
        spec,
    );
    assert_eq!(fa.annotation_type(), AnnotationType::FileAttachment);
    assert_eq!(fa.icon(), FileIcon::PushPin);
    assert_eq!(fa.file().name(), "/tmp/data.bin");
    fa.set_icon(FileIcon::Paperclip);
    assert_eq!(fa.icon(), FileIcon::Paperclip);
    // Opacity inherited from the markup base.
    assert_eq!(fa.opacity(), 1.0);
}

/// An annotation's dictionary used to carry four entries — subtype,
/// rectangle, contents, name — and nothing else, so an ink annotation
/// reached the file with no strokes at all and a viewer had nothing to
/// draw. The defining entry of each subtype is now written (§12.5.6),
/// together with the colour and flags §12.5.2 gives every annotation.
#[test]
fn an_ink_annotation_carries_its_strokes() {
    let doc = Document::open(fixture("pages_tree/single.pdf")).unwrap();
    {
        let page = doc.pages().get(1).unwrap();
        let mut ink = InkAnnotation::new(
            &doc,
            vec![vec![Point::new(20.0, 30.0), Point::new(40.0, 50.0)]],
        );
        ink.set_rect(Rectangle::new(10.0, 10.0, 200.0, 200.0, false));
        ink.set_color(aspose_pdf_foss::Color::from_rgb(1.0, 0.0, 0.0));
        page.annotations().add(Box::new(ink));
    }
    let mut out = Vec::new();
    doc.save_to_writer(&mut out).unwrap();
    let text = String::from_utf8_lossy(&out);
    assert!(text.contains("/InkList"), "the strokes must reach the file");
    assert!(text.contains("20") && text.contains("30"), "with their coordinates");
    assert!(text.contains("/C ["), "and the colour a viewer draws it in");
}

#[test]
fn a_hidden_annotation_carries_its_flag() {
    use aspose_pdf_foss::AnnotationFlags;
    let doc = Document::open(fixture("pages_tree/single.pdf")).unwrap();
    {
        let page = doc.pages().get(1).unwrap();
        let mut caret = CaretAnnotation::new(&doc);
        caret.set_rect(Rectangle::new(1.0, 2.0, 3.0, 4.0, false));
        caret.set_flags(AnnotationFlags::Hidden);
        page.annotations().add(Box::new(caret));
    }
    let mut out = Vec::new();
    doc.save_to_writer(&mut out).unwrap();
    assert!(
        String::from_utf8_lossy(&out).contains("/F 2"),
        "a hidden annotation must say so, or a viewer shows it"
    );
}

/// The markup entries §12.5.6.2 defines are written once for every
/// subtype that carries a markup base, rather than per type: the
/// author, the subject, the reply type and the opacity.
#[test]
fn markup_entries_reach_the_file() {
    use aspose_pdf_foss::ReplyType;
    let doc = Document::open(fixture("pages_tree/single.pdf")).unwrap();
    {
        let page = doc.pages().get(1).unwrap();
        let mut caret = CaretAnnotation::new(&doc);
        caret.set_rect(Rectangle::new(1.0, 2.0, 30.0, 40.0, false));
        caret.set_title("A. Reviewer".into());
        caret.set_subject("Typo".into());
        caret.set_reply_type(ReplyType::Group);
        caret.set_opacity(0.5);
        page.annotations().add(Box::new(caret));
    }
    let mut out = Vec::new();
    doc.save_to_writer(&mut out).unwrap();
    let text = String::from_utf8_lossy(&out);
    assert!(text.contains("/T (A. Reviewer)"), "the author");
    assert!(text.contains("/Subj (Typo)"), "the subject");
    assert!(text.contains("/RT /Group"), "the reply type is a name, not an array");
    assert!(text.contains("/CA 0.5"), "the constant opacity");
    // A full opacity is the default and must stay unwritten.
    assert_eq!(text.matches("/CA").count(), 1, "only the one that was set");
}

/// `/BS` is a dictionary, not two loose entries — the width and the
/// style name live inside it (§12.5.4).
#[test]
fn a_border_width_is_written_as_a_style_dictionary() {
    let doc = Document::open(fixture("pages_tree/single.pdf")).unwrap();
    {
        let page = doc.pages().get(1).unwrap();
        let mut caret = CaretAnnotation::new(&doc);
        caret.set_rect(Rectangle::new(1.0, 2.0, 30.0, 40.0, false));
        let mut border = aspose_pdf_foss::Border::new(&caret);
        border.set_width(3);
        caret.set_border(border);
        page.annotations().add(Box::new(caret));
    }
    let mut out = Vec::new();
    doc.save_to_writer(&mut out).unwrap();
    let text = String::from_utf8_lossy(&out);
    assert!(text.contains("/BS <<"), "the border style is a dictionary");
    assert!(text.contains("/W 3"), "carrying the width");
    assert!(text.contains("/S /S"), "and the style name");
}

/// Five subtypes contributed no dictionary entry of their own, so each
/// reached the file as a bare rectangle: a file attachment with no
/// attachment (§12.5.6.15 marks /FS required), a stamp and a note with
/// no picked icon, a caret with no symbol, a pop-up that never opened.
#[test]
fn the_remaining_subtypes_carry_their_defining_entries() {
    let doc = Document::open(fixture("pages_tree/single.pdf")).unwrap();
    {
        let page = doc.pages().get(1).unwrap();
        let r = Rectangle::new(10.0, 10.0, 40.0, 40.0, false);

        let mut spec = FileSpecification::with_file_and_description(
            "/tmp/report.csv".into(),
            "last quarter".into(),
        );
        spec.set_name("report.csv".into());
        let mut fa = FileAttachmentAnnotation::on_page(&page, r, spec);
        fa.set_icon(FileIcon::Paperclip);
        page.annotations().add(Box::new(fa));

        let mut stamp = StampAnnotation::on_page(&page, r);
        stamp.set_icon(StampIcon::Confidential);
        page.annotations().add(Box::new(stamp));

        let mut caret = CaretAnnotation::on_page(&page, r);
        caret.set_symbol(CaretSymbol::Paragraph);
        caret.set_frame(Rectangle::new(1.0, 2.0, 3.0, 4.0, false));
        page.annotations().add(Box::new(caret));

        let mut popup = PopupAnnotation::on_page(&page, r);
        popup.set_open(true);
        page.annotations().add(Box::new(popup));
    }
    let mut out = Vec::new();
    doc.save_to_writer(&mut out).unwrap();
    let text = String::from_utf8_lossy(&out);

    // The attachment names its file through the required /FS.
    assert!(text.contains("/FS <<"), "the file specification dictionary");
    assert!(text.contains("/Filespec"), "typed as a file specification");
    assert!(text.contains("(report.csv)"), "naming the file");
    assert!(text.contains("(last quarter)"), "and describing it");
    assert!(text.contains("/Name /Paperclip"), "the attachment icon");

    assert!(text.contains("/Name /Confidential"), "the stamp drawn");
    assert!(text.contains("/Sy /P"), "the caret symbol");
    assert!(text.contains("/RD ["), "the caret's inset");
    assert!(text.contains("/Open true"), "the pop-up starts open");
}

/// Each of those entries has a default that an absent entry already
/// means, so a subtype left alone writes none of them.
#[test]
fn the_default_icons_and_states_write_no_entries() {
    let doc = Document::open(fixture("pages_tree/single.pdf")).unwrap();
    {
        let page = doc.pages().get(1).unwrap();
        let r = Rectangle::new(10.0, 10.0, 40.0, 40.0, false);
        page.annotations().add(Box::new(StampAnnotation::on_page(&page, r)));
        page.annotations().add(Box::new(CaretAnnotation::on_page(&page, r)));
        page.annotations().add(Box::new(PopupAnnotation::on_page(&page, r)));
    }
    let mut out = Vec::new();
    doc.save_to_writer(&mut out).unwrap();
    let text = String::from_utf8_lossy(&out);
    assert!(text.contains("/Stamp"), "the annotations are written");
    assert!(!text.contains("/Name /Draft"), "the default stamp stays out");
    assert!(!text.contains("/Sy"), "no symbol for the default caret");
    assert!(!text.contains("/Open"), "and a closed pop-up says nothing");
}

/// A page's /Annots is written whole, so every annotation on a dirty
/// page is re-emitted from the model — and the model named four keys.
/// Adding one annotation to a document that already had some therefore
/// stripped the others down to those four: a highlight lost its
/// /QuadPoints, a link its /A, anything its appearance stream. The
/// entries this model cannot name now travel with the loaded
/// annotation.
#[test]
fn adding_an_annotation_does_not_strip_the_ones_already_there() {
    use aspose_pdf_foss::{HighlightAnnotation, Point, TextMarkupAnnotation};

    // First document: a highlight with quadrilaterals and a colour.
    let first = common::unique_temp("aspose_rust_carried_one.pdf");
    {
        let doc = Document::open(fixture("pages_tree/single.pdf")).unwrap();
        {
            let page = doc.pages().get(1).unwrap();
            let mut h = HighlightAnnotation::on_page(
                &page,
                Rectangle::new(10.0, 20.0, 60.0, 32.0, false),
            );
            h.set_quad_points(vec![
                Point::new(10.0, 20.0),
                Point::new(60.0, 20.0),
                Point::new(10.0, 32.0),
                Point::new(60.0, 32.0),
            ]);
            h.set_color(aspose_pdf_foss::Color::from_rgb(1.0, 1.0, 0.0));
            page.annotations().add(Box::new(h));
        }
        doc.save_to(&first).unwrap();
    }
    let before = String::from_utf8_lossy(&std::fs::read(&first).unwrap()).into_owned();
    assert!(before.contains("/QuadPoints"), "the highlight is marked");

    // Reopen and add a second, unrelated annotation to the same page.
    let second = common::unique_temp("aspose_rust_carried_two.pdf");
    {
        let doc = Document::open(&first).unwrap();
        {
            let page = doc.pages().get(1).unwrap();
            assert_eq!(page.annotations().count(), 1, "the highlight is read back");
            let mut caret = CaretAnnotation::on_page(
                &page,
                Rectangle::new(100.0, 100.0, 120.0, 120.0, false),
            );
            caret.set_symbol(CaretSymbol::Paragraph);
            page.annotations().add(Box::new(caret));
        }
        doc.save_to(&second).unwrap();
    }
    let after = String::from_utf8_lossy(&std::fs::read(&second).unwrap()).into_owned();

    // The newest /Annots carries both, and the highlight kept what the
    // model never names.
    let last_page = after.rfind("/Annots").expect("a rewritten page");
    let tail = &after[last_page..];
    assert!(tail.contains("/Highlight") || after.contains("/Highlight"), "both subtypes");
    assert!(after.contains("/Caret"), "the new annotation");
    assert_eq!(
        after.matches("/QuadPoints").count(),
        2,
        "the highlight's quadrilaterals survive the rewrite (once per revision)"
    );
    assert!(after.contains("/C ["), "and so does its colour");

    let _ = std::fs::remove_file(&first);
    let _ = std::fs::remove_file(&second);
}

/// §12.5.4's border effect and the border's dash are plain dictionary
/// entries, siblings of /BS — a viewer honours them whether or not this
/// crate draws a border itself. Both were carried and unwritten, and
/// the note explaining why blamed the missing appearance stream, which
/// is not what stood in their way.
#[test]
fn a_cloudy_dashed_border_reaches_the_file() {
    use aspose_pdf_foss::{Border, BorderEffect, BorderStyle, Dash, SquareAnnotation};

    let doc = Document::open(fixture("pages_tree/single.pdf")).unwrap();
    {
        let page = doc.pages().get(1).unwrap();
        let mut square =
            SquareAnnotation::on_page(&page, Rectangle::new(10.0, 10.0, 90.0, 90.0, false));
        let mut border = Border::new(&square);
        border.set_width(3);
        border.set_style(BorderStyle::Dashed);
        border.set_dash(Dash::new(4, 2));
        border.set_effect(BorderEffect::Cloudy);
        border.set_effect_intensity(2);
        square.set_border(border);
        page.annotations().add(Box::new(square));
    }
    let mut out = Vec::new();
    doc.save_to_writer(&mut out).unwrap();
    let text = String::from_utf8_lossy(&out);

    assert!(text.contains("/W 3"), "the width");
    assert!(text.contains("/S /D"), "the dashed style");
    assert!(text.contains("/D [ 4 2 ]"), "and the pattern it dashes with");
    assert!(text.contains("/BE <<"), "the border effect dictionary");
    assert!(text.contains("/S /C"), "cloudy");
    assert!(text.contains("/I 2"), "at the intensity asked for");
}

/// A plain border writes neither: an absent /BE is no effect, and an
/// all-zero dash pattern strokes nothing, so it is the unset state
/// rather than an instruction.
#[test]
fn a_plain_border_writes_no_effect_and_no_dash() {
    use aspose_pdf_foss::{Border, SquareAnnotation};

    let doc = Document::open(fixture("pages_tree/single.pdf")).unwrap();
    {
        let page = doc.pages().get(1).unwrap();
        let mut square =
            SquareAnnotation::on_page(&page, Rectangle::new(10.0, 10.0, 90.0, 90.0, false));
        let mut border = Border::new(&square);
        border.set_width(1);
        square.set_border(border);
        page.annotations().add(Box::new(square));
    }
    let mut out = Vec::new();
    doc.save_to_writer(&mut out).unwrap();
    let text = String::from_utf8_lossy(&out);

    assert!(text.contains("/W 1"), "the border is written");
    assert!(!text.contains("/BE"), "with no effect");
    assert!(!text.contains("/D ["), "and no dash array");
}

#[path = "../common/mod.rs"]
mod common;
