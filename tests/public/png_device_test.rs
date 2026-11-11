// Public-API smoke driver — Resolution + Device hierarchy + PngDevice.
//
// Validates the canonical surface shape (constructors, accessors,
// transparent_background toggle) and exercises the full
// rasterise + encode path end-to-end on a vendored fixture by
// asserting the produced bytes carry a valid PNG signature and
// decode (via the lib's internal png_decoder, mounted via the
// usual #[path] back-channel) to a buffer whose dimensions match
// the page rendered at the device's effective DPI.

use std::io::Cursor;
use std::path::PathBuf;

use aspose_pdf_foss::{
    Document, Error, FormPresentationMode, ImageDevice, PageCoordinateType,
    PageDevice, PageSize, PixelFormat, PngDevice, RenderingOptions, Resolution,
};

// ============================================================
// Internal png_decoder mount — integration-test crate can't see
// the lib's crate-private internal::png_decoder directly, so the
// driver re-mounts it via the same #[path] convention as the
// foundation gates. The decoder only depends on `flate`, which
// has no further internal deps.
// ============================================================
#[path = "../../src/internal/flate.rs"]
mod flate;

#[path = "../../src/internal/png_decoder.rs"]
mod png_decoder;

fn fixture(rel: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(rel)
}

const PNG_SIGNATURE: [u8; 8] = [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];

// ============================================================
// Resolution — constructors + accessors
// ============================================================

#[test]
fn resolution_square_ctor() {
    let r = Resolution::new(150).expect("ctor");
    assert_eq!(r.x(), 150);
    assert_eq!(r.y(), 150);
}

#[test]
fn resolution_axes_ctor() {
    let r = Resolution::with_axes(96, 200).expect("ctor");
    assert_eq!(r.x(), 96);
    assert_eq!(r.y(), 200);
}

#[test]
fn resolution_rejects_zero_or_negative() {
    assert!(matches!(Resolution::new(0), Err(Error::InvalidResolution(0))));
    assert!(matches!(Resolution::new(-50), Err(Error::InvalidResolution(-50))));
    assert!(matches!(
        Resolution::with_axes(0, 96),
        Err(Error::InvalidResolution(0))
    ));
    assert!(matches!(
        Resolution::with_axes(96, -1),
        Err(Error::InvalidResolution(-1))
    ));
}

#[test]
fn resolution_is_copy_and_clone() {
    let a = Resolution::new(96).unwrap();
    let b = a; // Copy
    let c = a;
    assert_eq!(b.x(), 96);
    assert_eq!(c.x(), 96);
}

// ============================================================
// PngDevice — constructors + accessors
// ============================================================

#[test]
fn png_device_default_ctor() {
    let d = PngDevice::new();
    assert!(d.resolution().is_none());
    assert_eq!(d.width(), 0);
    assert_eq!(d.height(), 0);
    assert!(!d.transparent_background());
}

#[test]
fn png_device_resolution_ctor() {
    let d = PngDevice::with_resolution(Resolution::new(150).unwrap());
    assert_eq!(d.resolution().unwrap().x(), 150);
    assert_eq!(d.width(), 0);
    assert_eq!(d.height(), 0);
}

#[test]
fn png_device_dimensions_ctor() {
    let d = PngDevice::with_dimensions(800, 600);
    assert!(d.resolution().is_none());
    assert_eq!(d.width(), 800);
    assert_eq!(d.height(), 600);
}

#[test]
fn png_device_dimensions_and_resolution_ctor() {
    let d = PngDevice::with_dimensions_and_resolution(
        800, 600, Resolution::new(96).unwrap(),
    );
    assert_eq!(d.resolution().unwrap().x(), 96);
    assert_eq!(d.width(), 800);
    assert_eq!(d.height(), 600);
}

#[test]
fn png_device_page_size_ctor() {
    let d = PngDevice::with_page_size(PageSize::a4());
    assert!(d.resolution().is_none());
    // A4 = 595 x 842 user-space units → truncating cast to i32.
    assert_eq!(d.width(), 595);
    assert_eq!(d.height(), 842);
}

