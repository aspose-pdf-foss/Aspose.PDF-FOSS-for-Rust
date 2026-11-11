//! `Aspose.Pdf.Table` — a grid of rows and cells, per
//! `capabilities/table.yaml`.
//!
//! A [`BaseParagraph`](crate::BaseParagraph) holding [`Rows`] plus
//! column widths, borders, and default cell styling. Added to a page
//! via the page's paragraph collection (page wiring + render-on-save
//! are a follow-on).

use crate::public::base_paragraph::{impl_base_paragraph, ParagraphBase};
use crate::public::border_info::BorderInfo;
use crate::public::color::Color;
use crate::public::horizontal_alignment::HorizontalAlignment;
use crate::public::margin_info::MarginInfo;
use crate::public::rows::Rows;
use crate::public::text_state::TextState;
use crate::public::vertical_alignment::VerticalAlignment;

/// A table: rows of cells with shared column / border / cell styling.
#[derive(Debug)]
pub struct Table {
    base: ParagraphBase,
    rows: Rows,
    background_color: Color,
    repeating_columns_count: i32,
    repeating_rows_count: i32,
    is_broken: bool,
    column_widths: String,
    default_cell_border: BorderInfo,
    default_column_width: String,
    border: BorderInfo,
    default_cell_padding: MarginInfo,
    default_cell_text_state: TextState,
    alignment: HorizontalAlignment,
    left: f32,
    top: f32,
    is_borders_included: bool,
}

impl Default for Table {
    fn default() -> Self {
        Self {
            base: ParagraphBase::default(),
            rows: Rows::new(),
            background_color: Color::empty(),
            repeating_columns_count: 0,
            repeating_rows_count: 0,
            is_broken: false,
            column_widths: String::new(),
            default_cell_border: BorderInfo::new(),
            default_column_width: String::new(),
            border: BorderInfo::new(),
            default_cell_padding: MarginInfo::new(),
            default_cell_text_state: TextState::default(),
            alignment: HorizontalAlignment::None,
            left: 0.0,
            top: 0.0,
            is_borders_included: true,
        }
    }
}

impl Table {
    /// Construct an empty table.
    pub fn new() -> Self {
        Self::default()
    }

    /// The table's rows.
    pub fn rows(&self) -> &Rows {
        &self.rows
    }
    /// The table's rows, mutably.
    pub fn rows_mut(&mut self) -> &mut Rows {
        &mut self.rows
    }

    /// The table background colour.
    pub fn background_color(&self) -> Color {
        self.background_color
    }
    /// Set the table background colour.
    pub fn set_background_color(&mut self, value: Color) {
        self.background_color = value;
    }

    /// The number of leading columns repeated on each page.
    /// **Carried, not applied.** A broken table flows whole ROWS onto
    /// continuation pages and repeats
    /// [`repeating_rows_count`](Table::repeating_rows_count) of them;
    /// it never splits by column, so there is no column split for this
    /// count to repeat across.
    pub fn repeating_columns_count(&self) -> i32 {
        self.repeating_columns_count
    }
    /// Set the number of leading columns repeated on each page.
    pub fn set_repeating_columns_count(&mut self, value: i32) {
        self.repeating_columns_count = value;
    }

    /// The number of leading rows repeated on each page.
    pub fn repeating_rows_count(&self) -> i32 {
        self.repeating_rows_count
    }
    /// Set the number of leading rows repeated on each page.
    pub fn set_repeating_rows_count(&mut self, value: i32) {
        self.repeating_rows_count = value;
    }

    /// Whether the table may break across pages.
    pub fn is_broken(&self) -> bool {
        self.is_broken
    }
    /// Set whether the table may break across pages.
    pub fn set_is_broken(&mut self, value: bool) {
        self.is_broken = value;
    }

    /// The column widths, as a space-separated string of points.
    pub fn column_widths(&self) -> &str {
        &self.column_widths
    }
    /// Set the column widths, as a space-separated string of points.
    pub fn set_column_widths(&mut self, value: impl Into<String>) {
        self.column_widths = value.into();
    }

    /// The default border applied to every cell.
    pub fn default_cell_border(&self) -> &BorderInfo {
        &self.default_cell_border
    }
    /// Set the default border applied to every cell.
    pub fn set_default_cell_border(&mut self, value: BorderInfo) {
        self.default_cell_border = value;
    }

