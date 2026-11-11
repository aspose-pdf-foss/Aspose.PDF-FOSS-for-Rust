// Public-API smoke driver — TiffDevice (Phase 4 Beat 4).
//
// Validates the canonical surface shape (constructors,
// Device/PageDevice/DocumentDevice trait wire-up) and exercises
// the full rasterise + encode path end-to-end on a vendored
// fixture by asserting:
//   * The produced bytes carry the TIFF byte-order marker + magic
//     (II / 0x002A for little-endian baseline TIFFs).
//   * Single-page Process(Page) emits a single IFD.
//   * Document-range Process emits one IFD per page (verified by
//     counting reachable IFDs along the chain).

use std::path::PathBuf;
use std::sync::Arc;

use aspose_pdf_foss::{
    BitmapInfo, ColorDepth, CompressionType, Document, DocumentDevice,
    FormPresentationMode, IIndexBitmapConverter, Margins, PageCoordinateType,
    PageDevice, PageSize, PixelFormat, RenderingOptions, Resolution, ShapeType,
    TiffDevice, TiffSettings,
};

fn fixture(rel: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(rel)
}

// Little-endian baseline TIFF: byte-order "II" then version 42 (LE).
const TIFF_LE_HEADER: [u8; 4] = [b'I', b'I', 0x2A, 0x00];

// Count IFDs reachable from the first IFD offset in `bytes`,
// assuming little-endian baseline. Returns 0 on any malformed
// chain.
fn count_ifds_le(bytes: &[u8]) -> usize {
    if bytes.len() < 8 || bytes[..4] != TIFF_LE_HEADER {
        return 0;
    }
    let mut offset =
        u32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]) as usize;
    let mut count = 0usize;
    while offset != 0 {
        if offset + 2 > bytes.len() {
            return 0;
        }
        let entries =
            u16::from_le_bytes([bytes[offset], bytes[offset + 1]]) as usize;
        let next_off = offset + 2 + entries * 12;
        if next_off + 4 > bytes.len() {
            return 0;
        }
        let next = u32::from_le_bytes([
            bytes[next_off],
            bytes[next_off + 1],
            bytes[next_off + 2],
            bytes[next_off + 3],
        ]) as usize;
        count += 1;
        if next == 0 {
            break;
        }
        offset = next;
        if count > 1024 {
            return 0;
        }
    }
    count
}

// ============================================================
// Constructors + accessors
// ============================================================

#[test]
fn tiff_device_default_ctor() {
    let d = TiffDevice::new();
    assert!(d.resolution().is_none());
    assert_eq!(d.width(), 0);
    assert_eq!(d.height(), 0);
}

#[test]
fn tiff_device_resolution_ctor() {
    let d = TiffDevice::with_resolution(Resolution::new(150).unwrap());
    assert_eq!(d.resolution().unwrap().x(), 150);
    assert_eq!(d.width(), 0);
    assert_eq!(d.height(), 0);
}

#[test]
fn tiff_device_dimensions_ctor() {
    let d = TiffDevice::with_dimensions(800, 600);
    assert!(d.resolution().is_none());
    assert_eq!(d.width(), 800);
    assert_eq!(d.height(), 600);
}

#[test]
fn tiff_device_dimensions_and_resolution_ctor() {
    let d = TiffDevice::with_dimensions_and_resolution(
        800, 600, Resolution::new(96).unwrap(),
    );
    assert_eq!(d.resolution().unwrap().x(), 96);
    assert_eq!(d.width(), 800);
    assert_eq!(d.height(), 600);
}

// ============================================================
// Trait wire-up
// ============================================================

#[test]
fn tiff_device_implements_page_device_trait() {
    let d: Box<dyn PageDevice> = Box::new(TiffDevice::new());
    let _ = d;
}

#[test]
fn tiff_device_implements_document_device_trait() {
    let d: Box<dyn DocumentDevice> = Box::new(TiffDevice::new());
    let _ = d;
}

// ============================================================
// End-to-end — single page through PageDevice::process
// ============================================================

#[test]
fn process_page_emits_single_ifd_tiff() {
    let doc = Document::open(fixture("page_renderer/red_rect.pdf"))
        .expect("open red_rect.pdf");
    let page = doc.pages().get(1).expect("page 1");

    let device = TiffDevice::new();
    let mut out: Vec<u8> = Vec::new();
    device.process(&page, &mut out).expect("process red_rect");

    assert!(out.len() > 8, "output too short for TIFF");
    assert_eq!(&out[..4], &TIFF_LE_HEADER, "missing TIFF II42 header");
    assert_eq!(
        count_ifds_le(&out),
        1,
        "Process(Page) must emit a single-IFD TIFF",
    );
}

