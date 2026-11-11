// Public-API smoke driver — Table / Row / Rows / Cell / Cells.
//
// Ports the cpp tables-cluster smoke tests (table, cells, rows,
// cell_image, table_break, table_render). A Table added to a page's
// Paragraphs renders its grid (cell + outer borders as stroked line
// segments) and cell text into the page content stream on save, and a
// broken table flows its overflow rows onto new pages (asserted below).
//
// A cell's background IMAGE stays in memory, and the reason is
// structural rather than unfinished work: a paragraph's render declares
// content bytes and font resources without knowing its page, so it
// cannot add an image XObject — see the note on `RenderedParagraph`.

use aspose_pdf_foss::{
    BaseParagraph, BorderInfo, BorderSide, Cell, Cells, Color, Document, HorizontalAlignment,
    Rectangle, Row, Rows, Table, TextState,
};

fn fixture(name: &str) -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/text_extractor")
        .join(name)
}

fn contains(haystack: &[u8], needle: &[u8]) -> bool {
    haystack.windows(needle.len()).any(|w| w == needle)
}

// ---- table_smoke_test.cpp ----------------------------------------

#[test]
fn table_build_grid_and_properties() {
    let mut table = Table::new();
    table.set_column_widths("100 100 100");
    table.set_border(BorderInfo::with_side_width(BorderSide::ALL, 1.0));
    table.set_default_cell_border(BorderInfo::with_side(BorderSide::ALL));

    // Build a 2x3 grid.
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

    // Table is a BaseParagraph.
    table.set_is_in_new_page(true);
    assert!(table.is_in_new_page());

    table.set_left(72.0);
    table.set_top(700.0);
    assert_eq!(table.left(), 72.0);
    assert!(table.is_borders_included());
}

// ---- cells_smoke_test.cpp ----------------------------------------

#[test]
fn cell_defaults() {
    let c = Cell::new();
    assert!(!c.is_no_border());
    assert_eq!(c.col_span(), 1);
    assert_eq!(c.row_span(), 1);
    assert!(c.is_word_wrapped());
    assert_eq!(c.alignment(), HorizontalAlignment::None);

    let sized = Cell::with_rectangle(Rectangle::new(0.0, 0.0, 120.0, 40.0, false));
    assert_eq!(sized.width(), 120.0);
}

#[test]
fn cell_properties() {
    let mut c = Cell::new();
    c.set_border(BorderInfo::with_side_width(BorderSide::ALL, 1.5));
    c.set_background_color(Color::from_rgb(0.9, 0.9, 0.9));
    c.set_alignment(HorizontalAlignment::Center);
    c.set_col_span(2);
    c.set_row_span(3);
    c.set_is_no_border(true);

    assert_eq!(c.border().top().line_width(), 1.5);
    assert_eq!(c.background_color().a(), 1.0);
    assert_eq!(c.alignment(), HorizontalAlignment::Center);
    assert_eq!(c.col_span(), 2);
    assert_eq!(c.row_span(), 3);
    assert!(c.is_no_border());
}

#[test]
fn cells_add_overloads_and_indexer() {
    let mut cells = Cells::new();
    assert_eq!(cells.count(), 0);

    cells.add();
    cells.add_text("hello");
    let styled = cells.add_text("world");
    styled.set_alignment(HorizontalAlignment::Right);

    assert_eq!(cells.count(), 3);
    // 0-based indexer returns the same stored cells (the third added).
    assert_eq!(cells.get(2).unwrap().alignment(), HorizontalAlignment::Right);

    cells.remove_range(0, 1); // drop the first empty cell
    assert_eq!(cells.count(), 2);
}

// ---- rows_smoke_test.cpp -----------------------------------------

