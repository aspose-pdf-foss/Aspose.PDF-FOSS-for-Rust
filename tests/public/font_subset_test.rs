// Public-API driver — Font.is_subset (E6). When an opened TrueType
// font is embedded with subsetting on, only the used glyph outlines are
// kept, so the embedded program (and the saved file) is much smaller
// than a full embed, while the text still extracts and the font's
// /BaseFont carries the canonical 6-letter '+' subset tag.

use aspose_pdf_foss::{
    Document, FontRepository, Position, TextAbsorber, TextBuilder, TextFragment,
};
use std::path::{Path, PathBuf};

fn hello_world() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/text_extractor/hello_world.pdf")
}

fn liberation_sans() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("src/internal/standard14_outlines.fonts/LiberationSans-Regular.ttf")
}

fn contains(haystack: &[u8], needle: &[u8]) -> bool {
    haystack.windows(needle.len()).any(|w| w == needle)
}

/// Draw "Hi" at (100, 500), 14pt, with the opened font (optionally
/// subset) and return the saved bytes — mirrors the C++ `DrawAndSave`.
fn draw_and_save(subset: bool) -> Vec<u8> {
    let doc = Document::open(hello_world()).unwrap();
    {
        let page = doc.pages().get(1).unwrap();
        let builder = TextBuilder::new(&page);
        let mut frag = TextFragment::with_text("Hi");
        let mut font = FontRepository::open_font(liberation_sans()).unwrap();
        font.set_is_subset(subset);
        frag.text_state_mut().set_font(font);
        frag.text_state_mut().set_font_size(14.0);
        frag.set_position(Position::new(100.0, 500.0));
        builder.append_text(&frag);
    }
    let mut out = Vec::new();
    doc.save_to_writer(&mut out).unwrap();
    out
}

#[test]
fn subset_shrinks_embed() {
    let full = draw_and_save(false);
    let sub = draw_and_save(true);

    // The subset embed produces a strictly smaller file.
    assert!(
        sub.len() < full.len(),
        "subset {} not smaller than full {}",
        sub.len(),
        full.len()
    );
    // The subset BaseFont carries the canonical 6-letter '+' tag, and a
    // /FontFile2 program is still embedded.
    assert!(contains(&sub, b"+"), "missing subset '+' tag");
    assert!(contains(&sub, b"FontFile2"), "missing /FontFile2");
}

#[test]
fn subset_text_still_extracts() {
    let out = draw_and_save(true);
    let doc = Document::from_bytes(out).unwrap();
    let mut absorber = TextAbsorber::new();
    absorber.visit_document(&doc).unwrap();
    assert!(absorber.text().contains("Hi"), "'Hi' not extracted: {:?}", absorber.text());
}
