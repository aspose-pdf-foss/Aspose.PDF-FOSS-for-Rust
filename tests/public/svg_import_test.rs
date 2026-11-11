//! SVG cluster: Document::open_svg + SvgLoadOptions.

use aspose_pdf_foss::{Document, SvgLoadOptions};
use std::io::Write;

fn write_temp_svg(name: &str, body: &str) -> std::path::PathBuf {
    let path = common::unique_temp(name);
    let mut f = std::fs::File::create(&path).unwrap();
    f.write_all(body.as_bytes()).unwrap();
    path
}

#[test]
fn imports_svg_with_width_height() {
    let svg = "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"200\" height=\"150\">\
        <rect x=\"10\" y=\"10\" width=\"80\" height=\"40\" fill=\"#ff0000\"/>\
        <circle cx=\"150\" cy=\"100\" r=\"30\" fill=\"blue\"/>\
        <line x1=\"0\" y1=\"0\" x2=\"200\" y2=\"150\" stroke=\"black\"/>\
        </svg>";
    let path = write_temp_svg("asp_rust_in.svg", svg);

    let doc = Document::open_svg(&path, &SvgLoadOptions::new()).unwrap();
    assert_eq!(doc.pages().count(), 1);
    let page = doc.pages().get(1).unwrap();
    assert!((page.width() - 200.0).abs() < 1.0);
    assert!((page.height() - 150.0).abs() < 1.0);

    // The produced PDF re-opens cleanly with one page.
    let mut out = Vec::new();
    doc.save_to_writer(&mut out).unwrap();
    let reopened = Document::from_bytes(out).unwrap();
    assert_eq!(reopened.pages().count(), 1);

    let _ = std::fs::remove_file(&path);
}

#[path = "../common/mod.rs"]
mod common;