    /// The default column width.
    pub fn default_column_width(&self) -> &str {
        &self.default_column_width
    }
    /// Set the default column width.
    pub fn set_default_column_width(&mut self, value: impl Into<String>) {
        self.default_column_width = value.into();
    }

    /// The table's outer border.
    pub fn border(&self) -> &BorderInfo {
        &self.border
    }
    /// Set the table's outer border.
    pub fn set_border(&mut self, value: BorderInfo) {
        self.border = value;
    }

    /// The default padding applied to every cell.
    ///
    /// Carried, not applied, for the reason
    /// [`Row::default_cell_padding`](crate::Row::default_cell_padding)
    /// gives: a cell's own padding is unapplied — the renderer insets a
    /// cell's text by a fixed amount — so a table-wide default has
    /// nothing to override.
    pub fn default_cell_padding(&self) -> MarginInfo {
        self.default_cell_padding
    }
    /// Set the default padding applied to every cell.
    pub fn set_default_cell_padding(&mut self, value: MarginInfo) {
        self.default_cell_padding = value;
    }

    /// The default text state for every cell.
    pub fn default_cell_text_state(&self) -> &TextState {
        &self.default_cell_text_state
    }
    /// Set the default text state for every cell.
    pub fn set_default_cell_text_state(&mut self, value: TextState) {
        self.default_cell_text_state = value;
    }

    /// The table's horizontal alignment on the page.
    pub fn alignment(&self) -> HorizontalAlignment {
        self.alignment
    }
    /// Set the table's horizontal alignment on the page.
    pub fn set_alignment(&mut self, value: HorizontalAlignment) {
        self.alignment = value;
    }

    /// The table's left position, in points.
    pub fn left(&self) -> f32 {
        self.left
    }
    /// Set the table's left position, in points.
    pub fn set_left(&mut self, value: f32) {
        self.left = value;
    }

    /// The table's top position, in points.
    pub fn top(&self) -> f32 {
        self.top
    }
    /// Set the table's top position, in points.
    pub fn set_top(&mut self, value: f32) {
        self.top = value;
    }

    /// Whether cell borders are included in cell width.
    ///
    /// **The default is what already happens, and the other setting is
    /// what is missing.** A column width is taken as the drawn width,
    /// borders and all — which is this flag's default, `true`. What
    /// nothing reads it for is the `false` case, where the border would
    /// be drawn outside the declared width and the table would come out
    /// wider by it; how much wider — one border per column, or per cell
    /// edge — is arithmetic the catalogue does not give.
    pub fn is_borders_included(&self) -> bool {
        self.is_borders_included
    }
    /// Set whether cell borders are included in cell width.
    pub fn set_is_borders_included(&mut self, value: bool) {
        self.is_borders_included = value;
    }
}

/// Default row height (points) when a row sets neither a fixed nor a
/// minimum height: a 12pt line plus padding.
const DEFAULT_ROW_HEIGHT: f64 = 18.0;
/// Left/right text inset within a cell, in points.
const CELL_TEXT_PAD: f64 = 2.0;
/// Bottom margin (points) below which a broken table breaks to a new page.
const BOTTOM_MARGIN: f64 = 36.0;
/// Top margin (points) for a broken table's continuation pages.
const CONTINUATION_TOP_MARGIN: f64 = 36.0;
/// Continuation-page width (points) — US Letter (the host fixture).
const CONTINUATION_PAGE_WIDTH: f64 = 612.0;

impl Table {
    /// The column widths (points) from [`column_widths`](Table::column_widths),
    /// padded with `default_column_width` (or 100pt) up to `columns`.
    fn resolved_column_widths(&self, columns: usize) -> Vec<f64> {
        let mut widths: Vec<f64> = self
            .column_widths
            .split_whitespace()
            .filter_map(|t| t.parse::<f64>().ok())
            .collect();
        let fallback = self.default_column_width.trim().parse::<f64>().unwrap_or(100.0);
        while widths.len() < columns {
            widths.push(fallback);
        }
        widths
    }

