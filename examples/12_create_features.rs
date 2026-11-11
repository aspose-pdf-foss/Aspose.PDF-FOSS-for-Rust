// Build a ten-page "feature showcase" PDF *from scratch* through the
// creation surface — the mirror image of the other examples here, which
// all open an existing file. One program that exercises every creation
// capability the crate ships.
//
//   usage: 12_create_features <output.pdf>
//
// Pages produced (one capability family each):
//   1  Cover — document metadata + a bookmark outline over the whole file
//   2  Contents — a table of contents with dot leaders and page numbers
//   3  Text — Standard-14 font grid, a word-wrapped paragraph, a watermark
//   4  Image — a page this program rendered to PNG, embedded back as an
//      image XObject
//   5  AcroForm — text / checkbox / radio / combo / list / button fields
//   6  Annotations — text, square, circle, line, ink, polygon, stamp,
//      link and file-attachment
//   7  Table — an itemised bill with a column-spanned total row
//   8  Vector graphics — line, rectangle, circle, ellipse
//   9  Bar chart drawn from rectangles (landscape page)
//  10  Attachment — the document-level embedded file, and a closing note
//
// The program runs in two phases: it builds and saves the ten pages,
// reopens the result, rasterises page 8 through PngDevice, and embeds
// that PNG on page 4 — so the image on page 4 is the library's own
// rendering of its own vector page.
//
// Text, tables and vector graphics paint into the page content stream and
// show up in our own rasteriser (render any page with 07_render_png).
// Two things on these pages are structure rather than paint. A
// watermark's opacity is carried as a property but not applied: it needs
// an /ExtGState resource the content flush does not write, even though
// the renderer honours one. And form fields and annotations are written
// as interactive objects — page /Annots and the document /AcroForm —
// carrying the entries their own §12.5.6 / §12.7 tables define: a
// widget's /FT, /Ff, /H, /MK and /Opt, an ink annotation's /InkList, a
// file attachment's /FS. What they do not carry is an appearance stream,
// which is the one thing a viewer may draw from rather than from its own
// defaults — and it is the only thing our rasteriser draws an annotation
// from, so pages 5 and 6 render here as their heading and labels alone
// while a viewer shows the fields and annotations themselves.
use std::process::ExitCode;

use aspose_pdf_foss::{
    Annotation, BorderInfo, BorderSide, ButtonField, CheckboxField, ChoiceField, Circle,
    CircleAnnotation, Color, ComboBoxField, Document, DrawingRectangle, Ellipse, Error, Field,
    FileAttachmentAnnotation, FileSpecification, FontRepository, Graph, GraphInfo,
    HorizontalAlignment, InkAnnotation, Line, LineAnnotation, LinkAnnotation, ListBoxField,
    MarkupAnnotation, OutlineItemCollection, Page, PageDevice, PngDevice, Point, PolygonAnnotation,
    Position, RadioButtonField, Rectangle, Resolution, Shape, SquareAnnotation, StampAnnotation,
    Table, TextAnnotation, TextBoxField, TextBuilder, TextFragment, TextParagraph, TextState,
    WatermarkArtifact, XYZExplicitDestination,
};

const MARGIN: f64 = 64.0;
const TOP: f64 = 720.0;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 2 {
        eprintln!("usage: 12_create_features <output.pdf>");
        return ExitCode::from(2);
    }
    match build(&args[1]) {
        Ok(()) => {
            println!(
                "Wrote the ten-page feature showcase to {}. Rasterise any \
                 page with:\n    cargo run --example 07_render_png -- {} 7 /tmp/page7.png",
                args[1], args[1]
            );
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("build {}: {e}", args[1]);
            ExitCode::from(1)
        }
    }
}

