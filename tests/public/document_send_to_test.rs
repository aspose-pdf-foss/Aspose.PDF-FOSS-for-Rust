// Public-API smoke driver — Document::send_to family +
// convert_page_to_png (Phase 4 Beat 15).
//
// Verifies the SendTo cluster on Document delegates correctly to
// the supplied DocumentDevice trait object — both in-memory
// (Stream → `&mut dyn Write`) and file-path overloads — and that
// convert_page_to_png produces a valid PNG byte stream through a
// default PngDevice. Single-page (red_rect) and multi-page
// (three_sizes via the pages_tree corpus) fixtures cover the
// full-document vs page-range branches.
//
// Delegation is verified two ways:
//   * end-to-end through the real TiffDevice, asserting the same
//     output bytes as a direct DocumentDevice::process_document
//     call — proves SendTo is a pure forward;
//   * via a stub DocumentDevice that records the receiver,
//     from_page, to_page, and writes a known marker — proves the
//     dyn-dispatch wire-up + path-vs-stream split.

use std::cell::Cell;
use std::path::PathBuf;

use aspose_pdf_foss::{
    Device, Document, DocumentDevice, Error, PageDevice, TiffDevice,
};

fn fixture(rel: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(rel)
}

// Little-endian baseline TIFF header — "II" + 42 LE.
const TIFF_LE_HEADER: [u8; 4] = [b'I', b'I', 0x2A, 0x00];
// PNG signature — first 8 bytes of any PNG datastream.
const PNG_SIGNATURE: [u8; 8] =
    [0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];

// ============================================================
// Stub DocumentDevice — records dispatch args + emits a marker
// ============================================================

#[derive(Default)]
struct StubDocDevice {
    range_calls: Cell<u32>,
    last_from: Cell<usize>,
    last_to: Cell<usize>,
}

impl Device for StubDocDevice {}

impl PageDevice for StubDocDevice {
    fn process(
        &self,
        _page: &aspose_pdf_foss::Page,
        _output: &mut dyn std::io::Write,
    ) -> Result<(), Error> {
        Ok(())
    }
}

impl DocumentDevice for StubDocDevice {
    fn process_document_range(
        &self,
        _document: &Document,
        from_page: usize,
        to_page: usize,
        output: &mut dyn std::io::Write,
    ) -> Result<(), Error> {
        self.range_calls.set(self.range_calls.get() + 1);
        self.last_from.set(from_page);
        self.last_to.set(to_page);
        output.write_all(b"STUB")?;
        Ok(())
    }
}

// ============================================================
// End-to-end SendTo through TiffDevice
// ============================================================

#[test]
fn send_to_stream_matches_process_document() {
    let doc = Document::open(fixture("page_renderer/red_rect.pdf"))
        .expect("open red_rect.pdf");

    let device = TiffDevice::new();

    let mut direct: Vec<u8> = Vec::new();
    device
        .process_document(&doc, &mut direct)
        .expect("process_document");

    let mut via_send_to: Vec<u8> = Vec::new();
    doc.send_to(&device, &mut via_send_to).expect("send_to");

    assert_eq!(&via_send_to[..4], &TIFF_LE_HEADER);
    assert_eq!(
        via_send_to, direct,
        "Document::send_to must produce identical bytes to a \
         direct DocumentDevice::process_document call",
    );
}

#[test]
fn send_to_range_stream_matches_process_document_range() {
    let doc = Document::open(fixture("pages_tree/three_sizes.pdf"))
        .expect("open three_sizes.pdf");

    let device = TiffDevice::new();

    let mut direct: Vec<u8> = Vec::new();
    device
        .process_document_range(&doc, 1, 2, &mut direct)
        .expect("process_document_range");

    let mut via_send_to: Vec<u8> = Vec::new();
    doc.send_to_range(&device, 1, 2, &mut via_send_to)
        .expect("send_to_range");

    assert_eq!(&via_send_to[..4], &TIFF_LE_HEADER);
    assert_eq!(
        via_send_to, direct,
        "Document::send_to_range must produce identical bytes to \
         a direct DocumentDevice::process_document_range call",
    );
}

#[test]
fn send_to_path_writes_file_with_full_document_bytes() {
    let doc = Document::open(fixture("page_renderer/red_rect.pdf"))
        .expect("open red_rect.pdf");

    let device = TiffDevice::new();

    let tmp = common::unique_temp("aspose_pdf_foss_send_to_path_test.tif");
    if tmp.exists() {
        std::fs::remove_file(&tmp).ok();
    }

    doc.send_to_path(&device, &tmp).expect("send_to_path");

    let bytes = std::fs::read(&tmp).expect("read written TIFF");
    assert_eq!(&bytes[..4], &TIFF_LE_HEADER);

    // Same bytes as the stream variant.
    let mut via_stream: Vec<u8> = Vec::new();
    doc.send_to(&device, &mut via_stream).expect("send_to");
    assert_eq!(bytes, via_stream);

    std::fs::remove_file(&tmp).ok();
}

