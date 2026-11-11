// Add /Info metadata and save via incremental update.
use std::process::ExitCode;

use aspose_pdf_foss::Document;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 3 {
        eprintln!("usage: 03_write_metadata <input.pdf> <output.pdf>");
        return ExitCode::from(2);
    }
    let input = &args[1];
    let output = &args[2];

    let mut doc = match Document::open(input) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("open {input}: {e}");
            return ExitCode::from(1);
        }
    };

    doc.set_title("Hello from Aspose.Pdf Rust FOSS");

    let info = doc.info_mut();
    info.set_author("Demo author");
    info.set_creator("Aspose.Pdf FOSS Rust v0");
    info.set_subject("DocumentInfo round-trip demo");
    if let Err(e) = info.add("Custom-Key", "Custom value via add()") {
        eprintln!("info.add: {e}");
        return ExitCode::from(1);
    }

    if let Err(e) = doc.save_to(output) {
        eprintln!("save {output}: {e}");
        return ExitCode::from(1);
    }

    println!(
        "Wrote {output} with /Info edits flushed via incremental update."
    );
    ExitCode::SUCCESS
}
