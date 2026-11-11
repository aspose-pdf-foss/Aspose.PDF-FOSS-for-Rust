// Public-API driver — drawing render-on-save via Page.Paragraphs.
//
// A Graph added to a page's paragraph collection renders its shapes
// into the page content stream on save: a stroked rectangle, a poly-
// line, and a filled circle become PDF path + paint operators. An
// untouched page resaves verbatim.

use aspose_pdf_foss::{
    Circle, Color, Document, DrawingRectangle, Graph, GraphInfo, Line, Position, Shape,
    TextAbsorber, TextFragment,
};
use std::path::{Path, PathBuf};

// The integration-test crate cannot see the lib's crate-private
// internal modules, so the PNG decoder is re-mounted by path — the
// same back-channel png_device_test uses. It depends only on flate.
#[path = "../../src/internal/flate.rs"]
mod flate;
#[path = "../../src/internal/png_decoder.rs"]
mod png_decoder;

fn hello_world() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/text_extractor/hello_world.pdf")
}

fn contains(haystack: &[u8], needle: &[u8]) -> bool {
    haystack.windows(needle.len()).any(|w| w == needle)
}

fn stroked(color: Color, width: f32) -> GraphInfo {
    let mut gi = GraphInfo::new();
    gi.set_color(color);
    gi.set_line_width(width);
    gi
}

#[test]
fn graph_shapes_render_into_page_content() {
    let doc = Document::open(hello_world()).unwrap();
    {
        let mut graph = Graph::new(200.0, 100.0);
        graph.set_left(50.0);
        graph.set_top(60.0);

        let mut rect = DrawingRectangle::new(0.0, 0.0, 80.0, 40.0);
        rect.set_graph_info(stroked(Color::from_argb_rgb(255, 0, 0), 2.0));
        graph.add_shape(Box::new(rect));

        let mut line = Line::new(vec![0.0, 0.0, 200.0, 100.0]);
        line.set_graph_info(stroked(Color::from_argb_rgb(0, 0, 255), 1.0));
        graph.add_shape(Box::new(line));

        let mut circle = Circle::new(100.0, 50.0, 30.0);
        let mut filled = GraphInfo::new();
        filled.set_fill_color(Color::from_argb_rgb(0, 200, 0));
        circle.set_graph_info(filled);
        graph.add_shape(Box::new(circle));

        doc.pages().get(1).unwrap().paragraphs().add(Box::new(graph));
    }

    let mut out = Vec::new();
    doc.save_to_writer(&mut out).unwrap();

    // Path-construction + paint operators land in the content stream.
    for op in [
        &b" re\n"[..], // rectangle
        b" m\n",       // line / curve move
        b" l\n",       // line segment
        b" c\n",       // circle Bézier
        b" cm\n",      // graph translate
        b"\nS\n",      // stroke (rect / line)
        b"\nf\n",      // fill (circle)
        b" RG\n",      // stroke colour
        b" rg\n",      // fill colour
    ] {
        assert!(contains(&out, op), "missing operator {:?}", std::str::from_utf8(op).unwrap());
    }
}

#[test]
fn text_fragment_paragraph_renders_and_round_trips() {
    let doc = Document::open(hello_world()).unwrap();
    {
        let mut frag = TextFragment::with_text("ParagraphText");
        frag.text_state_mut().set_font_size(14.0);
        frag.set_position(Position::new(120.0, 300.0));
        doc.pages().get(1).unwrap().paragraphs().add(Box::new(frag));
    }
    let mut out = Vec::new();
    doc.save_to_writer(&mut out).unwrap();

    // The fragment's text is extractable from the saved document.
    let reloaded = Document::from_bytes(out).unwrap();
    let mut a = TextAbsorber::new();
    a.visit_document(&reloaded).unwrap();
    assert!(a.text().contains("ParagraphText"), "got: {:?}", a.text());
}

#[test]
fn untouched_paragraphs_resave_verbatim() {
    let original = std::fs::read(hello_world()).unwrap();
    let doc = Document::open(hello_world()).unwrap();
    let _ = doc.pages().get(1).unwrap().paragraphs().count();
    let mut out = Vec::new();
    doc.save_to_writer(&mut out).unwrap();
    assert_eq!(out, original, "untouched page paragraphs must resave verbatim");
}

