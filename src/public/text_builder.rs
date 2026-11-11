//! `Aspose.Pdf.Text.TextBuilder` — appends text to a page, per
//! `capabilities/text_builder.yaml`.
//!
//! Built from a [`Page`], it writes positioned text into the page's
//! content stream: [`append_text`](TextBuilder::append_text) places
//! a single [`TextFragment`] at its position, and
//! [`append_paragraph`](TextBuilder::append_paragraph) lays a
//! [`TextParagraph`]'s lines into its box with the paragraph's
//! alignment and indents. The edits are buffered on the document and
//! persisted on [`Document::save`](crate::Document::save) as an
//! incremental update (new content stream + base font resource).
//!
//! Fonts render under their base name; embedding an opened font's
//! program (`FontFile2`) is a follow-on. The operator-collection and
//! `IList` constructors are dropped.

use crate::internal::standard14_metrics;
use crate::public::horizontal_alignment::HorizontalAlignment;
use crate::public::page::Page;
use crate::public::page_edits::{EditBuffer, EditFont, PageContentEdit};
use crate::public::text_fragment::TextFragment;
use crate::public::text_paragraph::TextParagraph;

const DEFAULT_FONT: &str = "Helvetica";
const DEFAULT_SIZE: f64 = 12.0;

/// Appends text to a page's content stream.
pub struct TextBuilder {
    leaf_id: crate::public::page_edits::LeafRef,
    edits: EditBuffer,
}

impl TextBuilder {
    /// Construct a builder bound to `page`.
    pub fn new(page: &Page) -> Self {
        Self { leaf_id: page.leaf_ref(), edits: std::rc::Rc::clone(&page.edits) }
    }

    /// Append a single positioned text fragment.
    pub fn append_text(&self, fragment: &TextFragment) {
        if let Some((content, font)) = render_text_fragment(fragment) {
            self.edits.borrow_mut().content.push(PageContentEdit {
                leaf_id: std::rc::Rc::clone(&self.leaf_id),
                content,
                fonts: vec![font],
            });
        }
    }

    /// Append a single positioned text fragment, rotated about its own
    /// position by `degrees` anticlockwise. Crate-internal: no
    /// canonical member asks for a rotated fragment, but the stamping
    /// facades carry a rotation and need a way to write it.
    pub(crate) fn append_text_rotated(&self, fragment: &TextFragment, degrees: f64) {
        let ts = &fragment.text_state().state;
        let mut style = RunStyle::of(ts);
        style.rotation = (degrees != 0.0).then_some(degrees);
        if let Some((content, font)) =
            render_text_fragment_styled(fragment, ts.font(), ts.font_size(), style)
        {
            self.edits.borrow_mut().content.push(PageContentEdit {
                leaf_id: std::rc::Rc::clone(&self.leaf_id),
                content,
                fonts: vec![font],
            });
        }
    }