fn build(output: &str) -> Result<(), Error> {
    let mut doc = Document::new();

    doc.info_mut().set_title("Aspose.PDF FOSS for Rust — feature showcase");
    doc.info_mut().set_author("Aspose.PDF FOSS for Rust");
    doc.info_mut().set_subject("Every creation capability the crate ships");
    doc.info_mut().set_keywords("pdf; rust; showcase");
    doc.info_mut().set_creator("examples/12_create_features.rs");

    let pages: Vec<Page> = (0..10).map(|_| doc.pages().add()).collect();

    cover(&pages[0]);
    contents(&pages[1]);
    text_page(&pages[2]);
    image_page(&pages[3]);
    annotations(&pages[5]);
    table(&pages[6]);
    vector_graphics(&pages[7]);
    bar_chart(&pages[8]);
    closing(&pages[9]);

    form_fields(&mut doc, &pages[4]);
    attachment(&mut doc, &pages[9]);
    bookmarks(&doc);

    // Phase two: rasterise the vector page and embed that PNG on page 4,
    // so the image is the library's own rendering of its own output.
    let mut buf = Vec::new();
    doc.save_to_writer(&mut buf)?;
    let reopened = Document::from_bytes(buf)?;

    let mut png = Vec::new();
    let device = PngDevice::with_resolution(Resolution::new(72)?);
    device.process(&reopened.pages().get(8)?, &mut png)?;
    reopened.pages().get(4)?.add_image(
        &png,
        Rectangle::new(MARGIN, 360.0, 548.0, 660.0, false),
        None,
        false,
    )?;

    reopened.save_to(output)
}

// ---- page 1 — cover ------------------------------------------------

fn cover(page: &Page) {
    put_font(page, "Aspose.PDF FOSS", MARGIN, 640.0, 34.0, "Helvetica-Bold");
    put_font(page, "for Rust", MARGIN, 600.0, 34.0, "Helvetica-Bold");
    put(page, "A feature showcase, built from scratch", MARGIN, 548.0, 14.0);
    put_font(
        page,
        "Every page below is created by examples/12_create_features.rs",
        MARGIN,
        528.0,
        10.0,
        "Courier",
    );

    let mut rule = Graph::new(420.0, 2.0);
    rule.set_left(MARGIN);
    rule.set_top(792.0 - 572.0);
    let mut bar = DrawingRectangle::new(0.0, 0.0, 420.0, 2.0);
    let mut filled = GraphInfo::new();
    filled.set_fill_color(Color::from_argb_rgb(0, 90, 160));
    bar.set_graph_info(filled);
    rule.add_shape(Box::new(bar));
    page.paragraphs().add(Box::new(rule));

    put(page, "Strict subset of the canonical Aspose.PDF public API.", MARGIN, 492.0, 11.0);
    put(page, "No dependencies: every primitive is hand-authored from spec.", MARGIN, 476.0, 11.0);
}

// ---- page 2 — contents ---------------------------------------------

fn contents(page: &Page) {
    heading(page, "Contents");

    let entries = [
        ("Cover", 1),
        ("Contents", 2),
        ("Text, fonts and watermarks", 3),
        ("Image embedding", 4),
        ("AcroForm fields", 5),
        ("Annotations", 6),
        ("Tables", 7),
        ("Vector graphics", 8),
        ("Bar chart", 9),
        ("Attachments", 10),
    ];

    let mut y = 670.0;
    for (title, number) in entries {
        // Dot leaders are drawn as text: the crate carries TabStop as a
        // value type, but no layout consumer reads it yet.
        let dots = ".".repeat(58usize.saturating_sub(title.len()));
        put_font(page, &format!("{title} {dots} {number}"), MARGIN, y, 11.0, "Courier");
        y -= 22.0;
    }
}

// ---- page 3 — text -------------------------------------------------

