// Extract all text from a document via TextAbsorber.
use std::process::ExitCode;

use aspose_pdf_foss::{Document, TextAbsorber};

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 2 {
        eprintln!("usage: 04_text_extraction <input.pdf>");
        return ExitCode::from(2);
    }
    let input = &args[1];

    let doc = match Document::open(input) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("open {input}: {e}");
            return ExitCode::from(1);
        }
    };
    let mut absorber = TextAbsorber::new();
    if let Err(e) = absorber.visit_document(&doc) {
        eprintln!("visit_document: {e}");
        return ExitCode::from(1);
    }

    println!("--- text from {input} ---");
    println!("{}", absorber.text());
    println!("--- (HasErrors={}) ---", absorber.has_errors());
    ExitCode::SUCCESS
}
