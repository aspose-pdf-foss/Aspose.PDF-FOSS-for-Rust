// Public-API smoke driver — Color.
//
// Ports the C++ color_smoke_test contract: the alpha accessor is
// normalised 0..1, FromArgb honours / defaults alpha, FromRgb is
// opaque, the default / Empty / Transparent colours have alpha 0,
// and the named presets are opaque. Preset RGB values follow the
// W3C / CSS3 standard table.

use aspose_pdf_foss::Color;

const EPS: f64 = 1e-9;

fn approx(a: f64, b: f64) {
    assert!((a - b).abs() < EPS, "{a} !~= {b}");
}

#[test]
fn from_argb_honours_alpha() {
    let c = Color::from_argb(128, 64, 192, 32);
    approx(c.a(), 128.0 / 255.0);
}

#[test]
fn from_argb_three_arg_defaults_alpha_opaque() {
    let c = Color::from_argb_rgb(64, 192, 32);
    approx(c.a(), 1.0);
}

#[test]
fn from_rgb_double_is_opaque() {
    let c = Color::from_rgb(0.25, 0.5, 0.75);
    approx(c.a(), 1.0);
}

#[test]
fn default_constructed_is_empty() {
    let c = Color::new();
    approx(c.a(), 0.0);
    approx(Color::empty().a(), 0.0);
    // Default trait agrees with new().
    let d: Color = Default::default();
    assert_eq!(d, Color::new());
}

#[test]
fn transparent_has_alpha_zero() {
    approx(Color::transparent().a(), 0.0);
}

#[test]
fn presets_are_opaque() {
    for a in [
        Color::black().a(),
        Color::white().a(),
        Color::red().a(),
        Color::green().a(),
        Color::blue().a(),
        Color::yellow().a(),
        Color::alice_blue().a(),
        Color::yellow_green().a(),
    ] {
        approx(a, 1.0);
    }
}

#[test]
fn representative_presets_construct() {
    // A spread across the table — just that they build and compare.
    let _ = [
        Color::alice_blue(),
        Color::brown(),
        Color::crimson(),
        Color::dark_blue(),
        Color::firebrick(),
        Color::gold(),
        Color::hot_pink(),
        Color::indian_red(),
        Color::khaki(),
        Color::lime(),
        Color::magenta(),
        Color::navy(),
        Color::olive(),
        Color::plum(),
        Color::red(),
        Color::salmon(),
        Color::tan(),
        Color::violet(),
        Color::wheat(),
        Color::yellow(),
    ];
}

#[test]
fn equal_constructions_compare_equal() {
    // Aqua and Cyan share the same hex in the W3C table.
    assert_eq!(Color::aqua(), Color::cyan());
    // Round-trip an ARGB build against itself.
    assert_eq!(Color::from_argb(10, 20, 30, 40), Color::from_argb(10, 20, 30, 40));
    assert_ne!(Color::red(), Color::blue());
}
