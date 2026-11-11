// Read DocumentInfo (PDF /Info dictionary).
use std::process::ExitCode;

use aspose_pdf_foss::Document;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 2 {
        eprintln!("usage: 02_read_metadata <input.pdf>");
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
    let info = doc.info();
    let show = |label: &str, value: &str| {
        let v = if value.is_empty() { "<empty>" } else { value };
        println!("  {label}: {v}");
    };

    println!("Metadata of {input}:");
    show("Title", info.title());
    show("Author", info.author());
    show("Creator", info.creator());
    show("Producer", info.producer());
    show("Subject", info.subject());
    show("Keywords", info.keywords());
    ExitCode::SUCCESS
}
