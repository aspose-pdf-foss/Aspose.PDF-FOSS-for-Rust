// Public-API smoke driver — F-5 TextBox subclasses.
//
// Barcode / Date / Number / Password / RichText / FileSelectBox —
// each a TextBoxField subclass. They expose their own members plus
// the inherited Field surface (partial_name / value / ...).

use aspose_pdf_foss::{
    Annotation, AnnotationType, BarcodeField, DateField, DateTime, Document, Field, FileSelectBoxField,
    Justification, NumberField, PasswordBoxField, Rectangle, RichTextBoxField, Symbology,
};
use std::path::PathBuf;

fn page_doc() -> Document {
    Document::open(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("fixtures")
            .join("pages_tree/single.pdf"),
    )
    .unwrap()
}

fn rect() -> Rectangle {
    Rectangle::new(0.0, 0.0, 50.0, 20.0, false)
}

#[test]
fn barcode_field() {
    let doc = page_doc();
    let page = doc.pages().get(1).unwrap();
    let b = BarcodeField::on_page(&page, rect());
    assert_eq!(b.annotation_type(), AnnotationType::Widget);
    assert_eq!(b.symbology(), Symbology::PDF417);
    assert_eq!(b.resolution(), 0);
    assert_eq!(b.ecc(), 0);
}

#[test]
fn date_field() {
    let doc = Document::new();
    let mut d = DateField::new(&doc);
    assert_eq!(d.annotation_type(), AnnotationType::Widget);
    assert_eq!(d.date_format(), "");
    d.set_date_format("mm/dd/yyyy".into());
    assert_eq!(d.date_format(), "mm/dd/yyyy");
    // DateField.Value is the DateTime value (canonically hides the
    // base string value); the string is reached via the Field trait.
    d.set_value(DateTime::new(2026, 6, 3, 0, 0, 0));
    assert_eq!(d.value().year(), 2026);
    Field::set_value(&mut d, "2026-06-03".into());
    assert_eq!(Field::value(&d), "2026-06-03");
}

#[test]
fn number_field() {
    let doc = page_doc();
    let page = doc.pages().get(1).unwrap();
    let mut n = NumberField::on_page(&page, rect());
    assert_eq!(n.allowed_chars(), "");
    n.set_allowed_chars("0123456789".into());
    assert_eq!(n.allowed_chars(), "0123456789");
}

#[test]
fn rich_text_field() {
    let doc = page_doc();
    let page = doc.pages().get(1).unwrap();
    let mut r = RichTextBoxField::on_page(&page, rect());
    assert_eq!(r.annotation_type(), AnnotationType::Widget);
    assert_eq!(r.justify(), Justification::Left);
    r.set_rich_text_value("<b>x</b>".into());
    r.set_justify(Justification::Center);
    assert_eq!(r.rich_text_value(), "<b>x</b>");
    assert_eq!(r.justify(), Justification::Center);
}

#[test]
fn empty_subclasses() {
    let doc = Document::new();
    let mut p = PasswordBoxField::new(&doc);
    assert_eq!(p.annotation_type(), AnnotationType::Widget);
    p.set_partial_name("pwd".into()); // inherited Field property
    assert_eq!(p.partial_name(), "pwd");

    let f = FileSelectBoxField::new(&doc);
    assert_eq!(f.annotation_type(), AnnotationType::Widget);
}

/// A rich-text box reached the file as a plain text field: its own
/// HasFieldBase impl took the trait defaults instead of forwarding to
/// the embedded TextBoxField, so the Table 228 bits and /MaxLen were
/// lost, and §12.7.4.3's /RV, /DS and /Q were never written at all.
#[test]
fn a_rich_text_box_carries_its_value_style_and_flags() {
    let doc = page_doc();
    let mut f = RichTextBoxField::on_page(&doc.pages().get(1).unwrap(), rect());
    f.set_partial_name("notes".into());
    f.set_multiline(true);
    f.set_max_len(40);
    f.set_rich_text_value("<body>hi</body>".into());
    f.set_style("font-size:12pt".into());
    f.set_justify(Justification::Right);

    let mut doc = doc;
    doc.form_mut().add_at(Box::new(f), 1);
    let mut out = Vec::new();
    doc.save_to_writer(&mut out).unwrap();
    let text = String::from_utf8_lossy(&out);

    assert!(text.contains("/RV (<body>hi</body>)"), "the rich-text value");
    assert!(text.contains("/DS (font-size:12pt)"), "the default style");
    assert!(text.contains("/Q 2"), "right quadding");
    assert!(text.contains("/MaxLen 40"), "the inherited length limit");
    // Multiline (1 << 12) | RichText (1 << 25) = 33558528.
    assert!(text.contains("/Ff 33558528"), "the inherited and own bits");
}

/// The same forwarding gap on the other five text-box subclasses: a
/// multiline number or date field wrote none of its base's bits.
#[test]
fn a_number_field_keeps_the_bits_of_its_base() {
    let doc = page_doc();
    let mut f = NumberField::on_page(&doc.pages().get(1).unwrap(), rect());
    f.set_partial_name("qty".into());
    f.set_multiline(true);
    f.set_max_len(5);

    let mut doc = doc;
    doc.form_mut().add_at(Box::new(f), 1);
    let mut out = Vec::new();
    doc.save_to_writer(&mut out).unwrap();
    let text = String::from_utf8_lossy(&out);
    assert!(text.contains("/Ff 4096"), "Multiline survives the subclass");
    assert!(text.contains("/MaxLen 5"), "and so does /MaxLen");
}