fn text_page(page: &Page) {
    heading(page, "Text, fonts and watermarks");

    let fonts = [
        "Helvetica",
        "Helvetica-Bold",
        "Helvetica-Oblique",
        "Times-Roman",
        "Times-Bold",
        "Times-Italic",
        "Courier",
        "Courier-Bold",
    ];
    let mut y = 670.0;
    for name in fonts {
        put_font(page, &format!("{name} - The quick brown fox jumps over the lazy dog"), MARGIN, y, 12.0, name);
        y -= 24.0;
    }

    // A TextParagraph is laid into the page by a TextBuilder bound to
    // that page; its rectangle is the wrap box, so this one paragraph
    // breaks itself across as many lines as the width needs. Single
    // positioned runs go through Page.paragraphs() instead.
    let mut para = TextParagraph::new();
    para.set_rectangle(Rectangle::new(MARGIN, 300.0, 480.0, 450.0, true));
    para.set_horizontal_alignment(HorizontalAlignment::Left);
    para.set_subsequent_lines_indent(14.0);
    para.append_line(
        "A TextParagraph wraps its own text inside the rectangle it is given, \
         breaking at whitespace and indenting the lines after the first. \
         Each appended line is a logical paragraph of its own.",
    );
    para.append_line("This second appended line starts below the first one.");
    let builder = TextBuilder::new(page);
    builder.append_paragraph(&para);

    let mut watermark = WatermarkArtifact::new();
    let mut state = TextState::default();
    state.set_font_size(48.0);
    state.set_foreground_color(Color::from_argb_rgb(200, 200, 200));
    watermark.set_text_and_state("DRAFT", state);
    watermark.set_position(Point::new(150.0, 250.0));
    watermark.set_rotation(45.0);
    watermark.set_opacity(0.35);
    watermark.set_is_background(true);
    page.artifacts().add(watermark);
}

// ---- page 4 — image ------------------------------------------------

fn image_page(page: &Page) {
    heading(page, "Image embedding");
    put(
        page,
        "Below: page 8 of this document, rasterised by PngDevice at 72 DPI",
        MARGIN,
        686.0,
        11.0,
    );
    put(
        page,
        "and embedded back into this page as a /Resources /XObject /Image.",
        MARGIN,
        670.0,
        11.0,
    );
    // Below the picture — the image covers everything from y=360 to
    // y=660 and would paint over anything placed inside that band.
    put(
        page,
        "The picture above is this document's own page 8, drawn by the",
        MARGIN,
        330.0,
        11.0,
    );
    put(page, "same engine that rasterises this page back.", MARGIN, 314.0, 11.0);
}

// ---- page 5 — AcroForm ---------------------------------------------

fn form_fields(doc: &mut Document, page: &Page) {
    heading(page, "AcroForm fields");

    let labels = [
        ("Full name", 640.0),
        ("Subscribe", 600.0),
        ("Plan", 560.0),
        ("Country", 500.0),
        ("Languages", 440.0),
    ];
    for (label, y) in labels {
        put(page, label, MARGIN, y, 11.0);
    }

    let mut name = TextBoxField::on_page(page, Rectangle::new(190.0, 634.0, 450.0, 654.0, false));
    name.set_partial_name("fullname".into());
    name.set_value("Ada Lovelace".into());

    let mut subscribe = CheckboxField::on_page(page, Rectangle::new(190.0, 594.0, 206.0, 610.0, false));
    subscribe.set_partial_name("subscribe".into());
    subscribe.set_checked(true);

    let mut plan = RadioButtonField::from_page(page);
    plan.set_partial_name("plan".into());
    plan.add_option_at("monthly".into(), Rectangle::new(190.0, 554.0, 206.0, 570.0, false));
    plan.add_option_at("yearly".into(), Rectangle::new(280.0, 554.0, 296.0, 570.0, false));
    put(page, "monthly", 212.0, 558.0, 10.0);
    put(page, "yearly", 302.0, 558.0, 10.0);

    let mut country = ComboBoxField::on_page(page, Rectangle::new(190.0, 494.0, 380.0, 514.0, false));
    country.set_partial_name("country".into());
    country.add_option("United Kingdom".into());
    country.add_option("Norway".into());
    country.add_option("Japan".into());
    country.set_selected(1);

    let mut languages = ListBoxField::on_page(page, Rectangle::new(190.0, 420.0, 380.0, 480.0, false));
    languages.set_partial_name("languages".into());
    languages.add_option("Rust".into());
    languages.add_option("C++".into());
    languages.add_option("C#".into());
    languages.set_multi_select(true);

    let mut submit = ButtonField::on_page(page, Rectangle::new(190.0, 370.0, 290.0, 396.0, false));
    submit.set_partial_name("submit".into());
    submit.set_normal_caption("Submit".into());

    doc.form_mut().add(Box::new(name));
    doc.form_mut().add(Box::new(subscribe));
    doc.form_mut().add(Box::new(plan));
    doc.form_mut().add(Box::new(country));
    doc.form_mut().add(Box::new(languages));
    doc.form_mut().add(Box::new(submit));
}

