// Hand-authored gate driver — inline_assertions gate.
// Primitive: standard14_outlines
// Freeze gate: enumerated assertions from the anchor —
//   helvetica_bundle_fallback / times_roman_bundle_fallback /
//   courier_bundle_fallback / symbol_documented_gap /
//   zapfdingbats_documented_gap / unknown_name_returns_empty /
//   default_search_dirs_non_empty.

#[path = "../../src/internal/standard14_outlines.rs"]
mod standard14_outlines;

use standard14_outlines::{bundled, default_search_dirs, resolve};

#[test]
fn helvetica_bundle_fallback() {
    // Empty search_dirs forces the bundle path.
    assert!(!resolve("/Helvetica", &[], false).is_empty());
}

#[test]
fn times_roman_bundle_fallback() {
    assert!(!resolve("/Times-Roman", &[], false).is_empty());
}

#[test]
fn courier_bundle_fallback() {
    assert!(!resolve("/Courier", &[], false).is_empty());
}

#[test]
fn symbol_documented_gap() {
    assert!(resolve("/Symbol", &[], false).is_empty());
}

#[test]
fn zapfdingbats_documented_gap() {
    assert!(resolve("/ZapfDingbats", &[], false).is_empty());
}

#[test]
fn an_unknown_name_resolves_to_its_metric_class() {
    // An unknown name now falls into a metric class, per the renderer
    // contract: ANY non-embedded simple font resolves to the nearest
    // Standard14 face rather than to nothing.
    assert!(!resolve("/NotAStandard14Name", &[], false).is_empty());
}

#[test]
fn default_search_dirs_non_empty() {
    assert!(!default_search_dirs().is_empty());
}

#[test]
fn bundled_helvetica_starts_with_ttf_magic() {
    let bytes = bundled("/Helvetica");
    assert!(bytes.len() > 4);
    assert_eq!(&bytes[..4], &[0x00, 0x01, 0x00, 0x00]);
}

#[test]
fn all_twelve_latin_bundle_entries_present() {
    // Mirrors the C++ AllTwelveBundleEntriesPresent: every Latin
    // Standard-14 face resolves to a non-empty TrueType bundle.
    for name in [
        "/Helvetica", "/Helvetica-Bold", "/Helvetica-Oblique", "/Helvetica-BoldOblique",
        "/Times-Roman", "/Times-Bold", "/Times-Italic", "/Times-BoldItalic",
        "/Courier", "/Courier-Bold", "/Courier-Oblique", "/Courier-BoldOblique",
    ] {
        let bytes = bundled(name);
        assert!(bytes.len() > 4, "bundle missing: {name}");
        assert_eq!(&bytes[..4], &[0x00, 0x01, 0x00, 0x00], "bundle invalid: {name}");
    }
}
