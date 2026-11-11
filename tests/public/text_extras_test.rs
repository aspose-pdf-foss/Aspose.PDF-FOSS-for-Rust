// Public-API smoke driver — text styling / search extras.
//
// Ports the C++ smoke tests:
//   * text_state_smoke_test.cpp          -> TextState defaults,
//     colour round-trip, decoration / position flags, spacing,
//     scaling, horizontal alignment.
//   * text_fragment_absorber_smoke_test.cpp -> phrase search over a
//     document / page, match-all (default) mode, fragment geometry,
//     accumulation across visits, out-of-range indexing.
//   * text_paragraph_smoke_test.cpp      -> in-memory paragraph
//     build / property surface + render-through-Save round-trips and
//     the FloatingBox property surface / Page.Paragraphs placement.

use std::path::PathBuf;

use aspose_pdf_foss::{
    BorderInfo, BorderSide, Color, Document, FloatingBox, HorizontalAlignment,
    FontRepository, Rectangle, TextBuilder, TextFragmentAbsorber, TextParagraph,
    TextState,
};

fn fixture(rel: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(rel)
}

fn hello_world() -> PathBuf {
    fixture("text_extractor/hello_world.pdf")
}

fn count_matches(path: &std::path::Path, phrase: &str) -> usize {
    let doc = Document::open(path).expect("open saved pdf");
    let mut a = TextFragmentAbsorber::with_phrase(phrase);
    a.visit_document(&doc).expect("visit");
    a.text_fragments().count()
}

// ============================================================
// TextState — text_state_smoke_test.cpp
// ============================================================

#[test]
fn text_state_defaults() {
    let ts = TextState::default();
    assert!(!ts.underline());
    assert!(!ts.strike_out());
    assert!(!ts.invisible());
    assert_eq!(ts.character_spacing(), 0.0);
    assert_eq!(ts.horizontal_scaling(), 100.0);
    assert_eq!(ts.horizontal_alignment(), HorizontalAlignment::None);
}

#[test]
fn text_state_colour_properties() {
    let mut ts = TextState::default();
    // Default colours are transparent (alpha 0); a set colour round-trips.
    assert_eq!(ts.foreground_color().a(), 0.0);
    ts.set_foreground_color(Color::from_rgb(1.0, 0.0, 0.0));
    assert_eq!(ts.foreground_color().a(), 1.0);
    ts.set_stroking_color(Color::from_rgb(0.0, 1.0, 0.0));
    assert_eq!(ts.stroking_color().a(), 1.0);
    ts.set_background_color(Color::transparent());
    assert_eq!(ts.background_color().a(), 0.0);
}

#[test]
fn text_state_flags_and_spacing() {
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

    assert!(ts.underline());
    assert!(ts.strike_out());
    assert!(ts.subscript());
    assert!(ts.superscript());
    assert!(ts.invisible());
    assert_eq!(ts.character_spacing(), 2.5);
    assert_eq!(ts.word_spacing(), 1.5);
    assert_eq!(ts.line_spacing(), 14.0);
    assert_eq!(ts.horizontal_scaling(), 80.0);
    assert_eq!(ts.horizontal_alignment(), HorizontalAlignment::Center);
}

// ============================================================
// TextFragmentAbsorber — text_fragment_absorber_smoke_test.cpp
// ============================================================

#[test]
fn absorber_finds_present_phrase() {
    let doc = Document::open(hello_world()).expect("open"); // "Hello World"
    let mut absorber = TextFragmentAbsorber::with_phrase("World");
    absorber.visit_document(&doc).expect("visit");

    let frags = absorber.text_fragments();
    assert_eq!(frags.count(), 1);
    assert_eq!(frags.get(1).unwrap().text(), "World"); // 1-based indexer
}

#[test]
fn absorber_absent_phrase_yields_none() {
    let doc = Document::open(hello_world()).expect("open");
    let mut absorber = TextFragmentAbsorber::with_phrase("Goodbye");
    absorber.visit_document(&doc).expect("visit");
    assert_eq!(absorber.text_fragments().count(), 0);
}

#[test]
fn absorber_empty_phrase_yields_none() {
    let doc = Document::open(hello_world()).expect("open");
    let mut absorber = TextFragmentAbsorber::with_phrase("");
    absorber.visit_document(&doc).expect("visit");
    assert_eq!(absorber.text_fragments().count(), 0);
}

#[test]
fn absorber_visit_page_searches_one_page() {
    let doc = Document::open(hello_world()).expect("open");
    let page = doc.pages().get(1).expect("page 1");
    let mut absorber = TextFragmentAbsorber::with_phrase("Hello");
    absorber.visit_page(&page).expect("visit_page");
    assert_eq!(absorber.text_fragments().count(), 1);
    assert_eq!(absorber.text_fragments().get(1).unwrap().text(), "Hello");
}