// ---- page 6 — annotations ------------------------------------------

fn annotations(page: &Page) {
    heading(page, "Annotations");

    let mut note = TextAnnotation::on_page(page, Rectangle::new(MARGIN, 640.0, 84.0, 660.0, false));
    note.set_contents("A sticky note.".into());
    note.set_title("Showcase".into());

    let mut square = SquareAnnotation::on_page(page, Rectangle::new(140.0, 620.0, 240.0, 680.0, false));
    square.set_color(Color::from_argb_rgb(200, 40, 40));

    let mut circle = CircleAnnotation::on_page(page, Rectangle::new(260.0, 620.0, 360.0, 680.0, false));
    circle.set_color(Color::from_argb_rgb(40, 120, 200));

    let line = LineAnnotation::on_page(
        page,
        Rectangle::new(380.0, 620.0, 540.0, 680.0, false),
        Point::new(380.0, 620.0),
        Point::new(540.0, 680.0),
    );

    let ink = InkAnnotation::on_page(
        page,
        Rectangle::new(MARGIN, 500.0, 260.0, 590.0, false),
        vec![vec![
            Point::new(70.0, 520.0),
            Point::new(110.0, 580.0),
            Point::new(150.0, 510.0),
            Point::new(200.0, 575.0),
            Point::new(250.0, 515.0),
        ]],
    );

    let polygon = PolygonAnnotation::on_page(
        page,
        Rectangle::new(280.0, 500.0, 400.0, 590.0, false),
        vec![
            Point::new(340.0, 590.0),
            Point::new(400.0, 540.0),
            Point::new(370.0, 500.0),
            Point::new(310.0, 500.0),
            Point::new(280.0, 540.0),
        ],
    );

    let mut stamp = StampAnnotation::on_page(page, Rectangle::new(420.0, 500.0, 540.0, 560.0, false));
    stamp.set_contents("Approved".into());

    let mut link = LinkAnnotation::on_page(page, Rectangle::new(MARGIN, 430.0, 220.0, 450.0, false));
    link.set_destination(Box::new(XYZExplicitDestination::new(1, 0.0, 792.0, 0.0)));
    put(page, "Back to the cover (link annotation)", MARGIN, 434.0, 11.0);

    let mut attached = FileSpecification::with_file_and_description(
        concat!(env!("CARGO_MANIFEST_DIR"), "/examples/12_create_features.rs").into(),
        "The program that produced this document".into(),
    );
    attached.set_name("12_create_features.rs".into());
    attached.set_mime_type("text/plain".into());
    let attachment = FileAttachmentAnnotation::on_page(
        page,
        Rectangle::new(260.0, 430.0, 280.0, 450.0, false),
        attached,
    );
    put(page, "A file attachment annotation", 296.0, 434.0, 11.0);

    let annots = page.annotations();
    annots.add(Box::new(note));
    annots.add(Box::new(square));
    annots.add(Box::new(circle));
    annots.add(Box::new(line));
    annots.add(Box::new(ink));
    annots.add(Box::new(polygon));
    annots.add(Box::new(stamp));
    annots.add(Box::new(link));
    annots.add(Box::new(attachment));
}

// ---- page 7 — table ------------------------------------------------