// ============================================================
// End-to-end — full document through DocumentDevice::process_document
// ============================================================

#[test]
fn process_document_emits_one_ifd_per_page() {
    // red_rect.pdf is single-page; we exercise the document path
    // and count the IFD chain to confirm it matches pages().count().
    let doc = Document::open(fixture("page_renderer/red_rect.pdf"))
        .expect("open red_rect.pdf");
    let expected = doc.pages().count();

    let device = TiffDevice::new();
    let mut out: Vec<u8> = Vec::new();
    device.process_document(&doc, &mut out).expect("process_document");

    assert_eq!(&out[..4], &TIFF_LE_HEADER);
    assert_eq!(
        count_ifds_le(&out),
        expected,
        "Process(Document) must emit one IFD per page (expected {expected})",
    );
}

#[test]
fn process_document_range_validates_bounds() {
    let doc = Document::open(fixture("page_renderer/red_rect.pdf"))
        .expect("open red_rect.pdf");
    let count = doc.pages().count();

    let device = TiffDevice::new();
    let mut out: Vec<u8> = Vec::new();

    // from_page < 1 is rejected.
    assert!(
        device.process_document_range(&doc, 0, count, &mut out).is_err(),
        "from_page=0 must be rejected"
    );
    out.clear();
    // to_page > count is rejected.
    assert!(
        device
            .process_document_range(&doc, 1, count + 1, &mut out)
            .is_err(),
        "to_page > count must be rejected"
    );
    out.clear();
    // to_page < from_page is rejected.
    if count >= 1 {
        assert!(
            device.process_document_range(&doc, 1, 0, &mut out).is_err(),
            "to_page < from_page must be rejected"
        );
    }
}

#[test]
fn process_document_to_path_writes_file() {
    let doc = Document::open(fixture("page_renderer/red_rect.pdf"))
        .expect("open red_rect.pdf");

    let tmp = common::unique_temp("aspose_pdf_foss_tiff_device_test.tif");

    let device = TiffDevice::new();
    device
        .process_document_to_path(&doc, &tmp)
        .expect("process_document_to_path");

    let bytes = std::fs::read(&tmp).expect("read written TIFF");
    assert_eq!(&bytes[..4], &TIFF_LE_HEADER);
    std::fs::remove_file(&tmp).ok();
}

// ============================================================
// TiffSettings — Beat 10 wired the four TiffSettings-only ctors
// plus the Settings getter, and routes settings.compression()
// through the tiff_encoder codec selector.
// ============================================================

#[test]
fn tiff_device_default_carries_default_settings() {
    let d = TiffDevice::new();
    let s = d.settings();
    assert_eq!(s.compression(), CompressionType::Lzw);
    assert_eq!(s.depth(), ColorDepth::Default);
    assert_eq!(s.shape(), ShapeType::None);
    assert_eq!(s.coordinate_type(), PageCoordinateType::CropBox);
    assert!((s.brightness() - 0.5).abs() < f32::EPSILON);
    assert!(!s.skip_blank_pages());
}

#[test]
fn tiff_device_with_settings_ctor_carries_settings() {
    let settings =
        TiffSettings::with_compression(CompressionType::None);
    let d = TiffDevice::with_settings(settings);
    assert!(d.resolution().is_none());
    assert_eq!(d.settings().compression(), CompressionType::None);
}

#[test]
fn tiff_device_with_resolution_and_settings_ctor() {
    let settings = TiffSettings::with_compression(CompressionType::Rle);
    let d = TiffDevice::with_resolution_and_settings(
        Resolution::new(96).unwrap(),
        settings,
    );
    assert_eq!(d.resolution().unwrap().x(), 96);
    assert_eq!(d.settings().compression(), CompressionType::Rle);
}

#[test]
fn tiff_device_with_dimensions_and_settings_ctor() {
    let settings = TiffSettings::with_depth(ColorDepth::Format24bpp);
    let d = TiffDevice::with_dimensions_and_settings(800, 600, settings);
    assert_eq!(d.width(), 800);
    assert_eq!(d.height(), 600);
    assert_eq!(d.settings().depth(), ColorDepth::Format24bpp);
}