#[test]
fn absorber_indexer_out_of_range_is_none() {
    // Rust models the canonical out-of-range throw as `None`.
    let doc = Document::open(hello_world()).expect("open");
    let mut absorber = TextFragmentAbsorber::with_phrase("World");
    absorber.visit_document(&doc).expect("visit");
    assert!(absorber.text_fragments().get(2).is_none());
    assert!(absorber.text_fragments().get(0).is_none());
}

#[test]
fn absorber_fragment_has_non_degenerate_rectangle() {
    let doc = Document::open(hello_world()).expect("open");
    let mut absorber = TextFragmentAbsorber::with_phrase("World");
    absorber.visit_document(&doc).expect("visit");
    assert_eq!(absorber.text_fragments().count(), 1);
    let rect = absorber.text_fragments().get(1).unwrap().rectangle();
    assert!(rect.width() > 0.0);
    assert!(rect.height() > 0.0);
}

#[test]
fn absorber_world_is_right_of_hello() {
    let doc = Document::open(hello_world()).expect("open"); // "Hello World"
    let mut h = TextFragmentAbsorber::with_phrase("Hello");
    h.visit_document(&doc).expect("visit");
    let mut w = TextFragmentAbsorber::with_phrase("World");
    w.visit_document(&doc).expect("visit");
    assert_eq!(h.text_fragments().count(), 1);
    assert_eq!(w.text_fragments().count(), 1);
    let hx = h.text_fragments().get(1).unwrap().rectangle().llx();
    let wx = w.text_fragments().get(1).unwrap().rectangle().llx();
    assert!(hx < wx, "Hello llx {hx} should be left of World llx {wx}");
}

#[test]
fn absorber_default_ctor_extracts_all_runs() {
    let doc = Document::open(hello_world()).expect("open"); // "Hello World"
    let mut absorber = TextFragmentAbsorber::new(); // match-all
    absorber.visit_document(&doc).expect("visit");
    let frags = absorber.text_fragments();
    assert!(frags.count() >= 1);
    // The whole run is captured, not a sub-phrase.
    assert!(frags.get(1).unwrap().text().contains("Hello"));
}

#[test]
fn absorber_match_all_fragments_carry_position_and_state() {
    let doc = Document::open(hello_world()).expect("open");
    let mut absorber = TextFragmentAbsorber::new();
    absorber.visit_document(&doc).expect("visit");
    assert!(absorber.text_fragments().count() >= 1);
    let frag = absorber.text_fragments().get(1).unwrap();

    let rect = frag.rectangle();
    assert!(rect.width() > 0.0);
    assert!(rect.height() > 0.0);

    let pos = frag.position();
    assert_eq!(pos.x_indent(), rect.llx());
    assert_eq!(pos.y_indent(), rect.lly());

    assert!(frag.text_state().font_size() > 0.0);
}

#[test]
fn absorber_matches_accumulate_across_visits() {
    let doc = Document::open(hello_world()).expect("open");
    let mut absorber = TextFragmentAbsorber::with_phrase("o"); // 2 'o's
    absorber.visit_document(&doc).expect("visit");
    assert_eq!(absorber.text_fragments().count(), 2);
    absorber.visit_document(&doc).expect("visit"); // accumulates
    assert_eq!(absorber.text_fragments().count(), 4);
}

// ============================================================
// TextParagraph / FloatingBox — text_paragraph_smoke_test.cpp
// ============================================================

#[test]
fn paragraph_wrapped_paragraph_renders() {
    let out = common::unique_temp("aspose_rust_tpara.pdf");
    {
        let doc = Document::open(hello_world()).expect("open");
        let mut para = TextParagraph::new();
        para.set_rectangle(Rectangle::new(72.0, 500.0, 300.0, 720.0, true)); // w=228
        para.set_horizontal_alignment(HorizontalAlignment::Left);
        para.append_line(
            "The quick brown fox jumps over the lazy dog again and again",
        );
        para.append_line("Second line of the paragraph block");

        let page = doc.pages().get(1).expect("page 1");
        let builder = TextBuilder::new(&page);
        builder.append_paragraph(&para);
        doc.save_to(&out).expect("save");
    }
    // Words from both logical lines are present after rendering.
    assert!(count_matches(&out, "quick") >= 1);
    assert!(count_matches(&out, "Second") >= 1);
    let _ = std::fs::remove_file(&out);
}