#[test]
fn row_cells_and_properties() {
    let mut row = Row::new();
    row.cells_mut().add_text("a");
    row.cells_mut().add_text("b");
    assert_eq!(row.cells().count(), 2);

    row.set_background_color(Color::from_rgb(0.8, 0.8, 0.8));
    row.set_min_row_height(20.0);
    row.set_fixed_row_height(25.0);
    row.set_border(BorderInfo::with_side(BorderSide::ALL));
    assert_eq!(row.min_row_height(), 20.0);
    assert_eq!(row.fixed_row_height(), 25.0);
    assert_eq!(row.border().left().line_width(), 1.0);
}

#[test]
fn rows_add_indexer_remove() {
    let mut rows = Rows::new();
    assert_eq!(rows.count(), 0);

    rows.add().cells_mut().add_text("r1c1");
    rows.add().cells_mut().add_text("r2c1");
    assert_eq!(rows.count(), 2);

    assert_eq!(rows.get(1).unwrap().cells().count(), 1);

    rows.remove_at(0);
    assert_eq!(rows.count(), 1);
    // The surviving row is the former second row.
    assert_eq!(rows.get(0).unwrap().cells().count(), 1);
}

// ---- cell_image_smoke_test.cpp -----------------------------------

// The cell background-image source round-trips on the in-memory cell.
// (Embedding it as a page /XObject at Save is render-on-save, which
// the Rust builder does not implement.)
#[test]
fn cell_background_image_file_property() {
    let mut cell = Cell::new();
    assert!(cell.background_image_file().is_empty());
    cell.set_background_image_file("gradient_32.jpg");
    assert_eq!(cell.background_image_file(), "gradient_32.jpg");
}

// ---- table_break_smoke_test.cpp ----------------------------------

#[test]
fn table_is_broken_property() {
    let mut table = Table::new();
    assert!(!table.is_broken());
    table.set_is_broken(true);
    assert!(table.is_broken());
}

#[test]
fn table_break_repeating_rows_count() {
    let mut table = Table::new();
    table.set_is_broken(true);
    table.set_repeating_rows_count(1);
    table.set_column_widths("200");
    table.set_default_cell_border(BorderInfo::with_side_width(BorderSide::ALL, 1.0));

    for i in 1..=70 {
        let row = table.rows_mut().add();
        row.cells_mut().add_text(format!("Row{i}"));
    }

    assert!(table.is_broken());
    assert_eq!(table.repeating_rows_count(), 1);
    assert_eq!(table.rows().count(), 70);
    assert_eq!(table.rows().get(69).unwrap().cells().count(), 1);
}

// ---- table_render_smoke_test.cpp ---------------------------------

// A Table added to a page draws its grid (borders) and cell text into
// the page content stream at Save.
#[test]
fn table_renders_grid_and_text_on_save() {
    let doc = Document::open(fixture("two_pages.pdf")).unwrap();
    {
        let mut table = Table::new();
        table.set_left(72.0);
        table.set_top(700.0);
        table.set_column_widths("80 80 80");
        table.set_default_cell_border(BorderInfo::with_side_width(BorderSide::ALL, 1.0));
        table.set_border(BorderInfo::with_side_width(BorderSide::ALL, 1.5));

        let labels = [["a1", "b1", "c1"], ["a2", "b2", "c2"]];
        for row_labels in labels {
            let row = table.rows_mut().add();
            for label in row_labels {
                row.cells_mut().add_text(label);
            }
        }
        assert_eq!(table.rows().count(), 2);

        let page = doc.pages().get(1).unwrap();
        page.paragraphs().add(Box::new(table));
        assert_eq!(page.paragraphs().count(), 1);
    }

    let mut out = Vec::new();
    doc.save_to_writer(&mut out).unwrap();

    // Structure intact on reopen.
    let reloaded = Document::from_bytes(out.clone()).unwrap();
    assert_eq!(reloaded.pages().count(), 2);

    // The content stream carries the table: a font resource, cell-text
    // show operators, and stroked grid lines.
    assert!(contains(&out, b"ASPHelvetica"), "font resource missing");
    assert!(contains(&out, b"(a1)"), "cell text a1 missing");
    assert!(contains(&out, b"(c2)"), "cell text c2 missing");
    assert!(contains(&out, b" l\n"), "grid line-to operator missing");
    assert!(contains(&out, b" Tj"), "text-show operator missing");
}

