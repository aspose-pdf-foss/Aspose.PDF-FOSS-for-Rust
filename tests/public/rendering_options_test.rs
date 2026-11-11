// Public-API smoke driver — RenderingOptions.
//
// Coverage:
//   * Default ctor (new / Default::default) yields the canonical
//     "all flags false, numeric extras zero, font name empty,
//     cache caps zero" baseline.
//   * Each of the 13 properties round-trips through its setter.
//   * set_default_font_name accepts both &str and owned String via
//     impl Into<String>.
//   * Setting one property doesn't disturb the others.
//   * Clone + PartialEq — bag is comparable for parity checks.

use aspose_pdf_foss::RenderingOptions;

#[test]
fn default_ctor_is_engine_baseline() {
    let o = RenderingOptions::new();
    assert!(!o.analyze_fonts());
    assert!(!o.barcode_optimization());
    assert!(!o.convert_fonts_to_unicode_ttf());
    assert_eq!(o.default_font_name(), "");
    assert_eq!(o.height_extra_units(), 0.0);
    assert!(!o.ignore_resource_font_errors());
    assert!(!o.interpolation_high_quality());
    assert_eq!(o.max_fonts_cache_size(), 0);
    assert_eq!(o.max_symbols_cache_size(), 0);
    assert!(!o.optimize_dimensions());
    assert!(!o.system_fonts_native_rendering());
    assert!(!o.use_font_hinting());
    assert_eq!(o.width_extra_units(), 0.0);
}

#[test]
fn default_trait_matches_explicit_new() {
    let o: RenderingOptions = Default::default();
    assert_eq!(o, RenderingOptions::new());
}

#[test]
fn all_bool_setters_round_trip() {
    let mut o = RenderingOptions::new();
    o.set_analyze_fonts(true);
    o.set_barcode_optimization(true);
    o.set_convert_fonts_to_unicode_ttf(true);
    o.set_ignore_resource_font_errors(true);
    o.set_interpolation_high_quality(true);
    o.set_optimize_dimensions(true);
    o.set_system_fonts_native_rendering(true);
    o.set_use_font_hinting(true);

    assert!(o.analyze_fonts());
    assert!(o.barcode_optimization());
    assert!(o.convert_fonts_to_unicode_ttf());
    assert!(o.ignore_resource_font_errors());
    assert!(o.interpolation_high_quality());
    assert!(o.optimize_dimensions());
    assert!(o.system_fonts_native_rendering());
    assert!(o.use_font_hinting());
}

#[test]
fn numeric_setters_round_trip() {
    let mut o = RenderingOptions::new();
    o.set_height_extra_units(1.5);
    o.set_width_extra_units(-2.25);
    o.set_max_fonts_cache_size(64);
    o.set_max_symbols_cache_size(128);

    assert_eq!(o.height_extra_units(), 1.5);
    assert_eq!(o.width_extra_units(), -2.25);
    assert_eq!(o.max_fonts_cache_size(), 64);
    assert_eq!(o.max_symbols_cache_size(), 128);
}

#[test]
fn default_font_name_accepts_str_and_owned_string() {
    let mut o = RenderingOptions::new();
    o.set_default_font_name("Liberation Serif");
    assert_eq!(o.default_font_name(), "Liberation Serif");
    o.set_default_font_name(String::from("Inter"));
    assert_eq!(o.default_font_name(), "Inter");
}

#[test]
fn setting_one_property_does_not_touch_others() {
    let mut o = RenderingOptions::new();
    o.set_analyze_fonts(true);
    o.set_max_fonts_cache_size(64);
    o.set_default_font_name("X");

    // Only the three touched fields changed; everything else is
    // still at default.
    let mut expected = RenderingOptions::new();
    expected.set_analyze_fonts(true);
    expected.set_max_fonts_cache_size(64);
    expected.set_default_font_name("X");
    assert_eq!(o, expected);
}

#[test]
fn clone_is_independent_after_mutation() {
    let mut a = RenderingOptions::new();
    a.set_analyze_fonts(true);
    let b = a.clone();
    a.set_analyze_fonts(false);
    assert!(!a.analyze_fonts());
    assert!(b.analyze_fonts());
}