#[test]
fn png_device_page_size_and_resolution_ctor() {
    let d = PngDevice::with_page_size_and_resolution(
        PageSize::page_letter(),
        Resolution::new(150).unwrap(),
    );
    assert_eq!(d.resolution().unwrap().x(), 150);
    // PageLetter = 612 x 792 user-space units.
    assert_eq!(d.width(), 612);
    assert_eq!(d.height(), 792);
}

#[test]
fn png_device_coordinate_type_default_and_setter() {
    let mut d = PngDevice::new();
    assert_eq!(d.coordinate_type(), PageCoordinateType::CropBox);
    d.set_coordinate_type(PageCoordinateType::MediaBox);
    assert_eq!(d.coordinate_type(), PageCoordinateType::MediaBox);
}

#[test]
fn png_device_rendering_options_default_and_setter() {
    let mut d = PngDevice::new();
    assert_eq!(d.rendering_options(), &RenderingOptions::new());

    // Replace by value.
    let mut bag = RenderingOptions::new();
    bag.set_analyze_fonts(true);
    d.set_rendering_options(bag);
    assert!(d.rendering_options().analyze_fonts());

    // In-place mutation through *_mut().
    d.rendering_options_mut().set_use_font_hinting(true);
    assert!(d.rendering_options().use_font_hinting());
}

#[test]
fn png_device_form_presentation_mode_default_and_setter() {
    let mut d = PngDevice::new();
    assert_eq!(d.form_presentation_mode(), FormPresentationMode::Production);
    d.set_form_presentation_mode(FormPresentationMode::Editor);
    assert_eq!(d.form_presentation_mode(), FormPresentationMode::Editor);
}

#[test]
fn png_device_transparent_background_toggle() {
    let mut d = PngDevice::new();
    assert!(!d.transparent_background());
    d.set_transparent_background(true);
    assert!(d.transparent_background());
    d.set_transparent_background(false);
    assert!(!d.transparent_background());
}

// ============================================================
// Inheritance / dyn dispatch — the marker trait + PageDevice
// trait wire correctly through to the concrete PngDevice.
// ============================================================

#[test]
fn png_device_implements_page_device_trait() {
    let d: Box<dyn PageDevice> = Box::new(PngDevice::new());
    // The cast itself is the assertion; it would fail to compile
    // if PngDevice didn't impl PageDevice. The borrow keeps the
    // box live until end of test.
    let _ = d;
}

// ============================================================
// End-to-end — process a real fixture page and validate output.
// ============================================================

#[test]
fn process_red_rect_emits_valid_png() {
    let doc = Document::open(fixture("page_renderer/red_rect.pdf"))
        .expect("open red_rect.pdf");
    let page = doc.pages().get(1).expect("page 1");

    let device = PngDevice::new();
    let mut out: Vec<u8> = Vec::new();
    device
        .process(&page, &mut out)
        .expect("process red_rect");

    // 1. PNG signature
    assert!(out.len() > 8, "output too short for PNG");
    assert_eq!(
        &out[..8],
        &PNG_SIGNATURE,
        "missing PNG signature"
    );

    // 2. Decodes via internal png_decoder + matches expected geometry
    let decoded = png_decoder::decode(&out).expect("decode produced PNG");
    // red_rect.pdf at 72 dpi -> matches the page_renderer sidecar size
    let expected_pixels = std::fs::read(
        fixture("page_renderer/red_rect.p0.pixels"),
    )
    .expect("read sidecar");
    assert_eq!(
        decoded.pixels.len(),
        expected_pixels.len(),
        "decoded buffer length differs from page_renderer sidecar"
    );
}

