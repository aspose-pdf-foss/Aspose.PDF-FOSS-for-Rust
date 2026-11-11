//! `Aspose.Pdf.Text.TextState` — the text styling state applied to
//! drawn / extracted text, per `capabilities/text_state.yaml`.
//!
//! A strict subset: the font-mutation surface (`font`, `font_size`),
//! the colour properties, the decoration / position flags, the
//! spacing / scaling scalars, and the horizontal alignment. The
//! enum-typed `RenderingMode` / `FontStyle` / `CoordinateOrigin`
//! members, `TabStops`, and the canonical constructors / measure
//! helpers are deferred. The type is `Default`-constructed (the
//! canonical constructors are not in the shipped subset).

use crate::public::color::Color;
use crate::public::font::Font;
use crate::public::horizontal_alignment::HorizontalAlignment;

/// Text styling state (font, colours, decoration, spacing).
#[derive(Debug, Clone, PartialEq)]
pub struct TextState {
    font: Option<Font>,
    font_size: f32,
    foreground_color: Color,
    background_color: Color,
    stroking_color: Color,
    underline: bool,
    strike_out: bool,
    subscript: bool,
    superscript: bool,
    invisible: bool,
    character_spacing: f32,
    word_spacing: f32,
    line_spacing: f32,
    horizontal_scaling: f32,
    horizontal_alignment: HorizontalAlignment,
}

impl Default for TextState {
    fn default() -> Self {
        Self {
            font: None,
            font_size: 0.0,
            foreground_color: Color::transparent(),
            background_color: Color::transparent(),
            stroking_color: Color::transparent(),
            underline: false,
            strike_out: false,
            subscript: false,
            superscript: false,
            invisible: false,
            character_spacing: 0.0,
            word_spacing: 0.0,
            line_spacing: 0.0,
            horizontal_scaling: 100.0,
            horizontal_alignment: HorizontalAlignment::None,
        }
    }
}

impl TextState {
    /// The width of `value` in user-space units under this state's font
    /// and size. Mirrors the canonical `TextState.MeasureString`.
    ///
    /// Measured from the Standard-14 metrics of the font's base name,
    /// which is what this crate's text placement uses; a character the
    /// metrics do not carry is measured as the fallback advance, the
    /// same substitution the renderer applies. With no font set the
    /// state has nothing to measure against and the width is zero.
    pub fn measure_string(&self, value: &str) -> f64 {
        let Some(font) = self.font.as_ref() else { return 0.0 };
        crate::public::text_builder::text_width(
            font.base_font(),
            value,
            self.font_size as f64,
        )
    }

    /// The font, or `None` when no font has been assigned.
    pub fn font(&self) -> Option<&Font> {
        self.font.as_ref()
    }
    /// Set the font.
    pub fn set_font(&mut self, value: Font) {
        self.font = Some(value);
    }

    /// The font size, in points.
    pub fn font_size(&self) -> f32 {
        self.font_size
    }
    /// Set the font size, in points.
    pub fn set_font_size(&mut self, value: f32) {
        self.font_size = value;
    }

    /// The fill (foreground) colour.
    pub fn foreground_color(&self) -> Color {
        self.foreground_color
    }
    /// Set the fill (foreground) colour.
    /// Set the colour the text paints in. A fully transparent colour is
    /// the unset state — text keeps painting in the viewer's default
    /// rather than disappearing. The colour reaches the page through the
    /// surfaces that take a whole state: a
    /// [`TextParagraph`](crate::TextParagraph) line and a table
    /// [`Cell`](crate::Cell)'s default cell text state.
    pub fn set_foreground_color(&mut self, value: Color) {
        self.foreground_color = value;
    }

    /// The background colour.
    ///
    /// Carried, not painted. A background is a filled rectangle behind
    /// the run, the way the underline and strike-out are filled
    /// rectangles through it — but those need only a position and a
    /// thickness, while a background needs the box the text occupies,
    /// and its height is the face's ascent and descent. The
    /// Standard-14 metric table exposes glyph widths and nothing
    /// vertical, so there is no measurement to draw the box from.
    pub fn background_color(&self) -> Color {
        self.background_color
    }
    /// Set the background colour.
    pub fn set_background_color(&mut self, value: Color) {
        self.background_color = value;
    }