#[test]
fn tiff_device_with_dimensions_resolution_and_settings_ctor() {
    let settings = TiffSettings::with_compression_depth_margins(
        CompressionType::Lzw,
        ColorDepth::Format24bpp,
        Margins::with_sides(10, 20, 30, 40),
    );
    let d = TiffDevice::with_dimensions_resolution_and_settings(
        800,
        600,
        Resolution::new(96).unwrap(),
        settings,
    );
    assert_eq!(d.width(), 800);
    assert_eq!(d.height(), 600);
    assert_eq!(d.resolution().unwrap().x(), 96);
    assert_eq!(d.settings().margins().left(), 10);
    assert_eq!(d.settings().margins().top(), 30);
}

#[test]
fn compression_none_produces_uncompressed_strips() {
    // CompressionType::None routes to tiff_encoder::Compression::None.
    // TIFF Compression tag (0x0103) carries SHORT value 1 for none.
    let doc = Document::open(fixture("page_renderer/red_rect.pdf"))
        .expect("open red_rect.pdf");
    let page = doc.pages().get(1).expect("page 1");

    let device = TiffDevice::with_settings(TiffSettings::with_compression(
        CompressionType::None,
    ));
    let mut out: Vec<u8> = Vec::new();
    device.process(&page, &mut out).expect("process");

    assert_eq!(&out[..4], &TIFF_LE_HEADER);
    let compression_value = read_short_tag(&out, 0x0103);
    assert_eq!(
        compression_value,
        Some(1),
        "CompressionType::None must produce TIFF Compression tag = 1",
    );
}

#[test]
fn compression_lzw_produces_lzw_compressed_strips() {
    // CompressionType::Lzw routes to tiff_encoder::Compression::Lzw,
    // emitting TIFF Compression tag value 5.
    let doc = Document::open(fixture("page_renderer/red_rect.pdf"))
        .expect("open red_rect.pdf");
    let page = doc.pages().get(1).expect("page 1");

    let device = TiffDevice::with_settings(TiffSettings::with_compression(
        CompressionType::Lzw,
    ));
    let mut out: Vec<u8> = Vec::new();
    device.process(&page, &mut out).expect("process");

    assert_eq!(&out[..4], &TIFF_LE_HEADER);
    assert_eq!(
        read_short_tag(&out, 0x0103),
        Some(5),
        "CompressionType::Lzw must produce TIFF Compression tag = 5",
    );
}

#[test]
fn compression_rle_produces_packbits_strips() {
    // CompressionType::Rle routes to tiff_encoder::Compression::PackBits,
    // emitting TIFF Compression tag value 32773.
    let doc = Document::open(fixture("page_renderer/red_rect.pdf"))
        .expect("open red_rect.pdf");
    let page = doc.pages().get(1).expect("page 1");

    let device = TiffDevice::with_settings(TiffSettings::with_compression(
        CompressionType::Rle,
    ));
    let mut out: Vec<u8> = Vec::new();
    device.process(&page, &mut out).expect("process");

    assert_eq!(&out[..4], &TIFF_LE_HEADER);
    assert_eq!(
        read_short_tag(&out, 0x0103),
        Some(32773),
        "CompressionType::Rle must produce TIFF Compression tag = 32773",
    );
}

#[test]
fn ccitt_compression_with_non_1bpp_depth_falls_back_to_uncompressed() {
    // CompressionType::Ccitt3 / Ccitt4 require `Depth = Format1bpp`
    // — when the depth knob is left at its default the device
    // degrades to uncompressed strips (TIFF Compression tag = 1)
    // so the RGB workflow keeps producing valid TIFF.
    let doc = Document::open(fixture("page_renderer/red_rect.pdf"))
        .expect("open red_rect.pdf");
    let page = doc.pages().get(1).expect("page 1");

    for c in [CompressionType::Ccitt3, CompressionType::Ccitt4] {
        let device =
            TiffDevice::with_settings(TiffSettings::with_compression(c));
        let mut out: Vec<u8> = Vec::new();
        device.process(&page, &mut out).expect("process");
        assert_eq!(
            read_short_tag(&out, 0x0103),
            Some(1),
            "CCITT must fall back to uncompressed (tag = 1) when Depth != Format1bpp ({:?})",
            c,
        );
    }
}

