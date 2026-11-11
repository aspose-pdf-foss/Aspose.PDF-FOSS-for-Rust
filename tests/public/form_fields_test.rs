// Public-API smoke driver — F-3 form fields (TextBox / Button /
// Checkbox). Exercises the field-specific surface plus the inherited
// Field + WidgetAnnotation properties (through the trait chain).

use aspose_pdf_foss::{
    Annotation, AnnotationType, BoxStyle, ButtonField, CheckboxField, Document, Field,
    IconCaptionPosition, Rectangle, ScalingMode, ScalingReason, TextBoxField, VerticalAlignment,
};
use std::path::{Path, PathBuf};

fn two_pages() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/text_extractor/two_pages.pdf")
}

#[test]
fn text_box_field() {
    let doc = Document::new();
    let mut t = TextBoxField::new(&doc);
    assert_eq!(t.annotation_type(), AnnotationType::Widget);
    // field-specific
    assert!(!t.multiline());
    assert!(t.scrollable());
    assert_eq!(t.max_len(), 0);
    assert_eq!(t.text_vertical_alignment(), VerticalAlignment::None);
    t.set_multiline(true);
    t.set_max_len(50);
    t.set_text_vertical_alignment(VerticalAlignment::Center);
    assert!(t.multiline());
    assert_eq!(t.max_len(), 50);
    assert_eq!(t.text_vertical_alignment(), VerticalAlignment::Center);

    // inherited Field properties
    t.set_partial_name("txt".into());
    t.set_value("hello".into());
    assert_eq!(t.partial_name(), "txt");
    assert_eq!(t.value(), "hello");
    // inherited widget property
    t.set_read_only(true);
    assert!(t.read_only());
}

#[test]
fn button_field() {
    let doc = Document::new();
    let mut b = ButtonField::for_document(&doc, aspose_pdf_foss::Rectangle::new(0.0, 0.0, 50.0, 20.0, false));
    assert_eq!(b.annotation_type(), AnnotationType::Widget);
    assert_eq!(b.ic_position(), IconCaptionPosition::NoIcon);
    b.set_normal_caption("OK".into());
    b.set_ic_position(IconCaptionPosition::CaptionBelowIcon);
    assert_eq!(b.normal_caption(), "OK");
    assert_eq!(b.ic_position(), IconCaptionPosition::CaptionBelowIcon);
    b.set_partial_name("btn".into());
    assert_eq!(b.partial_name(), "btn");

    // The get-only IconFit accessor exposes the button's default
    // icon-scaling options.
    let fit = b.icon_fit();
    assert_eq!(fit.scaling_reason(), ScalingReason::Always);
    assert_eq!(fit.scaling_mode(), ScalingMode::Proportional);
    assert_eq!(fit.leftover_left(), 0.0);
    assert_eq!(fit.leftover_bottom(), 0.0);
    assert!(!fit.spread_on_border());
}

#[test]
fn checkbox_field() {
    let doc = Document::new();
    let mut c = CheckboxField::new(&doc);
    assert_eq!(c.annotation_type(), AnnotationType::Widget);
    assert_eq!(c.style(), BoxStyle::Check);
    assert!(!c.checked());
    assert!(c.allowed_states().is_empty());

    c.set_checked(true);
    c.set_style(BoxStyle::Cross);
    c.set_export_value("Yes".into());
    c.add_option("On".into());
    c.add_option("Maybe".into());
    assert!(c.checked());
    assert_eq!(c.style(), BoxStyle::Cross);
    assert_eq!(c.export_value(), "Yes");
    assert_eq!(c.allowed_states(), &["On".to_string(), "Maybe".to_string()]);
}

#[test]
fn fields_are_polymorphic_via_field_trait() {
    // Each field type used through the Field trait.
    let doc = Document::new();
    let mut t = TextBoxField::new(&doc);
    let mut c = CheckboxField::new(&doc);
    t.set_tab_order(1);
    c.set_tab_order(2);
    let fields: Vec<&dyn Field> = vec![&t, &c];
    assert_eq!(fields[0].tab_order(), 1);
    assert_eq!(fields[1].tab_order(), 2);
    assert!(fields.iter().all(|f| !f.is_group()));
}