/// A dash pattern set on a shape has to reach the file. It did not:
/// `GraphInfo` accepted the array and the content writer emitted the
/// width, the colours, the path and the paint operator — but never
/// `d`, so the pattern was stored and dropped.
///
/// The assertion is on the rendered page, not on the bytes. "The
/// operator is present" would also pass for `[0 0] 0 d`, which
/// strokes solid.
#[test]
fn a_dash_pattern_reaches_the_rendered_page() {
    // The line spans 200 units; 6 on / 6 off gives about 17 runs.
    let dashed = ink_runs_along_the_line(Some((vec![6, 6], 0)));
    let solid = ink_runs_along_the_line(None);
    assert_eq!(solid, 1, "an undashed line is one unbroken run");
    assert!(dashed >= 8, "the pattern should break the line up, got {dashed}");
}

/// Draw one horizontal line on page 1 of hello_world.pdf, optionally
/// dashed, render at 72 dpi and count the separate inked runs along
/// the row it sits on.
fn ink_runs_along_the_line(dash: Option<(Vec<i32>, i32)>) -> usize {
    use aspose_pdf_foss::{PageDevice, PngDevice, Resolution};

    let doc = Document::open(hello_world()).unwrap();
    {
        let mut graph = Graph::new(200.0, 20.0);
        graph.set_left(20.0);
        graph.set_top(400.0);
        let mut line = Line::new(vec![0.0, 10.0, 200.0, 10.0]);
        let mut gi = stroked(Color::from_argb_rgb(0, 0, 0), 2.0);
        if let Some((pattern, phase)) = dash {
            gi.set_dash_array(pattern);
            gi.set_dash_phase(phase);
        }
        line.set_graph_info(gi);
        graph.add_shape(Box::new(line));
        doc.pages().get(1).unwrap().paragraphs().add(Box::new(graph));
    }
    let mut saved = Vec::new();
    doc.save_to_writer(&mut saved).unwrap();

    let reopened = Document::from_bytes(saved).unwrap();
    let page = reopened.pages().get(1).unwrap();
    let mut png = Vec::new();
    PngDevice::with_resolution(Resolution::new(72).unwrap())
        .process(&page, &mut png)
        .unwrap();
    let decoded = png_decoder::decode(&png).expect("decode the rendered page");

    // The graph is placed 400 from the top of a 792-unit page and is
    // 20 tall, so its origin sits at user y 372; the line is 10 above
    // that, which at 72 dpi is device row 792 - 382 = 410.
    let width = decoded.width as usize;
    let row = 410usize;
    let (mut runs, mut prev) = (0usize, false);
    for x in 0..width {
        let i = (row * width + x) * 4;
        let ink = decoded.pixels[i] < 200;
        if ink && !prev {
            runs += 1;
        }
        prev = ink;
    }
    runs
}

/// `GraphInfo` carries six members that nothing reads when a shape is
/// written: the doubled-line flag, the two skew angles, the two
/// scaling rates and the rotation angle. Their rustdoc says so, and
/// this pins it — setting all six must change nothing that can be
/// seen, so the day someone wires one of them up, this test fails and
/// the documentation gets corrected with it.
///
/// The comparison is on rendered pixels rather than on the saved
/// bytes, because that is the claim being made: no visible effect.
#[test]
fn the_affine_descriptors_are_carried_without_effect() {
    let plain = render_square(false);
    let styled = render_square(true);
    assert_eq!(
        plain, styled,
        "setting is_doubled / skew / scale / rotation changed the rendered page — \
         if that is now intended, the 'carried, not applied' note on GraphInfo is stale"
    );
}

/// Draw one square on page 1 and render it, optionally setting every
/// affine descriptor `GraphInfo` carries.
fn render_square(styled: bool) -> Vec<u8> {
    use aspose_pdf_foss::{PageDevice, PngDevice, Resolution};

    let doc = Document::open(hello_world()).unwrap();
    {
        let mut graph = Graph::new(120.0, 120.0);
        graph.set_left(40.0);
        graph.set_top(300.0);
        let mut rect = DrawingRectangle::new(10.0, 10.0, 90.0, 90.0);
        let mut gi = stroked(Color::from_argb_rgb(0, 0, 0), 2.0);
        if styled {
            gi.set_is_doubled(true);
            gi.set_skew_angle_x(30.0);
            gi.set_skew_angle_y(15.0);
            gi.set_scaling_rate_x(2.0);
            gi.set_scaling_rate_y(0.5);
            gi.set_rotation_angle(45.0);
        }
        rect.set_graph_info(gi);
        graph.add_shape(Box::new(rect));
        doc.pages().get(1).unwrap().paragraphs().add(Box::new(graph));
    }
    let mut saved = Vec::new();
    doc.save_to_writer(&mut saved).unwrap();
    let reopened = Document::from_bytes(saved).unwrap();
    let page = reopened.pages().get(1).unwrap();
    let mut png = Vec::new();
    PngDevice::with_resolution(Resolution::new(72).unwrap())
        .process(&page, &mut png)
        .unwrap();
    png
}