#[test]
fn format1bpp_emits_blackiszero_1bpp_strips() {
    let doc = Document::open(fixture("page_renderer/red_rect.pdf"))
        .expect("open red_rect.pdf");
    let page = doc.pages().get(1).expect("page 1");

    let device =
        TiffDevice::with_settings(TiffSettings::with_depth(ColorDepth::Format1bpp));
    let mut out: Vec<u8> = Vec::new();
    device.process(&page, &mut out).expect("process");

    assert_eq!(&out[..4], &TIFF_LE_HEADER);
    // Photometric = 1 (BlackIsZero).
    assert_eq!(read_short_tag(&out, 0x0106), Some(1));
    // BitsPerSample = 1 (SHORT inline).
    assert_eq!(read_short_tag(&out, 0x0102), Some(1));
    // SamplesPerPixel = 1.
    assert_eq!(read_short_tag(&out, 0x0115), Some(1));
    // Default depth + Lzw compression — Compression tag = 5.
    assert_eq!(read_short_tag(&out, 0x0103), Some(5));
}

#[test]
fn ccitt3_with_format1bpp_emits_compression_3_and_t4options() {
    let doc = Document::open(fixture("page_renderer/red_rect.pdf"))
        .expect("open red_rect.pdf");
    let page = doc.pages().get(1).expect("page 1");

    let settings = TiffSettings::with_compression_depth_margins(
        CompressionType::Ccitt3,
        ColorDepth::Format1bpp,
        Margins::new(),
    );
    let device = TiffDevice::with_settings(settings);
    let mut out: Vec<u8> = Vec::new();
    device.process(&page, &mut out).expect("process");

    // Compression = 3 (CCITT T.4).
    assert_eq!(read_short_tag(&out, 0x0103), Some(3));
    // T4Options = 0 (LONG, count=1).
    assert_eq!(read_long_tag(&out, 0x0124), Some(0));
}

#[test]
fn ccitt4_with_format1bpp_emits_compression_4_and_t6options() {
    let doc = Document::open(fixture("page_renderer/red_rect.pdf"))
        .expect("open red_rect.pdf");
    let page = doc.pages().get(1).expect("page 1");

    let settings = TiffSettings::with_compression_depth_margins(
        CompressionType::Ccitt4,
        ColorDepth::Format1bpp,
        Margins::new(),
    );
    let device = TiffDevice::with_settings(settings);
    let mut out: Vec<u8> = Vec::new();
    device.process(&page, &mut out).expect("process");

    assert_eq!(read_short_tag(&out, 0x0103), Some(4));
    // T6Options = 0 (LONG, count=1).
    assert_eq!(read_long_tag(&out, 0x0125), Some(0));
}

/// Read the first LONG-typed value (count==1) for the given
/// TIFF tag from a little-endian baseline TIFF; returns
/// `None` if the tag isn't present or the file is malformed.
fn read_long_tag(bytes: &[u8], tag: u16) -> Option<u32> {
    if bytes.len() < 8 || bytes[..4] != TIFF_LE_HEADER {
        return None;
    }
    let offset =
        u32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]) as usize;
    if offset + 2 > bytes.len() {
        return None;
    }
    let entries =
        u16::from_le_bytes([bytes[offset], bytes[offset + 1]]) as usize;
    for i in 0..entries {
        let e = offset + 2 + i * 12;
        if e + 12 > bytes.len() {
            return None;
        }
        let entry_tag = u16::from_le_bytes([bytes[e], bytes[e + 1]]);
        if entry_tag != tag {
            continue;
        }
        let entry_type = u16::from_le_bytes([bytes[e + 2], bytes[e + 3]]);
        // TIFF type 4 = LONG.
        if entry_type != 4 {
            return None;
        }
        return Some(u32::from_le_bytes([
            bytes[e + 8],
            bytes[e + 9],
            bytes[e + 10],
            bytes[e + 11],
        ]));
    }
    None
}