#[test]
fn document_form_save_reopen_round_trip() {
    let out = common::unique_temp("aspose_rust_form.pdf");
    {
        let mut doc = Document::open(two_pages()).unwrap();
        assert_eq!(doc.form().count(), 0);

        // A text field on page 1, and a read-only one on page 2.
        let field1 = {
            let page = doc.pages().get(1).unwrap();
            let mut f = TextBoxField::on_page(&page, Rectangle::new(72.0, 700.0, 272.0, 720.0, false));
            f.set_partial_name("fullname".into());
            f.set_value("Ada Lovelace".into());
            f
        };
        let field2 = {
            let page = doc.pages().get(2).unwrap();
            let mut f = TextBoxField::on_page(&page, Rectangle::new(72.0, 600.0, 272.0, 620.0, false));
            f.set_partial_name("locked".into());
            f.set_value("fixed".into());
            f.set_read_only(true);
            f
        };
        doc.form_mut().add(Box::new(field1));
        doc.form_mut().add(Box::new(field2));

        doc.save_to(&out).unwrap();
    }

    let saved = std::fs::read(&out).unwrap();
    for marker in [&b"/AcroForm"[..], b"/Widget", b"Ada Lovelace", b"fullname"] {
        assert!(
            saved.windows(marker.len()).any(|w| w == marker),
            "expected {:?} in the saved PDF",
            std::str::from_utf8(marker).unwrap()
        );
    }

    let doc = Document::open(&out).unwrap();
    assert_eq!(doc.form().count(), 2);

    let f1 = doc.form().get_by_name("fullname").unwrap();
    assert_eq!(f1.value(), "Ada Lovelace");
    assert!(!f1.read_only());

    let f2 = doc.form().get_by_name("locked").unwrap();
    assert_eq!(f2.value(), "fixed");
    assert!(f2.read_only());

    // The widgets also land in their pages' /Annots.
    assert_eq!(doc.pages().get(1).unwrap().annotations().count(), 1);
    assert_eq!(doc.pages().get(2).unwrap().annotations().count(), 1);

    let _ = std::fs::remove_file(&out);
}

#[test]
fn document_form_verbatim_resave_keeps_no_acroform() {
    let out = common::unique_temp("aspose_rust_form_verbatim.pdf");
    let original = std::fs::read(two_pages()).unwrap();
    {
        let doc = Document::open(two_pages()).unwrap();
        let _ = doc.form().count();
        doc.save_to(&out).unwrap();
    }
    let resaved = std::fs::read(&out).unwrap();
    assert_eq!(resaved, original, "untouched form must resave verbatim");
    let _ = std::fs::remove_file(&out);
}

#[path = "../common/mod.rs"]
mod common;

/// A push button's captions live in its /MK appearance-characteristics
/// dictionary (§12.5.6.19 Table 189). All three captions and the
/// icon / caption layout were stored and never written, so every button
/// reached the file blank.
#[test]
fn a_button_carries_its_captions() {
    let mut doc = Document::open(two_pages()).unwrap();
    let mut b = ButtonField::for_document(&doc, Rectangle::new(0.0, 0.0, 50.0, 20.0, false));
    b.set_partial_name("submit".into());
    b.set_normal_caption("OK".into());
    b.set_rollover_caption("Go on".into());
    b.set_alternate_caption("Pressed".into());
    b.set_ic_position(IconCaptionPosition::CaptionBelowIcon);
    doc.form_mut().add_at(Box::new(b), 1);

    let mut out = Vec::new();
    doc.save_to_writer(&mut out).unwrap();
    let text = String::from_utf8_lossy(&out);
    assert!(text.contains("/MK"), "the appearance-characteristics dictionary");
    assert!(text.contains("/CA (OK)"), "the normal caption");
    assert!(text.contains("/RC (Go on)"), "the rollover caption");
    assert!(text.contains("/AC (Pressed)"), "the down-state caption");
    assert!(text.contains("/TP 2"), "the caption / icon layout");
}

/// A button with no caption and the default layout needs no /MK at
/// all, so none is written.
#[test]
fn a_bare_button_writes_no_appearance_dictionary() {
    let mut doc = Document::open(two_pages()).unwrap();
    let mut b = ButtonField::for_document(&doc, Rectangle::new(0.0, 0.0, 50.0, 20.0, false));
    b.set_partial_name("bare".into());
    doc.form_mut().add_at(Box::new(b), 1);

    let mut out = Vec::new();
    doc.save_to_writer(&mut out).unwrap();
    let text = String::from_utf8_lossy(&out);
    assert!(text.contains("/Btn"), "the button itself is written");
    assert!(!text.contains("/MK"), "with no empty /MK dictionary");
}

/// The canonical `IconFit` member is a getter returning an object a
/// caller mutates through. A shared reference cannot be mutated, so
/// the five scaling options were unreachable until the `_mut`
/// companion existed.
#[test]
fn a_buttons_icon_fit_can_be_configured() {
    let doc = Document::new();
    let mut b = ButtonField::for_document(&doc, Rectangle::new(0.0, 0.0, 50.0, 20.0, false));
    b.icon_fit_mut().set_scaling_reason(ScalingReason::Never);
    b.icon_fit_mut().set_scaling_mode(ScalingMode::Anamorphic);
    b.icon_fit_mut().set_leftover_left(0.25);
    b.icon_fit_mut().set_leftover_bottom(0.75);
    b.icon_fit_mut().set_spread_on_border(true);

    assert_eq!(b.icon_fit().scaling_reason(), ScalingReason::Never);
    assert_eq!(b.icon_fit().scaling_mode(), ScalingMode::Anamorphic);
    assert_eq!(b.icon_fit().leftover_left(), 0.25);
    assert_eq!(b.icon_fit().leftover_bottom(), 0.75);
    assert!(b.icon_fit().spread_on_border());
}