    /// The render height of `row` in points.
    fn row_height(&self, row: &crate::public::row::Row) -> f64 {
        if row.fixed_row_height() > 0.0 {
            row.fixed_row_height()
        } else {
            row.min_row_height().max(DEFAULT_ROW_HEIGHT)
        }
    }

    /// The effective border line-width drawn around `cell` — its own
    /// border if set, else the table's default cell border.
    fn cell_border_width(
        &self,
        row: &crate::public::row::Row,
        cell: &crate::public::cell::Cell,
    ) -> f64 {
        // Narrowest setting first: the cell's own, then its row's
        // default, then the table's. A zero width is the unset state at
        // every level, which is what makes the chain expressible.
        for width in [
            cell.border().top().line_width(),
            row.default_cell_border().top().line_width(),
            self.default_cell_border.top().line_width(),
        ] {
            if width > 0.0 {
                return width as f64;
            }
        }
        0.0
    }

    /// The text state a cell of `row` falls back to: its own default
    /// where it set one, then the row's, then the table's. Each
    /// attribute is resolved on its own, by the unset conventions
    /// [`Cell::resolved_text_style`](crate::public::cell::Cell) already
    /// uses — no font, a zero size, a transparent colour, zero spacing.
    fn inherited_cell_text_state(
        &self,
        row: &crate::public::row::Row,
        cell: &crate::public::cell::Cell,
    ) -> TextState {
        let mut state = cell.default_cell_text_state().clone();
        for wider in [row.default_cell_text_state(), &self.default_cell_text_state] {
            if state.font().is_none() {
                if let Some(font) = wider.font() {
                    state.set_font(font.clone());
                }
            }
            if state.font_size() <= 0.0 {
                state.set_font_size(wider.font_size());
            }
            if state.foreground_color().a() <= 0.0 {
                state.set_foreground_color(wider.foreground_color());
            }
            if state.character_spacing() == 0.0 {
                state.set_character_spacing(wider.character_spacing());
            }
            if state.word_spacing() == 0.0 {
                state.set_word_spacing(wider.word_spacing());
            }
        }
        state
    }

    /// Assign the table's rows to pages. A non-broken table is one page
    /// (all rows, overflowing off the bottom if too tall). A broken
    /// table flows rows that don't fit onto continuation pages, each
    /// repeating the first [`repeating_rows_count`](Table::repeating_rows_count)
    /// rows as a header. Returns one row-index list per page (`[0]` is
    /// the host page).
    fn paginate(&self, row_count: usize, host_top: f64, page_height: f64) -> Vec<Vec<usize>> {
        if !self.is_broken {
            return vec![(0..row_count).collect()];
        }
        let repeating = (self.repeating_rows_count.max(0) as usize).min(row_count);
        let header: Vec<usize> = (0..repeating).collect();
        let cont_top = page_height - CONTINUATION_TOP_MARGIN;

        let mut pages: Vec<Vec<usize>> = Vec::new();
        let mut cur: Vec<usize> = Vec::new();
        let mut y = host_top;
        let mut placed_since_break = 0usize;
        let mut i = 0;
        while i < row_count {
            let rh = self.rows.get(i).map(|r| self.row_height(r)).unwrap_or(DEFAULT_ROW_HEIGHT);
            if y - rh < BOTTOM_MARGIN && placed_since_break > 0 {
                pages.push(std::mem::take(&mut cur));
                // Continuation page: repeat the header rows, then retry row i.
                y = cont_top;
                for &h in &header {
                    cur.push(h);
                    y -= self.rows.get(h).map(|r| self.row_height(r)).unwrap_or(DEFAULT_ROW_HEIGHT);
                }
                placed_since_break = 0;
                continue;
            }
            cur.push(i);
            y -= rh;
            placed_since_break += 1;
            i += 1;
        }
        if !cur.is_empty() {
            pages.push(cur);
        }
        pages
    }