#[test]
fn binarize_bradley_round_trips_a_synthetic_tiff() {
    // Build a 32x32 RGBA TIFF that's all-white with three
    // isolated black pixels. Bradley/Roth's local-mean
    // threshold should flag exactly those three pixels as
    // foreground (black) and leave everything else background
    // (white). A uniform half-half witness wouldn't work —
    // Bradley intentionally classifies large uniform regions
    // as background regardless of luma magnitude.
    use aspose_pdf_foss::TiffDevice;

    let w: u32 = 32;
    let h: u32 = 32;
    let dark_pixels = [(8u32, 8u32), (16u32, 16u32), (24u32, 24u32)];
    let mut rgba = Vec::with_capacity((w * h * 4) as usize);
    for y in 0..h {
        for x in 0..w {
            let dark = dark_pixels.iter().any(|&(px, py)| px == x && py == y);
            let v: u8 = if dark { 0 } else { 0xFF };
            rgba.push(v);
            rgba.push(v);
            rgba.push(v);
            rgba.push(0xFF);
        }
    }

    let tiff_in = build_rgb_tiff_le(w, h, &rgba);
    let device = TiffDevice::new();
    let mut output: Vec<u8> = Vec::new();
    device
        .binarize_bradley(&mut std::io::Cursor::new(tiff_in), &mut output, 0.15)
        .expect("binarize");

    assert_eq!(&output[..4], &TIFF_LE_HEADER);
    let strip_offset = read_long_tag(&output, 273).expect("strip offset");
    let strip_len = read_long_tag(&output, 279).expect("strip length");
    let strip =
        &output[strip_offset as usize..(strip_offset + strip_len) as usize];
    // 24-bpp RGB output: 3 bytes per pixel.
    assert_eq!(strip.len(), (w * h * 3) as usize);

    for y in 0..h {
        for x in 0..w {
            let i = (y * w + x) as usize * 3;
            let v = strip[i];
            assert!(v == 0 || v == 0xFF, "non-saturated pixel at ({x}, {y})");
            assert_eq!(strip[i + 1], v);
            assert_eq!(strip[i + 2], v);
            let expected_dark =
                dark_pixels.iter().any(|&(px, py)| px == x && py == y);
            assert_eq!(
                v == 0,
                expected_dark,
                "pixel ({x}, {y}) misclassified by Bradley",
            );
        }
    }
}

#[test]
fn binarize_bradley_rejects_empty_input() {
    use aspose_pdf_foss::TiffDevice;
    let device = TiffDevice::new();
    let mut out: Vec<u8> = Vec::new();
    let err = device
        .binarize_bradley(&mut std::io::Cursor::new(Vec::<u8>::new()), &mut out, 0.15)
        .unwrap_err();
    let msg = format!("{err}");
    assert!(msg.contains("empty"), "expected empty-input error, got: {msg}");
}

/// Build a minimal little-endian baseline RGB 24-bpp TIFF for
/// `pixels` (RGBA8 input — alpha is dropped). Used to seed
/// the Bradley binariser without depending on a Document.
fn build_rgb_tiff_le(w: u32, h: u32, rgba: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    // Header.
    out.extend_from_slice(b"II");
    out.extend_from_slice(&42u16.to_le_bytes());
    let first_ifd_slot = out.len();
    out.extend_from_slice(&0u32.to_le_bytes());

    // Strip = RGB pass-through.
    let strip_offset = out.len();
    for chunk in rgba.chunks_exact(4) {
        out.extend_from_slice(&chunk[..3]);
    }
    let strip_len = out.len() - strip_offset;

    // Out-of-line BitsPerSample (3 SHORTs).
    while out.len() & 1 != 0 {
        out.push(0);
    }
    let bps_offset = out.len();
    out.extend_from_slice(&8u16.to_le_bytes());
    out.extend_from_slice(&8u16.to_le_bytes());
    out.extend_from_slice(&8u16.to_le_bytes());

    // Out-of-line RATIONAL resolution (96 dpi).
    while out.len() & 1 != 0 {
        out.push(0);
    }
    let xres_offset = out.len();
    out.extend_from_slice(&96u32.to_le_bytes());
    out.extend_from_slice(&1u32.to_le_bytes());
    let yres_offset = out.len();
    out.extend_from_slice(&96u32.to_le_bytes());
    out.extend_from_slice(&1u32.to_le_bytes());

    // IFD.
    while out.len() & 1 != 0 {
        out.push(0);
    }
    let ifd_offset = out.len();
    let entries: [(u16, u16, u32, u32); 12] = [
        (256, 4, 1, w),
        (257, 4, 1, h),
        (258, 3, 3, bps_offset as u32),
        (259, 3, 1, 1),     // Compression = None
        (262, 3, 1, 2),     // Photometric = RGB
        (273, 4, 1, strip_offset as u32),
        (277, 3, 1, 3),
        (278, 4, 1, h),
        (279, 4, 1, strip_len as u32),
        (282, 5, 1, xres_offset as u32),
        (283, 5, 1, yres_offset as u32),
        (296, 3, 1, 2),
    ];
    out.extend_from_slice(&(entries.len() as u16).to_le_bytes());
    for (tag, ty, count, val) in entries.iter() {
        out.extend_from_slice(&tag.to_le_bytes());
        out.extend_from_slice(&ty.to_le_bytes());
        out.extend_from_slice(&count.to_le_bytes());
        out.extend_from_slice(&val.to_le_bytes());
    }
    out.extend_from_slice(&0u32.to_le_bytes());

    // Back-patch first IFD offset.
    let bytes = (ifd_offset as u32).to_le_bytes();
    out[first_ifd_slot..first_ifd_slot + 4].copy_from_slice(&bytes);
    out
}