    /// Append a laid-out paragraph, positioned within its box (or at its
    /// position when no box is set) with its alignment and indents.
    ///
    /// Each appended line is one logical paragraph and is broken across
    /// as many rendered lines as its box width needs. Breaks fall at
    /// whitespace: a single word wider than the box takes a line of its
    /// own rather than being split. Without a box there is no width to
    /// measure against, so each appended line renders as one run.
    pub fn append_paragraph(&self, paragraph: &TextParagraph) {
        let rect = paragraph.rectangle();
        let use_rect = !rect.is_trivial();
        let (llx, urx, top_y) = if use_rect {
            (rect.llx(), rect.urx(), rect.ury())
        } else {
            let p = paragraph.position();
            (p.x_indent(), p.x_indent(), p.y_indent())
        };

        let mut content = Vec::new();
        let mut fonts: Vec<EditFont> = Vec::new();
        let mut y = top_y;
        let mut first = true;

        for line in &paragraph.lines {
            let size = positive_or(
                line.state.as_ref().map_or(0.0, |s| s.font_size() as f64),
                DEFAULT_SIZE,
            );
            let font = line.state.as_ref().and_then(|s| s.font());
            let base_font = font
                .map(|f| f.base_font().to_string())
                .unwrap_or_else(|| DEFAULT_FONT.to_string());
            let program = embeddable_program(font);
            let is_subset = wants_subset(font);
            let res = font_resource_name(&base_font);
            let line_height =
                positive_or(line.line_spacing.map_or(0.0, |s| s as f64), size * 1.2);

            let indent_of = |first_line: bool| -> f64 {
                if first_line {
                    paragraph.first_line_indent() as f64
                } else {
                    paragraph.subsequent_lines_indent() as f64
                }
            };
            let width_for = |first_line: bool| -> Option<f64> {
                if use_rect {
                    Some((urx - llx - indent_of(first_line)).max(0.0))
                } else {
                    None
                }
            };

            for text in wrap_line(&line.text, &base_font, size, width_for(first), width_for(false))
            {
                if first {
                    y = if use_rect { top_y - size } else { top_y };
                } else {
                    y -= line_height;
                }

                let indent = indent_of(first);
                let x = if use_rect {
                    match paragraph.horizontal_alignment() {
                        HorizontalAlignment::Center => {
                            let w = text_width(&base_font, &text, size);
                            llx + (urx - llx - w) / 2.0
                        }
                        HorizontalAlignment::Right => {
                            let w = text_width(&base_font, &text, size);
                            urx - w
                        }
                        _ => llx + indent,
                    }
                } else {
                    llx + indent
                };

                let style = line.state.as_ref().map(RunStyle::of).unwrap_or_default();
                content.extend_from_slice(&render_run(
                    &res, &base_font, size, x, y, &text, style,
                ));
                let line_codes = used_codes_of(&text);
                match fonts.iter_mut().find(|f| f.resource_name == res) {
                    Some(f) => f.used_codes.extend_from_slice(&line_codes),
                    None => fonts.push(EditFont {
                        resource_name: res.clone(),
                        base_font: base_font.clone(),
                        program: program.clone(),
                        is_subset,
                        used_codes: line_codes,
                    }),
                }
                first = false;
            }
        }

        if !content.is_empty() {
            self.edits.borrow_mut().content.push(PageContentEdit {
                leaf_id: std::rc::Rc::clone(&self.leaf_id),
                content,
                fonts,
            });
        }
    }
}

/// Break `text` into the lines it occupies, given the width available to
/// the first line and to every line after it. Breaks fall at whitespace;
/// a word wider than the line gets a line to itself rather than being
/// split mid-word.
///
/// Returns the text unchanged as a single line when it fits, or when
/// there is no width to measure against — so a caller that never set a
/// box gets exactly what it wrote, spacing included.
fn wrap_line(
    text: &str,
    base_font: &str,
    size: f64,
    first_width: Option<f64>,
    rest_width: Option<f64>,
) -> Vec<String> {
    let (Some(first_width), Some(rest_width)) = (first_width, rest_width) else {
        return vec![text.to_string()];
    };
    if first_width <= 0.0 || text_width(base_font, text, size) <= first_width {
        return vec![text.to_string()];
    }

    let mut lines: Vec<String> = Vec::new();
    let mut current = String::new();
    for word in text.split_whitespace() {
        if current.is_empty() {
            current.push_str(word);
            continue;
        }
        let limit = if lines.is_empty() { first_width } else { rest_width };
        let mut candidate = String::with_capacity(current.len() + 1 + word.len());
        candidate.push_str(&current);
        candidate.push(' ');
        candidate.push_str(word);
        if text_width(base_font, &candidate, size) <= limit {
            current = candidate;
        } else {
            lines.push(std::mem::take(&mut current));
            current.push_str(word);
        }
    }
    if !current.is_empty() {
        lines.push(current);
    }
    if lines.is_empty() {
        lines.push(text.to_string());
    }
    lines
}

fn positive_or(v: f64, default: f64) -> f64 {
    if v > 0.0 {
        v
    } else {
        default
    }
}

