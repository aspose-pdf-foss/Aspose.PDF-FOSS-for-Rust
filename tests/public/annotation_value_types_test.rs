// Public-API smoke driver — annotation value types (A-0b).
//
// Characteristics (Rotate only; Background/Border are
// System.Drawing.Color and drop) and DefaultAppearance (font name /
// size / resource name + assembled text; TextColor + the colour /
// Font constructors drop). Border ships with the Annotation base
// (A-1), since its only constructor takes an Annotation.

use aspose_pdf_foss::{Characteristics, DefaultAppearance, Rotation};

#[test]
fn characteristics_rotate() {
    let mut c = Characteristics::new();
    assert_eq!(c.rotate(), Rotation::None);
    c.set_rotate(Rotation::On90);
    assert_eq!(c.rotate(), Rotation::On90);
    assert_eq!(Characteristics::default(), Characteristics::new());
}

#[test]
fn default_appearance_defaults() {
    let d = DefaultAppearance::new();
    assert_eq!(d.font_size(), 0.0);
    assert_eq!(d.font_name(), "");
    assert_eq!(d.font_resource_name(), "");
    assert_eq!(d.text(), ""); // no font set
}

#[test]
fn default_appearance_round_trip_and_text() {
    let mut d = DefaultAppearance::new();
    d.set_font_size(12.0);
    d.set_font_name("Helvetica".into());
    d.set_font_resource_name("Helv".into());
    assert_eq!(d.font_size(), 12.0);
    assert_eq!(d.font_name(), "Helvetica");
    assert_eq!(d.font_resource_name(), "Helv");
    // Assembled DA string prefers the resource name.
    assert_eq!(d.text(), "/Helv 12 Tf 0 g");
}

#[test]
fn default_appearance_text_falls_back_to_font_name() {
    let mut d = DefaultAppearance::new();
    d.set_font_size(10.0);
    d.set_font_name("Courier".into());
    assert_eq!(d.text(), "/Courier 10 Tf 0 g");
}