#[test]
fn process_to_writer_with_explicit_resolution() {
    let doc = Document::open(fixture("page_renderer/red_rect.pdf"))
        .expect("open red_rect.pdf");
    let page = doc.pages().get(1).expect("page 1");

    let device = PngDevice::with_resolution(Resolution::new(96).unwrap());
    let mut buf: Vec<u8> = Vec::new();
    let mut cursor = Cursor::new(&mut buf);
    device.process(&page, &mut cursor).expect("process");

    assert_eq!(&buf[..8], &PNG_SIGNATURE);
    let decoded = png_decoder::decode(&buf).expect("decode 96dpi PNG");
    // 96 dpi > 72 dpi, so the rendered buffer must be strictly
    // larger than the 72-dpi baseline sidecar.
    let baseline = std::fs::read(fixture("page_renderer/red_rect.p0.pixels"))
        .expect("read sidecar");
    assert!(
        decoded.pixels.len() > baseline.len(),
        "96dpi render not larger than 72dpi baseline ({} vs {})",
        decoded.pixels.len(),
        baseline.len(),
    );
}

#[test]
fn process_transparent_background_passes_through_to_canvas_init() {
    // Two renders of the same page — opaque vs transparent — must
    // differ byte-wise because the canvas-init pixels dominate
    // areas not covered by page content. Verifies the flag plumbs
    // through to page_renderer::render, not just the encoder's
    // color-type selection.
    let doc = Document::open(fixture("page_renderer/red_rect.pdf"))
        .expect("open red_rect.pdf");
    let page = doc.pages().get(1).expect("page 1");

    let opaque = PngDevice::new();
    let mut trans = PngDevice::new();
    trans.set_transparent_background(true);

    let mut opaque_out: Vec<u8> = Vec::new();
    let mut trans_out: Vec<u8> = Vec::new();
    opaque.process(&page, &mut opaque_out).expect("opaque render");
    let page2 = doc.pages().get(1).expect("page 1 again");
    trans.process(&page2, &mut trans_out).expect("transparent render");

    assert_ne!(
        opaque_out, trans_out,
        "transparent_background must change canvas-init bytes \
         (opaque and transparent renders differ)"
    );
}

#[test]
fn process_transparent_background_uses_rgba() {
    let doc = Document::open(fixture("page_renderer/blank.pdf"))
        .expect("open blank.pdf");
    let page = doc.pages().get(1).expect("page 1");

    let mut device = PngDevice::new();
    device.set_transparent_background(true);
    let mut out: Vec<u8> = Vec::new();
    device.process(&page, &mut out).expect("process transparent");

    // IHDR chunk follows the 8-byte signature and starts at byte 8.
    // PNG chunk layout: 4B length, 4B type, payload, 4B CRC.
    // IHDR payload: width(4) height(4) bit_depth(1) color_type(1) ...
    // The color_type byte sits at offset 8 + 4 + 4 + 4 + 4 + 1 = 25.
    assert_eq!(&out[..8], &PNG_SIGNATURE);
    let color_type_byte = out[25];
    assert_eq!(
        color_type_byte, 6, // PNG color type 6 = Rgba
        "transparent_background=true must emit Rgba (color type 6)"
    );
}

#[test]
fn process_opaque_background_uses_rgb() {
    let doc = Document::open(fixture("page_renderer/blank.pdf"))
        .expect("open blank.pdf");
    let page = doc.pages().get(1).expect("page 1");

    let device = PngDevice::new(); // transparent=false default
    let mut out: Vec<u8> = Vec::new();
    device.process(&page, &mut out).expect("process opaque");

    assert_eq!(&out[..8], &PNG_SIGNATURE);
    let color_type_byte = out[25];
    assert_eq!(
        color_type_byte, 2, // PNG color type 2 = Rgb
        "transparent_background=false must emit Rgb (color type 2)"
    );
}

// ============================================================
// ImageDevice::get_bitmap — Beat 9 wired BitmapInfo as the
// return shape of the canonical .NET ImageDevice.GetBitmap(Page).
// Exercised via PngDevice (the only ImageDevice today); the
// trait default body rasterises through page_renderer and emits
// an Rgba32 buffer.
// ============================================================

