// Public-API driver — FontFile2 embedding of an opened TrueType font.
//
// A font opened via FontRepository.open_font carries its program and is
// embedded by default. Drawing text with it through TextBuilder embeds a
// /TrueType font with a /FontDescriptor + /FontFile2 program stream on
// save; a name-resolved font stays a non-embedded base /Type1.

use aspose_pdf_foss::{Document, FontRepository, Position, TextBuilder, TextFragment};
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

#[test]
fn opened_font_embeds_fontfile2_on_save() {
    let program = std::fs::read(liberation_sans()).unwrap();

    let doc = Document::open(hello_world()).unwrap();
    {
        let page = doc.pages().get(1).unwrap();
        let builder = TextBuilder::new(&page);
        let mut frag = TextFragment::with_text("Embedded");
        frag.text_state_mut().set_font(FontRepository::open_font(liberation_sans()).unwrap());
        frag.text_state_mut().set_font_size(18.0);
        frag.set_position(Position::new(100.0, 400.0));
        builder.append_text(&frag);
    }
    let mut out = Vec::new();
    doc.save_to_writer(&mut out).unwrap();

    // The embedded font dict + descriptor + program stream are written.
    for marker in [
        &b"/TrueType"[..],
        b"/FontDescriptor",
        b"/FontFile2",
        b"/WinAnsiEncoding",
        b"/Widths",
    ] {
        assert!(contains(&out, marker), "missing {:?}", std::str::from_utf8(marker).unwrap());
    }
    // The TrueType program is embedded verbatim (check its sfnt header
    // and a distinctive interior slice).
    assert!(contains(&out, &program[..16]), "font program header not embedded");
    assert!(contains(&out, &program[256..320]), "font program body not embedded");
    // /Length1 records the uncompressed program length.
    assert!(
        contains(&out, format!("/Length1 {}", program.len()).as_bytes()),
        "missing /Length1 with the program length"
    );
}

#[test]
fn name_resolved_font_stays_non_embedded() {
    let doc = Document::open(hello_world()).unwrap();
    {
        let page = doc.pages().get(1).unwrap();
        let builder = TextBuilder::new(&page);
        let mut frag = TextFragment::with_text("Helv");
        frag.text_state_mut().set_font(FontRepository::find_font("Helvetica"));
        frag.set_position(Position::new(100.0, 360.0));
        builder.append_text(&frag);
    }
    let mut out = Vec::new();
    doc.save_to_writer(&mut out).unwrap();

    assert!(contains(&out, b"/Type1"), "name-resolved font should be a base /Type1");
    assert!(!contains(&out, b"/FontFile2"), "name-resolved font must not embed a program");
}
