// Public-API smoke driver — TextReplaceOptions, its three nested
// enums, and the PdfContentEditor text-options getters.

use aspose_pdf_foss::{
    FontSizeAdjustment, NoCharacterAction, PdfContentEditor, Rectangle, ReplaceAdjustment, Scope,
    TextReplaceOptions,
};

#[test]
fn nested_enum_ordinals_match_canonical() {
    assert_eq!(Scope::ReplaceFirst as i32, 0);
    assert_eq!(Scope::ReplaceAll as i32, 1);

    assert_eq!(ReplaceAdjustment::None as i32, 0);
    assert_eq!(ReplaceAdjustment::AdjustSpaceWidth as i32, 1);
    assert_eq!(ReplaceAdjustment::WholeWordsHyphenation as i32, 2);
    assert_eq!(ReplaceAdjustment::IsFormFillingMode as i32, 4);
    assert_eq!(ReplaceAdjustment::ShiftRestOfLine as i32, 8);

    assert_eq!(FontSizeAdjustment::None as i32, 0);
    assert_eq!(FontSizeAdjustment::ShrinkToFit as i32, 1);
    assert_eq!(FontSizeAdjustment::ScaleToFill as i32, 2);

    assert_eq!(Scope::default(), Scope::ReplaceFirst);
    assert_eq!(ReplaceAdjustment::default(), ReplaceAdjustment::None);
}

#[test]
fn options_constructors_and_round_trip() {
    let a = TextReplaceOptions::with_scope(Scope::ReplaceAll);
    assert_eq!(a.replace_scope(), Scope::ReplaceAll);

    let b = TextReplaceOptions::with_adjustment(ReplaceAdjustment::ShiftRestOfLine);
    assert_eq!(b.replace_adjustment_action(), ReplaceAdjustment::ShiftRestOfLine);

    let mut o = TextReplaceOptions::default();
    assert_eq!(o.replace_scope(), Scope::ReplaceFirst);
    assert!(o.rectangle().is_none());

    o.set_left_adjustment(1.5);
    o.set_right_adjustment(2.5);
    o.set_adjustment_new_line_spacing(3.0);
    o.set_ignore_paragraphs(true);
    o.set_font_size_adjustment_action(FontSizeAdjustment::ShrinkToFit);
    o.set_rectangle(Rectangle::new(0.0, 0.0, 10.0, 20.0, false));

    assert_eq!(o.left_adjustment(), 1.5);
    assert_eq!(o.right_adjustment(), 2.5);
    assert_eq!(o.adjustment_new_line_spacing(), 3.0);
    assert!(o.ignore_paragraphs());
    assert_eq!(o.font_size_adjustment_action(), FontSizeAdjustment::ShrinkToFit);
    assert_eq!(o.rectangle().unwrap().ury(), 20.0);
}

#[test]
fn content_editor_exposes_option_bags() {
    let ed = PdfContentEditor::new();
    assert_eq!(ed.text_replace_options().replace_scope(), Scope::ReplaceFirst);
    assert!(!ed.text_search_options().is_regular_expression_used());
    assert_eq!(
        ed.text_edit_options().no_character_behavior(),
        NoCharacterAction::ThrowException
    );
}
