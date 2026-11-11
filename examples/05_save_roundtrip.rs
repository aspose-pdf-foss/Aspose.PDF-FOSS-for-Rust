// Open + save with no edits — should byte-roundtrip exactly.
use std::fs;
use std::process::ExitCode;

use aspose_pdf_foss::Document;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 3 {
        eprintln!("usage: 05_save_roundtrip <input.pdf> <output.pdf>");
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
    if let Err(e) = doc.save_to(output) {
        eprintln!("save {output}: {e}");
        return ExitCode::from(1);
    }

    let in_bytes = fs::read(input).expect("read input");
    let out_bytes = fs::read(output).expect("read output");

    println!("input:  {} bytes", in_bytes.len());
    println!("output: {} bytes", out_bytes.len());
    println!(
        "byte-identical: {}",
        if in_bytes == out_bytes { "yes" } else { "no" },
    );
    ExitCode::SUCCESS
}