#[test]
fn paragraph_centered_paragraph_renders() {
    let out = common::unique_temp("aspose_rust_tpara_c.pdf");
    {
        let doc = Document::open(hello_world()).expect("open");
        let mut para = TextParagraph::new();
        para.set_rectangle(Rectangle::new(72.0, 400.0, 400.0, 700.0, true));
        para.set_horizontal_alignment(HorizontalAlignment::Center);
        para.set_first_line_indent(10.0);
        para.append_line("Centered heading text");

        let page = doc.pages().get(1).expect("page 1");
        let builder = TextBuilder::new(&page);
        builder.append_paragraph(&para);
        doc.save_to(&out).expect("save");
    }
    assert!(count_matches(&out, "Centered") >= 1);
    let _ = std::fs::remove_file(&out);
}

#[test]
fn paragraph_floating_box_frame_round_trips() {
    let out = common::unique_temp("aspose_rust_fbox.pdf");
    {
        let doc = Document::open(hello_world()).expect("open");
        let mut boxed = FloatingBox::with_dimensions(200.0, 120.0);
        boxed.set_border(BorderInfo::with_side_width(BorderSide::ALL, 1.5));
        assert_eq!(boxed.width(), 200.0);
        assert_eq!(boxed.height(), 120.0);

        let page = doc.pages().get(1).expect("page 1");
        page.paragraphs().add(Box::new(boxed));
        assert_eq!(page.paragraphs().count(), 1);
        doc.save_to(&out).expect("save");
    }
    // Reloads without error and the page survives.
    let doc = Document::open(&out).expect("reopen");
    assert_eq!(doc.pages().count(), 1);
    let _ = std::fs::remove_file(&out);
}

#[test]
fn paragraph_floating_box_properties() {
    let mut boxed = FloatingBox::new();
    assert_eq!(boxed.width(), 0.0);
    boxed.set_width(150.0);
    boxed.set_height(90.0);
    boxed.set_is_need_repeating(true);
    assert_eq!(boxed.width(), 150.0);
    assert_eq!(boxed.height(), 90.0);
    assert!(boxed.is_need_repeating());
    assert_eq!(boxed.paragraphs().count(), 0);
}

// A TextParagraph's rectangle is its wrap box: text longer than the box
// is broken across lines at whitespace, and each line stays inside it.
// The type is documented as word-wrapped, and laid out one run per
// appended line until this was pinned.
//
// Courier is the measuring stick: every glyph is exactly 600/1000 em, so
// at 12pt a 144-point box holds exactly 20 characters and the expected
// break points are arithmetic rather than a guess.
#[test]
fn paragraph_breaks_a_long_line_inside_its_box() {
    const SIZE: f64 = 12.0;
    const BOX_WIDTH: f64 = 144.0;
    let max_chars = (BOX_WIDTH / (SIZE * 0.6)) as usize; // 20
    let words = "Alpha bravo charlie delta echo foxtrot golf hotel india juliett kilo";

    let out = common::unique_temp("aspose_rust_wrap.pdf");
    {
        let doc = Document::open(hello_world()).expect("open");
        let mut para = TextParagraph::new();
        para.set_rectangle(Rectangle::new(72.0, 400.0, 72.0 + BOX_WIDTH, 700.0, true));
        let mut state = TextState::default();
        state.set_font_size(SIZE as f32);
        state.set_font(aspose_pdf_foss::FontRepository::find_font("Courier"));
        para.append_line_with_state(words, state);

        let page = doc.pages().get(1).expect("page 1");
        TextBuilder::new(&page).append_paragraph(&para);
        doc.save_to(&out).expect("save");
    }

    let saved = std::fs::read(&out).expect("read");
    let runs = rendered_runs(&saved);
    assert!(runs.len() > 1, "the line was not broken: {runs:?}");

    // Every word survives, in order, across the broken lines.
    assert_eq!(
        runs.join(" ").split_whitespace().collect::<Vec<_>>(),
        words.split_whitespace().collect::<Vec<_>>()
    );
    // And no line is wider than the box.
    for run in &runs {
        assert!(
            run.chars().count() <= max_chars,
            "line of {} chars overflows the {max_chars}-char box: {run:?}",
            run.chars().count()
        );
    }
}

/// The text of each `(...) Tj` run in a saved PDF, in file order.
fn rendered_runs(saved: &[u8]) -> Vec<String> {
    let text = String::from_utf8_lossy(saved);
    text.match_indices(" Td\n(")
        .filter_map(|(i, _)| {
            let rest = &text[i + 5..];
            rest.find(") Tj").map(|end| rest[..end].to_string())
        })
        .collect()
}