    /// The stroking (outline) colour.
    ///
    /// Setting one asks for the glyph outlines to be painted, which
    /// only a stroking rendering mode does. This subset ships no
    /// rendering-mode member, so a stroking colour selects fill-and-
    /// stroke (`2 Tr`, §9.3.6) on its own. An invisible run stays
    /// invisible: painting nothing wins over painting the outline.
    pub fn stroking_color(&self) -> Color {
        self.stroking_color
    }
    /// Set the stroking (outline) colour.
    pub fn set_stroking_color(&mut self, value: Color) {
        self.stroking_color = value;
    }

    /// Whether the text is underlined.
    ///
    /// The format has no underline operator, so the rule is drawn: a
    /// filled rectangle the width of the run, at the position and
    /// thickness every Standard-14 family states in its own metrics
    /// (UnderlinePosition -100, UnderlineThickness 50, per mille of the
    /// font size). [`strike_out`](TextState::strike_out) draws the same
    /// rectangle a quarter of the size above the baseline — the metrics
    /// say nothing about that one, so the position is this crate's
    /// convention. Both follow the run's own fill colour, and an
    /// invisible run draws neither.
    ///
    /// Everything this state carries now reaches the page: the font and
    /// size, the fill and stroking colours, the character and word
    /// spacing, the horizontal scaling, the invisibility flag, the
    /// baseline shift and these two rules. One caveat on the shift —
    /// see [`superscript`](TextState::superscript).
    pub fn underline(&self) -> bool {
        self.underline
    }
    /// Set whether the text is underlined.
    pub fn set_underline(&mut self, value: bool) {
        self.underline = value;
    }

    /// Whether the text is struck through. The rule is drawn — see
    /// [`underline`](TextState::underline).
    pub fn strike_out(&self) -> bool {
        self.strike_out
    }
    /// Set whether the text is struck through.
    pub fn set_strike_out(&mut self, value: bool) {
        self.strike_out = value;
    }

    /// Whether the text is rendered as subscript: the run is written
    /// with a negative text rise (`Ts`, §9.3.7) of a fifth of the font
    /// size. See [`superscript`](TextState::superscript) for how the
    /// amount is chosen and what this crate's own renderer does with
    /// it.
    pub fn subscript(&self) -> bool {
        self.subscript
    }
    /// Set whether the text is rendered as subscript.
    pub fn set_subscript(&mut self, value: bool) {
        self.subscript = value;
    }

    /// Whether the text is rendered as superscript: the run is written
    /// with a text rise (`Ts`, §9.3.7) of a third of the font size.
    /// Superscript wins when both it and
    /// [`subscript`](TextState::subscript) are set.
    ///
    /// `Ts` is a plain number in the format, so the amount is the
    /// typographic convention this crate writes rather than a value the
    /// canonical surface gives. Note also that this crate's own
    /// renderer does not track `Ts` (nor `Tz`): a viewer honours the
    /// shift, while a page rastered here draws the run on the baseline.
    pub fn superscript(&self) -> bool {
        self.superscript
    }
    /// Set whether the text is rendered as superscript.
    pub fn set_superscript(&mut self, value: bool) {
        self.superscript = value;
    }

    /// Whether the text is invisible (rendering mode 3).
    /// Emitted as text-rendering mode 3 (`3 Tr`, §9.3.6): the text is
    /// laid out and selectable but never painted. Honoured by this
    /// crate's own rasteriser too.
    pub fn invisible(&self) -> bool {
        self.invisible
    }
    /// Set whether the text is invisible.
    pub fn set_invisible(&mut self, value: bool) {
        self.invisible = value;
    }

    /// The extra spacing between characters, in points.
    pub fn character_spacing(&self) -> f32 {
        self.character_spacing
    }
    /// Set the extra spacing between characters, in points — the `Tc`
    /// of PDF 32000-1:2008 §9.3.2. Zero is the default and emits no
    /// operator. Reaches the page through the surfaces that take a whole
    /// state: a [`TextFragment`](crate::TextFragment)'s own, a
    /// [`TextParagraph`](crate::TextParagraph) line, and a table
    /// [`Cell`](crate::Cell)'s default.
    pub fn set_character_spacing(&mut self, value: f32) {
        self.character_spacing = value;
    }