fn table(page: &Page) {
    heading(page, "Tables");

    let mut table = Table::new();
    table.set_left(MARGIN as f32);
    table.set_top(120.0);
    table.set_column_widths("240 80 80 80");
    table.set_default_cell_border(BorderInfo::with_side_width(BorderSide::ALL, 0.5));
    table.set_border(BorderInfo::with_side_width(BorderSide::ALL, 1.0));

    let mut header_style = TextState::default();
    header_style.set_font_size(11.0);
    header_style.set_font(FontRepository::find_font("Helvetica-Bold"));
    header_style.set_foreground_color(Color::from_argb_rgb(30, 58, 92));

    let header = table.rows_mut().add();
    for label in ["Item", "Qty", "Unit", "Amount"] {
        let cell = header.cells_mut().add_text_with_state(label, header_style.clone());
        cell.set_background_color(Color::from_argb_rgb(226, 232, 240));
        cell.set_alignment(HorizontalAlignment::Center);
    }

    let lines = [
        ("Espresso", 2, "2.40", "4.80"),
        ("Cinnamon bun", 1, "3.10", "3.10"),
        ("Sparkling water", 3, "1.20", "3.60"),
    ];
    for (item, qty, unit, amount) in lines {
        let row = table.rows_mut().add();
        row.cells_mut().add_text(item);
        row.cells_mut().add_text(qty.to_string()).set_alignment(HorizontalAlignment::Center);
        row.cells_mut().add_text(unit).set_alignment(HorizontalAlignment::Right);
        row.cells_mut().add_text(amount).set_alignment(HorizontalAlignment::Right);
    }

    let total = table.rows_mut().add();
    let label = total.cells_mut().add_text("Total");
    label.set_col_span(3);
    label.set_alignment(HorizontalAlignment::Right);
    total.cells_mut().add_text("11.50").set_alignment(HorizontalAlignment::Right);

    page.paragraphs().add(Box::new(table));
}

// ---- page 8 — vector graphics --------------------------------------

fn vector_graphics(page: &Page) {
    heading(page, "Vector graphics");

    let mut graph = Graph::new(440.0, 380.0);
    graph.set_left(MARGIN);
    graph.set_top(160.0);

    let mut frame = DrawingRectangle::new(0.0, 0.0, 440.0, 380.0);
    frame.set_graph_info(stroked(Color::from_argb_rgb(120, 120, 120), 1.0));
    graph.add_shape(Box::new(frame));

    let mut diagonal = Line::new(vec![0.0, 0.0, 440.0, 380.0]);
    diagonal.set_graph_info(stroked(Color::from_argb_rgb(0, 90, 160), 2.0));
    graph.add_shape(Box::new(diagonal));

    let mut circle = Circle::new(120.0, 260.0, 70.0);
    let mut fill = GraphInfo::new();
    fill.set_fill_color(Color::from_argb_rgb(240, 180, 60));
    circle.set_graph_info(fill);
    graph.add_shape(Box::new(circle));

    let mut ellipse = Ellipse::new(240.0, 60.0, 170.0, 90.0);
    ellipse.set_graph_info(stroked(Color::from_argb_rgb(200, 40, 40), 2.0));
    graph.add_shape(Box::new(ellipse));

    // A dashed rule, to show the dash pattern surviving the whole
    // round trip: set here, written as `d` into the content stream,
    // and stroked span by span when the page is rendered.
    let mut dashed = Line::new(vec![0.0, 190.0, 440.0, 190.0]);
    let mut dash_info = stroked(Color::from_argb_rgb(60, 60, 60), 1.5);
    dash_info.set_dash_array(vec![8, 5]);
    dash_info.set_dash_phase(0);
    dashed.set_graph_info(dash_info);
    graph.add_shape(Box::new(dashed));

    page.paragraphs().add(Box::new(graph));
}

// ---- page 9 — bar chart --------------------------------------------