// A line that already fits is emitted unchanged — including its own
// spacing, which a re-flow would collapse.
#[test]
fn paragraph_leaves_a_fitting_line_verbatim() {
    let out = common::unique_temp("aspose_rust_nowrap.pdf");
    {
        let doc = Document::open(hello_world()).expect("open");
        let mut para = TextParagraph::new();
        para.set_rectangle(Rectangle::new(72.0, 400.0, 500.0, 700.0, true));
        para.append_line("Two  spaces  survive");

        let page = doc.pages().get(1).expect("page 1");
        TextBuilder::new(&page).append_paragraph(&para);
        doc.save_to(&out).expect("save");
    }
    let saved = std::fs::read(&out).expect("read");
    assert!(
        String::from_utf8_lossy(&saved).contains("(Two  spaces  survive) Tj"),
        "a fitting line must not be re-flowed"
    );
    let _ = std::fs::remove_file(&out);
}

// Without a box there is no width to measure against, so a long line
// stays one run rather than being broken at an arbitrary width.
#[test]
fn paragraph_without_a_box_does_not_wrap() {
    let out = common::unique_temp("aspose_rust_nobox.pdf");
    let long = "one two three four five six seven eight nine ten eleven twelve thirteen";
    {
        let doc = Document::open(hello_world()).expect("open");
        let mut para = TextParagraph::new();
        para.set_position(aspose_pdf_foss::Position::new(72.0, 700.0));
        para.append_line(long);

        let page = doc.pages().get(1).expect("page 1");
        TextBuilder::new(&page).append_paragraph(&para);
        doc.save_to(&out).expect("save");
    }
    let saved = std::fs::read(&out).expect("read");
    assert!(
        String::from_utf8_lossy(&saved).contains(&format!("({long}) Tj")),
        "an unboxed paragraph must stay one run"
    );
    let _ = std::fs::remove_file(&out);
}

#[path = "../common/mod.rs"]
mod common;

// ---- text colour ---------------------------------------------------
//
// A TextState's ForegroundColor was carried and never emitted: text set
// to red came out black, and the setter looked like it worked. The
// colour reaches text through the surfaces that take a whole TextState —
// a paragraph line and a table cell's default — since TextFragmentState
// catalogues only Font and FontSize.

fn red_state(size: f32) -> TextState {
    let mut state = TextState::default();
    state.set_font_size(size);
    state.set_foreground_color(aspose_pdf_foss::Color::from_argb_rgb(204, 0, 0));
    state
}

#[test]
fn a_paragraph_line_paints_in_its_colour() {
    let out = common::unique_temp("aspose_rust_colour_para.pdf");
    {
        let doc = Document::open(hello_world()).expect("open");
        let mut para = TextParagraph::new();
        para.set_rectangle(Rectangle::new(72.0, 500.0, 400.0, 700.0, true));
        para.append_line_with_state("Coloured line", red_state(12.0));

        let page = doc.pages().get(1).expect("page 1");
        TextBuilder::new(&page).append_paragraph(&para);
        doc.save_to(&out).expect("save");
    }
    let saved = std::fs::read(&out).expect("read");
    assert!(
        String::from_utf8_lossy(&saved).contains("0.8 0 0 rg"),
        "a paragraph line ignored its colour"
    );
    let _ = std::fs::remove_file(&out);
}

// Text with no colour set emits no colour operator at all, so it keeps
// painting in the viewer's default rather than being forced to black —
// and an untouched document still resaves byte-for-byte.
#[test]
fn uncoloured_text_emits_no_colour_operator() {
    let out = common::unique_temp("aspose_rust_nocolour.pdf");
    {
        let doc = Document::open(hello_world()).expect("open");
        let mut fragment = aspose_pdf_foss::TextFragment::with_text("Plain");
        fragment.set_position(aspose_pdf_foss::Position::new(72.0, 700.0));
        doc.pages().get(1).unwrap().paragraphs().add(Box::new(fragment));
        doc.save_to(&out).expect("save");
    }
    let saved = std::fs::read(&out).expect("read");
    let content = String::from_utf8_lossy(&saved).into_owned();
    assert!(content.contains("(Plain) Tj"), "the text is missing");
    assert!(!content.contains(" rg\nBT"), "an unset colour must not be emitted");
    let _ = std::fs::remove_file(&out);
}

// ---- the fragment state's inherited surface ------------------------
//
// TextFragmentState derives from TextState canonically, and every peer
// track ships the base surface on it through inheritance. Rust has none,
// so a fragment's state offered font and font_size and nothing else —
// a fragment could not be given a colour at all.

