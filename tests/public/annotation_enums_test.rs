// Public-API smoke driver — annotation leaf enums (A-0a).
//
// Pins the canonical Aspose.PDF 26.4.0 ordinals for the annotation
// enum family. AnnotationType is checked exhaustively (all 33
// values); the rest spot-check representative / boundary variants.

use aspose_pdf_foss::*;

#[test]
fn annotation_type_all_33() {
    let expect = [
        (AnnotationType::Text, 0),
        (AnnotationType::Circle, 1),
        (AnnotationType::Polygon, 2),
        (AnnotationType::PolyLine, 3),
        (AnnotationType::Line, 4),
        (AnnotationType::Square, 5),
        (AnnotationType::FreeText, 6),
        (AnnotationType::Highlight, 7),
        (AnnotationType::Underline, 8),
        (AnnotationType::Squiggly, 9),
        (AnnotationType::StrikeOut, 10),
        (AnnotationType::Caret, 11),
        (AnnotationType::Ink, 12),
        (AnnotationType::Link, 13),
        (AnnotationType::Popup, 14),
        (AnnotationType::FileAttachment, 15),
        (AnnotationType::Sound, 16),
        (AnnotationType::Movie, 17),
        (AnnotationType::Screen, 18),
        (AnnotationType::Widget, 19),
        (AnnotationType::Watermark, 20),
        (AnnotationType::TrapNet, 21),
        (AnnotationType::PrinterMark, 22),
        (AnnotationType::Redaction, 23),
        (AnnotationType::Stamp, 24),
        (AnnotationType::RichMedia, 25),
        (AnnotationType::Unknown, 26),
        (AnnotationType::PDF3D, 27),
        (AnnotationType::ColorBar, 28),
        (AnnotationType::TrimMark, 29),
        (AnnotationType::BleedMark, 30),
        (AnnotationType::RegistrationMark, 31),
        (AnnotationType::PageInformation, 32),
    ];
    for (variant, ord) in expect {
        assert_eq!(variant as i32, ord);
    }
}

#[test]
fn flag_enums() {
    assert_eq!(AnnotationFlags::Default as i32, 0);
    assert_eq!(AnnotationFlags::Print as i32, 4);
    assert_eq!(AnnotationFlags::LockedContents as i32, 512);
    assert_eq!(RichTextFontStyles::ClearExisting as i32, 1);
    assert_eq!(RichTextFontStyles::Underline as i32, 8);
    // PrinterMarksKind.All is the OR of every individual mark bit.
    assert_eq!(PrinterMarksKind::All as i32, 31);
    assert_eq!(PrinterMarksKind::PageInformation as i32, 16);
}

#[test]
fn representative_ordinals() {
    assert_eq!(AnnotationState::None as i32, 7);
    assert_eq!(AnnotationStateModel::Review as i32, 2);
    assert_eq!(BorderEffect::Cloudy as i32, 1);
    assert_eq!(BorderStyle::Underline as i32, 4);
    assert_eq!(CapStyle::Rectangular as i32, 1); // note: starts at 1
    assert_eq!(CaptionPosition::Top as i32, 1);
    assert_eq!(CaretSymbol::Paragraph as i32, 1);
    assert_eq!(ColorsOfCMYK::Black as i32, 3);
    assert_eq!(FileIcon::Tag as i32, 3);
    assert_eq!(FreeTextIntent::FreeTextTypeWriter as i32, 2);
    assert_eq!(HighlightingMode::Toggle as i32, 4);
    assert_eq!(Justification::Right as i32, 2);
    assert_eq!(LineEnding::Slash as i32, 9);
    assert_eq!(LineIntent::LineDimension as i32, 2);
    assert_eq!(PolyIntent::PolygonDimension as i32, 3);
    assert_eq!(PrinterMarkCornerPosition::BottomRight as i32, 3);
    assert_eq!(PrinterMarkSidePosition::Right as i32, 3);
    assert_eq!(ReplyType::Group as i32, 2);
    assert_eq!(SoundIcon::Mic as i32, 1);
    assert_eq!(StampIcon::TopSecret as i32, 13);
    assert_eq!(TextAlignment::Right as i32, 2);
    assert_eq!(TextIcon::Star as i32, 10);
}

#[test]
fn defaults() {
    assert_eq!(AnnotationType::default(), AnnotationType::Text);
    assert_eq!(AnnotationState::default(), AnnotationState::Undefined);
    assert_eq!(BorderStyle::default(), BorderStyle::Solid);
    // CapStyle has no zero value; default falls on the first variant.
    assert_eq!(CapStyle::default(), CapStyle::Rectangular);
}
