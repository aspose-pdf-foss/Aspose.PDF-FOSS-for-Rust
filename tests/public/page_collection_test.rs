// Public-API smoke driver — PageCollection accessors.
//
// Walks every fixture in tests/fixtures/pages_tree/ and asserts
// page count + per-page geometry (number, width, height,
// rotation) against the canonical sidecars used by the
// foundation pages_tree freeze gate. Also exercises the
// out-of-range error path on PageCollection::get.

use std::path::PathBuf;

use aspose_pdf_foss::{Document, Error};

fn fixture(rel: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(rel)
}

#[derive(Debug)]
struct Expected {
    number: u64,
    width: f64,
    height: f64,
    rotation: i32,
}

fn parse_expected(rel: &str) -> Vec<Expected> {
    // Sidecar shape: one line per leaf, "page N W H R\n" where
    // W / H are decimal, R is one of 0 / 90 / 180 / 270.
    let path = fixture(rel);
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    let mut out = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let parts: Vec<&str> = line.split_whitespace().collect();
        assert_eq!(parts.len(), 5, "bad sidecar line {line:?}");
        assert_eq!(parts[0], "page");
        out.push(Expected {
            number: parts[1].parse().expect("page number"),
            width: parts[2].parse().expect("page width"),
            height: parts[3].parse().expect("page height"),
            rotation: parts[4].parse().expect("page rotation"),
        });
    }
    out
}

#[test]
fn page_geometry_matches_sidecars() {
    let cases = [
        ("pages_tree/single.pdf", "pages_tree/single.canonical"),
        ("pages_tree/three_sizes.pdf", "pages_tree/three_sizes.canonical"),
        ("pages_tree/nested_tree.pdf", "pages_tree/nested_tree.canonical"),
    ];
    for (pdf, sidecar) in cases {
        let doc = Document::open(fixture(pdf))
            .unwrap_or_else(|e| panic!("open {pdf}: {e}"));
        let expected = parse_expected(sidecar);
        assert_eq!(
            doc.pages().count(),
            expected.len(),
            "count mismatch for {pdf}"
        );
        for (i, exp) in expected.iter().enumerate() {
            let page = doc
                .pages()
                .get(i + 1)
                .unwrap_or_else(|e| panic!("get({}) on {pdf}: {e}", i + 1));
            assert_eq!(page.number(), exp.number, "{pdf} page {i}: number");
            assert!(
                (page.width() - exp.width).abs() < 1e-6,
                "{pdf} page {i}: width {} vs {}",
                page.width(),
                exp.width
            );
            assert!(
                (page.height() - exp.height).abs() < 1e-6,
                "{pdf} page {i}: height {} vs {}",
                page.height(),
                exp.height
            );
            assert_eq!(
                page.rotation(),
                exp.rotation,
                "{pdf} page {i}: rotation"
            );
        }
    }
}

#[test]
fn get_zero_index_rejects() {
    let doc = Document::open(fixture("pages_tree/single.pdf")).unwrap();
    match doc.pages().get(0) {
        Err(Error::PageIndexOutOfRange { index: 0, count: 1 }) => {}
        other => panic!("expected PageIndexOutOfRange(0, 1), got {other:?}"),
    }
}

#[test]
fn get_past_end_rejects() {
    let doc = Document::open(fixture("pages_tree/three_sizes.pdf")).unwrap();
    match doc.pages().get(4) {
        Err(Error::PageIndexOutOfRange { index: 4, count: 3 }) => {}
        other => panic!("expected PageIndexOutOfRange(4, 3), got {other:?}"),
    }
}

#[test]
fn iter_yields_pages_in_order() {
    let doc = Document::open(fixture("pages_tree/three_sizes.pdf")).unwrap();
    let numbers: Vec<u64> = doc.pages().iter().map(|p| p.number()).collect();
    assert_eq!(numbers, vec![1, 2, 3]);
}