#[test]
fn a_fragment_state_carries_the_base_surface() {
    let mut fragment = aspose_pdf_foss::TextFragment::with_text("Styled");
    let state = fragment.text_state_mut();

    // Declared on the type itself (the canonical overrides).
    state.set_font_size(15.0);
    state.set_font(aspose_pdf_foss::FontRepository::find_font("Courier"));
    // Inherited from TextState.
    state.set_foreground_color(aspose_pdf_foss::Color::from_argb_rgb(204, 0, 0));
    state.set_underline(true);
    state.set_character_spacing(1.5);
    state.set_horizontal_alignment(HorizontalAlignment::Right);

    assert_eq!(state.font_size(), 15.0);
    assert_eq!(state.font().unwrap().base_font(), "Courier");
    assert_eq!(state.foreground_color().a(), 1.0);
    assert!(state.underline());
    assert_eq!(state.character_spacing(), 1.5);
    assert_eq!(state.horizontal_alignment(), HorizontalAlignment::Right);
}

// The colour set on a fragment's own state is the one it paints in —
// the inherited setter reaches the same state the render path reads.
#[test]
fn a_fragment_paints_in_the_colour_set_on_its_state() {
    let out = common::unique_temp("aspose_rust_frag_colour.pdf");
    {
        let doc = Document::open(hello_world()).expect("open");
        let mut fragment = aspose_pdf_foss::TextFragment::with_text("Red");
        fragment.text_state_mut().set_font_size(14.0);
        fragment
            .text_state_mut()
            .set_foreground_color(aspose_pdf_foss::Color::from_argb_rgb(204, 0, 0));
        fragment.set_position(aspose_pdf_foss::Position::new(72.0, 700.0));
        doc.pages().get(1).unwrap().paragraphs().add(Box::new(fragment));
        doc.save_to(&out).expect("save");
    }
    let saved = std::fs::read(&out).expect("read");
    assert!(
        String::from_utf8_lossy(&saved).contains("0.8 0 0 rg"),
        "the fragment's own colour did not reach the page"
    );
    let _ = std::fs::remove_file(&out);
}

// TextFragmentAbsorber derives from TextAbsorber canonically, so the
// accumulated text and the error flag belong to it too. Both had
// vanished — and `text()` must answer for real, not return an empty
// string because nothing fed the base.
#[test]
fn a_fragment_absorber_carries_the_absorbed_text() {
    let doc = Document::open(hello_world()).expect("open");
    let mut absorber = aspose_pdf_foss::TextFragmentAbsorber::with_phrase("Hello");
    absorber.visit_document(&doc).expect("visit");

    assert_eq!(absorber.text_fragments().count(), 1, "the search itself still works");
    // Inherited from TextAbsorber, and matching what a plain
    // TextAbsorber reports for the same document.
    let mut plain = aspose_pdf_foss::TextAbsorber::new();
    plain.visit_document(&doc).expect("visit");
    assert_eq!(absorber.text(), plain.text());
    assert!(!absorber.text().is_empty(), "the inherited text is empty");
    assert!(!absorber.has_errors());
}

// ---- character and word spacing ------------------------------------
//
// Tc and Tw were carried in the text state and never emitted, so a
// caller could ask for letter-spacing and get none. They are graphics
// state, so they have to be scoped like the fill colour — left loose,
// the spacing would apply to everything drawn after the run.

#[test]
fn a_run_emits_the_spacing_its_state_carries() {
    let out = common::unique_temp("aspose_rust_spacing.pdf");
    {
        let doc = Document::open(hello_world()).expect("open");
        let mut fragment = aspose_pdf_foss::TextFragment::with_text("wide load");
        fragment.text_state_mut().set_font_size(12.0);
        fragment.text_state_mut().set_character_spacing(2.5);
        fragment.text_state_mut().set_word_spacing(4.0);
        fragment.set_position(aspose_pdf_foss::Position::new(72.0, 700.0));
        doc.pages().get(1).unwrap().paragraphs().add(Box::new(fragment));
        doc.save_to(&out).expect("save");
    }
    let saved = std::fs::read(&out).expect("read");
    let content = String::from_utf8_lossy(&saved).into_owned();

    assert!(content.contains("2.5 Tc"), "no character spacing emitted: {content:?}");
    assert!(content.contains("4 Tw"), "no word spacing emitted: {content:?}");
    // Scoped, so it does not leak into later content.
    assert!(content.contains("q\nBT"), "the spacing is not scoped");
    assert!(content.contains("ET\nQ"), "the scope is not closed");
    let _ = std::fs::remove_file(&out);
}

#[test]
fn text_without_spacing_emits_no_spacing_operator() {
    let out = common::unique_temp("aspose_rust_nospacing.pdf");
    {
        let doc = Document::open(hello_world()).expect("open");
        let mut fragment = aspose_pdf_foss::TextFragment::with_text("plain");
        fragment.set_position(aspose_pdf_foss::Position::new(72.0, 700.0));
        doc.pages().get(1).unwrap().paragraphs().add(Box::new(fragment));
        doc.save_to(&out).expect("save");
    }
    let content = String::from_utf8_lossy(&std::fs::read(&out).expect("read")).into_owned();
    assert!(!content.contains(" Tc\n"), "an unset character spacing must not be emitted");
    assert!(!content.contains(" Tw\n"), "an unset word spacing must not be emitted");
    let _ = std::fs::remove_file(&out);
}

