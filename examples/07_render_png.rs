// Rasterise a PDF page to PNG via PngDevice::process(page, sink).
use std::fs::File;
use std::process::ExitCode;

use aspose_pdf_foss::{Document, PageDevice, PngDevice, Resolution};

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 4 {
        eprintln!(
            "usage: 07_render_png <input.pdf> <page_number> <output.png>"
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
        "Rendered page {page_number} of {input} to {output} (PNG @ 150 DPI)"
    );
    ExitCode::SUCCESS
}