/// The embeddable TrueType program for a font, when it was opened from
/// a file and embedding is enabled.
fn embeddable_program(font: Option<&crate::public::font::Font>) -> Option<Vec<u8>> {
    font.filter(|f| f.is_embedded()).and_then(|f| f.program.clone())
}

/// Whether the font requests glyph subsetting on embed.
fn wants_subset(font: Option<&crate::public::font::Font>) -> bool {
    font.is_some_and(|f| f.is_embedded() && f.is_subset())
}

/// The WinAnsi byte char-codes a text run draws (each char clamped to a
/// single Latin-1 byte, matching the literal-string emission).
fn used_codes_of(text: &str) -> Vec<u8> {
    text.chars()
        .map(|ch| if (ch as u32) <= 0xFF { ch as u8 } else { b'?' })
        .collect()
}

/// Render a single positioned [`TextFragment`] to `(content operators,
/// font resource)`, or `None` for empty text. Shared by
/// [`TextBuilder::append_text`] and the fragment's paragraph
/// render-on-save.
pub(crate) fn render_text_fragment(fragment: &TextFragment) -> Option<(Vec<u8>, EditFont)> {
    let ts = &fragment.text_state().state;
    render_text_fragment_styled(fragment, ts.font(), ts.font_size(), RunStyle::of(ts))
}

/// The rendered width of `fragment` at a font and size resolved by its
/// container. `None` for empty text, matching `rendered_width`.
pub(crate) fn fragment_width(
    fragment: &TextFragment,
    font: Option<&crate::public::font::Font>,
    font_size: f32,
) -> Option<f64> {
    if fragment.text().is_empty() {
        return None;
    }
    let size = positive_or(font_size as f64, DEFAULT_SIZE);
    let base_font = font
        .map(|f| f.base_font().to_string())
        .unwrap_or_else(|| DEFAULT_FONT.to_string());
    Some(text_width(&base_font, fragment.text(), size))
}

/// Render `fragment` with a font and size resolved by its container —
/// a [`Table`](crate::Table) cell applying its `DefaultCellTextState`.
/// A size of zero falls back to the default size, an absent font to the
/// default base font, exactly as an unstyled fragment does.
pub(crate) fn render_text_fragment_styled(
    fragment: &TextFragment,
    font: Option<&crate::public::font::Font>,
    font_size: f32,
    style: RunStyle,
) -> Option<(Vec<u8>, EditFont)> {
    let text = fragment.text();
    if text.is_empty() {
        return None;
    }
    let size = positive_or(font_size as f64, DEFAULT_SIZE);
    let base_font = font
        .map(|f| f.base_font().to_string())
        .unwrap_or_else(|| DEFAULT_FONT.to_string());
    let program = embeddable_program(font);
    let is_subset = wants_subset(font);
    let used_codes = used_codes_of(text);
    let res = font_resource_name(&base_font);
    let pos = fragment.position();
    let content = render_run(
        &res,
        &base_font,
        size,
        pos.x_indent(),
        pos.y_indent(),
        text,
        style,
    );
    Some((content, EditFont { resource_name: res, base_font, program, is_subset, used_codes }))
}

/// A content-stream resource name for a base font: `ASP` + the base
/// font name with non-alphanumeric bytes mapped to `_`.
fn font_resource_name(base_font: &str) -> Vec<u8> {
    let mut name = b"ASP".to_vec();
    for b in base_font.bytes() {
        name.push(if b.is_ascii_alphanumeric() { b } else { b'_' });
    }
    name
}