// The spacing is not decoration: it moves the glyphs. Rendered at a wide
// Tc, the same text must occupy more of the page than at none.
#[test]
fn character_spacing_widens_the_rendered_text() {
    fn ink_width(spacing: f32) -> usize {
        // A page of our own, so the measured ink is this run and nothing
        // else — the shared fixture already carries text of its own.
        let doc = Document::new();
        let page = doc.pages().add();
        {
            let mut fragment = aspose_pdf_foss::TextFragment::with_text("IIIIIIIIII");
            fragment.text_state_mut().set_font_size(12.0);
            if spacing != 0.0 {
                fragment.text_state_mut().set_character_spacing(spacing);
            }
            fragment.set_position(aspose_pdf_foss::Position::new(20.0, 400.0));
            page.paragraphs().add(Box::new(fragment));
        }
        let mut saved = Vec::new();
        doc.save_to_writer(&mut saved).expect("save");

        let reopened = Document::from_bytes(saved).expect("reopen");
        let mut png = Vec::new();
        <aspose_pdf_foss::PngDevice as aspose_pdf_foss::PageDevice>::process(
            &aspose_pdf_foss::PngDevice::new(),
            &reopened.pages().get(1).unwrap(),
            &mut png,
        )
        .expect("render");
        let decoded = png_decoder::decode(&png).expect("decode");

        let w = decoded.width as usize;
        let mut min_x = w;
        let mut max_x = 0usize;
        for (i, px) in decoded.pixels.chunks_exact(4).enumerate() {
            if px[0] < 128 && px[1] < 128 && px[2] < 128 && px[3] > 0 {
                let x = i % w;
                min_x = min_x.min(x);
                max_x = max_x.max(x);
            }
        }
        assert!(max_x >= min_x, "the page rendered no ink at all");
        max_x - min_x
    }

    let plain = ink_width(0.0);
    let spaced = ink_width(6.0);
    assert!(
        spaced > plain + 20,
        "spacing did not move the glyphs: {plain} -> {spaced}"
    );
}

#[path = "../../src/internal/flate.rs"]
mod flate;

#[path = "../../src/internal/png_decoder.rs"]
mod png_decoder;

#[test]
fn text_state_measures_a_string_against_its_font() {
    // Courier is the measuring stick: every glyph is exactly 600/1000
    // of an em, so five characters at 10 points are 5 × 0.6 × 10 = 30.
    let mut ts = TextState::default();
    ts.set_font(FontRepository::find_font("Courier"));
    ts.set_font_size(10.0);
    assert!((ts.measure_string("ABCDE") - 30.0).abs() < 1e-9);
    // Width scales with the size and with the character count.
    ts.set_font_size(20.0);
    assert!((ts.measure_string("ABCDE") - 60.0).abs() < 1e-9);
    assert!((ts.measure_string("") - 0.0).abs() < 1e-9);
}

#[test]
fn a_proportional_font_measures_differently_per_character() {
    // Helvetica is proportional, so an `i` is narrower than a `W` —
    // a fixed-pitch measurement would report them equal.
    let mut ts = TextState::default();
    ts.set_font(FontRepository::find_font("Helvetica"));
    ts.set_font_size(12.0);
    let narrow = ts.measure_string("iii");
    let wide = ts.measure_string("WWW");
    assert!(narrow < wide, "iii {narrow} should be narrower than WWW {wide}");
}

#[test]
fn a_state_without_a_font_measures_zero() {
    // Nothing to measure against — the canonical state has no font.
    let ts = TextState::default();
    assert_eq!(ts.measure_string("anything"), 0.0);
}

// ============================================================
// Which parts of a TextState reach the page
// ============================================================

/// Render page 1 after appending `text` at (100, 400) with `style`
/// applied to its state, and report how many pixels took ink.
fn inked_pixels_for(text: &str, style: impl FnOnce(&mut aspose_pdf_foss::TextFragmentState)) -> usize {
    use aspose_pdf_foss::{PageDevice, PngDevice, Position, Resolution, TextFragment};

    let doc = Document::open(hello_world()).unwrap();
    {
        let page = doc.pages().get(1).unwrap();
        let builder = TextBuilder::new(&page);
        let mut frag = TextFragment::with_text(text);
        frag.text_state_mut().set_font(FontRepository::find_font("Courier"));
        frag.text_state_mut().set_font_size(24.0);
        style(frag.text_state_mut());
        frag.set_position(Position::new(100.0, 400.0));
        builder.append_text(&frag);
    }
    let mut saved = Vec::new();
    doc.save_to_writer(&mut saved).unwrap();
    let reopened = Document::from_bytes(saved).unwrap();
    let page = reopened.pages().get(1).unwrap();
    let mut png = Vec::new();
    PngDevice::with_resolution(Resolution::new(72).unwrap())
        .process(&page, &mut png)
        .unwrap();
    // The page already carries "Hello World"; compare against a run of
    // the same text rather than against zero.
    png_ink(&png)
}