// A broken table taller than the page flows its overflow rows onto a
// newly-created continuation page; the late rows land there.
#[test]
fn broken_table_flows_to_new_page() {
    use aspose_pdf_foss::TextFragmentAbsorber;
    let doc = Document::open(fixture("hello_world.pdf")).unwrap();
    assert_eq!(doc.pages().count(), 1);
    {
        let mut table = Table::new();
        table.set_is_broken(true);
        table.set_repeating_rows_count(1);
        table.set_column_widths("200");
        table.set_default_cell_border(BorderInfo::with_side_width(BorderSide::ALL, 1.0));
        for i in 1..=70 {
            let row = table.rows_mut().add();
            row.cells_mut().add_text(format!("Row{i}"));
        }
        doc.pages().get(1).unwrap().paragraphs().add(Box::new(table));
    }
    let mut out = Vec::new();
    doc.save_to_writer(&mut out).unwrap();

    let reloaded = Document::from_bytes(out).unwrap();
    // A continuation page was created.
    assert!(reloaded.pages().count() >= 2, "pages: {}", reloaded.pages().count());
    // The late row flowed onto the continuation page and is extractable.
    let mut a = TextFragmentAbsorber::with_phrase("Row70");
    a.visit_document(&reloaded).unwrap();
    assert!(a.text_fragments().count() >= 1, "Row70 not found");
}

// Without IsBroken the table stays on its page (no continuation page).
#[test]
fn unbroken_table_stays_single_page() {
    let doc = Document::open(fixture("hello_world.pdf")).unwrap();
    assert_eq!(doc.pages().count(), 1);
    {
        let mut table = Table::new(); // is_broken defaults to false
        table.set_column_widths("200");
        table.set_default_cell_border(BorderInfo::with_side_width(BorderSide::ALL, 1.0));
        for i in 1..=70 {
            let row = table.rows_mut().add();
            row.cells_mut().add_text(format!("Row{i}"));
        }
        doc.pages().get(1).unwrap().paragraphs().add(Box::new(table));
    }
    let mut out = Vec::new();
    doc.save_to_writer(&mut out).unwrap();
    let reloaded = Document::from_bytes(out).unwrap();
    assert_eq!(reloaded.pages().count(), 1);
}

// ---- cell styling reaches the page ---------------------------------
//
// ColSpan, BackgroundColor and Alignment were stored on the cell and
// dropped at render: a spanned cell was drawn one column wide, a
// coloured cell painted nothing, and every cell's text sat at the left
// edge. These pin the operators, with the numbers worked out rather
// than eyeballed.
//
// Courier is the measuring stick — every glyph is exactly 600/1000 em,
// so a string's width at a given size is arithmetic.

fn courier_cell(text: &str, size: f32) -> Cell {
    let mut fragment = aspose_pdf_foss::TextFragment::with_text(text);
    fragment.text_state_mut().set_font_size(size);
    fragment
        .text_state_mut()
        .set_font(aspose_pdf_foss::FontRepository::find_font("Courier"));
    let mut cell = Cell::new();
    cell.paragraphs_mut().add(Box::new(fragment));
    cell
}

/// One-row table at a known origin, saved, as content-stream text.
fn render_single_row(cells: Vec<Cell>) -> String {
    let doc = Document::open(fixture("hello_world.pdf")).unwrap();
    {
        let mut table = Table::new();
        table.set_left(100.0);
        table.set_top(92.0); // page is 792 high → top edge at y = 700
        table.set_column_widths("100 100");
        table.set_default_cell_border(BorderInfo::with_side_width(BorderSide::ALL, 1.0));
        let row = table.rows_mut().add();
        for cell in cells {
            row.cells_mut().add_cell(cell);
        }
        doc.pages().get(1).unwrap().paragraphs().add(Box::new(table));
    }
    let mut out = Vec::new();
    doc.save_to_writer(&mut out).unwrap();
    String::from_utf8_lossy(&out).into_owned()
}