/// Read the first SHORT-typed value (count==1) for the given
/// TIFF tag from a little-endian baseline TIFF; returns
/// `None` if the tag isn't present or the file is malformed.
fn read_short_tag(bytes: &[u8], tag: u16) -> Option<u16> {
    if bytes.len() < 8 || bytes[..4] != TIFF_LE_HEADER {
        return None;
    }
    let offset =
        u32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]) as usize;
    if offset + 2 > bytes.len() {
        return None;
    }
    let entries =
        u16::from_le_bytes([bytes[offset], bytes[offset + 1]]) as usize;
    for i in 0..entries {
        let e = offset + 2 + i * 12;
        if e + 12 > bytes.len() {
            return None;
        }
        let entry_tag = u16::from_le_bytes([bytes[e], bytes[e + 1]]);
        if entry_tag != tag {
            continue;
        }
        let entry_type = u16::from_le_bytes([bytes[e + 2], bytes[e + 3]]);
        // TIFF type 3 = SHORT.
        if entry_type != 3 {
            return None;
        }
        return Some(u16::from_le_bytes([bytes[e + 8], bytes[e + 9]]));
    }
    None
}

// ============================================================
// IIndexBitmapConverter — Beat 11 wired the four converter-
// taking TiffDevice ctors (Arc<dyn IIndexBitmapConverter>
// storage shape preserves Clone). The converter rides on the
// device but is not invoked in the v1 24-bpp RGB encode path;
// these smokes confirm the ctors accept it and the device
// continues to produce valid TIFF bytes.
// ============================================================

#[derive(Debug, Default)]
struct NoopConverter;

impl IIndexBitmapConverter for NoopConverter {
    fn get_1bpp_image(&self, src: &BitmapInfo) -> BitmapInfo {
        BitmapInfo::new(
            src.pixel_bytes().to_vec(),
            src.width(),
            src.height(),
            PixelFormat::Rgb24,
        )
    }
    fn get_4bpp_image(&self, src: &BitmapInfo) -> BitmapInfo {
        BitmapInfo::new(
            src.pixel_bytes().to_vec(),
            src.width(),
            src.height(),
            PixelFormat::Rgb24,
        )
    }
    fn get_8bpp_image(&self, src: &BitmapInfo) -> BitmapInfo {
        BitmapInfo::new(
            src.pixel_bytes().to_vec(),
            src.width(),
            src.height(),
            PixelFormat::Rgb24,
        )
    }
}

fn noop_converter() -> Arc<dyn IIndexBitmapConverter> {
    Arc::new(NoopConverter)
}

#[test]
fn tiff_device_with_settings_and_converter_ctor() {
    let d = TiffDevice::with_settings_and_converter(
        TiffSettings::new(),
        noop_converter(),
    );
    assert!(d.resolution().is_none());
    // Carrier accepts the converter; subsequent process emits
    // valid TIFF (the v1 encode path bypasses the converter,
    // so the output matches the no-converter baseline).
}

#[test]
fn tiff_device_with_resolution_settings_and_converter_ctor() {
    let d = TiffDevice::with_resolution_settings_and_converter(
        Resolution::new(96).unwrap(),
        TiffSettings::new(),
        noop_converter(),
    );
    assert_eq!(d.resolution().unwrap().x(), 96);
}

#[test]
fn tiff_device_with_dimensions_settings_and_converter_ctor() {
    let d = TiffDevice::with_dimensions_settings_and_converter(
        800,
        600,
        TiffSettings::with_depth(ColorDepth::Format1bpp),
        noop_converter(),
    );
    assert_eq!(d.width(), 800);
    assert_eq!(d.height(), 600);
    assert_eq!(d.settings().depth(), ColorDepth::Format1bpp);
}

#[test]
fn tiff_device_with_dimensions_resolution_settings_and_converter_ctor() {
    let d = TiffDevice::with_dimensions_resolution_settings_and_converter(
        800,
        600,
        Resolution::new(150).unwrap(),
        TiffSettings::with_compression(CompressionType::Rle),
        noop_converter(),
    );
    assert_eq!(d.width(), 800);
    assert_eq!(d.height(), 600);
    assert_eq!(d.resolution().unwrap().x(), 150);
    assert_eq!(d.settings().compression(), CompressionType::Rle);
}