/// The styling a run carries beyond its font and size. Each field is
/// `None` when the caller set nothing, in which case no operator is
/// emitted and the viewer's default stands — that is what keeps an
/// untouched document byte-verbatim on resave.
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct RunStyle {
    /// Fill colour as `(r, g, b)`, each in `0.0..=1.0`.
    pub color: Option<(f64, f64, f64)>,
    /// Extra spacing between characters, in points (`Tc`).
    pub character_spacing: Option<f64>,
    /// Extra spacing added at each space character, in points (`Tw`).
    pub word_spacing: Option<f64>,
    /// Horizontal scaling as a percentage (`Tz`, §9.3.5). `None` is the
    /// default 100.
    pub horizontal_scaling: Option<f64>,
    /// `true` when the run is invisible — text-rendering mode 3
    /// (`3 Tr`, §9.3.6): laid out and selectable, never painted.
    pub invisible: bool,
    /// Glyph outline colour as `(r, g, b)`. A stroking colour is only
    /// visible in a rendering mode that strokes, and this subset ships
    /// no rendering-mode member, so setting one selects fill-and-stroke
    /// (`2 Tr`, §9.3.6).
    pub stroke_color: Option<(f64, f64, f64)>,
    /// `true` when the run sits above the baseline (`Ts`, §9.3.7).
    /// Superscript wins when both are asked for.
    pub superscript: bool,
    /// `true` when the run sits below the baseline (`Ts`, §9.3.7).
    pub subscript: bool,
    /// `true` when a line is drawn under the run.
    pub underline: bool,
    /// `true` when a line is drawn through the run.
    pub strike_out: bool,
    /// The run's rotation about its own origin, in degrees
    /// anticlockwise. `None` writes the usual `Td`; a rotation needs
    /// the text matrix instead (`Tm`, §9.4.2). Not reachable from a
    /// text state — the facades that stamp rotated text set it.
    pub rotation: Option<f64>,
}

/// How far a superscript or subscript moves off the baseline, as a
/// fraction of the font size. The format leaves the amount to the
/// producer — `Ts` is a plain number — so these are the typographic
/// convention this crate writes, not values read off the catalogue.
const SUPERSCRIPT_RISE: f64 = 1.0 / 3.0;
const SUBSCRIPT_RISE: f64 = -1.0 / 5.0;

/// Where an underline sits and how thick it is, as fractions of the
/// font size. Every Standard-14 family states the same pair in its
/// metrics — UnderlinePosition -100, UnderlineThickness 50, per mille —
/// so these are that convention rather than a guess.
const UNDERLINE_POSITION: f64 = -0.100;
const UNDERLINE_THICKNESS: f64 = 0.050;

/// Where a line through the run sits, as a fraction of the font size.
/// The metrics state nothing about this one; a quarter of the size puts
/// it through the middle of the lowercase letters.
const STRIKE_OUT_POSITION: f64 = 0.25;

impl RunStyle {
    /// The styling a text state asks for. Zero spacing is the PDF
    /// default, so it counts as "not set" and emits nothing.
    pub(crate) fn of(state: &crate::public::text_state::TextState) -> Self {
        Self {
            color: fill_color_of(&state.foreground_color()),
            character_spacing: nonzero(state.character_spacing()),
            word_spacing: nonzero(state.word_spacing()),
            // 100 % is the PDF default and needs no operator.
            horizontal_scaling: (state.horizontal_scaling() != 100.0)
                .then(|| state.horizontal_scaling() as f64),
            invisible: state.invisible(),
            stroke_color: fill_color_of(&state.stroking_color()),
            superscript: state.superscript(),
            subscript: state.subscript(),
            underline: state.underline(),
            strike_out: state.strike_out(),
            rotation: None,
        }
    }

    fn is_empty(&self) -> bool {
        self.color.is_none()
            && self.character_spacing.is_none()
            && self.word_spacing.is_none()
            && self.horizontal_scaling.is_none()
            && !self.invisible
            && self.stroke_color.is_none()
            && !self.superscript
            && !self.subscript
            && !self.underline
            && !self.strike_out
            && self.rotation.is_none()
    }

    /// The text rise this style asks for, in points at `size`, or
    /// `None` when the run sits on the baseline.
    fn rise(&self, size: f64) -> Option<f64> {
        if self.superscript {
            Some(size * SUPERSCRIPT_RISE)
        } else if self.subscript {
            Some(size * SUBSCRIPT_RISE)
        } else {
            None
        }
    }
}

fn nonzero(v: f32) -> Option<f64> {
    (v != 0.0).then_some(v as f64)
}

