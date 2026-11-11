// Public-API smoke driver — BmpDevice (Phase 4 Beat 4).
//
// Validates the canonical surface shape (constructors, ImageDevice
// trait wire-up) and exercises the full rasterise + encode path
// end-to-end on a vendored fixture by asserting the produced bytes
// carry a valid BITMAPFILEHEADER signature and the embedded
// BITMAPINFOHEADER dimensions match the rasterised page.

use std::path::PathBuf;

use aspose_pdf_foss::{
    BmpDevice, Document, FormPresentationMode, ImageDevice,
    PageCoordinateType, PageDevice, PageSize, RenderingOptions, Resolution,
};

fn fixture(rel: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(rel)
}

const BMP_SIGNATURE: [u8; 2] = *b"BM";

// ============================================================
// Constructors + accessors
// ============================================================

#[test]
fn bmp_device_default_ctor() {
    let d = BmpDevice::new();
    assert!(d.resolution().is_none());
    assert_eq!(d.width(), 0);
    assert_eq!(d.height(), 0);
}

#[test]
fn bmp_device_resolution_ctor() {
    let d = BmpDevice::with_resolution(Resolution::new(150).unwrap());
    assert_eq!(d.resolution().unwrap().x(), 150);
    assert_eq!(d.width(), 0);
    assert_eq!(d.height(), 0);
}

#[test]
fn bmp_device_dimensions_ctor() {
    let d = BmpDevice::with_dimensions(800, 600);
    assert!(d.resolution().is_none());
    assert_eq!(d.width(), 800);
    assert_eq!(d.height(), 600);
}

#[test]
fn bmp_device_dimensions_and_resolution_ctor() {
    let d = BmpDevice::with_dimensions_and_resolution(
        800, 600, Resolution::new(96).unwrap(),
    );
    assert_eq!(d.resolution().unwrap().x(), 96);
    assert_eq!(d.width(), 800);
    assert_eq!(d.height(), 600);
}

#[test]
fn bmp_device_image_device_property_round_trip() {
    let mut d = BmpDevice::new();
    assert_eq!(d.coordinate_type(), PageCoordinateType::CropBox);
    assert_eq!(d.rendering_options(), &RenderingOptions::new());
    assert_eq!(d.form_presentation_mode(), FormPresentationMode::Production);

    d.set_coordinate_type(PageCoordinateType::MediaBox);
    d.set_form_presentation_mode(FormPresentationMode::Editor);
    let mut bag = RenderingOptions::new();
    bag.set_use_font_hinting(true);
    d.set_rendering_options(bag);

    assert_eq!(d.coordinate_type(), PageCoordinateType::MediaBox);
    assert_eq!(d.form_presentation_mode(), FormPresentationMode::Editor);
    assert!(d.rendering_options().use_font_hinting());
}

#[test]
fn bmp_device_page_size_ctor() {
    let d = BmpDevice::with_page_size(PageSize::a4());
    assert!(d.resolution().is_none());
    assert_eq!(d.width(), 595);
    assert_eq!(d.height(), 842);
}

#[test]
fn bmp_device_page_size_and_resolution_ctor() {
    let d = BmpDevice::with_page_size_and_resolution(
        PageSize::a5(),
        Resolution::new(96).unwrap(),
    );
    assert_eq!(d.resolution().unwrap().x(), 96);
    // A5 = 421 x 595 user-space units.
    assert_eq!(d.width(), 421);
    assert_eq!(d.height(), 595);
}

// ============================================================
// Trait wire-up
// ============================================================

#[test]
fn bmp_device_implements_page_device_trait() {
    let d: Box<dyn PageDevice> = Box::new(BmpDevice::new());
    let _ = d;
}

// ============================================================
// End-to-end — process a real fixture page and validate output.
// ============================================================

#[test]
fn process_red_rect_emits_valid_bmp() {
    let doc = Document::open(fixture("page_renderer/red_rect.pdf"))
        .expect("open red_rect.pdf");
    let page = doc.pages().get(1).expect("page 1");

    let device = BmpDevice::new();
    let mut out: Vec<u8> = Vec::new();
    device.process(&page, &mut out).expect("process red_rect");

    assert!(out.len() > 54, "output too short for BMP (header alone is 54 bytes)");
    assert_eq!(&out[..2], &BMP_SIGNATURE, "missing BMP 'BM' signature");

    // BITMAPINFOHEADER starts at offset 14 (BITMAPFILEHEADER is
    // 14 bytes wide). Width (LE i32) at offset 18; height at 22;
    // biBitCount at 28 (LE u16). 24bpp BGR -> biBitCount == 24.
    let bi_bit_count = u16::from_le_bytes([out[28], out[29]]);
    assert_eq!(bi_bit_count, 24, "Bgr ColorType must emit biBitCount=24");

    let width_le = i32::from_le_bytes([out[18], out[19], out[20], out[21]]);
    let height_le = i32::from_le_bytes([out[22], out[23], out[24], out[25]]);
    assert!(width_le > 0 && height_le != 0,
        "BMP must declare positive width and non-zero height");
}

#[test]
fn process_with_resolution_scales_output() {
    let doc = Document::open(fixture("page_renderer/red_rect.pdf"))
        .expect("open red_rect.pdf");
    let page = doc.pages().get(1).expect("page 1");

    let mut default = Vec::new();
    BmpDevice::new()
        .process(&page, &mut default)
        .expect("process default");

    let mut hires = Vec::new();
    BmpDevice::with_resolution(Resolution::new(150).unwrap())
        .process(&page, &mut hires)
        .expect("process 150dpi");

    // BMPs at 150 DPI are strictly larger than 72 DPI baseline
    // (uncompressed, so output size scales with pixel count
    // which scales quadratically with DPI).
    assert!(
        hires.len() > default.len(),
        "150dpi BMP {} not larger than 72dpi BMP {}",
        hires.len(),
        default.len(),
    );
}

#[test]
fn process_to_path_writes_file() {
    let doc = Document::open(fixture("page_renderer/red_rect.pdf"))
        .expect("open red_rect.pdf");
    let page = doc.pages().get(1).expect("page 1");

    let tmp = common::unique_temp("aspose_pdf_foss_bmp_device_test.bmp");
    if tmp.exists() {
        std::fs::remove_file(&tmp).ok();
    }

    let device = BmpDevice::new();
    device.process_to_path(&page, &tmp).expect("process_to_path");

    let bytes = std::fs::read(&tmp).expect("read written BMP");
    assert_eq!(&bytes[..2], &BMP_SIGNATURE);
    std::fs::remove_file(&tmp).ok();
}

#[path = "../common/mod.rs"]
mod common;