    /// Render the given `row_indices` starting at `top_y`, returning the
    /// content-stream bytes, the fonts referenced, and the total height
    /// consumed.
    fn render_rows(
        &self,
        row_indices: &[usize],
        widths: &[f64],
        top_y: f64,
        page_height: f64,
    ) -> (Vec<u8>, Vec<crate::public::page_edits::EditFont>, f64) {
        use crate::public::shape::fmt_num;
        let left = self.left as f64;
        let mut content: Vec<u8> = Vec::new();
        let mut fonts: Vec<crate::public::page_edits::EditFont> = Vec::new();
        let mut total_height = 0.0;
        let mut y = top_y;
        for &r in row_indices {
            let row = match self.rows.get(r) {
                Some(row) => row,
                None => continue,
            };
            let rh = self.row_height(row);
            let y_bottom = y - rh;
            total_height += rh;

            // A row's own background fills the whole row, behind every
            // cell — so it is drawn before them, and a cell's own fill
            // paints over it.
            let row_fill = row.background_color();
            if !row_fill.is_empty() {
                let table_width: f64 = widths.iter().sum();
                fill_rect(&mut content, left, y_bottom, left + table_width, y, row_fill);
            }
            let mut x = left;
            // Column cursor, advanced by each cell's span — a cell with
            // ColSpan > 1 covers that many column widths, and the cell
            // after it starts past all of them.
            let mut column = 0usize;
            for c in 0..row.cells().count() {
                let cell = match row.cells().get(c) {
                    Some(cell) => cell,
                    None => continue,
                };
                let span = (cell.col_span().max(1) as usize).min(widths.len().saturating_sub(column).max(1));
                let cw: f64 = widths
                    .iter()
                    .skip(column)
                    .take(span)
                    .sum::<f64>();
                column += span;

                let fill = cell.background_color();
                if !fill.is_empty() {
                    fill_rect(&mut content, x, y_bottom, x + cw, y, fill);
                }
                // `Cell::is_no_border` suppresses the cell's own frame
                // whatever width the table would otherwise draw.
                let bw =
                    if cell.is_no_border() { 0.0 } else { self.cell_border_width(row, cell) };
                if bw > 0.0 {
                    stroke_rect(&mut content, x, y_bottom, x + cw, y, bw);
                }
                // The size the content will actually render at, which
                // the cell may be the one supplying — placement and
                // painting have to agree on it.
                let inherited = self.inherited_cell_text_state(row, cell);
                let font_size = cell_font_size(cell, &inherited);
                let content_width = cell_content_width(cell, &inherited);
                let text_x = match (cell.alignment(), content_width) {
                    (HorizontalAlignment::Center, Some(w)) => x + (cw - w) / 2.0,
                    (HorizontalAlignment::Right, Some(w)) => x + cw - w - CELL_TEXT_PAD,
                    // Left, None, and anything unmeasurable start at the
                    // cell's left padding.
                    _ => x + CELL_TEXT_PAD,
                };
                // The content occupies one line of `font_size`; place it
                // in the row per the cell's vertical alignment. Top (and
                // the unset default) hangs it from the row's top edge.
                let baseline = match cell.vertical_alignment() {
                    VerticalAlignment::Center => y - (rh - font_size) / 2.0 - font_size,
                    VerticalAlignment::Bottom => y_bottom + CELL_TEXT_PAD,
                    _ => y - font_size - CELL_TEXT_PAD,
                };
                for rp in render_cell_paragraphs(cell, &inherited, page_height) {
                    content.extend_from_slice(
                        format!("q\n1 0 0 1 {} {} cm\n", fmt_num(text_x), fmt_num(baseline))
                            .as_bytes(),
                    );
                    content.extend_from_slice(&rp.content);
                    content.extend_from_slice(b"Q\n");
                    for f in rp.fonts {
                        match fonts.iter_mut().find(|e| e.resource_name == f.resource_name) {
                            Some(existing) => {
                                existing.used_codes.extend_from_slice(&f.used_codes);
                                existing.is_subset |= f.is_subset;
                            }
                            None => fonts.push(f),
                        }
                    }
                }
                x += cw;
            }
            y = y_bottom;
        }
        (content, fonts, total_height)
    }
}