#[test]
fn get_bitmap_returns_rgba32_buffer() {
    let doc = Document::open(fixture("page_renderer/red_rect.pdf"))
        .expect("open red_rect.pdf");
    let page = doc.pages().get(1).expect("page 1");

    let device = PngDevice::new();
    let info = device.get_bitmap(&page).expect("get_bitmap");

    assert_eq!(info.format(), PixelFormat::Rgba32);
    assert!(info.width() > 0);
    assert!(info.height() > 0);
    let expected_len = (info.width() as usize) * (info.height() as usize) * 4;
    assert_eq!(info.pixel_bytes().len(), expected_len);

    // The 72-dpi page_renderer sidecar carries the same RGBA8
    // pixel buffer the device produces under the default DPI.
    let baseline = std::fs::read(fixture("page_renderer/red_rect.p0.pixels"))
        .expect("read sidecar");
    assert_eq!(info.pixel_bytes(), &baseline[..]);
}

#[test]
fn get_bitmap_honours_resolution() {
    let doc = Document::open(fixture("page_renderer/red_rect.pdf"))
        .expect("open red_rect.pdf");
    let page = doc.pages().get(1).expect("page 1");

    let baseline_device = PngDevice::new();
    let baseline = baseline_device.get_bitmap(&page).expect("get_bitmap 72dpi");

    let hires = PngDevice::with_resolution(Resolution::new(144).unwrap());
    let upscaled = hires.get_bitmap(&page).expect("get_bitmap 144dpi");

    assert!(
        upscaled.pixel_bytes().len() > baseline.pixel_bytes().len(),
        "144dpi bitmap not larger than 72dpi baseline ({} vs {})",
        upscaled.pixel_bytes().len(),
        baseline.pixel_bytes().len(),
    );
}

#[test]
fn process_to_path_writes_file() {
    let doc = Document::open(fixture("page_renderer/red_rect.pdf"))
        .expect("open red_rect.pdf");
    let page = doc.pages().get(1).expect("page 1");

    let tmp = common::unique_temp("aspose_pdf_foss_png_device_test.png");
    if tmp.exists() {
        std::fs::remove_file(&tmp).ok();
    }

    let device = PngDevice::new();
    device.process_to_path(&page, &tmp).expect("process_to_path");

    let bytes = std::fs::read(&tmp).expect("read written PNG");
    assert_eq!(&bytes[..8], &PNG_SIGNATURE);
    std::fs::remove_file(&tmp).ok();
}

// A page whose only content is text in a Standard-14 font — no embedded
// font program — must come back with ink on it. The glyph outlines come
// from the metric-class substitute `internal::standard14_outlines`
// resolves; without that path the page rasterises blank, which no
// structural assertion above would notice.
#[test]
fn standard14_text_rasterises_with_ink() {
    let doc = Document::open(fixture("text_extractor/hello_world.pdf"))
        .expect("open hello_world.pdf");
    let page = doc.pages().get(1).expect("page 1");

    let mut out: Vec<u8> = Vec::new();
    PngDevice::new().process(&page, &mut out).expect("process hello_world");
    let decoded = png_decoder::decode(&out).expect("decode produced PNG");

    let dark = decoded
        .pixels
        .chunks_exact(4)
        .filter(|px| px[0] < 128 && px[1] < 128 && px[2] < 128 && px[3] > 0)
        .count();
    assert!(dark > 100, "page rendered without glyphs: {dark} dark pixels");

    // The ink is where the text is — the top-left quadrant — rather than
    // spread over the page by a runaway fill.
    let width = decoded.width as usize;
    let height = decoded.height as usize;
    let outside = decoded
        .pixels
        .chunks_exact(4)
        .enumerate()
        .filter(|(i, px)| {
            let (x, y) = (i % width, i / width);
            (x > width / 2 || y > height / 2)
                && px[0] < 128
                && px[1] < 128
                && px[2] < 128
                && px[3] > 0
        })
        .count();
    assert_eq!(outside, 0, "ink outside the text area: {outside} dark pixels");
}