    /// The extra spacing between words, in points.
    pub fn word_spacing(&self) -> f32 {
        self.word_spacing
    }
    /// Set the extra spacing added at each space character, in points —
    /// the `Tw` of PDF 32000-1:2008 §9.3.3. Zero is the default and
    /// emits no operator.
    pub fn set_word_spacing(&mut self, value: f32) {
        self.word_spacing = value;
    }

    /// The line spacing, in points.
    pub fn line_spacing(&self) -> f32 {
        self.line_spacing
    }
    /// Set the line spacing, in points.
    pub fn set_line_spacing(&mut self, value: f32) {
        self.line_spacing = value;
    }

    /// The horizontal scaling, as a percentage (100 = normal).
    /// Emitted as `Tz` (§9.3.5), so a viewer honours it. This crate's
    /// own rasteriser does not: the shared renderer contract puts the
    /// `Tz` text-state setter out of scope for v1, so a page saved with
    /// a scaling other than 100 renders unscaled here and scaled
    /// elsewhere.
    pub fn horizontal_scaling(&self) -> f32 {
        self.horizontal_scaling
    }
    /// Set the horizontal scaling, as a percentage.
    pub fn set_horizontal_scaling(&mut self, value: f32) {
        self.horizontal_scaling = value;
    }

    /// The horizontal alignment.
    pub fn horizontal_alignment(&self) -> HorizontalAlignment {
        self.horizontal_alignment
    }
    /// Set the horizontal alignment.
    pub fn set_horizontal_alignment(&mut self, value: HorizontalAlignment) {
        self.horizontal_alignment = value;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::public::font_repository::FontRepository;

    #[test]
    fn defaults() {
        let ts = TextState::default();
        assert!(!ts.underline());
        assert!(!ts.strike_out());
        assert!(!ts.invisible());
        assert_eq!(ts.character_spacing(), 0.0);
        assert_eq!(ts.horizontal_scaling(), 100.0);
        assert_eq!(ts.horizontal_alignment(), HorizontalAlignment::None);
        assert!(ts.font().is_none());
    }

    #[test]
    fn colour_properties() {
        let mut ts = TextState::default();
        assert_eq!(ts.foreground_color().a(), 0.0);
        ts.set_foreground_color(Color::from_rgb(1.0, 0.0, 0.0));
        assert_eq!(ts.foreground_color().a(), 1.0);
        ts.set_stroking_color(Color::from_rgb(0.0, 1.0, 0.0));
        assert_eq!(ts.stroking_color().a(), 1.0);
        ts.set_background_color(Color::transparent());
        assert_eq!(ts.background_color().a(), 0.0);
    }

    #[test]
    fn flags_spacing_and_font() {
        let mut ts = TextState::default();
        ts.set_underline(true);
        ts.set_strike_out(true);
        ts.set_subscript(true);
        ts.set_superscript(true);
        ts.set_invisible(true);
        ts.set_character_spacing(2.5);
        ts.set_word_spacing(1.5);
        ts.set_line_spacing(14.0);
        ts.set_horizontal_scaling(80.0);
        ts.set_horizontal_alignment(HorizontalAlignment::Center);
        ts.set_font(FontRepository::find_font("Helvetica"));
        ts.set_font_size(12.0);

        assert!(ts.underline() && ts.strike_out() && ts.subscript());
        assert!(ts.superscript() && ts.invisible());
        assert_eq!(ts.character_spacing(), 2.5);
        assert_eq!(ts.word_spacing(), 1.5);
        assert_eq!(ts.line_spacing(), 14.0);
        assert_eq!(ts.horizontal_scaling(), 80.0);
        assert_eq!(ts.horizontal_alignment(), HorizontalAlignment::Center);
        assert_eq!(ts.font().unwrap().font_name(), "Helvetica");
        assert_eq!(ts.font_size(), 12.0);
    }
}