impl crate::public::base_paragraph::sealed::Renderable for Table {
    /// Render the table's grid (cell backgrounds, cell + outer borders
    /// as stroked line segments) and each cell's text into the page
    /// content stream. The table's top-left is placed at
    /// `(left, page_height - top)`; rows flow downward, columns
    /// left-to-right per the resolved column widths. A cell's `ColSpan`
    /// widens it across that many columns, and the cell after it starts
    /// past all of them. Cell text is each cell paragraph rendered at
    /// its local origin then translated into the cell via a `cm`
    /// matrix, placed by the cell's `Alignment` when its content can be
    /// measured and by its `VerticalAlignment` within the row. `RowSpan`
    /// is not applied — every cell occupies the row it was added to.
    fn render_content(
        &self,
        page_height: f64,
    ) -> Option<crate::public::base_paragraph::sealed::RenderedParagraph> {
        use crate::public::base_paragraph::sealed::{RenderedPage, RenderedParagraph};

        let row_count = self.rows.count();
        if row_count == 0 {
            return None;
        }
        // A row is as wide as its cells' spans add up to, not as its
        // cell count — a row of two cells one of which spans three
        // columns is four columns wide.
        let columns = (0..row_count)
            .filter_map(|r| self.rows.get(r))
            .map(|row| {
                (0..row.cells().count())
                    .filter_map(|c| row.cells().get(c))
                    .map(|cell| cell.col_span().max(1) as usize)
                    .sum::<usize>()
            })
            .max()
            .unwrap_or(0);
        if columns == 0 {
            return None;
        }
        let widths = self.resolved_column_widths(columns);
        let table_width: f64 = widths.iter().take(columns).sum();

        let left = self.left as f64;
        let host_top = page_height - self.top as f64;

        let pages = self.paginate(row_count, host_top, page_height);

        // Host page (the first row-set), rendered at the table's top.
        let (mut content, fonts, total_height) =
            self.render_rows(&pages[0], &widths, host_top, page_height);

        // Outer table border — only meaningful for a single-page table.
        if pages.len() == 1 {
            let outer = self.border.top().line_width() as f64;
            if outer > 0.0 {
                stroke_rect(
                    &mut content,
                    left,
                    host_top - total_height,
                    left + table_width,
                    host_top,
                    outer,
                );
            }
        }

        // Continuation pages (a broken table that overflowed): each is a
        // whole new page with the overflow rows (+ repeated header).
        let cont_top = page_height - CONTINUATION_TOP_MARGIN;
        let media_box = [0.0, 0.0, CONTINUATION_PAGE_WIDTH, page_height];
        let continuations: Vec<RenderedPage> = pages[1..]
            .iter()
            .map(|rows| {
                let (c, f, _) = self.render_rows(rows, &widths, cont_top, page_height);
                RenderedPage { content: c, fonts: f, media_box }
            })
            .collect();

        if content.is_empty() && continuations.is_empty() {
            return None;
        }
        Some(RenderedParagraph { content, fonts, continuations })
    }
}

/// Render a cell's paragraphs, with the cell's `DefaultCellTextState`
/// supplying the font and size its text fragments leave unset. Anything
/// that is not a text fragment renders as it stands.
fn render_cell_paragraphs(
    cell: &crate::public::cell::Cell,
    inherited: &TextState,
    page_height: f64,
) -> Vec<crate::public::base_paragraph::sealed::RenderedParagraph> {
    use crate::public::base_paragraph::sealed::RenderedParagraph;
    let mut out = Vec::new();
    for i in 0..cell.paragraphs().count() {
        let Some(p) = cell.paragraphs().get(i) else { continue };
        match p.as_text_fragment() {
            Some(fragment) => {
                let (font, size, state) =
                    cell.resolved_text_style(fragment, inherited);
                if let Some((content, font_res)) =
                    crate::public::text_builder::render_text_fragment_styled(
                        fragment,
                        font.as_ref(),
                        size,
                        crate::public::text_builder::RunStyle::of(&state),
                    )
                {
                    out.push(RenderedParagraph {
                        content,
                        fonts: vec![font_res],
                        continuations: Vec::new(),
                    });
                }
            }
            None => out.extend(p.render_content(page_height)),
        }
    }
    out
}

