// Open a PDF, print the page count, iterate through pages.
use std::process::ExitCode;

use aspose_pdf_foss::Document;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 2 {
        eprintln!("usage: 01_pages <input.pdf>");
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
    let pages = doc.pages();
    println!("File:  {input}");
    println!("Pages: {}", pages.count());
    for i in 1..=pages.count() {
        let page = pages.get(i).expect("1..=count is in-bounds");
        println!("  page {i} (Number={})", page.number());
    }
    ExitCode::SUCCESS
}