#[test]
fn send_to_range_to_path_writes_file_with_range_bytes() {
    let doc = Document::open(fixture("pages_tree/three_sizes.pdf"))
        .expect("open three_sizes.pdf");

    let device = TiffDevice::new();

    let tmp = common::unique_temp("aspose_pdf_foss_send_to_range_to_path_test.tif");
    if tmp.exists() {
        std::fs::remove_file(&tmp).ok();
    }

    doc.send_to_range_to_path(&device, 2, 3, &tmp)
        .expect("send_to_range_to_path");

    let bytes = std::fs::read(&tmp).expect("read written TIFF");
    assert_eq!(&bytes[..4], &TIFF_LE_HEADER);

    let mut via_stream: Vec<u8> = Vec::new();
    doc.send_to_range(&device, 2, 3, &mut via_stream)
        .expect("send_to_range");
    assert_eq!(bytes, via_stream);

    std::fs::remove_file(&tmp).ok();
}

// ============================================================
// Stub-based dispatch verification
// ============================================================

#[test]
fn send_to_dispatches_through_dyn_to_process_document() {
    let doc = Document::open(fixture("pages_tree/three_sizes.pdf"))
        .expect("open three_sizes.pdf");
    let stub = StubDocDevice::default();

    let mut out: Vec<u8> = Vec::new();
    doc.send_to(&stub, &mut out).expect("send_to");

    // process_document defaults to [1, pages().count()].
    assert_eq!(stub.range_calls.get(), 1);
    assert_eq!(stub.last_from.get(), 1);
    assert_eq!(stub.last_to.get(), doc.pages().count());
    assert_eq!(&out, b"STUB");
}

#[test]
fn send_to_range_passes_explicit_page_range_to_device() {
    let doc = Document::open(fixture("pages_tree/three_sizes.pdf"))
        .expect("open three_sizes.pdf");
    let stub = StubDocDevice::default();

    let mut out: Vec<u8> = Vec::new();
    doc.send_to_range(&stub, 2, 3, &mut out)
        .expect("send_to_range");

    assert_eq!(stub.range_calls.get(), 1);
    assert_eq!(stub.last_from.get(), 2);
    assert_eq!(stub.last_to.get(), 3);
    assert_eq!(&out, b"STUB");
}

#[test]
fn send_to_propagates_device_errors() {
    // TiffDevice rejects from_page=0 (1-based convention). Verify
    // the error surface bubbles through send_to_range unchanged.
    let doc = Document::open(fixture("page_renderer/red_rect.pdf"))
        .expect("open red_rect.pdf");
    let device = TiffDevice::new();
    let mut out: Vec<u8> = Vec::new();
    let err = doc
        .send_to_range(&device, 0, doc.pages().count(), &mut out)
        .expect_err("from_page=0 must be rejected");
    // Error variant intentionally unchecked — any device-level
    // rejection counts; the assertion is that send_to_range
    // surfaced an error rather than swallowing it.
    let _ = err;
}

// ============================================================
// ConvertPageToPNGMemoryStream
// ============================================================

#[test]
fn convert_page_to_png_emits_valid_png_signature() {
    let doc = Document::open(fixture("page_renderer/red_rect.pdf"))
        .expect("open red_rect.pdf");
    let page = doc.pages().get(1).expect("page 1");

    let png = doc.convert_page_to_png(&page).expect("convert_page_to_png");

    assert!(png.len() > 8, "png buffer too short");
    assert_eq!(
        &png[..8],
        &PNG_SIGNATURE,
        "convert_page_to_png must emit a valid PNG datastream",
    );
}

#[test]
fn convert_page_to_png_matches_default_png_device_output() {
    // The canonical contract: "Equivalent to constructing a
    // default PngDevice and calling Process into a memory
    // buffer." Verify byte-identity.
    use aspose_pdf_foss::PngDevice;

    let doc = Document::open(fixture("page_renderer/red_rect.pdf"))
        .expect("open red_rect.pdf");
    let page = doc.pages().get(1).expect("page 1");

    let via_doc = doc
        .convert_page_to_png(&page)
        .expect("convert_page_to_png");

    let mut via_device: Vec<u8> = Vec::new();
    PngDevice::new()
        .process(&page, &mut via_device)
        .expect("PngDevice::process");

    assert_eq!(
        via_doc, via_device,
        "convert_page_to_png must equal default PngDevice output",
    );
}

#[path = "../common/mod.rs"]
mod common;