#[test]
fn a_cell_background_is_painted() {
    let mut cell = courier_cell("x", 10.0);
    cell.set_background_color(Color::from_argb_rgb(0, 128, 255));
    let content = render_single_row(vec![cell]);

    // Row height for a 10pt cell, filling the first 100-wide column at
    // the table's origin.
    assert!(content.contains(" rg\n"), "no fill colour was set");
    assert!(
        content.contains("100 "),
        "the fill rectangle should start at the table's left edge"
    );
    assert!(content.contains(" re\nf\n"), "no filled rectangle");
    // The blue channel is 255/255 = 1, the green 128/255.
    assert!(content.contains("0 0.502 1 rg"), "wrong fill colour: {content:?}");
}

#[test]
fn a_right_aligned_cell_ends_at_its_right_edge() {
    // "12.34" is five Courier glyphs: 5 × 0.6 × 10 = 30 points.
    let mut cell = courier_cell("12.34", 10.0);
    cell.set_alignment(HorizontalAlignment::Right);
    let content = render_single_row(vec![cell]);

    // Column spans x = 100..200; 2 points of padding inside the right
    // edge leaves the run starting at 200 − 30 − 2 = 168.
    assert!(
        content.contains("1 0 0 1 168 "),
        "right-aligned text is not at the right edge: {content:?}"
    );
}

#[test]
fn a_centred_cell_sits_in_the_middle_of_its_column() {
    let mut cell = courier_cell("12.34", 10.0);
    cell.set_alignment(HorizontalAlignment::Center);
    let content = render_single_row(vec![cell]);

    // (100 − 30) / 2 = 35 points of slack on each side → x = 135.
    assert!(
        content.contains("1 0 0 1 135 "),
        "centred text is not centred: {content:?}"
    );
}

#[test]
fn a_left_aligned_cell_keeps_its_padding() {
    let cell = courier_cell("12.34", 10.0);
    let content = render_single_row(vec![cell]);
    assert!(
        content.contains("1 0 0 1 102 "),
        "unaligned text should start at the left padding: {content:?}"
    );
}

#[test]
fn a_spanned_cell_covers_every_column_it_spans() {
    let mut spanning = courier_cell("wide", 10.0);
    spanning.set_col_span(2);
    let content = render_single_row(vec![spanning]);

    // Two 100-wide columns: the cell's border runs from x = 100 to 300,
    // not to 200.
    assert!(
        content.contains("300 "),
        "the spanned cell was drawn one column wide: {content:?}"
    );
}

#[test]
fn a_cell_after_a_span_starts_past_it() {
    let mut spanning = courier_cell("wide", 10.0);
    spanning.set_col_span(2);
    let mut table = Table::new();
    table.set_column_widths("100 100 100");
    table.set_left(100.0);
    table.set_top(92.0);
    table.set_default_cell_border(BorderInfo::with_side_width(BorderSide::ALL, 1.0));
    {
        let row = table.rows_mut().add();
        row.cells_mut().add_cell(spanning);
        let mut tail = courier_cell("tail", 10.0);
        tail.set_alignment(HorizontalAlignment::Right);
        row.cells_mut().add_cell(tail);
    }
    let doc = Document::open(fixture("hello_world.pdf")).unwrap();
    doc.pages().get(1).unwrap().paragraphs().add(Box::new(table));
    let mut out = Vec::new();
    doc.save_to_writer(&mut out).unwrap();
    let content = String::from_utf8_lossy(&out).into_owned();

    // The trailing cell occupies the third column, x = 300..400, so its
    // right-aligned text (4 × 6 = 24 wide) starts at 400 − 24 − 2 = 374.
    assert!(
        content.contains("1 0 0 1 374 "),
        "the cell after a span did not start past it: {content:?}"
    );
}

