// Render a PDF that references a Standard14 font WITHOUT
// embedding it. This is the most common shape of real-world PDFs:
// Office "Save as PDF" with default Calibri/Arial, older
// scientific papers with Times-Roman, etc.
//
// PDF 32000-1:2008 §9.6.2.2 says the 14 standard PostScript fonts
// (Helvetica / Times-Roman / Courier × 4 styles each, plus Symbol +
// ZapfDingbats) are ALWAYS available to a conforming reader without
// being embedded. The bundled fixture hello_world.pdf carries
// exactly this shape — its font dict is just
// `<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>`, no
// /FontDescriptor, no /FontFile, no embedded glyph data.
//
// The missing outlines come from a platform font where one is found,
// and otherwise from the bundled Liberation Sans / Serif / Mono faces
// (12 TTFs, 4.2 MB, SIL OFL 1.1, compiled into the lib). Any simple
// font with no embedded program is mapped onto the nearest of those
// faces by family, weight and slope, so a document's /Arial or
// /Verdana renders too — not only the fourteen names themselves.
//
// Symbol and ZapfDingbats are the exception, and not a pending one:
// there is no metric class for a symbol font, so no text face is the
// nearest one, and painting their codes with a text face would draw
// the wrong glyphs. The two are left unpainted instead.
use std::fs::File;
use std::process::ExitCode;

use aspose_pdf_foss::{Document, PageDevice, PngDevice, Resolution};

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 3 {
        eprintln!("usage: 11_render_standard14 <input.pdf> <output.png>");
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
        "Rendered {input} (Standard14 reference, no embedded font) \
         to {output} (PNG @ 150 DPI). Glyphs come from the bundled \
         Liberation substitute via internal/standard14_outlines."
    );
    ExitCode::SUCCESS
}
