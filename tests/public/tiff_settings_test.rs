// Public-API smoke driver — TiffSettings + supporting enums.
//
// Coverage:
//   * Default ctor carries the canonical Aspose.PDF baseline
//     (LZW compression, default depth, no shape, brightness 0.5,
//     CropBox coordinate type, no skip-blank-pages, zero margins).
//   * Each per-axis ctor (with_shape / with_compression /
//     with_depth / with_margins / with_skip_blank_pages) sets
//     only the named axis; the others stay at the default.
//   * The combination ctors wire every passed field.
//   * Per-axis setters round-trip on a default-constructed bag.
//   * Enum variant ordinals match the canonical
//     Aspose.Pdf.Devices.* + Aspose.Pdf.PageCoordinateType
//     ordering so the discriminant round-trips through interop.

use aspose_pdf_foss::{
    ColorDepth, CompressionType, Margins, PageCoordinateType,
    ShapeType, TiffSettings,
};

// ============================================================
// TiffSettings — default ctor + per-axis builders.
// ============================================================

#[test]
fn default_ctor_matches_canonical_baseline() {
    let s = TiffSettings::new();
    assert_eq!(s.compression(), CompressionType::Lzw);
    assert_eq!(s.depth(), ColorDepth::Default);
    assert_eq!(s.shape(), ShapeType::None);
    assert_eq!(s.coordinate_type(), PageCoordinateType::CropBox);
    assert!((s.brightness() - 0.5).abs() < f32::EPSILON);
    assert!(!s.skip_blank_pages());
    assert_eq!(s.margins(), Margins::new());
}

#[test]
fn default_trait_matches_explicit_new() {
    let s: TiffSettings = Default::default();
    assert_eq!(s, TiffSettings::new());
}

#[test]
fn with_shape_only_touches_shape() {
    let s = TiffSettings::with_shape(ShapeType::Landscape);
    assert_eq!(s.shape(), ShapeType::Landscape);
    assert_eq!(s.compression(), CompressionType::Lzw);
    assert_eq!(s.depth(), ColorDepth::Default);
}

#[test]
fn with_compression_only_touches_compression() {
    let s = TiffSettings::with_compression(CompressionType::None);
    assert_eq!(s.compression(), CompressionType::None);
    assert_eq!(s.shape(), ShapeType::None);
    assert_eq!(s.depth(), ColorDepth::Default);
}

#[test]
fn with_depth_only_touches_depth() {
    let s = TiffSettings::with_depth(ColorDepth::Format8bpp);
    assert_eq!(s.depth(), ColorDepth::Format8bpp);
    assert_eq!(s.compression(), CompressionType::Lzw);
}

#[test]
fn with_margins_only_touches_margins() {
    let m = Margins::with_sides(1, 2, 3, 4);
    let s = TiffSettings::with_margins(m);
    assert_eq!(s.margins(), m);
    assert_eq!(s.compression(), CompressionType::Lzw);
}

#[test]
fn with_skip_blank_pages_only_touches_skip() {
    let s = TiffSettings::with_skip_blank_pages(true);
    assert!(s.skip_blank_pages());
    assert_eq!(s.compression(), CompressionType::Lzw);
}

#[test]
fn with_compression_depth_margins_combo() {
    let m = Margins::with_sides(5, 6, 7, 8);
    let s = TiffSettings::with_compression_depth_margins(
        CompressionType::Rle,
        ColorDepth::Format24bpp,
        m,
    );
    assert_eq!(s.compression(), CompressionType::Rle);
    assert_eq!(s.depth(), ColorDepth::Format24bpp);
    assert_eq!(s.margins(), m);
    assert!(!s.skip_blank_pages());
    assert_eq!(s.shape(), ShapeType::None);
}

#[test]
fn with_compression_depth_margins_skip_combo() {
    let m = Margins::with_sides(9, 10, 11, 12);
    let s = TiffSettings::with_compression_depth_margins_skip(
        CompressionType::Ccitt4,
        ColorDepth::Format1bpp,
        m,
        true,
    );
    assert_eq!(s.compression(), CompressionType::Ccitt4);
    assert_eq!(s.depth(), ColorDepth::Format1bpp);
    assert_eq!(s.margins(), m);
    assert!(s.skip_blank_pages());
    assert_eq!(s.shape(), ShapeType::None);
}

