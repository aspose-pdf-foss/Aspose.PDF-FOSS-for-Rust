// Public-API smoke driver — F-4 choice fields + Option/OptionCollection.
//
// ComboBox / ListBox / RadioButton (ChoiceField) + RadioButtonOptionField.
// Exercises the ChoiceField trait surface (options / selected /
// add_option) plus the inherited Field properties.

use aspose_pdf_foss::{
    Annotation, AnnotationType, BoxStyle, ChoiceField, ComboBoxField, Document, Field,
    ListBoxField, Option as FormOption, OptionCollection, RadioButtonField,
    RadioButtonOptionField, Rectangle, TextFragment,
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

#[test]
fn option_and_collection() {
    let mut o = FormOption::new();
    o.set_value("v1".into());
    o.set_name("One".into());
    assert_eq!(o.value(), "v1");
    assert!(!o.selected());

    let mut col = OptionCollection::new();
    col.add(o.clone());
    let mut o2 = FormOption::new();
    o2.set_name("Two".into());
    col.add(o2);
    assert_eq!(col.count(), 2);
    assert_eq!(col.at(0).unwrap().name(), "One");
    assert_eq!(col.at(1).unwrap().index(), 1); // index assigned on add
    assert!(col.by_name("Two").is_some());
    assert!(col.contains(&o));
    assert!(col.remove(&o));
    assert_eq!(col.count(), 1);
}

#[test]
fn combo_box() {
    let doc = Document::new();
    let mut c = ComboBoxField::new(&doc);
    assert_eq!(c.annotation_type(), AnnotationType::Widget);
    assert!(!c.editable());
    assert!(!c.multi_select()); // from ChoiceField
    c.set_editable(true);
    c.add_option("Red".into());
    c.add_option_named("g".into(), "Green".into());
    assert!(c.editable());
    assert_eq!(c.options().count(), 2);
    assert_eq!(c.options().by_name("Green").unwrap().value(), "g");
    c.delete_option("Red");
    assert_eq!(c.options().count(), 1);
    // inherited Field property
    c.set_partial_name("combo".into());
    assert_eq!(c.partial_name(), "combo");
}

#[test]
fn list_box() {
    let doc = page_doc();
    let page = doc.pages().get(1).unwrap();
    let mut l = ListBoxField::on_page(&page, Rectangle::new(0.0, 0.0, 50.0, 40.0, false));
    assert_eq!(l.annotation_type(), AnnotationType::Widget);
    assert_eq!(l.top_index(), 0);
    l.set_top_index(2);
    l.set_multi_select(true);
    l.add_option("a".into());
    assert_eq!(l.top_index(), 2);
    assert!(l.multi_select());
    assert_eq!(l.options().count(), 1);
}

#[test]
fn radio_button() {
    let doc = page_doc();
    let page = doc.pages().get(1).unwrap();
    let mut r = RadioButtonField::new(&doc);
    assert_eq!(r.annotation_type(), AnnotationType::Widget);
    assert_eq!(r.style(), BoxStyle::Circle);
    assert!(!r.no_toggle_to_off());
    r.set_no_toggle_to_off(true);

    let mut opt = RadioButtonOptionField::on_page(&page, Rectangle::new(0.0, 0.0, 10.0, 10.0, false));
    opt.set_option_name("Yes".into());
    assert_eq!(opt.caption().text(), "");
    opt.set_caption(TextFragment::with_text("Yes"));
    assert_eq!(opt.caption().text(), "Yes");
    r.add(opt);
    r.add_option("No".into());
    assert!(r.no_toggle_to_off());
    assert_eq!(r.options().count(), 2);
    assert_eq!(r.options().at(0).unwrap().value(), "Yes");
}