/// Render one positioned text run as `BT /res size Tf x y Td (text) Tj ET`.
///
/// Anything `style` carries is emitted inside a `q` / `Q` pair: the fill
/// colour and the `Tc` / `Tw` spacing are graphics state, and without the
/// pair they would apply to everything the content stream draws next.
fn render_run(
    res: &[u8],
    base_font: &str,
    size: f64,
    x: f64,
    y: f64,
    text: &str,
    style: RunStyle,
) -> Vec<u8> {
    use crate::public::shape::fmt_num;
    let scoped = !style.is_empty();
    let mut out = Vec::new();
    if scoped {
        out.extend_from_slice(b"q\n");
    }
    if let Some((r, g, b)) = style.color {
        out.extend_from_slice(
            format!("{} {} {} rg\n", fmt_num(r), fmt_num(g), fmt_num(b)).as_bytes(),
        );
    }
    if let Some((r, g, b)) = style.stroke_color {
        out.extend_from_slice(
            format!("{} {} {} RG\n", fmt_num(r), fmt_num(g), fmt_num(b)).as_bytes(),
        );
    }
    out.extend_from_slice(b"BT\n/");
    out.extend_from_slice(res);
    out.extend_from_slice(format!(" {size} Tf\n").as_bytes());
    if let Some(tc) = style.character_spacing {
        out.extend_from_slice(format!("{} Tc\n", fmt_num(tc)).as_bytes());
    }
    if let Some(tw) = style.word_spacing {
        out.extend_from_slice(format!("{} Tw\n", fmt_num(tw)).as_bytes());
    }
    if let Some(tz) = style.horizontal_scaling {
        out.extend_from_slice(format!("{} Tz\n", fmt_num(tz)).as_bytes());
    }
    // §9.3.6: one rendering mode at a time. Invisible means painting
    // nothing, so it wins over a stroking colour that asks for the
    // outline to be painted too.
    if style.invisible {
        out.extend_from_slice(b"3 Tr\n");
    } else if style.stroke_color.is_some() {
        out.extend_from_slice(b"2 Tr\n");
    }
    let rise = style.rise(size);
    if let Some(ts) = rise {
        out.extend_from_slice(format!("{} Ts\n", fmt_num(ts)).as_bytes());
    }
    // §9.4.2: Td offsets the line matrix, which cannot rotate. A
    // rotated run therefore sets the text matrix itself, with the
    // rotation's cosine and sine and the same origin Td would have
    // used.
    match style.rotation {
        Some(degrees) => {
            let (sin, cos) = degrees.to_radians().sin_cos();
            out.extend_from_slice(
                format!(
                    "{} {} {} {} {} {} Tm\n(",
                    fmt_num(cos),
                    fmt_num(sin),
                    fmt_num(-sin),
                    fmt_num(cos),
                    fmt_num(x),
                    fmt_num(y)
                )
                .as_bytes(),
            );
        }
        None => out.extend_from_slice(format!("{x} {y} Td\n(").as_bytes()),
    }
    escape_pdf_literal(text, &mut out);
    out.extend_from_slice(b") Tj\nET\n");
    // A line under or through the run is drawn, not declared: the
    // format has no operator for either, so each is a filled rectangle
    // the width of the run. `re` and `f` are path operators and cannot
    // sit inside a text object, so they follow ET — still inside the
    // q / Q pair, which is what gives them the run's own fill colour.
    if (style.underline || style.strike_out) && !style.invisible {
        let mut width = text_width(base_font, text, size);
        if let Some(tz) = style.horizontal_scaling {
            width *= tz / 100.0;
        }
        let baseline = y + rise.unwrap_or(0.0);
        let thickness = size * UNDERLINE_THICKNESS;
        if style.underline {
            out.extend_from_slice(
                decoration_rect(x, baseline + size * UNDERLINE_POSITION, width, thickness)
                    .as_bytes(),
            );
        }
        if style.strike_out {
            out.extend_from_slice(
                decoration_rect(x, baseline + size * STRIKE_OUT_POSITION, width, thickness)
                    .as_bytes(),
            );
        }
    }
    if scoped {
        out.extend_from_slice(b"Q\n");
    }
    out
}