fn bar_chart(page: &Page) {
    page.set_page_size(792.0, 612.0);
    put_font(page, "Bar chart", MARGIN, 540.0, 18.0, "Helvetica-Bold");

    let data = [
        ("C#", 28.0),
        ("C++", 143.0),
        ("Go", 255.0),
        ("Java", 29.0),
        ("Python", 29.0),
        ("Rust", 97.0),
        ("TypeScript", 29.0),
    ];
    let max = data.iter().map(|(_, v)| *v).fold(0.0f64, f64::max);

    let chart_w = 640.0;
    let chart_h = 340.0;
    let mut chart = Graph::new(chart_w, chart_h);
    chart.set_left(MARGIN);
    chart.set_top(120.0);

    let mut axis = Line::new(vec![0.0, 0.0, chart_w as f32, 0.0]);
    axis.set_graph_info(stroked(Color::from_argb_rgb(60, 60, 60), 1.0));
    chart.add_shape(Box::new(axis));

    let slot = chart_w / data.len() as f64;
    for (i, (_, value)) in data.iter().enumerate() {
        let height = (value / max * (chart_h - 40.0)) as f32;
        let left = (i as f64 * slot) as f32 + 14.0;
        let mut bar = DrawingRectangle::new(left, 0.0, slot as f32 - 28.0, height);
        let mut fill = GraphInfo::new();
        fill.set_fill_color(Color::from_argb_rgb(0, 90, 160));
        bar.set_graph_info(fill);
        chart.add_shape(Box::new(bar));
    }
    page.paragraphs().add(Box::new(chart));

    // The graph's own origin is its bottom-left corner, which sits at
    // page height - top - height; the labels are placed just below it.
    let axis_y = 612.0 - 120.0 - chart_h;
    for (i, (label, value)) in data.iter().enumerate() {
        let x = MARGIN + i as f64 * slot + 16.0;
        put_font(page, label, x, axis_y - 16.0, 9.0, "Helvetica-Bold");
        put(page, &format!("{value:.0}"), x, axis_y - 30.0, 9.0);
    }
    put(page, "Public-API source files per FOSS track", MARGIN, axis_y - 56.0, 10.0);
}

// ---- page 10 — attachment + closing --------------------------------

fn closing(page: &Page) {
    heading(page, "Attachments");
    put(page, "This document carries its own source as a document-level", MARGIN, 686.0, 11.0);
    put(page, "attachment (catalog /Names /EmbeddedFiles) — open the", MARGIN, 670.0, 11.0);
    put(page, "attachment panel of a viewer to extract it.", MARGIN, 654.0, 11.0);
    put_font(page, "Rendered and written by Aspose.PDF FOSS for Rust.", MARGIN, 120.0, 10.0, "Courier");
}

fn attachment(doc: &mut Document, _page: &Page) {
    let mut spec = FileSpecification::with_file_and_description(
        concat!(env!("CARGO_MANIFEST_DIR"), "/examples/12_create_features.rs").into(),
        "The program that produced this document".into(),
    );
    spec.set_name("12_create_features.rs".into());
    spec.set_mime_type("text/plain".into());
    doc.embedded_files_mut().add_with_key("source".into(), spec);
}

// ---- bookmarks -----------------------------------------------------

fn bookmarks(doc: &Document) {
    let titles = [
        "Cover",
        "Contents",
        "Text, fonts and watermarks",
        "Image embedding",
        "AcroForm fields",
        "Annotations",
        "Tables",
        "Vector graphics",
        "Bar chart",
        "Attachments",
    ];
    let root = doc.outlines();
    for (i, title) in titles.iter().enumerate() {
        let item = OutlineItemCollection::new(root);
        item.set_title((*title).into());
        item.set_bold(i == 0);
        item.set_destination(Box::new(XYZExplicitDestination::new(
            i as i32 + 1,
            0.0,
            792.0,
            0.0,
        )));
        root.add(&item);
    }
}

// ---- helpers -------------------------------------------------------

fn heading(page: &Page, title: &str) {
    put_font(page, title, MARGIN, TOP, 18.0, "Helvetica-Bold");
}

fn put(page: &Page, text: &str, x: f64, y: f64, size: f32) {
    let mut fragment = TextFragment::with_text(text);
    fragment.text_state_mut().set_font_size(size);
    fragment.set_position(Position::new(x, y));
    page.paragraphs().add(Box::new(fragment));
}

fn put_font(page: &Page, text: &str, x: f64, y: f64, size: f32, font: &str) {
    let mut fragment = TextFragment::with_text(text);
    fragment.text_state_mut().set_font_size(size);
    fragment.text_state_mut().set_font(FontRepository::find_font(font));
    fragment.set_position(Position::new(x, y));
    page.paragraphs().add(Box::new(fragment));
}

fn stroked(color: Color, width: f32) -> GraphInfo {
    let mut info = GraphInfo::new();
    info.set_color(color);
    info.set_line_width(width);
    info
}
