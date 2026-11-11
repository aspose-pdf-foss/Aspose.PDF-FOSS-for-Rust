// Public-API smoke driver — TextDevice.
//
// Coverage:
//   * Default ctor produces a device with UTF-8 encoding.
//   * with_encoding stores the supplied Encoding; set_encoding
//     round-trips it.
//   * process(page, sink) writes the extracted page text encoded
//     via the configured Encoding (UTF-8 default).
//   * process_to_path writes the same bytes to disk.
//   * Multi-page document: each Page resolves to its own extracted
//     text (verifies internal_leaf_index routing through the device).
//   * Encoding setter influences the bytes emitted by process —
//     UTF-8 vs Windows-1252 differ for codepoints outside ASCII.
//   * TextDevice usable through &dyn PageDevice + &dyn Device.

use std::path::{Path, PathBuf};

use aspose_pdf_foss::{Device, Document, Encoding, PageDevice, TextDevice};

fn fixture(rel: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(rel)
}

fn load_canonical(rel: &str) -> String {
    let raw = std::fs::read_to_string(fixture(rel)).expect("read canonical");
    raw.trim_end_matches('\n').to_string()
}

// ============================================================
// Surface — encoding accessor / mutator
// ============================================================

#[test]
fn default_ctor_uses_utf8() {
    let dev = TextDevice::new();
    assert_eq!(dev.encoding(), Encoding::utf8());
}

#[test]
fn default_trait_matches_explicit_new() {
    let dev: TextDevice = Default::default();
    assert_eq!(dev.encoding(), Encoding::utf8());
}

#[test]
fn with_encoding_stores_supplied_value() {
    let dev = TextDevice::with_encoding(Encoding::windows_1252());
    assert_eq!(dev.encoding(), Encoding::windows_1252());
}

#[test]
fn encoding_setter_round_trips() {
    let mut dev = TextDevice::new();
    dev.set_encoding(Encoding::latin1());
    assert_eq!(dev.encoding(), Encoding::latin1());
}

// ============================================================
// process(page, sink) — extracted text → encoded bytes
// ============================================================

#[test]
fn process_writes_utf8_page_text() {
    let doc = Document::open(fixture("text_extractor/two_lines.pdf"))
        .expect("open two_lines.pdf");
    let page = doc.pages().get(1).expect("page 1");
    let dev = TextDevice::new();

    let mut out: Vec<u8> = Vec::new();
    dev.process(&page, &mut out).expect("process");
    let text = String::from_utf8(out).expect("UTF-8 output");

    assert!(text.contains("Line one"));
    assert!(text.contains("Line two"));
}

#[test]
fn process_hello_world_matches_canonical() {
    let doc = Document::open(fixture("text_extractor/hello_world.pdf"))
        .expect("open hello_world.pdf");
    let page = doc.pages().get(1).expect("page 1");
    let dev = TextDevice::new();
    let expected = load_canonical("text_extractor/hello_world.canonical");

    let mut out: Vec<u8> = Vec::new();
    dev.process(&page, &mut out).expect("process");

    assert_eq!(out, expected.as_bytes());
}

#[test]
fn process_to_path_writes_same_bytes() {
    let doc = Document::open(fixture("text_extractor/hello_world.pdf"))
        .expect("open hello_world.pdf");
    let page = doc.pages().get(1).expect("page 1");
    let dev = TextDevice::new();

    let tmp = common::unique_temp(format!(
        "rust_text_device_{}.txt",
        std::process::id()
    ));
    let _cleanup = scopeguard_remove(&tmp);
    dev.process_to_path(&page, &tmp).expect("process_to_path");

    let on_disk = std::fs::read(&tmp).expect("read tmp");
    let mut in_memory: Vec<u8> = Vec::new();
    dev.process(&page, &mut in_memory).expect("process");
    assert_eq!(on_disk, in_memory);
    assert_eq!(String::from_utf8(on_disk).unwrap().trim(), "Hello World");
}

// ============================================================
// Multi-page routing
// ============================================================

#[test]
fn multi_page_each_page_resolves_to_own_text() {
    let doc = Document::open(fixture("text_extractor/two_pages.pdf"))
        .expect("open two_pages.pdf");
    let dev = TextDevice::new();

    let p1 = doc.pages().get(1).expect("page 1");
    let mut out1: Vec<u8> = Vec::new();
    dev.process(&p1, &mut out1).expect("process p1");
    let t1 = String::from_utf8(out1).unwrap();

    let p2 = doc.pages().get(2).expect("page 2");
    let mut out2: Vec<u8> = Vec::new();
    dev.process(&p2, &mut out2).expect("process p2");
    let t2 = String::from_utf8(out2).unwrap();

    assert!(t1.contains("Page one"));
    assert!(!t1.contains("Page two"));
    assert!(t2.contains("Page two"));
    assert!(!t2.contains("Page one"));
}

// ============================================================
// Encoding influences the emitted bytes
// ============================================================

#[test]
fn encoding_setter_changes_output_bytes() {
    let doc = Document::open(fixture("text_extractor/encoding_winansi.pdf"))
        .expect("open encoding_winansi.pdf");
    let page = doc.pages().get(1).expect("page 1");

    let mut utf8_dev = TextDevice::new();
    let mut utf8_out: Vec<u8> = Vec::new();
    utf8_dev.process(&page, &mut utf8_out).expect("process utf8");

    utf8_dev.set_encoding(Encoding::windows_1252());
    let mut cp1252_out: Vec<u8> = Vec::new();
    utf8_dev.process(&page, &mut cp1252_out).expect("process cp1252");

    // The fixture exercises Windows-1252 high-half codepoints
    // (e.g. \u{20AC} = €). UTF-8 emits the 3-byte sequence
    // 0xE2 0x82 0xAC; cp1252 emits the single byte 0x80. The two
    // byte streams must therefore disagree on a fixture that
    // covers any non-ASCII character.
    assert_ne!(utf8_out, cp1252_out);
}

// ============================================================
// Trait-object dispatch — Device / PageDevice
// ============================================================

#[test]
fn dispatch_through_page_device_trait_object() {
    let doc = Document::open(fixture("text_extractor/hello_world.pdf"))
        .expect("open hello_world.pdf");
    let page = doc.pages().get(1).expect("page 1");
    let dev = TextDevice::new();

    let dyn_dev: &dyn PageDevice = &dev;
    let mut out: Vec<u8> = Vec::new();
    dyn_dev.process(&page, &mut out).expect("process via trait");
    assert!(String::from_utf8(out).unwrap().contains("Hello World"));
}

#[test]
fn text_device_is_a_device_marker() {
    let dev = TextDevice::new();
    let _marker: &dyn Device = &dev;
}

// ------------------------------------------------------------
// Tiny RAII helper — wipes the temp file after the test. Avoids a
// scopeguard dep; the lib is std-only by strategy lock.
// ------------------------------------------------------------

struct RemoveOnDrop(PathBuf);
impl Drop for RemoveOnDrop {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}
fn scopeguard_remove(p: &Path) -> RemoveOnDrop {
    RemoveOnDrop(p.to_path_buf())
}

#[path = "../common/mod.rs"]
mod common;