// A cell's DefaultCellTextState styles the text it holds. It was stored
// on the cell and never consulted, so `add_text_with_state` chose a font
// and size that the render ignored — the text came out in the default
// face at the default size whatever the caller asked for.
#[test]
fn a_cell_text_state_styles_its_text() {
    let mut state = aspose_pdf_foss::TextState::default();
    state.set_font_size(18.0);
    state.set_font(aspose_pdf_foss::FontRepository::find_font("Courier"));

    let doc = Document::open(fixture("hello_world.pdf")).unwrap();
    {
        let mut table = Table::new();
        table.set_left(100.0);
        table.set_top(92.0);
        table.set_column_widths("200");
        let row = table.rows_mut().add();
        row.cells_mut().add_text_with_state("styled", state);
        doc.pages().get(1).unwrap().paragraphs().add(Box::new(table));
    }
    let mut out = Vec::new();
    doc.save_to_writer(&mut out).unwrap();
    let content = String::from_utf8_lossy(&out).into_owned();

    assert!(content.contains("/ASPCourier 18 Tf"), "the cell's state was ignored: {content:?}");
}

// The measurement that drives alignment uses the same resolved style, so
// a right-aligned cell whose size comes from the cell still lands at its
// right edge.
#[test]
fn alignment_measures_the_text_at_the_size_it_renders() {
    let mut state = aspose_pdf_foss::TextState::default();
    state.set_font_size(20.0);
    state.set_font(aspose_pdf_foss::FontRepository::find_font("Courier"));

    let doc = Document::open(fixture("hello_world.pdf")).unwrap();
    {
        let mut table = Table::new();
        table.set_left(100.0);
        table.set_top(92.0);
        table.set_column_widths("200");
        let row = table.rows_mut().add();
        let cell = row.cells_mut().add_text_with_state("abc", state);
        cell.set_alignment(HorizontalAlignment::Right);
        doc.pages().get(1).unwrap().paragraphs().add(Box::new(table));
    }
    let mut out = Vec::new();
    doc.save_to_writer(&mut out).unwrap();
    let content = String::from_utf8_lossy(&out).into_owned();

    // Three Courier glyphs at 20pt are 3 × 0.6 × 20 = 36 wide; the
    // column runs 100..300, so the run starts at 300 − 36 − 2 = 262.
    // Measured at the default 12pt it would have landed at 276.
    assert!(
        content.contains("1 0 0 1 262 "),
        "alignment measured the wrong size: {content:?}"
    );
}

// A cell that defers to its fragments leaves them exactly as styled.
#[test]
fn a_cell_can_defer_to_its_fragment() {
    let mut state = aspose_pdf_foss::TextState::default();
    state.set_font_size(18.0);
    state.set_font(aspose_pdf_foss::FontRepository::find_font("Courier"));

    let doc = Document::open(fixture("hello_world.pdf")).unwrap();
    {
        let mut table = Table::new();
        table.set_left(100.0);
        table.set_top(92.0);
        table.set_column_widths("200");
        let row = table.rows_mut().add();
        let cell = row.cells_mut().add_text_with_state("styled", state);
        cell.set_is_override_by_fragment(true);
        doc.pages().get(1).unwrap().paragraphs().add(Box::new(table));
    }
    let mut out = Vec::new();
    doc.save_to_writer(&mut out).unwrap();
    let content = String::from_utf8_lossy(&out).into_owned();

    assert!(
        content.contains("/ASPHelvetica 12 Tf"),
        "the fragment's own (unset) state should have won: {content:?}"
    );
}

