// Render a text-bearing PDF to PNG end-to-end. Demonstrates that
// PngDevice::process raster path handles glyphs from an embedded
// TrueType font, not just rectangles / vector primitives.
//
// pdfs/text_hello.pdf carries a 442-byte FontFile2 stream that
// supplies the glyph outlines. PDFs that reference Standard14 fonts
// (Helvetica, Times-Roman, …) WITHOUT embedding fonts work too —
// see example 11.
use std::fs::File;
use std::process::ExitCode;

use aspose_pdf_foss::{Document, PageDevice, PngDevice, Resolution};

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 3 {
        eprintln!("usage: 10_render_text_pdf <input.pdf> <output.png>");
        return ExitCode::from(2);
    }
    let input = &args[1];
    let output = &args[2];

    let doc = match Document::open(input) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("open {input}: {e}");
            return ExitCode::from(1);
        }
    };
    let page = match doc.pages().get(1) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("pages.get(1): {e}");
            return ExitCode::from(1);
        }
    };

    let png = PngDevice::with_resolution(Resolution::new(150).unwrap());
    let mut sink = match File::create(output) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("create {output}: {e}");
            return ExitCode::from(1);
        }
    };
    if let Err(e) = png.process(&page, &mut sink) {
        eprintln!("png.process: {e}");
        return ExitCode::from(1);
    }

    println!(
        "Rendered text page from {input} to {output} (PNG @ 150 DPI). \
         Glyphs come from the PDF's embedded font; the image should \
         show painted text, not a blank page."
    );
    ExitCode::SUCCESS
}