/// The size a cell's content renders at: the resolved size of its first
/// text fragment, else the cell's own default, else the render path's
/// default.
fn cell_font_size(cell: &crate::public::cell::Cell, inherited: &TextState) -> f64 {
    for i in 0..cell.paragraphs().count() {
        let Some(p) = cell.paragraphs().get(i) else { continue };
        if let Some(fragment) = p.as_text_fragment() {
            let (_, size, _) = cell.resolved_text_style(fragment, inherited);
            if size > 0.0 {
                return size as f64;
            }
        }
    }
    let own = inherited.font_size() as f64;
    if own > 0.0 {
        own
    } else {
        12.0
    }
}

/// The width of a cell's content, when it can be measured — the sum of
/// its paragraphs' rendered widths. `None` when nothing in the cell
/// reports a width, which leaves the caller to place it at the left.
fn cell_content_width(
    cell: &crate::public::cell::Cell,
    inherited: &TextState,
) -> Option<f64> {
    let mut total = 0.0;
    let mut measured = false;
    for i in 0..cell.paragraphs().count() {
        let Some(p) = cell.paragraphs().get(i) else { continue };
        let width = match p.as_text_fragment() {
            // Measure the text at the size it will actually render in,
            // which the cell may be the one supplying.
            Some(fragment) => {
                let (font, size, _) = cell.resolved_text_style(fragment, inherited);
                crate::public::text_builder::fragment_width(fragment, font.as_ref(), size)
            }
            None => p.rendered_width(),
        };
        if let Some(w) = width {
            total += w;
            measured = true;
        }
    }
    measured.then_some(total)
}

/// Append a filled rectangle in `color` to `out`.
fn fill_rect(
    out: &mut Vec<u8>,
    llx: f64,
    lly: f64,
    urx: f64,
    ury: f64,
    color: crate::public::color::Color,
) {
    use crate::public::shape::fmt_num;
    let (r, g, b) = color.rgb();
    out.extend_from_slice(b"q\n");
    out.extend_from_slice(
        format!("{} {} {} rg\n", fmt_num(r), fmt_num(g), fmt_num(b)).as_bytes(),
    );
    out.extend_from_slice(
        format!(
            "{} {} {} {} re\nf\n",
            fmt_num(llx),
            fmt_num(lly),
            fmt_num(urx - llx),
            fmt_num(ury - lly)
        )
        .as_bytes(),
    );
    out.extend_from_slice(b"Q\n");
}

/// Append a stroked rectangle (four `m`/`l` line segments) of the given
/// line width to `out`.
fn stroke_rect(out: &mut Vec<u8>, llx: f64, lly: f64, urx: f64, ury: f64, width: f64) {
    use crate::public::shape::fmt_num;
    out.extend_from_slice(format!("q\n{} w\n", fmt_num(width)).as_bytes());
    out.extend_from_slice(format!("{} {} m\n", fmt_num(llx), fmt_num(ury)).as_bytes());
    out.extend_from_slice(format!("{} {} l\n", fmt_num(urx), fmt_num(ury)).as_bytes());
    out.extend_from_slice(format!("{} {} l\n", fmt_num(urx), fmt_num(lly)).as_bytes());
    out.extend_from_slice(format!("{} {} l\n", fmt_num(llx), fmt_num(lly)).as_bytes());
    out.extend_from_slice(b"h\nS\nQ\n");
}

impl_base_paragraph!(Table);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::public::base_paragraph::BaseParagraph;
    use crate::public::border_side::BorderSide;

    #[test]
    fn build_grid_and_properties() {
        let mut table = Table::new();
        table.set_column_widths("100 100 100");
        table.set_border(BorderInfo::with_side_width(BorderSide::ALL, 1.0));
        table.set_default_cell_border(BorderInfo::with_side(BorderSide::ALL));

        for _ in 0..2 {
            let row = table.rows_mut().add();
            row.cells_mut().add_text("a");
            row.cells_mut().add_text("b");
            row.cells_mut().add_text("c");
        }

        assert_eq!(table.column_widths(), "100 100 100");
        assert_eq!(table.rows().count(), 2);
        assert_eq!(table.rows().get(1).unwrap().cells().count(), 3);
        assert_eq!(table.border().top().line_width(), 1.0);
        assert!(table.is_borders_included());

        // Table is a BaseParagraph.
        table.set_is_in_new_page(true);
        assert!(table.is_in_new_page());
        table.set_left(72.0);
        table.set_top(700.0);
        assert_eq!(table.left(), 72.0);
    }
}