// A cell's DefaultCellTextState carries a colour as well as a font, and
// the render now paints in it.
#[test]
fn a_cell_text_state_colours_its_text() {
    let mut state = aspose_pdf_foss::TextState::default();
    state.set_font_size(12.0);
    state.set_foreground_color(Color::from_argb_rgb(0, 102, 204));

    let doc = Document::open(fixture("hello_world.pdf")).unwrap();
    {
        let mut table = Table::new();
        table.set_left(100.0);
        table.set_top(92.0);
        table.set_column_widths("200");
        let row = table.rows_mut().add();
        row.cells_mut().add_text_with_state("blue", state);
        doc.pages().get(1).unwrap().paragraphs().add(Box::new(table));
    }
    let mut out = Vec::new();
    doc.save_to_writer(&mut out).unwrap();
    let content = String::from_utf8_lossy(&out).into_owned();

    // 102/255 = 0.4, 204/255 = 0.8.
    assert!(content.contains("0 0.4 0.8 rg"), "the cell's colour was ignored: {content:?}");
}

// A cell's VerticalAlignment was stored and ignored: every cell's text
// hung from the top of its row. The default row is 18 points high and
// the test cell's text is 10 points, so the three placements are
// arithmetic — the row runs from y=700 down to y=682.
#[test]
fn a_cell_places_its_text_by_its_vertical_alignment() {
    fn baseline_for(alignment: aspose_pdf_foss::VerticalAlignment) -> String {
        let mut cell = courier_cell("x", 10.0);
        cell.set_vertical_alignment(alignment);
        let content = render_single_row(vec![cell]);
        let at = content.rfind("1 0 0 1 ").expect("a placed run");
        content[at..].lines().next().expect("the cm line").to_string()
    }

    // Top: hung from the row's top edge, less the 2-point padding.
    assert!(
        baseline_for(aspose_pdf_foss::VerticalAlignment::Top).contains(" 688 cm"),
        "top: {}",
        baseline_for(aspose_pdf_foss::VerticalAlignment::Top)
    );
    // Centre: (18 - 10) / 2 = 4 points of slack above → 700 - 4 - 10.
    assert!(
        baseline_for(aspose_pdf_foss::VerticalAlignment::Center).contains(" 686 cm"),
        "centre: {}",
        baseline_for(aspose_pdf_foss::VerticalAlignment::Center)
    );
    // Bottom: resting on the row's bottom edge plus the padding.
    assert!(
        baseline_for(aspose_pdf_foss::VerticalAlignment::Bottom).contains(" 684 cm"),
        "bottom: {}",
        baseline_for(aspose_pdf_foss::VerticalAlignment::Bottom)
    );
}

/// `Cell::is_no_border` suppresses that cell's frame, whatever width
/// the table would otherwise draw around it. Counted in stroked
/// rectangles rather than in bytes: a table of two cells strokes two
/// frames, and marking one of them borderless leaves a single frame
/// plus the table's own outline.
#[test]
fn a_borderless_cell_draws_no_frame() {
    fn stroked_frames(no_border: bool) -> usize {
        let doc = Document::open(fixture("hello_world.pdf")).unwrap();
        {
            let mut table = Table::new();
            table.set_column_widths("100 100");
            table.set_default_cell_border(BorderInfo::with_side_width(BorderSide::ALL, 1.0));
            let row = table.rows_mut().add();
            row.cells_mut().add_text("left");
            row.cells_mut().add_text("right");
            if no_border {
                row.cells_mut().get_mut(1).unwrap().set_is_no_border(true);
            }
            doc.pages().get(1).unwrap().paragraphs().add(Box::new(table));
        }
        let mut out = Vec::new();
        doc.save_to_writer(&mut out).unwrap();
        // A cell frame is stroked as four line segments closed and
        // painted: `h S Q` ends each one.
        String::from_utf8_lossy(&out).matches("h\nS\nQ\n").count()
    }
    let both = stroked_frames(false);
    let one = stroked_frames(true);
    assert!(both > 0, "the table should stroke its cell frames at all");
    assert_eq!(one, both - 1, "marking a cell borderless removes exactly its frame");
}

