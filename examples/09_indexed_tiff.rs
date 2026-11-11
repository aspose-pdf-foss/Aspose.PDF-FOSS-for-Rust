// Render a PDF page to an 8-bit palettised TIFF (256-colour
// palette, median-cut quantisation via foundation/palette_quantiser).
//
// For caller-pluggable palette quantisation, implement
// IIndexBitmapConverter and pass it to one of the TiffDevice
// ctors that accept IIndexBitmapConverter — see the lib's
// IIndexBitmapConverter tests for a custom-converter example.
use std::fs::File;
use std::process::ExitCode;

use aspose_pdf_foss::{
    ColorDepth, Document, PageDevice, Resolution, TiffDevice, TiffSettings,
};

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 4 {
        eprintln!(
            "usage: 09_indexed_tiff <input.pdf> <page_number> <output.tiff>"
        );
        return ExitCode::from(2);
    }
    let input = &args[1];
    let page_number: usize = match args[2].parse() {
        Ok(n) if n >= 1 => n,
        _ => {
            eprintln!("page_number must be a positive integer");
            return ExitCode::from(2);
        }
    };
    let output = &args[3];

    let doc = match Document::open(input) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("open {input}: {e}");
            return ExitCode::from(1);
        }
    };
    let page = match doc.pages().get(page_number) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("pages.get({page_number}): {e}");
            return ExitCode::from(1);
        }
    };

    let mut settings = TiffSettings::new();
    settings.set_depth(ColorDepth::Format8bpp);

    let tiff = TiffDevice::with_resolution_and_settings(
        Resolution::new(150).unwrap(),
        settings,
    );
    let mut sink = match File::create(output) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("create {output}: {e}");
            return ExitCode::from(1);
        }
    };
    if let Err(e) = tiff.process(&page, &mut sink) {
        eprintln!("tiff.process: {e}");
        return ExitCode::from(1);
    }

    println!(
        "Rendered page {page_number} of {input} to {output} as 8bpp \
         palettised TIFF (256-colour palette via median-cut, LZW @ 150 DPI)"
    );
    ExitCode::SUCCESS
}
