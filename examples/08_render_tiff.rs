// Rasterise every page of a PDF to a single multi-page TIFF via
// TiffDevice::process_document_range(document, from_page, to_page,
// sink).
use std::fs::File;
use std::process::ExitCode;

use aspose_pdf_foss::{Document, DocumentDevice, Resolution, TiffDevice};

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 3 {
        eprintln!("usage: 08_render_tiff <input.pdf> <output.tiff>");
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
    let page_count = doc.pages().count();

    let tiff = TiffDevice::with_resolution(Resolution::new(150).unwrap());
    let mut sink = match File::create(output) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("create {output}: {e}");
            return ExitCode::from(1);
        }
    };
    if let Err(e) = tiff.process_document_range(&doc, 1, page_count, &mut sink) {
        eprintln!("tiff.process_document_range: {e}");
        return ExitCode::from(1);
    }

    println!(
        "Rendered all {page_count} page(s) of {input} to a multi-page \
         TIFF at {output} (LZW @ 150 DPI)"
    );
    ExitCode::SUCCESS
}