/// Pins what the layout layer's "carried, not applied" notes claim, so
/// wiring any of them fails here and takes the stale note with it.
///
/// A cell's background COLOUR is painted — a colour needs no resource.
/// Its background IMAGE is not: drawing one means adding an image
/// XObject to the page, and a paragraph's render declares content bytes
/// and font resources only, without knowing its page. Setting the file
/// path therefore changes no byte of the output.
#[test]
fn a_cells_background_image_changes_nothing_while_a_colour_paints() {
    fn rendered(with_colour: bool, with_image: bool) -> Vec<u8> {
        let doc = Document::open(fixture("hello_world.pdf")).unwrap();
        {
            let page = doc.pages().get(1).unwrap();
            let mut table = Table::new();
            table.set_left(20.0);
            table.set_top(40.0);
            table.set_column_widths("80");
            let row = table.rows_mut().add();
            let cell = row.cells_mut().add();
            if with_colour {
                cell.set_background_color(Color::from_rgb(0.2, 0.4, 0.6));
            }
            if with_image {
                cell.set_background_image_file("/tmp/does-not-matter.png");
            }
            page.paragraphs().add(Box::new(table));
        }
        let mut out = Vec::new();
        doc.save_to_writer(&mut out).unwrap();
        out
    }

    let plain = rendered(false, false);
    let coloured = rendered(true, false);
    let with_image = rendered(false, true);

    assert_ne!(coloured, plain, "a background colour reaches the page");
    assert_eq!(
        with_image, plain,
        "a background image does not — if that changed, the note on \
         Cell::background_image_file is stale"
    );
}

/// `Table::is_borders_included` defaults to `true`, and `true` is what
/// the renderer already does: a column width is the drawn width,
/// borders and all. Turning it off changes nothing, which is the half
/// that is missing.
#[test]
fn turning_off_borders_included_changes_nothing() {
    fn rendered(included: bool) -> Vec<u8> {
        let doc = Document::open(fixture("hello_world.pdf")).unwrap();
        {
            let page = doc.pages().get(1).unwrap();
            let mut table = Table::new();
            table.set_left(20.0);
            table.set_top(40.0);
            table.set_column_widths("80 80");
            table.set_is_borders_included(included);
            let row = table.rows_mut().add();
            row.cells_mut().add_text("a");
            row.cells_mut().add_text("b");
            page.paragraphs().add(Box::new(table));
        }
        let mut out = Vec::new();
        doc.save_to_writer(&mut out).unwrap();
        out
    }

    assert!(Table::new().is_borders_included(), "the default is true");
    assert_eq!(
        rendered(false),
        rendered(true),
        "only the false case is missing; if that changed, the note on \
         Table::is_borders_included is stale"
    );
}

/// A row sat between the cell and the table without being consulted:
/// `Row::DefaultCellBorder`, `Row::DefaultCellTextState` and
/// `Row::BackgroundColor` were stored and dropped, while the cell and
/// the table levels beside them worked — the border width already fell
/// back from the cell to the table, so the row was the missing middle
/// link rather than a missing mechanism.
#[test]
fn a_rows_defaults_sit_between_the_cell_and_the_table() {
    fn frames(row_width: f32) -> usize {
        let doc = Document::open(fixture("hello_world.pdf")).unwrap();
        {
            let mut table = Table::new();
            table.set_column_widths("100 100");
            let row = table.rows_mut().add();
            row.set_default_cell_border(BorderInfo::with_side_width(BorderSide::ALL, row_width));
            row.cells_mut().add_text("left");
            row.cells_mut().add_text("right");
            doc.pages().get(1).unwrap().paragraphs().add(Box::new(table));
        }
        let mut out = Vec::new();
        doc.save_to_writer(&mut out).unwrap();
        String::from_utf8_lossy(&out).matches("h\nS\nQ\n").count()
    }
    // With no border anywhere the cells are unframed; the row's default
    // is what frames them.
    assert_eq!(frames(0.0), 0, "nothing sets a width, so nothing is stroked");
    assert_eq!(frames(1.5), 2, "the row's default frames both of its cells");
}

