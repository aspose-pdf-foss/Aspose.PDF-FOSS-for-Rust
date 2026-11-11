// Public-API smoke driver — JpegDevice (Phase 4 Beat 4).
//
// Validates the canonical surface shape (constructors + the
// `quality` knob, ImageDevice/PageDevice trait wire-up) and
// exercises the full rasterise + encode path end-to-end on a
// vendored fixture by asserting the produced bytes carry a valid
// JFIF/JPEG SOI marker and have a non-trivial payload.

use std::path::PathBuf;

use aspose_pdf_foss::{
    Document, FormPresentationMode, ImageDevice, JpegDevice,
    PageCoordinateType, PageDevice, PageSize, RenderingOptions, Resolution,
};

fn fixture(rel: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(rel)
}

const JPEG_SOI: [u8; 2] = [0xFF, 0xD8];
const JPEG_EOI: [u8; 2] = [0xFF, 0xD9];

// ============================================================
// Constructors + accessors
// ============================================================

#[test]
fn jpeg_device_default_ctor() {
    let d = JpegDevice::new();
    assert!(d.resolution().is_none());
    assert_eq!(d.width(), 0);
    assert_eq!(d.height(), 0);
}

#[test]
fn jpeg_device_resolution_ctor() {
    let d = JpegDevice::with_resolution(Resolution::new(150).unwrap());
    assert_eq!(d.resolution().unwrap().x(), 150);
    assert_eq!(d.width(), 0);
    assert_eq!(d.height(), 0);
}

#[test]
fn jpeg_device_quality_only_ctor() {
    let d = JpegDevice::with_quality(50);
    assert!(d.resolution().is_none());
    assert_eq!(d.width(), 0);
    assert_eq!(d.height(), 0);
}

#[test]
fn jpeg_device_resolution_and_quality_ctor() {
    let d = JpegDevice::with_resolution_and_quality(
        Resolution::new(96).unwrap(),
        90,
    );
    assert_eq!(d.resolution().unwrap().x(), 96);
}

#[test]
fn jpeg_device_dimensions_ctor() {
    let d = JpegDevice::with_dimensions(800, 600);
    assert!(d.resolution().is_none());
    assert_eq!(d.width(), 800);
    assert_eq!(d.height(), 600);
}

#[test]
fn jpeg_device_dimensions_and_resolution_ctor() {
    let d = JpegDevice::with_dimensions_and_resolution(
        800, 600, Resolution::new(96).unwrap(),
    );
    assert_eq!(d.resolution().unwrap().x(), 96);
    assert_eq!(d.width(), 800);
    assert_eq!(d.height(), 600);
}

#[test]
fn jpeg_device_dimensions_resolution_and_quality_ctor() {
    let d = JpegDevice::with_dimensions_resolution_and_quality(
        800, 600, Resolution::new(96).unwrap(), 85,
    );
    assert_eq!(d.resolution().unwrap().x(), 96);
    assert_eq!(d.width(), 800);
    assert_eq!(d.height(), 600);
}

#[test]
fn jpeg_device_image_device_property_round_trip() {
    let mut d = JpegDevice::new();
    assert_eq!(d.coordinate_type(), PageCoordinateType::CropBox);
    assert_eq!(d.rendering_options(), &RenderingOptions::new());
    assert_eq!(d.form_presentation_mode(), FormPresentationMode::Production);

    d.set_coordinate_type(PageCoordinateType::MediaBox);
    d.set_form_presentation_mode(FormPresentationMode::Editor);
    d.rendering_options_mut().set_analyze_fonts(true);

    assert_eq!(d.coordinate_type(), PageCoordinateType::MediaBox);
    assert_eq!(d.form_presentation_mode(), FormPresentationMode::Editor);
    assert!(d.rendering_options().analyze_fonts());
}

#[test]
fn jpeg_device_page_size_ctor() {
    let d = JpegDevice::with_page_size(PageSize::a4());
    assert!(d.resolution().is_none());
    assert_eq!(d.width(), 595);
    assert_eq!(d.height(), 842);
}

#[test]
fn jpeg_device_page_size_and_resolution_ctor() {
    let d = JpegDevice::with_page_size_and_resolution(
        PageSize::page_letter(),
        Resolution::new(150).unwrap(),
    );
    assert_eq!(d.resolution().unwrap().x(), 150);
    assert_eq!(d.width(), 612);
    assert_eq!(d.height(), 792);
}

#[test]
fn jpeg_device_page_size_resolution_and_quality_ctor() {
    let d = JpegDevice::with_page_size_resolution_and_quality(
        PageSize::a5(),
        Resolution::new(96).unwrap(),
        85,
    );
    assert_eq!(d.resolution().unwrap().x(), 96);
    assert_eq!(d.width(), 421);
    assert_eq!(d.height(), 595);
}

// ============================================================
// Trait wire-up
// ============================================================

#[test]
fn jpeg_device_implements_page_device_trait() {
    let d: Box<dyn PageDevice> = Box::new(JpegDevice::new());
    let _ = d;
}

// ============================================================
// End-to-end — process a real fixture page and validate output.
// ============================================================

#[test]
fn process_red_rect_emits_valid_jpeg() {
    let doc = Document::open(fixture("page_renderer/red_rect.pdf"))
        .expect("open red_rect.pdf");
    let page = doc.pages().get(1).expect("page 1");

    let device = JpegDevice::new();
    let mut out: Vec<u8> = Vec::new();
    device.process(&page, &mut out).expect("process red_rect");

    assert!(out.len() > 4, "output too short for JPEG");
    assert_eq!(&out[..2], &JPEG_SOI, "missing JPEG SOI marker");
    assert_eq!(
        &out[out.len() - 2..],
        &JPEG_EOI,
        "missing JPEG EOI marker"
    );
}

#[test]
fn quality_change_alters_output_size() {
    let doc = Document::open(fixture("page_renderer/red_rect.pdf"))
        .expect("open red_rect.pdf");
    let page = doc.pages().get(1).expect("page 1");

    let mut high = Vec::new();
    JpegDevice::with_quality(95)
        .process(&page, &mut high)
        .expect("process q95");

    let mut low = Vec::new();
    JpegDevice::with_quality(20)
        .process(&page, &mut low)
        .expect("process q20");

    assert_eq!(&high[..2], &JPEG_SOI);
    assert_eq!(&low[..2], &JPEG_SOI);
    // Higher quality must produce strictly more bytes than lower
    // quality for the same source — the IJG quantisation table
    // scales monotonically with the quality knob.
    assert!(
        high.len() > low.len(),
        "q95 output {} not larger than q20 output {}",
        high.len(),
        low.len(),
    );
}

#[test]
fn process_to_path_writes_file() {
    let doc = Document::open(fixture("page_renderer/red_rect.pdf"))
        .expect("open red_rect.pdf");
    let page = doc.pages().get(1).expect("page 1");

    let tmp = common::unique_temp("aspose_pdf_foss_jpeg_device_test.jpg");
    if tmp.exists() {
        std::fs::remove_file(&tmp).ok();
    }

    let device = JpegDevice::new();
    device.process_to_path(&page, &tmp).expect("process_to_path");

    let bytes = std::fs::read(&tmp).expect("read written JPEG");
    assert_eq!(&bytes[..2], &JPEG_SOI);
    std::fs::remove_file(&tmp).ok();
}

#[path = "../common/mod.rs"]
mod common;