#[test]
fn with_compression_depth_margins_skip_shape_combo() {
    let m = Margins::with_sides(13, 14, 15, 16);
    let s = TiffSettings::with_compression_depth_margins_skip_shape(
        CompressionType::Lzw,
        ColorDepth::Format4bpp,
        m,
        true,
        ShapeType::Portrait,
    );
    assert_eq!(s.compression(), CompressionType::Lzw);
    assert_eq!(s.depth(), ColorDepth::Format4bpp);
    assert_eq!(s.margins(), m);
    assert!(s.skip_blank_pages());
    assert_eq!(s.shape(), ShapeType::Portrait);
}

#[test]
fn per_axis_setters_round_trip() {
    let mut s = TiffSettings::new();
    s.set_skip_blank_pages(true);
    s.set_compression(CompressionType::Rle);
    s.set_depth(ColorDepth::Format8bpp);
    s.set_shape(ShapeType::Landscape);
    s.set_brightness(0.25);
    s.set_coordinate_type(PageCoordinateType::MediaBox);

    assert!(s.skip_blank_pages());
    assert_eq!(s.compression(), CompressionType::Rle);
    assert_eq!(s.depth(), ColorDepth::Format8bpp);
    assert_eq!(s.shape(), ShapeType::Landscape);
    assert!((s.brightness() - 0.25).abs() < f32::EPSILON);
    assert_eq!(s.coordinate_type(), PageCoordinateType::MediaBox);
}

#[test]
fn copy_semantics_independent_after_mutation() {
    let s1 = TiffSettings::with_compression(CompressionType::None);
    let mut s2 = s1;
    s2.set_compression(CompressionType::Rle);
    assert_eq!(s1.compression(), CompressionType::None);
    assert_eq!(s2.compression(), CompressionType::Rle);
}

// ============================================================
// Enum ordinals — must match the canonical Aspose.PDF 26.4.0
// surface so the discriminants round-trip through interop.
// ============================================================

#[test]
fn color_depth_ordinals_match_canonical() {
    assert_eq!(ColorDepth::Default as i32, 0);
    assert_eq!(ColorDepth::Format24bpp as i32, 1);
    assert_eq!(ColorDepth::Format8bpp as i32, 2);
    assert_eq!(ColorDepth::Format4bpp as i32, 3);
    assert_eq!(ColorDepth::Format1bpp as i32, 4);
}

#[test]
fn compression_type_ordinals_match_canonical() {
    assert_eq!(CompressionType::Lzw as i32, 0);
    assert_eq!(CompressionType::Ccitt4 as i32, 1);
    assert_eq!(CompressionType::Ccitt3 as i32, 2);
    assert_eq!(CompressionType::Rle as i32, 3);
    assert_eq!(CompressionType::None as i32, 4);
}

#[test]
fn shape_type_ordinals_match_canonical() {
    assert_eq!(ShapeType::None as i32, 0);
    assert_eq!(ShapeType::Landscape as i32, 1);
    assert_eq!(ShapeType::Portrait as i32, 2);
}

#[test]
fn page_coordinate_type_ordinals_match_canonical() {
    assert_eq!(PageCoordinateType::MediaBox as i32, 0);
    assert_eq!(PageCoordinateType::CropBox as i32, 1);
}

#[test]
fn enum_defaults_match_canonical() {
    // ColorDepth::Default is the zero-valued default.
    assert_eq!(ColorDepth::default(), ColorDepth::Default);
    // CompressionType default is Lzw (zero-valued).
    assert_eq!(CompressionType::default(), CompressionType::Lzw);
    // ShapeType default is None (zero-valued).
    assert_eq!(ShapeType::default(), ShapeType::None);
    // PageCoordinateType default is CropBox (canonical engine
    // default, NOT the zero-valued variant).
    assert_eq!(PageCoordinateType::default(), PageCoordinateType::CropBox);
}