/// The cell's own setting still wins over its row's, and the row's over
/// the table's: the chain is narrowest-first, with a zero width the
/// unset state at every level.
#[test]
fn the_narrowest_border_setting_wins() {
    let doc = Document::open(fixture("hello_world.pdf")).unwrap();
    {
        let mut table = Table::new();
        table.set_column_widths("100 100");
        table.set_default_cell_border(BorderInfo::with_side_width(BorderSide::ALL, 3.0));
        let row = table.rows_mut().add();
        row.set_default_cell_border(BorderInfo::with_side_width(BorderSide::ALL, 2.0));
        row.cells_mut().add_text("inherits the row");
        let own = row.cells_mut().add_text("sets its own");
        own.set_border(BorderInfo::with_side_width(BorderSide::ALL, 1.0));
        doc.pages().get(1).unwrap().paragraphs().add(Box::new(table));
    }
    let mut out = Vec::new();
    doc.save_to_writer(&mut out).unwrap();
    let text = String::from_utf8_lossy(&out);
    assert!(text.contains("1 w"), "the cell that set its own width is stroked with it");
    assert!(text.contains("2 w"), "the other takes the row's, not the table's 3");
    assert!(!text.contains("3 w"), "the table's default is never reached here");
}

/// A row's background fills the whole row behind its cells, which is
/// the mechanism a cell's own background colour already used.
#[test]
fn a_row_background_fills_behind_its_cells() {
    fn rendered(with_row_fill: bool) -> String {
        let doc = Document::open(fixture("hello_world.pdf")).unwrap();
        {
            let mut table = Table::new();
            table.set_column_widths("100 100");
            let row = table.rows_mut().add();
            if with_row_fill {
                row.set_background_color(Color::from_argb_rgb(10, 20, 30));
            }
            row.cells_mut().add_text("left");
            doc.pages().get(1).unwrap().paragraphs().add(Box::new(table));
        }
        let mut out = Vec::new();
        doc.save_to_writer(&mut out).unwrap();
        String::from_utf8_lossy(&out).into_owned()
    }
    let plain = rendered(false);
    let filled = rendered(true);
    assert!(filled.len() > plain.len(), "the fill adds content");
    assert!(filled.contains(" rg\n"), "painted with a fill colour");
}

/// The same middle link for the text state, resolved per attribute by
/// the unset conventions the cell level already used — a zero size is
/// unset, so a row's default size reaches a cell that states none, and
/// the table's reaches a row that states none.
#[test]
fn a_font_size_falls_back_from_the_cell_to_the_row_to_the_table() {
    fn size_in(set: &str) -> String {
        let doc = Document::open(fixture("hello_world.pdf")).unwrap();
        {
            let mut table = Table::new();
            table.set_column_widths("200");
            if set == "table" {
                let mut ts = TextState::default();
                ts.set_font_size(21.0);
                table.set_default_cell_text_state(ts);
            }
            let row = table.rows_mut().add();
            if set == "row" {
                let mut ts = TextState::default();
                ts.set_font_size(17.0);
                row.set_default_cell_text_state(ts);
            }
            row.cells_mut().add_text("measured");
            doc.pages().get(1).unwrap().paragraphs().add(Box::new(table));
        }
        let mut out = Vec::new();
        doc.save_to_writer(&mut out).unwrap();
        String::from_utf8_lossy(&out).into_owned()
    }
    assert!(size_in("row").contains(" 17 Tf"), "the row's default size is used");
    assert!(size_in("table").contains(" 21 Tf"), "and the table's where no row sets one");
}