/// One filled rectangle, as the underline / strike-out line.
fn decoration_rect(x: f64, y: f64, width: f64, thickness: f64) -> String {
    use crate::public::shape::fmt_num;
    format!(
        "{} {} {} {} re\nf\n",
        fmt_num(x),
        fmt_num(y),
        fmt_num(width),
        fmt_num(thickness)
    )
}

/// A text colour to emit, or `None` when none was set. A fully
/// transparent colour is the unset state of a
/// [`TextState`](crate::TextState), so it paints in the default rather
/// than being treated as a request for invisible text.
pub(crate) fn fill_color_of(color: &crate::public::color::Color) -> Option<(f64, f64, f64)> {
    (color.a() > 0.0).then(|| color.rgb())
}

/// Append `text` as a PDF literal-string body (WinAnsi / Latin-1
/// bytes), escaping `(`, `)`, and `\`.
fn escape_pdf_literal(text: &str, out: &mut Vec<u8>) {
    for ch in text.chars() {
        let b = if (ch as u32) <= 0xFF { ch as u8 } else { b'?' };
        if matches!(b, b'(' | b')' | b'\\') {
            out.push(b'\\');
        }
        out.push(b);
    }
}

/// The rendered width of `text` (points) in the named base font at
/// `size`, from the Standard-14 metric table (default advance for
/// unknown fonts / glyphs).
pub(crate) fn text_width(base_font: &str, text: &str, size: f64) -> f64 {
    let mut total = 0.0;
    for ch in text.chars() {
        let code = if (ch as u32) <= 0xFF { ch as u8 } else { b'?' };
        let w = standard14_metrics::width(base_font, code).unwrap_or(500) as f64;
        total += w / 1000.0 * size;
    }
    total
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::public::document::Document;
    use crate::public::font_repository::FontRepository;
    use crate::public::position::Position;
    use crate::public::rectangle::Rectangle;
    use crate::public::text_absorber::TextAbsorber;
    use crate::public::text_fragment::TextFragment;

    fn hello_world_pdf() -> std::path::PathBuf {
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/text_extractor/hello_world.pdf")
    }

    #[test]
    fn append_text_renders_and_round_trips() {
        let doc = Document::open(hello_world_pdf()).unwrap();
        {
            let page = doc.pages().get(1).unwrap();
            let builder = TextBuilder::new(&page);
            let mut frag = TextFragment::with_text("Injected");
            frag.text_state_mut().set_font(FontRepository::find_font("Helvetica"));
            frag.text_state_mut().set_font_size(14.0);
            frag.set_position(Position::new(120.0, 400.0));
            builder.append_text(&frag);
        }
        let mut out = Vec::new();
        doc.save_to_writer(&mut out).unwrap();

        // The appended text is extractable from the saved bytes.
        let reloaded = Document::from_bytes(out).unwrap();
        let mut a = TextAbsorber::new();
        a.visit_document(&reloaded).unwrap();
        assert!(a.text().contains("Injected"), "got: {:?}", a.text());
    }

    #[test]
    fn append_paragraph_round_trips() {
        let doc = Document::open(hello_world_pdf()).unwrap();
        {
            let page = doc.pages().get(1).unwrap();
            let builder = TextBuilder::new(&page);
            let mut para = TextParagraph::new();
            para.set_rectangle(Rectangle::new(72.0, 500.0, 300.0, 720.0, true));
            para.set_horizontal_alignment(HorizontalAlignment::Left);
            para.append_line("The quick brown fox");
            para.append_line("second paragraph line");
            builder.append_paragraph(&para);
        }
        let mut out = Vec::new();
        doc.save_to_writer(&mut out).unwrap();
        let reloaded = Document::from_bytes(out).unwrap();
        let mut a = TextAbsorber::new();
        a.visit_document(&reloaded).unwrap();
        assert!(a.text().contains("quick brown fox"), "got: {:?}", a.text());
        assert!(a.text().contains("second paragraph"), "got: {:?}", a.text());
    }
}