// An image this crate embeds must come back from this crate's own
// renderer. `Page::add_image` writes a PNG as a FlateDecode image
// XObject, which the renderer read as nothing at all until it decoded
// sampled images — the library could not read what it had just written,
// and every structural assertion about the resource still passed.
#[test]
fn an_embedded_image_renders_back() {
    let png = std::fs::read(fixture("png_decoder/rgb_checker_16.png")).expect("read png");

    let doc = Document::new();
    let page = doc.pages().add();
    page.add_image(
        &png,
        aspose_pdf_foss::Rectangle::new(10.0, 10.0, 90.0, 90.0, false),
        None,
        false,
    )
    .expect("add_image");
    page.set_page_size(100.0, 100.0);

    let mut saved = Vec::new();
    doc.save_to_writer(&mut saved).expect("save");

    let reopened = Document::from_bytes(saved).expect("reopen");
    let mut out = Vec::new();
    PngDevice::new()
        .process(&reopened.pages().get(1).expect("page 1"), &mut out)
        .expect("render");
    let decoded = png_decoder::decode(&out).expect("decode the rendered page");

    // The checker fixture is saturated colour; assert the placement
    // area carries it and the margin around it stays white.
    let at = |x: usize, y: usize| {
        let i = (y * decoded.width as usize + x) * 4;
        (decoded.pixels[i], decoded.pixels[i + 1], decoded.pixels[i + 2])
    };
    let (w, h) = (decoded.width as usize, decoded.height as usize);
    assert_eq!(at(2, 2), (255, 255, 255), "the margin should stay white");
    let inside = at(w / 2, h / 2);
    assert_ne!(inside, (255, 255, 255), "the image did not paint: centre is white");
}

/// A device asked for a page that is only in the model answers with a
/// usable error. It used to hand the renderer `u64::MAX` — the sentinel
/// a freshly added page carries — and the complaint came back quoting a
/// wrapped index, which was true and told the caller nothing.
#[test]
fn rendering_a_page_that_is_not_in_the_bytes_yet_says_so() {
    let doc = Document::open(fixture("page_renderer/red_rect.pdf")).expect("open");
    let added = doc.pages().add();

    let mut out: Vec<u8> = Vec::new();
    let err = PngDevice::new()
        .process(&added, &mut out)
        .expect_err("a page with no leaf cannot be rendered");
    assert!(matches!(err, Error::PageNotPersisted), "the named error, got {err:?}");
    let message = err.to_string();
    assert!(
        message.contains("save the document first"),
        "and it says what to do: {message}"
    );
    assert!(out.is_empty(), "with nothing written");

    // The page a document was opened with renders as before.
    let first = doc.pages().get(1).expect("page 1");
    let mut ok: Vec<u8> = Vec::new();
    PngDevice::new().process(&first, &mut ok).expect("a persisted page renders");
    assert!(!ok.is_empty());
}

/// Saving is what makes the page renderable, which is what the error
/// asks for: the same page renders once the document has been written
/// and reopened.
#[test]
fn saving_makes_an_added_page_renderable() {
    let path = common::unique_temp("aspose_rust_added_page_render.pdf");
    {
        let doc = Document::open(fixture("page_renderer/red_rect.pdf")).expect("open");
        doc.pages().add();
        doc.save_to(&path).expect("save");
    }
    let doc = Document::open(&path).expect("reopen");
    assert_eq!(doc.pages().count(), 2);
    let second = doc.pages().get(2).expect("the added page");

    let mut out: Vec<u8> = Vec::new();
    PngDevice::new().process(&second, &mut out).expect("now it renders");
    assert!(!out.is_empty(), "a blank page is still a PNG");
    let _ = std::fs::remove_file(&path);
}

#[path = "../common/mod.rs"]
mod common;

#[test]
fn a_page_converts_itself_to_png() {
    // The page-level twin of Document::convert_page_to_png: same
    // bytes, obtained without going through the document.
    let doc = Document::open(fixture("page_renderer/red_rect.pdf")).expect("open");
    let page = doc.pages().get(1).unwrap();
    let from_page = page.convert_to_png().expect("convert");
    let from_doc = doc.convert_page_to_png(&page).expect("convert");
    assert_eq!(from_page[..8], PNG_SIGNATURE, "a PNG signature");
    assert_eq!(from_page, from_doc, "both routes render the same page");
}