#[test]
fn converter_taking_ctor_still_emits_valid_tiff() {
    // v1 encode path doesn't invoke the converter; the device
    // must keep producing a TIFF byte stream even when one is
    // attached.
    let doc = Document::open(fixture("page_renderer/red_rect.pdf"))
        .expect("open red_rect.pdf");
    let page = doc.pages().get(1).expect("page 1");

    let device = TiffDevice::with_settings_and_converter(
        TiffSettings::with_depth(ColorDepth::Format1bpp),
        noop_converter(),
    );
    let mut out: Vec<u8> = Vec::new();
    device.process(&page, &mut out).expect("process");
    assert_eq!(&out[..4], &TIFF_LE_HEADER);
}

#[test]
fn tiff_device_clones_with_converter_attached() {
    // TiffDevice::clone must propagate the Arc<dyn ...> so a
    // forked device shares the same converter strategy.
    let converter = noop_converter();
    let baseline = Arc::strong_count(&converter);
    let device =
        TiffDevice::with_settings_and_converter(TiffSettings::new(), converter);
    let cloned = device.clone();
    drop(device);
    drop(cloned);
    // After both drop, the test's local Arc reference is the only
    // remaining strong count. The baseline captured before any
    // device ctor reflects the pre-clone count.
    let _ = baseline;
}

#[test]
fn process_to_path_writes_file() {
    let doc = Document::open(fixture("page_renderer/red_rect.pdf"))
        .expect("open red_rect.pdf");
    let page = doc.pages().get(1).expect("page 1");

    let tmp = common::unique_temp("aspose_pdf_foss_tiff_device_page_test.tif");

    let device = TiffDevice::new();
    device.process_to_path(&page, &tmp).expect("process_to_path");

    let bytes = std::fs::read(&tmp).expect("read written TIFF");
    assert_eq!(&bytes[..4], &TIFF_LE_HEADER);
    std::fs::remove_file(&tmp).ok();
}

// ============================================================
// PageSize-on-device ctors — Beat 12 wired the six PageSize-
// taking TiffDevice ctors. The canonical algorithm is the C++
// peer's truncating cast straight from the page's user-space
// units to the device's width/height pixel fields (C# peer
// disagrees — leaves Width/Height at 0 — peer disagreement, the
// Rust track resolves to the C++ shape per workflow memory).
// ============================================================

// ============================================================
// RenderingOptions + FormPresentationMode properties — Beat 13
// wired both directly on TiffDevice (TiffDevice is a
// DocumentDevice, not an ImageDevice in the canonical class
// graph, so these accessors are concrete methods rather than
// trait methods).
// ============================================================

#[test]
fn tiff_device_rendering_options_default() {
    let d = TiffDevice::new();
    assert_eq!(d.rendering_options(), &RenderingOptions::new());
}

#[test]
fn tiff_device_rendering_options_round_trip() {
    let mut d = TiffDevice::new();
    let mut bag = RenderingOptions::new();
    bag.set_analyze_fonts(true);
    bag.set_interpolation_high_quality(true);
    d.set_rendering_options(bag);

    assert!(d.rendering_options().analyze_fonts());
    assert!(d.rendering_options().interpolation_high_quality());

    d.rendering_options_mut().set_use_font_hinting(true);
    assert!(d.rendering_options().use_font_hinting());
}

#[test]
fn tiff_device_form_presentation_mode_default_and_setter() {
    let mut d = TiffDevice::new();
    assert_eq!(d.form_presentation_mode(), FormPresentationMode::Production);
    d.set_form_presentation_mode(FormPresentationMode::Editor);
    assert_eq!(d.form_presentation_mode(), FormPresentationMode::Editor);
}

#[test]
fn tiff_device_with_page_size_ctor() {
    let d = TiffDevice::with_page_size(PageSize::a4());
    assert!(d.resolution().is_none());
    // A4 = 595 x 842 user-space units.
    assert_eq!(d.width(), 595);
    assert_eq!(d.height(), 842);
    // Default settings come along for the ride.
    assert_eq!(d.settings().compression(), CompressionType::Lzw);
}

#[test]
fn tiff_device_with_page_size_and_resolution_ctor() {
    let d = TiffDevice::with_page_size_and_resolution(
        PageSize::page_letter(),
        Resolution::new(96).unwrap(),
    );
    assert_eq!(d.resolution().unwrap().x(), 96);
    // PageLetter = 612 x 792.
    assert_eq!(d.width(), 612);
    assert_eq!(d.height(), 792);
}

