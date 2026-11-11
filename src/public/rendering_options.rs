//! `Aspose.Pdf.RenderingOptions` — mutable bag of fine-grained
//! rendering policy, accepted by the image-device pipelines
//! ([`PngDevice`](crate::PngDevice), [`JpegDevice`](crate::JpegDevice),
//! [`BmpDevice`](crate::BmpDevice), [`TiffDevice`](crate::TiffDevice)
//! via [`ImageDevice`](crate::ImageDevice)).
//!
//! Surface mirrors `capabilities/rendering_options.yaml`: 13
//! read-write properties. The devices carry the bag — every
//! [`ImageDevice`](crate::ImageDevice) exposes it for reading, mutation
//! and replacement.
//!
//! **Carried, not consulted.** The renderer reads none of it, and the
//! reason is structural rather than per-property: this bag is a *policy*
//! argument, and the shared renderer contract takes a page, a
//! resolution and a background, not a policy. Honouring any of these
//! would start by widening that contract, which is a decision for every
//! track that implements it, not for this one.
//!
//! What each would need beyond that: a glyph and font cache to size
//! (`max_fonts_cache_size`, `max_symbols_cache_size`), a hinting
//! pipeline (`use_font_hinting`), a resampling filter with a quality
//! switch (`interpolation_high_quality`), a font-analysis and
//! TTF-conversion pass (`analyze_fonts`,
//! `convert_fonts_to_unicode_ttf`), a barcode recogniser
//! (`barcode_optimization`), a native platform text rasteriser
//! (`system_fonts_native_rendering`), and canvas-geometry policy
//! (`optimize_dimensions`, `width_extra_units`, `height_extra_units`).
//!
//! Two of the thirteen are already answered elsewhere, and worth naming
//! so they are not looked for here. `ignore_resource_font_errors` asks
//! for a failed font lookup to be survivable, which it already is —
//! see the property's own note. And `default_font_name` would pick the
//! substitute for a font a document does not embed. The renderer does
//! substitute — it maps any such font onto the nearest Standard 14 face
//! by family, weight and slope, with the `/FontDescriptor` serif bit
//! deciding an unknown family — so text paints rather than coming out
//! blank, and this property would only override that choice.
//!
//! What DOES steer the devices is their own surface — the resolution,
//! the page size, the colour depth and the palette converter a TIFF
//! device is given.

/// Bag of rendering policy consulted by image-device pipelines.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct RenderingOptions {
    analyze_fonts: bool,
    barcode_optimization: bool,
    convert_fonts_to_unicode_ttf: bool,
    default_font_name: String,
    height_extra_units: f32,
    ignore_resource_font_errors: bool,
    interpolation_high_quality: bool,
    max_fonts_cache_size: i32,
    max_symbols_cache_size: i32,
    optimize_dimensions: bool,
    system_fonts_native_rendering: bool,
    use_font_hinting: bool,
    width_extra_units: f32,
}

impl RenderingOptions {
    /// Default-construct with engine-default rendering policy:
    /// every flag `false`, numeric extras `0`, default font name
    /// empty, cache caps `0` (engine picks).
    pub fn new() -> Self {
        Self::default()
    }

    /// Enables font-replacement analysis so missing or partial
    /// fonts are substituted via the engine's substitution
    /// algorithm.
    pub fn analyze_fonts(&self) -> bool {
        self.analyze_fonts
    }

    /// Set [`RenderingOptions::analyze_fonts`].
    pub fn set_analyze_fonts(&mut self, value: bool) {
        self.analyze_fonts = value;
    }

    /// Barcode-aware rasterisation tweaks.
    pub fn barcode_optimization(&self) -> bool {
        self.barcode_optimization
    }

    /// Set [`RenderingOptions::barcode_optimization`].
    pub fn set_barcode_optimization(&mut self, value: bool) {
        self.barcode_optimization = value;
    }

    /// Convert every font to a Unicode TrueType variant before
    /// rasterising.
    pub fn convert_fonts_to_unicode_ttf(&self) -> bool {
        self.convert_fonts_to_unicode_ttf
    }