fn png_ink(png: &[u8]) -> usize {
    let decoded = png_decoder::decode(png).expect("decode");
    decoded
        .pixels
        .chunks_exact(4)
        .filter(|px| px[0] < 200 || px[1] < 200 || px[2] < 200)
        .count()
}

/// `TextState::invisible` is emitted as text-rendering mode 3, and this
/// crate's own rasteriser honours it: the glyphs are laid out and never
/// painted. Measured against the same run drawn normally, so the
/// page's pre-existing text cannot mask the difference.
#[test]
fn invisible_text_is_laid_out_and_not_painted() {
    let visible = inked_pixels_for("INVISIBLE", |_| {});
    let hidden = inked_pixels_for("INVISIBLE", |st| st.set_invisible(true));
    assert!(visible > hidden, "visible text should ink more than invisible");
    let baseline = inked_pixels_for("", |_| {});
    assert_eq!(hidden, baseline, "invisible text must add no ink at all");
}

/// `TextState::horizontal_scaling` is emitted as `Tz`, so the operator
/// has to be in the file. The rendered result is deliberately NOT
/// asserted: the shared renderer contract puts `Tz` out of scope for
/// v1, so this crate's rasteriser draws the run unscaled — which is
/// exactly what the accessor's rustdoc says.
#[test]
fn horizontal_scaling_is_written_as_tz() {
    use aspose_pdf_foss::{Position, TextFragment};

    let doc = Document::open(hello_world()).unwrap();
    {
        let page = doc.pages().get(1).unwrap();
        let builder = TextBuilder::new(&page);
        let mut frag = TextFragment::with_text("WIDE");
        frag.text_state_mut().set_font_size(20.0);
        frag.text_state_mut().set_horizontal_scaling(150.0);
        frag.set_position(Position::new(100.0, 300.0));
        builder.append_text(&frag);
    }
    let mut saved = Vec::new();
    doc.save_to_writer(&mut saved).unwrap();
    let text = String::from_utf8_lossy(&saved);
    assert!(text.contains("150 Tz"), "the Tz operator should carry the scaling");
    // 100 % is the default and must not be written.
    let plain = {
        let doc = Document::open(hello_world()).unwrap();
        {
            let page = doc.pages().get(1).unwrap();
            let mut frag = TextFragment::with_text("PLAIN");
            frag.text_state_mut().set_font_size(20.0);
            frag.set_position(Position::new(100.0, 300.0));
            TextBuilder::new(&page).append_text(&frag);
        }
        let mut out = Vec::new();
        doc.save_to_writer(&mut out).unwrap();
        out
    };
    assert!(
        !String::from_utf8_lossy(&plain).contains(" Tz"),
        "the default scaling should emit no operator"
    );
}

/// Every text attribute now reaches the page. This used to pin the
/// opposite — that the five below changed nothing — and it is the test
/// that caught the change when they were wired, which is what it was
/// for. It now pins the direction: a decorated run inks more than a
/// plain one.
#[test]
fn the_text_attributes_reach_the_page() {
    let plain = inked_pixels_for("SAMPLE", |_| {});
    let decorated = inked_pixels_for("SAMPLE", |st| {
        st.set_underline(true);
        st.set_strike_out(true);
        st.set_stroking_color(aspose_pdf_foss::Color::from_rgb(1.0, 0.0, 0.0));
    });
    assert!(
        decorated > plain,
        "a ruled, outlined run must ink more than a plain one: {decorated} vs {plain}"
    );
}

/// `TextState::underline` has no operator in the format, so the rule is
/// drawn: a filled rectangle the width of the run at the position the
/// Standard-14 metrics state. Courier makes the arithmetic exact —
/// every glyph is 600/1000 em — so the extra ink is predictable rather
/// than eyeballed.
#[test]
fn an_underline_is_drawn_under_the_run() {
    let plain = inked_pixels_for("UNDER", |_| {});
    let ruled = inked_pixels_for("UNDER", |st| st.set_underline(true));

    // 5 Courier glyphs at 24 pt: 5 × 0.6 × 24 = 72 pt wide, and the
    // rule is 24 × 0.050 = 1.2 pt thick. At 72 dpi that is a 72-px row
    // 1.2 px tall, which lands on two pixel rows — so between one and
    // three rows take ink depending on where the edges fall.
    let added = ruled - plain;
    assert!(
        (72..=216).contains(&added),
        "an underline under 5 Courier glyphs should add one to three 72-px rows, added {added}"
    );
}