#[test]
fn tiff_device_with_page_size_and_settings_ctor() {
    let settings = TiffSettings::with_compression(CompressionType::None);
    let d = TiffDevice::with_page_size_and_settings(PageSize::a5(), settings);
    assert!(d.resolution().is_none());
    // A5 = 421 x 595.
    assert_eq!(d.width(), 421);
    assert_eq!(d.height(), 595);
    assert_eq!(d.settings().compression(), CompressionType::None);
}

#[test]
fn tiff_device_with_page_size_resolution_and_settings_ctor() {
    let settings = TiffSettings::with_depth(ColorDepth::Format24bpp);
    let d = TiffDevice::with_page_size_resolution_and_settings(
        PageSize::a4(),
        Resolution::new(150).unwrap(),
        settings,
    );
    assert_eq!(d.resolution().unwrap().x(), 150);
    assert_eq!(d.width(), 595);
    assert_eq!(d.height(), 842);
    assert_eq!(d.settings().depth(), ColorDepth::Format24bpp);
}

#[test]
fn tiff_device_with_page_size_settings_and_converter_ctor() {
    let d = TiffDevice::with_page_size_settings_and_converter(
        PageSize::a4(),
        TiffSettings::new(),
        noop_converter(),
    );
    assert_eq!(d.width(), 595);
    assert_eq!(d.height(), 842);
}

#[test]
fn tiff_device_with_page_size_resolution_settings_and_converter_ctor() {
    let d = TiffDevice::with_page_size_resolution_settings_and_converter(
        PageSize::page_letter(),
        Resolution::new(300).unwrap(),
        TiffSettings::with_compression(CompressionType::Rle),
        noop_converter(),
    );
    assert_eq!(d.width(), 612);
    assert_eq!(d.height(), 792);
    assert_eq!(d.resolution().unwrap().x(), 300);
    assert_eq!(d.settings().compression(), CompressionType::Rle);
}

/// `skip_blank_pages` was carried and never consulted, so a multi-page
/// TIFF kept every empty page. A page is blank when it took no ink at
/// all — every pixel is the white it was composited onto — and the IFD
/// chain is what proves it was left out.
///
/// The document is saved and reopened first: the devices rasterise the
/// document's bytes, so a page only just added to the model is not
/// there to render yet.
#[test]
fn a_blank_page_is_skipped_when_asked_for() {
    let path = common::unique_temp("aspose_rust_tiff_blank.pdf");
    {
        let doc = Document::open(fixture("page_renderer/red_rect.pdf"))
            .expect("open red_rect.pdf");
        doc.pages().add();
        doc.pages().add();
        doc.save_to(&path).expect("save with two blank pages");
    }
    let doc = Document::open(&path).expect("reopen");
    assert_eq!(doc.pages().count(), 3);

    let keep_all = TiffDevice::with_settings(TiffSettings::new());
    let mut all: Vec<u8> = Vec::new();
    keep_all.process_document(&doc, &mut all).expect("keep every page");
    assert_eq!(count_ifds_le(&all), 3, "all three pages by default");

    let skipping = TiffDevice::with_settings(TiffSettings::with_skip_blank_pages(true));
    let mut kept: Vec<u8> = Vec::new();
    skipping.process_document(&doc, &mut kept).expect("skip the blank ones");
    assert_eq!(
        count_ifds_le(&kept),
        1,
        "only the page with the rectangle on it"
    );
    assert!(kept.len() < all.len(), "and the file is smaller for it");
    let _ = std::fs::remove_file(&path);
}

/// When every page in the range is blank, one is kept: a TIFF with no
/// image in it is not a TIFF.
#[test]
fn an_all_blank_document_still_emits_one_page() {
    let path = common::unique_temp("aspose_rust_tiff_all_blank.pdf");
    {
        let doc = Document::new();
        doc.pages().add();
        doc.pages().add();
        doc.save_to(&path).expect("save blank pages");
    }
    let doc = Document::open(&path).expect("reopen");

    let skipping = TiffDevice::with_settings(TiffSettings::with_skip_blank_pages(true));
    let mut out: Vec<u8> = Vec::new();
    skipping.process_document(&doc, &mut out).expect("still encodes");
    assert_eq!(&out[..4], &TIFF_LE_HEADER);
    assert_eq!(count_ifds_le(&out), 1, "the last page standing is kept");
    let _ = std::fs::remove_file(&path);
}

#[path = "../common/mod.rs"]
mod common;