    /// Set [`RenderingOptions::convert_fonts_to_unicode_ttf`].
    pub fn set_convert_fonts_to_unicode_ttf(&mut self, value: bool) {
        self.convert_fonts_to_unicode_ttf = value;
    }

    /// Substitute font name when a referenced font is missing.
    /// Default is an empty string.
    pub fn default_font_name(&self) -> &str {
        &self.default_font_name
    }

    /// Set [`RenderingOptions::default_font_name`].
    pub fn set_default_font_name(&mut self, value: impl Into<String>) {
        self.default_font_name = value.into();
    }

    /// Signed adjustment to rectangle height for the
    /// `AppendRectangle` operator — compensates for rasteriser
    /// rounding.
    pub fn height_extra_units(&self) -> f32 {
        self.height_extra_units
    }

    /// Set [`RenderingOptions::height_extra_units`].
    pub fn set_height_extra_units(&mut self, value: f32) {
        self.height_extra_units = value;
    }

    /// Whether font-resource lookup failures are suppressed.
    ///
    /// Carried, not consulted, like the rest of this bag — and worth
    /// reading with the module note above, because the outcome it names
    /// is what happens either way: the renderer substitutes a metric
    /// class for any font it cannot resolve, so a lookup failure never
    /// stops a page rendering and there is nothing for this to
    /// suppress.
    pub fn ignore_resource_font_errors(&self) -> bool {
        self.ignore_resource_font_errors
    }

    /// Set [`RenderingOptions::ignore_resource_font_errors`].
    pub fn set_ignore_resource_font_errors(&mut self, value: bool) {
        self.ignore_resource_font_errors = value;
    }

    /// Use the higher-quality (slower) image-interpolation kernel.
    pub fn interpolation_high_quality(&self) -> bool {
        self.interpolation_high_quality
    }

    /// Set [`RenderingOptions::interpolation_high_quality`].
    pub fn set_interpolation_high_quality(&mut self, value: bool) {
        self.interpolation_high_quality = value;
    }

    /// Upper bound on the per-document font cache. `0` (the
    /// default) lets the engine pick.
    pub fn max_fonts_cache_size(&self) -> i32 {
        self.max_fonts_cache_size
    }

    /// Set [`RenderingOptions::max_fonts_cache_size`].
    pub fn set_max_fonts_cache_size(&mut self, value: i32) {
        self.max_fonts_cache_size = value;
    }

    /// Upper bound on the per-document glyph / symbol cache. `0`
    /// (the default) lets the engine pick.
    pub fn max_symbols_cache_size(&self) -> i32 {
        self.max_symbols_cache_size
    }

    /// Set [`RenderingOptions::max_symbols_cache_size`].
    pub fn set_max_symbols_cache_size(&mut self, value: i32) {
        self.max_symbols_cache_size = value;
    }

    /// Snap rendered page dimensions to engine-preferred values.
    pub fn optimize_dimensions(&self) -> bool {
        self.optimize_dimensions
    }

    /// Set [`RenderingOptions::optimize_dimensions`].
    pub fn set_optimize_dimensions(&mut self, value: bool) {
        self.optimize_dimensions = value;
    }

    /// Render system-resolved fonts using the OS-native rasteriser
    /// instead of the engine's.
    pub fn system_fonts_native_rendering(&self) -> bool {
        self.system_fonts_native_rendering
    }

    /// Set [`RenderingOptions::system_fonts_native_rendering`].
    pub fn set_system_fonts_native_rendering(&mut self, value: bool) {
        self.system_fonts_native_rendering = value;
    }

    /// Glyph hinting — most effective for TrueType fonts at low DPI.
    pub fn use_font_hinting(&self) -> bool {
        self.use_font_hinting
    }

    /// Set [`RenderingOptions::use_font_hinting`].
    pub fn set_use_font_hinting(&mut self, value: bool) {
        self.use_font_hinting = value;
    }

    /// Signed adjustment to rectangle width for the
    /// `AppendRectangle` operator — mirror of
    /// [`RenderingOptions::height_extra_units`].
    pub fn width_extra_units(&self) -> f32 {
        self.width_extra_units
    }

    /// Set [`RenderingOptions::width_extra_units`].
    pub fn set_width_extra_units(&mut self, value: f32) {
        self.width_extra_units = value;
    }
}