/// The same rectangle, a quarter of the size above the baseline. Struck
/// through and underlined together ink more than either alone.
#[test]
fn a_strike_out_is_drawn_through_the_run() {
    let plain = inked_pixels_for("STRIKE", |_| {});
    let struck = inked_pixels_for("STRIKE", |st| st.set_strike_out(true));
    let both = inked_pixels_for("STRIKE", |st| {
        st.set_strike_out(true);
        st.set_underline(true);
    });

    assert!(struck > plain, "the rule has to add ink: {struck} vs {plain}");
    assert!(
        both > struck,
        "two rules ink more than one: {both} vs {struck}"
    );
    // The strike-out crosses the glyphs, so some of its area lands on
    // ink that was already there; the underline below them does not.
    assert!(
        struck - plain < both - struck + 20,
        "the crossing rule cannot add more than the clear one"
    );
}

/// An invisible run paints nothing at all — neither glyphs nor rules.
#[test]
fn an_invisible_run_draws_no_rules() {
    let baseline = inked_pixels_for("", |_| {});
    let hidden = inked_pixels_for("HIDDEN", |st| {
        st.set_invisible(true);
        st.set_underline(true);
        st.set_strike_out(true);
    });
    assert_eq!(hidden, baseline, "an invisible run must add no ink");
}

/// Subscript and superscript are written as a text rise (`Ts`, §9.3.7).
/// The rendered result is deliberately not asserted: like `Tz`, `Ts` is
/// out of scope for this crate's own rasteriser, which the accessor's
/// rustdoc says.
#[test]
fn a_baseline_shift_is_written_as_ts() {
    use aspose_pdf_foss::{Position, TextFragment};

    let written = |apply: fn(&mut aspose_pdf_foss::TextFragmentState)| {
        let doc = Document::open(hello_world()).unwrap();
        {
            let page = doc.pages().get(1).unwrap();
            let mut frag = TextFragment::with_text("SHIFT");
            frag.text_state_mut().set_font_size(24.0);
            apply(frag.text_state_mut());
            frag.set_position(Position::new(100.0, 300.0));
            TextBuilder::new(&page).append_text(&frag);
        }
        let mut out = Vec::new();
        doc.save_to_writer(&mut out).unwrap();
        String::from_utf8_lossy(&out).into_owned()
    };

    // A third of 24 pt up, a fifth of it down.
    let up = written(|st| st.set_superscript(true));
    assert!(up.contains("8 Ts"), "a superscript rises by a third of the size");
    let down = written(|st| st.set_subscript(true));
    assert!(down.contains("-4.8 Ts"), "a subscript falls by a fifth");
    // Superscript wins when both are asked for.
    let both = written(|st| {
        st.set_subscript(true);
        st.set_superscript(true);
    });
    assert!(both.contains("8 Ts"), "superscript wins over subscript");
    let plain = written(|_| {});
    assert!(!plain.contains("Ts"), "a run on the baseline writes none");
}

/// A stroking colour asks for the glyph outlines to be painted, which
/// only a stroking rendering mode does (§9.3.6). This subset ships no
/// rendering-mode member, so setting the colour selects fill-and-stroke
/// on its own — and an invisible run stays invisible.
#[test]
fn a_stroking_colour_selects_fill_and_stroke() {
    use aspose_pdf_foss::{Position, TextFragment};

    let written = |apply: fn(&mut aspose_pdf_foss::TextFragmentState)| {
        let doc = Document::open(hello_world()).unwrap();
        {
            let page = doc.pages().get(1).unwrap();
            let mut frag = TextFragment::with_text("OUTLINE");
            frag.text_state_mut().set_font_size(24.0);
            apply(frag.text_state_mut());
            frag.set_position(Position::new(100.0, 250.0));
            TextBuilder::new(&page).append_text(&frag);
        }
        let mut out = Vec::new();
        doc.save_to_writer(&mut out).unwrap();
        String::from_utf8_lossy(&out).into_owned()
    };

    let stroked = written(|st| st.set_stroking_color(Color::from_rgb(0.0, 1.0, 0.0)));
    assert!(stroked.contains("0 1 0 RG"), "the outline colour");
    assert!(stroked.contains("2 Tr"), "and the mode that paints it");

    let hidden = written(|st| {
        st.set_stroking_color(Color::from_rgb(0.0, 1.0, 0.0));
        st.set_invisible(true);
    });
    assert!(hidden.contains("3 Tr"), "invisible wins");
    assert!(!hidden.contains("2 Tr"), "and nothing else selects a mode");
}
