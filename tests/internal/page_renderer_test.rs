// Hand-authored gate driver — lossy_oracle_agreement_pdf gate.
// Primitive: page_renderer
// Freeze gate: for each `Feedstock/PDF/Fixtures/page_renderer/*.pdf`
// fixture X with sibling `<base>.p<index>.pixels` sidecars, assert
// that page_renderer::render(X, page_index, 72.0).pixels matches
// the sidecar bytes by PSNR ≥ 30 dB. Two-oracle agreement
// (mutool draw + pdftocairo) is established at fixture-AUTHORING
// time; this driver does no oracle bookkeeping.
//
// Differs from lossy_oracle_agreement (rasterizer / glyph_rasterizer
// / dctdecode / jbig2 / image_compositor — single Decode(byte[]) call,
// single .pixels sidecar) only in the call shape: render takes
// (input, page_index, dpi), and one PDF can carry multiple pages
// → multiple sidecars. PSNR + sidecar + authoring pipeline are
// identical.
//
// page_renderer composes the entire foundation read-side rendering
// path; the integration-test crate mounts the FULL transitive
// `#[path]` tree of every dep page_renderer.rs `use`s. Modules are
// re-mounted in the same shape as their `super::` imports so the
// body's `super::transform::…` etc. resolve at the test crate's
// internal-module root.

#[path = "../../src/internal/lexer.rs"]
mod lexer;

#[path = "../../src/internal/flate.rs"]
mod flate;

#[path = "../../src/internal/lzw.rs"]
mod lzw;

#[path = "../../src/internal/asciihex.rs"]
mod asciihex;

#[path = "../../src/internal/ascii85.rs"]
mod ascii85;

#[path = "../../src/internal/runlength.rs"]
mod runlength;

#[path = "../../src/internal/xref.rs"]
mod xref;

#[path = "../../src/internal/objects.rs"]
mod objects;

#[path = "../../src/internal/trailer.rs"]
mod trailer;

#[path = "../../src/internal/transform.rs"]
mod transform;

#[path = "../../src/internal/colorspace.rs"]
mod colorspace;

#[path = "../../src/internal/rasterizer.rs"]
mod rasterizer;

#[path = "../../src/internal/standard14_outlines.rs"]
mod standard14_outlines;

#[path = "../../src/internal/image_compositor.rs"]
mod image_compositor;

#[path = "../../src/internal/glyph_rasterizer.rs"]
mod glyph_rasterizer;

#[path = "../../src/internal/content_stream.rs"]
mod content_stream;

#[path = "../../src/internal/pages_tree.rs"]
mod pages_tree;

#[path = "../../src/internal/dctdecode.rs"]
mod dctdecode;

#[path = "../../src/internal/truetype.rs"]
mod truetype;

#[path = "../../src/internal/page_renderer.rs"]
mod page_renderer;

use std::fs;
use std::path::PathBuf;

const PSNR_MIN: f64 = 30.0;
const FIXTURE_DPI: f64 = 72.0;

/// Corpus fixtures this renderer cannot reproduce, each with the
/// capability it waits on. **Empty** — every fixture in the shared
/// corpus is held to the PSNR floor.
///
/// It stays here for the next time the corpus grows, because the list
/// is not a way to look green: everything absent from it is held to the
/// floor, so a regression fails the gate — and a listed fixture that
/// starts PASSING fails it too, which is what stops the list from
/// rotting once a capability lands.
const KNOWN_GAPS: &[(&str, &str)] = &[
];

fn known_gap(base: &str) -> Option<&'static str> {
    KNOWN_GAPS.iter().find(|(name, _)| *name == base).map(|(_, why)| *why)
}

fn corpus_dir() -> PathBuf {
    if let Ok(env_dir) = std::env::var("SPECDEV_CORPUS_DIR") {
        if !env_dir.is_empty() {
            return PathBuf::from(env_dir);
        }
    }
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("page_renderer")
}

fn psnr(a: &[u8], b: &[u8]) -> f64 {
    if a.len() != b.len() {
        return 0.0;
    }
    if a.is_empty() {
        return f64::INFINITY;
    }
    let mut sse: f64 = 0.0;
    for (x, y) in a.iter().zip(b.iter()) {
        let d = *x as f64 - *y as f64;
        sse += d * d;
    }
    if sse == 0.0 {
        return f64::INFINITY;
    }
    let mse = sse / a.len() as f64;
    10.0 * (255.0 * 255.0 / mse).log10()
}

/// Parse `<base>.p<index>.pixels` → (`base`, `index`) when the file
/// matches the per-page sidecar shape; else `None`.
fn parse_sidecar_name(name: &str) -> Option<(String, u64)> {
    let stem = name.strip_suffix(".pixels")?;
    let dot_p = stem.rfind(".p")?;
    let (base, suffix) = stem.split_at(dot_p);
    let idx_str = &suffix[2..]; // strip ".p"
    let idx: u64 = idx_str.parse().ok()?;
    Some((base.to_string(), idx))
}

#[test]
fn lossy_oracle_agreement_over_corpus() {
    let dir = corpus_dir();
    assert!(dir.is_dir(),
            "corpus dir not found: {} \
             (set SPECDEV_CORPUS_DIR or vendor tests/fixtures/page_renderer/)",
            dir.display());

    let mut sidecars: Vec<(PathBuf, String, u64)> = fs::read_dir(&dir)
        .expect("read corpus dir")
        .filter_map(Result::ok)
        .filter_map(|e| {
            let p = e.path();
            if !p.is_file() { return None; }
            let name = p.file_name()?.to_str()?.to_string();
            let (base, idx) = parse_sidecar_name(&name)?;
            Some((p, base, idx))
        })
        .collect();
    sidecars.sort();
    assert!(!sidecars.is_empty(),
            "no .p<N>.pixels sidecars under {}", dir.display());

    let mut checked = 0_usize;
    let mut failed = 0_usize;
    let mut gaps = 0_usize;
    let mut newly_passing: Vec<String> = Vec::new();
    for (sidecar_path, base, page_index) in &sidecars {
        let pdf_path = dir.join(format!("{base}.pdf"));
        eprintln!("TRY {} (page {})", pdf_path.display(), page_index);
        if !pdf_path.is_file() {
            panic!("missing PDF for sidecar {}: {}",
                   sidecar_path.display(), pdf_path.display());
        }

        let input = fs::read(&pdf_path).expect("read fixture");
        let expected = fs::read(sidecar_path).expect("read sidecar");

        let rendered = match page_renderer::render(&input, *page_index, FIXTURE_DPI, false) {
            Ok(r) => r,
            Err(e) => panic!(
                "render failed for {} page {}: {}",
                pdf_path.display(), page_index, e),
        };
        if rendered.pixels.len() != expected.len() {
            eprintln!(
                "MISMATCH {} p{}: pixel-buffer length differs \
                 (rendered {} vs expected {})",
                pdf_path.display(), page_index,
                rendered.pixels.len(), expected.len());
            if known_gap(base).is_none() {
                failed += 1;
            } else {
                gaps += 1;
            }
            checked += 1;
            continue;
        }
        let p = psnr(&rendered.pixels, &expected);
        match (p < PSNR_MIN, known_gap(base)) {
            (true, None) => {
                eprintln!(
                    "MISMATCH {} p{}: PSNR={p:.2} below threshold {PSNR_MIN}",
                    pdf_path.display(), page_index);
                failed += 1;
            }
            (true, Some(why)) => {
                eprintln!("KNOWN GAP {base} p{page_index}: PSNR={p:.2} — {why}");
                gaps += 1;
            }
            (false, Some(why)) => {
                eprintln!(
                    "NEWLY PASSING {base} p{page_index}: PSNR={p:.2} — {why} \
                     now renders; strike it off KNOWN_GAPS");
                newly_passing.push(base.clone());
            }
            (false, None) => {}
        }
        checked += 1;
    }
    eprintln!("page_renderer lossy_oracle_agreement_pdf: \
               checked={} failed={} known_gaps={} psnr_min={}",
              checked, failed, gaps, PSNR_MIN);
    assert!(
        newly_passing.is_empty(),
        "these fixtures now render and must come off KNOWN_GAPS: {newly_passing:?}"
    );
    assert_eq!(failed, 0, "{} fixture mismatches", failed);
}

// ============================================================
// Sampled image XObjects — the filter chains that carry raw samples
// rather than a terminal image codec (FlateDecode and friends, or no
// filter at all). The corpus above exercises DCTDecode only, which is
// how a renderer that decoded nothing else stayed green while every
// Flate-compressed image on a page rendered blank.
//
// These cases build their own PDFs so the shared fixture corpus is not
// changed — it is read by every target track's gate.
// ============================================================

/// A one-page PDF whose only content draws `image` over the whole page.
/// `entries` are the image XObject's dictionary entries, spelled as PDF
/// source; `/Length` is appended.
fn pdf_with_image(page_w: u32, page_h: u32, entries: &str, image: &[u8]) -> Vec<u8> {
    let content = format!("q {page_w} 0 0 {page_h} 0 0 cm /Im0 Do Q");
    let mut objects: Vec<Vec<u8>> = Vec::new();
    objects.push(b"<< /Type /Catalog /Pages 2 0 R >>".to_vec());
    objects.push(b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec());
    objects.push(
        format!(
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {page_w} {page_h}] \
             /Contents 4 0 R /Resources << /XObject << /Im0 5 0 R >> >> >>"
        )
        .into_bytes(),
    );
    let mut content_obj =
        format!("<< /Length {} >>\nstream\n", content.len()).into_bytes();
    content_obj.extend_from_slice(content.as_bytes());
    content_obj.extend_from_slice(b"\nendstream");
    objects.push(content_obj);
    let mut image_obj =
        format!("<< {entries} /Length {} >>\nstream\n", image.len()).into_bytes();
    image_obj.extend_from_slice(image);
    image_obj.extend_from_slice(b"\nendstream");
    objects.push(image_obj);

    let mut out = b"%PDF-1.7\n".to_vec();
    let mut offsets = Vec::with_capacity(objects.len());
    for (i, body) in objects.iter().enumerate() {
        offsets.push(out.len());
        out.extend_from_slice(format!("{} 0 obj\n", i + 1).as_bytes());
        out.extend_from_slice(body);
        out.extend_from_slice(b"\nendobj\n");
    }
    let startxref = out.len();
    out.extend_from_slice(format!("xref\n0 {}\n", objects.len() + 1).as_bytes());
    out.extend_from_slice(b"0000000000 65535 f \n");
    for off in &offsets {
        out.extend_from_slice(format!("{off:010} 00000 n \n").as_bytes());
    }
    out.extend_from_slice(
        format!(
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{startxref}\n%%EOF\n",
            objects.len() + 1
        )
        .as_bytes(),
    );
    out
}

/// The rendered pixel at the centre of the page, as `(r, g, b)`.
fn centre_pixel(pdf: &[u8]) -> (u8, u8, u8) {
    let page = page_renderer::render(pdf, 0, 72.0, false).expect("render");
    let x = (page.width / 2) as usize;
    let y = (page.height / 2) as usize;
    let i = (y * page.width as usize + x) * 4;
    (page.pixels[i], page.pixels[i + 1], page.pixels[i + 2])
}

#[test]
fn flate_rgb_image_paints_its_colours() {
    // 2x2 of one colour, so any sampling lands on it.
    let samples: Vec<u8> = [0xC0u8, 0x30, 0x60].repeat(4);
    let pdf = pdf_with_image(
        8,
        8,
        "/Type /XObject /Subtype /Image /Width 2 /Height 2 \
         /ColorSpace /DeviceRGB /BitsPerComponent 8 /Filter /FlateDecode",
        &flate::encode(&samples),
    );
    assert_eq!(centre_pixel(&pdf), (0xC0, 0x30, 0x60));
}

#[test]
fn uncompressed_image_paints_its_colours() {
    let samples: Vec<u8> = [0x10u8, 0x80, 0xF0].repeat(4);
    let pdf = pdf_with_image(
        8,
        8,
        "/Type /XObject /Subtype /Image /Width 2 /Height 2 \
         /ColorSpace /DeviceRGB /BitsPerComponent 8",
        &samples,
    );
    assert_eq!(centre_pixel(&pdf), (0x10, 0x80, 0xF0));
}

#[test]
fn gray_image_expands_to_rgb() {
    let samples = vec![0x40u8; 4];
    let pdf = pdf_with_image(
        8,
        8,
        "/Type /XObject /Subtype /Image /Width 2 /Height 2 \
         /ColorSpace /DeviceGray /BitsPerComponent 8 /Filter /FlateDecode",
        &flate::encode(&samples),
    );
    assert_eq!(centre_pixel(&pdf), (0x40, 0x40, 0x40));
}

#[test]
fn one_bit_gray_rows_are_byte_aligned() {
    // 4x2, all bits set → white. Each row occupies its own byte even
    // though four samples need only half of one; reading them packed
    // end-to-end would pull the second row's bits into the first.
    let samples = vec![0b1111_0000u8, 0b1111_0000];
    let pdf = pdf_with_image(
        8,
        8,
        "/Type /XObject /Subtype /Image /Width 4 /Height 2 \
         /ColorSpace /DeviceGray /BitsPerComponent 1",
        &samples,
    );
    assert_eq!(centre_pixel(&pdf), (0xFF, 0xFF, 0xFF));
}

#[test]
fn four_bit_gray_scales_to_full_range() {
    // 0xF is the top of the 4-bit ladder and must land on 255, not 15.
    let samples = vec![0xFFu8; 2];
    let pdf = pdf_with_image(
        8,
        8,
        "/Type /XObject /Subtype /Image /Width 2 /Height 2 \
         /ColorSpace /DeviceGray /BitsPerComponent 4",
        &samples,
    );
    assert_eq!(centre_pixel(&pdf), (0xFF, 0xFF, 0xFF));
}

#[test]
fn cmyk_image_converts_to_rgb() {
    // Pure cyan: absorbs red, so it lands near (0, 255, 255).
    let samples: Vec<u8> = [0xFFu8, 0x00, 0x00, 0x00].repeat(4);
    let pdf = pdf_with_image(
        8,
        8,
        "/Type /XObject /Subtype /Image /Width 2 /Height 2 \
         /ColorSpace /DeviceCMYK /BitsPerComponent 8 /Filter /FlateDecode",
        &flate::encode(&samples),
    );
    let (r, g, b) = centre_pixel(&pdf);
    assert!(r < 32, "cyan should absorb red, got r={r}");
    assert!(g > 200 && b > 200, "cyan should keep green/blue, got ({g}, {b})");
}

#[test]
fn icc_based_colour_space_follows_its_component_count() {
    // The ICC profile stream is never read; /N is what says how many
    // components each sample carries.
    let samples: Vec<u8> = [0x20u8, 0x90, 0xE0].repeat(4);
    let mut pdf = pdf_with_image(
        8,
        8,
        "/Type /XObject /Subtype /Image /Width 2 /Height 2 \
         /ColorSpace [/ICCBased 6 0 R] /BitsPerComponent 8 /Filter /FlateDecode",
        &flate::encode(&samples),
    );
    // Append the profile stream as an incremental object rather than
    // rebuilding the file: the renderer resolves it through the xref.
    pdf = append_profile_object(pdf, 6, b"<< /N 3 /Length 0 >>\nstream\n\nendstream");
    assert_eq!(centre_pixel(&pdf), (0x20, 0x90, 0xE0));
}

/// Append one more indirect object plus a fresh xref section for it.
fn append_profile_object(mut pdf: Vec<u8>, id: u32, body: &[u8]) -> Vec<u8> {
    let prev = find_startxref(&pdf);
    let offset = pdf.len();
    pdf.extend_from_slice(format!("{id} 0 obj\n").as_bytes());
    pdf.extend_from_slice(body);
    pdf.extend_from_slice(b"\nendobj\n");
    let startxref = pdf.len();
    pdf.extend_from_slice(format!("xref\n0 1\n0000000000 65535 f \n{id} 1\n").as_bytes());
    pdf.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
    pdf.extend_from_slice(
        format!(
            "trailer\n<< /Size {} /Root 1 0 R /Prev {prev} >>\nstartxref\n{startxref}\n%%EOF\n",
            id + 1
        )
        .as_bytes(),
    );
    pdf
}

fn find_startxref(pdf: &[u8]) -> usize {
    let text = String::from_utf8_lossy(pdf);
    let at = text.rfind("startxref").expect("startxref");
    text[at + "startxref".len()..]
        .split_whitespace()
        .next()
        .expect("offset")
        .parse()
        .expect("numeric offset")
}

#[test]
fn png_predictor_is_reversed_before_the_samples_are_read() {
    // Two identical rows under PNG filter 2 (Up): the second row's
    // bytes are all zero as differences. Without reversal they decode
    // as black, and the filter byte shears the row besides.
    let colour = [0x30u8, 0xA0, 0x50];
    let mut predicted = vec![0u8]; // row 0, filter None
    predicted.extend_from_slice(&colour.repeat(2));
    predicted.push(2); // row 1, filter Up
    predicted.extend_from_slice(&[0u8; 6]);

    let pdf = pdf_with_image(
        8,
        8,
        "/Type /XObject /Subtype /Image /Width 2 /Height 2 \
         /ColorSpace /DeviceRGB /BitsPerComponent 8 /Filter /FlateDecode \
         /DecodeParms << /Predictor 15 /Colors 3 /Columns 2 /BitsPerComponent 8 >>",
        &flate::encode(&predicted),
    );
    assert_eq!(centre_pixel(&pdf), (0x30, 0xA0, 0x50));
}

#[test]
fn tiff_predictor_is_reversed_before_the_samples_are_read() {
    // Horizontal differencing: the second pixel is stored as its
    // difference from the first, so both decode to the same colour.
    let mut predicted = vec![0x80u8, 0x40, 0x20];
    predicted.extend_from_slice(&[0u8; 3]);
    let row = predicted.clone();
    predicted.extend_from_slice(&row);

    let pdf = pdf_with_image(
        8,
        8,
        "/Type /XObject /Subtype /Image /Width 2 /Height 2 \
         /ColorSpace /DeviceRGB /BitsPerComponent 8 /Filter /FlateDecode \
         /DecodeParms << /Predictor 2 /Colors 3 /Columns 2 /BitsPerComponent 8 >>",
        &flate::encode(&predicted),
    );
    assert_eq!(centre_pixel(&pdf), (0x80, 0x40, 0x20));
}

#[test]
fn an_undecodable_filter_skips_the_image_rather_than_failing() {
    // JPXDecode has no foundation decoder. Per the anchor's error model
    // the /Do is skipped and the rest of the page still renders.
    let pdf = pdf_with_image(
        8,
        8,
        "/Type /XObject /Subtype /Image /Width 2 /Height 2 \
         /ColorSpace /DeviceRGB /BitsPerComponent 8 /Filter /JPXDecode",
        &[0u8; 16],
    );
    let page = page_renderer::render(&pdf, 0, 72.0, false).expect("render must not fail");
    assert!(page.pixels.iter().all(|b| *b == 0xFF), "page should be left blank");
}

// ============================================================
// Clipping (§8.5.4). `clip_rect` in the corpus covers the rectangular
// case against a real oracle; these cover the parts the corpus has no
// sidecar for — the line-traced rectangle, the non-rectangular mask,
// and the rule that a clip takes effect only after the painting
// operator that follows the W.
// ============================================================

/// A one-page 20x20 PDF whose content stream is `content`.
fn pdf_with_content(content: &str) -> Vec<u8> {
    let mut objects: Vec<Vec<u8>> = Vec::new();
    objects.push(b"<< /Type /Catalog /Pages 2 0 R >>".to_vec());
    objects.push(b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec());
    objects.push(
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 20 20] /Contents 4 0 R \
          /Resources << >> >>"
            .to_vec(),
    );
    let mut stream = format!("<< /Length {} >>\nstream\n", content.len()).into_bytes();
    stream.extend_from_slice(content.as_bytes());
    stream.extend_from_slice(b"\nendstream");
    objects.push(stream);

    let mut out = b"%PDF-1.7\n".to_vec();
    let mut offsets = Vec::new();
    for (i, body) in objects.iter().enumerate() {
        offsets.push(out.len());
        out.extend_from_slice(format!("{} 0 obj\n", i + 1).as_bytes());
        out.extend_from_slice(body);
        out.extend_from_slice(b"\nendobj\n");
    }
    let startxref = out.len();
    out.extend_from_slice(format!("xref\n0 {}\n", objects.len() + 1).as_bytes());
    out.extend_from_slice(b"0000000000 65535 f \n");
    for off in &offsets {
        out.extend_from_slice(format!("{off:010} 00000 n \n").as_bytes());
    }
    out.extend_from_slice(
        format!(
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{startxref}\n%%EOF\n",
            objects.len() + 1
        )
        .as_bytes(),
    );
    out
}

/// Render at 72 dpi and report, per device pixel, whether it took ink.
fn inked(content: &str) -> (Vec<bool>, usize) {
    let page = page_renderer::render(&pdf_with_content(content), 0, 72.0, false)
        .expect("render");
    let w = page.width as usize;
    let ink = page
        .pixels
        .chunks_exact(4)
        .map(|px| px[0] < 200 || px[1] < 200 || px[2] < 200)
        .collect();
    (ink, w)
}

fn ink_count(ink: &[bool]) -> usize {
    ink.iter().filter(|b| **b).count()
}

#[test]
fn a_rectangular_clip_bounds_a_later_fill() {
    // Clip to the left half, then fill the whole page black.
    let clipped = inked("0 0 10 20 re W n 0 0 20 20 re f");
    let whole = inked("0 0 20 20 re f");
    assert_eq!(ink_count(&whole.0), 400, "the unclipped fill should cover the page");
    assert_eq!(ink_count(&clipped.0), 200, "the clip should halve the fill");

    // And it is the LEFT half that survives.
    let (ink, w) = clipped;
    for y in 0..20 {
        assert!(ink[y * w], "left column should be inked at row {y}");
        assert!(!ink[y * w + 19], "right column should be clear at row {y}");
    }
}

#[test]
fn a_line_traced_rectangle_clips_like_a_re() {
    // The same region written as lines rather than `re` — what vector
    // output emits — must behave identically and without a mask.
    let traced = inked("0 0 m 10 0 l 10 20 l 0 20 l h W n 0 0 20 20 re f");
    let rect = inked("0 0 10 20 re W n 0 0 20 20 re f");
    assert_eq!(ink_count(&traced.0), ink_count(&rect.0));
    assert_eq!(traced.0, rect.0, "line-traced and re clips must agree pixel for pixel");
}

#[test]
fn a_non_rectangular_clip_masks_per_pixel() {
    // A triangle covering the lower-left half. A bounding box would keep
    // the whole page; the mask must drop the upper-right corner.
    let (ink, w) = inked("0 0 m 20 0 l 0 20 l h W n 0 0 20 20 re f");
    assert!(ink[19 * w], "the lower-left corner is inside the triangle");
    assert!(!ink[19], "the upper-right corner is outside it");
    let count = ink_count(&ink);
    assert!(
        (150..=250).contains(&count),
        "a half-page triangle should ink about half the pixels, got {count}"
    );
}

#[test]
fn a_clip_takes_effect_only_after_the_painting_operator() {
    // §8.5.4: the fill that carries the W is NOT itself clipped.
    let (ink, _) = inked("0 0 20 20 re W f");
    assert_eq!(ink_count(&ink), 400, "the operator carrying the clip paints unclipped");
}

#[test]
fn a_clip_is_restored_with_the_graphics_state() {
    // Clip inside q/Q, then fill again outside it.
    let (ink, _) = inked("q 0 0 5 5 re W n 0 0 20 20 re f Q 0 0 20 20 re f");
    assert_eq!(ink_count(&ink), 400, "the clip should not outlive its q/Q scope");
}

#[test]
fn nested_clips_intersect() {
    let (ink, w) = inked("0 0 10 20 re W n 0 0 20 10 re W n 0 0 20 20 re f");
    assert_eq!(ink_count(&ink), 100, "two half-page clips leave a quarter");
    assert!(ink[19 * w], "the shared corner survives");
    assert!(!ink[9 * w + 15], "outside the first clip is dropped");
}

// ============================================================
// Constant alpha (§11.6.4.4). `constant_alpha` in the corpus covers the
// fill case against an oracle; these pin the parts around it — the
// stroking entry, the q/Q scope, and the rule that a key the dictionary
// omits leaves its alpha alone.
// ============================================================

/// A one-page 20x20 PDF carrying `content`, with `gsdicts` spliced into
/// the page's `/Resources /ExtGState`.
fn pdf_with_ext_gstate(gsdicts: &str, content: &str) -> Vec<u8> {
    let mut objects: Vec<Vec<u8>> = Vec::new();
    objects.push(b"<< /Type /Catalog /Pages 2 0 R >>".to_vec());
    objects.push(b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec());
    objects.push(
        format!(
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 20 20] /Contents 4 0 R \
             /Resources << /ExtGState << {gsdicts} >> >> >>"
        )
        .into_bytes(),
    );
    let mut stream = format!("<< /Length {} >>\nstream\n", content.len()).into_bytes();
    stream.extend_from_slice(content.as_bytes());
    stream.extend_from_slice(b"\nendstream");
    objects.push(stream);

    let mut out = b"%PDF-1.7\n".to_vec();
    let mut offsets = Vec::new();
    for (i, body) in objects.iter().enumerate() {
        offsets.push(out.len());
        out.extend_from_slice(format!("{} 0 obj\n", i + 1).as_bytes());
        out.extend_from_slice(body);
        out.extend_from_slice(b"\nendobj\n");
    }
    let startxref = out.len();
    out.extend_from_slice(format!("xref\n0 {}\n", objects.len() + 1).as_bytes());
    out.extend_from_slice(b"0000000000 65535 f \n");
    for off in &offsets {
        out.extend_from_slice(format!("{off:010} 00000 n \n").as_bytes());
    }
    out.extend_from_slice(
        format!(
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{startxref}\n%%EOF\n",
            objects.len() + 1
        )
        .as_bytes(),
    );
    out
}

/// The red channel at the centre of a page rendered from `content`.
fn centre_red(gsdicts: &str, content: &str) -> u8 {
    let page = page_renderer::render(&pdf_with_ext_gstate(gsdicts, content), 0, 72.0, false)
        .expect("render");
    let (x, y) = (page.width as usize / 2, page.height as usize / 2);
    page.pixels[(y * page.width as usize + x) * 4]
}

#[test]
fn fill_alpha_lightens_the_paint() {
    // Black over white: opaque lands on 0, half-transparent near 128.
    let opaque = centre_red("/G << /ca 1 >>", "/G gs 0 0 20 20 re f");
    let half = centre_red("/G << /ca 0.5 >>", "/G gs 0 0 20 20 re f");
    assert_eq!(opaque, 0, "a fully opaque fill should be solid");
    assert!((120..=136).contains(&half), "half alpha should land mid-grey, got {half}");
}

#[test]
fn fill_alpha_does_not_touch_strokes() {
    // /ca is the non-stroking entry; a stroke keeps its own alpha.
    let stroked = centre_red("/G << /ca 0.5 >>", "/G gs 2 w 0 10 m 20 10 l S");
    assert_eq!(stroked, 0, "a stroke should ignore /ca, got {stroked}");
}

#[test]
fn stroke_alpha_lightens_the_stroke() {
    let half = centre_red("/G << /CA 0.5 >>", "/G gs 2 w 0 10 m 20 10 l S");
    assert!((120..=136).contains(&half), "half stroke alpha expected, got {half}");
}

#[test]
fn an_absent_entry_leaves_its_alpha_alone() {
    // The dictionary sets only /CA, so the fill stays opaque.
    let filled = centre_red("/G << /CA 0.2 >>", "/G gs 0 0 20 20 re f");
    assert_eq!(filled, 0, "a fill should be untouched by a dict without /ca");
}

#[test]
fn alpha_is_restored_with_the_graphics_state() {
    let after = centre_red("/G << /ca 0.5 >>", "q /G gs Q 0 0 20 20 re f");
    assert_eq!(after, 0, "alpha should not outlive its q/Q scope");
}

// ============================================================
// Line dash (§8.4.3.6). `dash_pattern` in the corpus pins the rendered
// result against an oracle; these pin the rules around it — what resets
// to solid, and that the pattern is measured in user space along the
// path rather than applied per segment.
// ============================================================

/// How many separate inked runs a horizontal line leaves along the
/// middle row of the page.
fn ink_runs(content: &str) -> usize {
    let page = page_renderer::render(&pdf_with_content(content), 0, 72.0, false)
        .expect("render");
    let w = page.width as usize;
    let y = page.height as usize / 2;
    let mut runs = 0;
    let mut prev = false;
    for x in 0..w {
        let px = &page.pixels[(y * w + x) * 4..];
        let ink = px[0] < 200 || px[1] < 200 || px[2] < 200;
        if ink && !prev {
            runs += 1;
        }
        prev = ink;
    }
    runs
}

#[test]
fn a_dash_pattern_breaks_the_stroke_into_spans() {
    // A 20-unit line, 4 on / 4 off: on-spans at 0-4, 8-12, 16-20.
    assert_eq!(ink_runs("[4 4] 0 d 1 w 0 10 m 20 10 l S"), 3);
    // Same line solid is one run.
    assert_eq!(ink_runs("1 w 0 10 m 20 10 l S"), 1);
}

#[test]
fn the_phase_shifts_where_dashing_starts() {
    // Four units into a 4-on/4-off cycle starts in the "off" span, so
    // the line begins blank: on-spans at 4-8 and 12-16.
    assert_eq!(ink_runs("[4 4] 4 d 1 w 0 10 m 20 10 l S"), 2);
}

#[test]
fn an_empty_array_strokes_solid() {
    assert_eq!(ink_runs("[] 0 d 1 w 0 10 m 20 10 l S"), 1);
}

#[test]
fn an_all_zero_pattern_strokes_solid() {
    // A pattern with no positive entry cannot advance; per the anchor it
    // strokes solid rather than hanging or painting nothing.
    assert_eq!(ink_runs("[0 0] 0 d 1 w 0 10 m 20 10 l S"), 1);
}

#[test]
fn the_pattern_restarts_on_each_subpath() {
    // Two identical rows, one subpath each. If the pattern carried over
    // from the first, the second would start mid-cycle and differ.
    let page = page_renderer::render(
        &pdf_with_content("[4 4] 0 d 1 w 0 6 m 20 6 l S 0 14 m 20 14 l S"),
        0,
        72.0,
        false,
    )
    .expect("render");
    let w = page.width as usize;
    let row = |y: usize| -> Vec<bool> {
        (0..w)
            .map(|x| {
                let px = &page.pixels[(y * w + x) * 4..];
                px[0] < 200 || px[1] < 200 || px[2] < 200
            })
            .collect()
    };
    // The page is 20 units tall at 72 dpi, so y=6 and y=14 in user space
    // are rows 14 and 6 from the top.
    assert_eq!(row(6), row(14), "both subpaths should dash identically");
    assert!(row(6).iter().any(|b| *b), "the rows should carry ink at all");
}

#[test]
fn a_dash_does_not_leak_into_a_later_solid_stroke() {
    let content = "q [4 4] 0 d 1 w 0 10 m 20 10 l S Q 1 w 0 10 m 20 10 l S";
    assert_eq!(ink_runs(content), 1, "the solid stroke after Q should be unbroken");
}

// ============================================================
// Form XObjects (§8.10). `form_xobject` in the corpus pins the rendered
// result against an oracle; these pin the mechanics around it — that a
// form is EXECUTED rather than composited, how its /Matrix joins the
// caller's CTM, that its state changes do not escape, which resource
// dictionary its names resolve against, and that a cycle terminates.
// ============================================================

/// A one-page 20x20 PDF whose `/Resources /XObject` is `xobjects`
/// (e.g. `"/A 5 0 R"`), carrying `forms` as objects 5, 6, … — each a
/// `(dict_extras, content)` pair — and `content` as the page stream.
fn pdf_with_forms(xobjects: &str, forms: &[(&str, &str)], content: &str) -> Vec<u8> {
    let mut objects: Vec<Vec<u8>> = Vec::new();
    objects.push(b"<< /Type /Catalog /Pages 2 0 R >>".to_vec());
    objects.push(b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec());
    objects.push(
        format!(
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 20 20] /Contents 4 0 R \
             /Resources << /XObject << {xobjects} >> >> >>"
        )
        .into_bytes(),
    );
    let mut stream = format!("<< /Length {} >>\nstream\n", content.len()).into_bytes();
    stream.extend_from_slice(content.as_bytes());
    stream.extend_from_slice(b"\nendstream");
    objects.push(stream);
    for (extras, body) in forms {
        let mut form = format!(
            "<< /Type /XObject /Subtype /Form /BBox [0 0 20 20] {extras} /Length {} >>\nstream\n",
            body.len()
        )
        .into_bytes();
        form.extend_from_slice(body.as_bytes());
        form.extend_from_slice(b"\nendstream");
        objects.push(form);
    }

    let mut out = b"%PDF-1.7\n".to_vec();
    let mut offsets = Vec::new();
    for (i, body) in objects.iter().enumerate() {
        offsets.push(out.len());
        out.extend_from_slice(format!("{} 0 obj\n", i + 1).as_bytes());
        out.extend_from_slice(body);
        out.extend_from_slice(b"\nendobj\n");
    }
    let startxref = out.len();
    out.extend_from_slice(format!("xref\n0 {}\n", objects.len() + 1).as_bytes());
    out.extend_from_slice(b"0000000000 65535 f \n");
    for off in &offsets {
        out.extend_from_slice(format!("{off:010} 00000 n \n").as_bytes());
    }
    out.extend_from_slice(
        format!(
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{startxref}\n%%EOF\n",
            objects.len() + 1
        )
        .as_bytes(),
    );
    out
}

/// Per-pixel ink of a form-bearing page, plus the row stride.
fn form_ink(xobjects: &str, forms: &[(&str, &str)], content: &str) -> (Vec<bool>, usize) {
    let pdf = pdf_with_forms(xobjects, forms, content);
    let page = page_renderer::render(&pdf, 0, 72.0, false).expect("render");
    let w = page.width as usize;
    let ink = page
        .pixels
        .chunks_exact(4)
        .map(|px| px[0] < 200 || px[1] < 200 || px[2] < 200)
        .collect();
    (ink, w)
}

/// The RGB triple at device pixel (`x`, `y`) of a form-bearing page.
fn form_pixel(
    xobjects: &str,
    forms: &[(&str, &str)],
    content: &str,
    x: usize,
    y: usize,
) -> (u8, u8, u8) {
    let pdf = pdf_with_forms(xobjects, forms, content);
    let page = page_renderer::render(&pdf, 0, 72.0, false).expect("render");
    let at = (y * page.width as usize + x) * 4;
    (page.pixels[at], page.pixels[at + 1], page.pixels[at + 2])
}

#[test]
fn a_form_xobject_is_executed() {
    // The form fills the bottom-left 10x10 of user space; at 72 dpi
    // with y flipped that is device rows 10..20, columns 0..10.
    let (ink, w) = form_ink("/A 5 0 R", &[("", "0 0 10 10 re f")], "/A Do");
    assert_eq!(ink_count(&ink), 100, "the form's own fill should reach the page");
    assert!(ink[15 * w + 5], "bottom-left quarter takes ink");
    assert!(!ink[5 * w + 15], "nothing outside it does");
}

#[test]
fn the_form_matrix_places_the_content() {
    // /Matrix translates the same 10x10 fill 10 units right, which at
    // 1 px per unit is device columns 10..20 of the bottom half.
    let (ink, w) = form_ink(
        "/A 5 0 R",
        &[("/Matrix [1 0 0 1 10 0]", "0 0 10 10 re f")],
        "/A Do",
    );
    assert_eq!(ink_count(&ink), 100, "a translate must not change the area");
    assert!(ink[15 * w + 15], "the fill moved to the bottom-right quarter");
    assert!(!ink[15 * w + 5], "and left the bottom-left one");
}

#[test]
fn the_form_matrix_composes_onto_the_caller_ctm() {
    // The caller lifts by 10, the form shifts right by 10: the 10x10
    // fill lands top-right, device rows 0..10 and columns 10..20.
    let (ink, w) = form_ink(
        "/A 5 0 R",
        &[("/Matrix [1 0 0 1 10 0]", "0 0 10 10 re f")],
        "q 1 0 0 1 0 10 cm /A Do Q",
    );
    assert_eq!(ink_count(&ink), 100);
    assert!(ink[5 * w + 15], "both translates applied");
    assert!(!ink[15 * w + 15], "the form did not stay where its matrix alone puts it");
}

#[test]
fn a_form_cannot_leak_its_state_to_the_caller() {
    // The form paints red and leaves the fill colour set; the page's
    // own fill after `Do` must still be the default black (§8.10.2 runs
    // a form as if between q and Q).
    let xo = "/A 5 0 R";
    let forms: &[(&str, &str)] = &[("", "1 0 0 rg 0 0 10 10 re f")];
    let content = "/A Do 10 10 10 10 re f";
    assert_eq!(form_pixel(xo, forms, content, 5, 15), (255, 0, 0), "the form paints red");
    assert_eq!(
        form_pixel(xo, forms, content, 15, 5),
        (0, 0, 0),
        "the caller's fill must not inherit the form's colour"
    );
}

#[test]
fn a_form_resolves_names_in_its_own_resources() {
    // /B exists ONLY inside the outer form's resources, so the inner
    // fill can only appear if the form's own dictionary took over.
    let (ink, _) = form_ink(
        "/A 5 0 R",
        &[
            ("/Resources << /XObject << /B 6 0 R >> >>", "/B Do"),
            ("", "0 0 10 10 re f"),
        ],
        "/A Do",
    );
    assert_eq!(ink_count(&ink), 100, "the nested form should have been reached");
}

#[test]
fn a_form_without_resources_inherits_the_callers() {
    // The outer form brings no /Resources, so /B must resolve against
    // the page's dictionary (§8.10.1).
    let (ink, _) = form_ink(
        "/A 5 0 R /B 6 0 R",
        &[("", "/B Do"), ("", "0 0 10 10 re f")],
        "/A Do",
    );
    assert_eq!(ink_count(&ink), 100, "the caller's resources should be inherited");
}

#[test]
fn a_self_referencing_form_terminates() {
    // Without the depth bound this recurses until the stack gives out.
    let (ink, w) = form_ink(
        "/A 5 0 R",
        &[("", "0 0 10 10 re f /A Do")],
        "/A Do",
    );
    assert_eq!(ink_count(&ink), 100, "the cycle repaints one square and stops");
    assert!(ink[15 * w + 5]);
}

// ============================================================
// Annotation appearances (§12.5.5). `annotation_ap` in the corpus pins
// the rendered result against an oracle; these pin what surrounds it —
// which annotations are skipped, which stream a state dictionary
// yields, and the /BBox-onto-/Rect fit.
// ============================================================

/// Assemble a PDF from object bodies: 1 catalog, 2 pages, 3 page with
/// `annots` in its /Annots array, 4 the page content stream, then
/// `extra` as objects 5, 6, …
fn pdf_with_annots(page_content: &str, annots: &str, extra: &[String]) -> Vec<u8> {
    let mut objects: Vec<Vec<u8>> = Vec::new();
    objects.push(b"<< /Type /Catalog /Pages 2 0 R >>".to_vec());
    objects.push(b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec());
    objects.push(
        format!(
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 20 20] /Contents 4 0 R \
             /Resources << >> /Annots [{annots}] >>"
        )
        .into_bytes(),
    );
    let mut stream =
        format!("<< /Length {} >>\nstream\n", page_content.len()).into_bytes();
    stream.extend_from_slice(page_content.as_bytes());
    stream.extend_from_slice(b"\nendstream");
    objects.push(stream);
    for body in extra {
        objects.push(body.clone().into_bytes());
    }

    let mut out = b"%PDF-1.7\n".to_vec();
    let mut offsets = Vec::new();
    for (i, body) in objects.iter().enumerate() {
        offsets.push(out.len());
        out.extend_from_slice(format!("{} 0 obj\n", i + 1).as_bytes());
        out.extend_from_slice(body);
        out.extend_from_slice(b"\nendobj\n");
    }
    let startxref = out.len();
    out.extend_from_slice(format!("xref\n0 {}\n", objects.len() + 1).as_bytes());
    out.extend_from_slice(b"0000000000 65535 f \n");
    for off in &offsets {
        out.extend_from_slice(format!("{off:010} 00000 n \n").as_bytes());
    }
    out.extend_from_slice(
        format!(
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{startxref}\n%%EOF\n",
            objects.len() + 1
        )
        .as_bytes(),
    );
    out
}

/// An appearance stream object body: a Form XObject carrying `extras`
/// (its /BBox, and /Matrix where a test needs one) and `content`.
fn appearance_object(extras: &str, content: &str) -> String {
    format!(
        "<< /Type /XObject /Subtype /Form {extras} /Length {} >>\nstream\n{content}\nendstream",
        content.len()
    )
}

/// A single-annotation page: object 5 is the annotation dictionary
/// `annot`, object 6 its appearance. Returns per-pixel ink and stride.
fn annot_ink(annot: &str, appearance: String) -> (Vec<bool>, usize) {
    let pdf = pdf_with_annots("", "5 0 R", &[annot.to_string(), appearance]);
    let page = page_renderer::render(&pdf, 0, 72.0, false).expect("render");
    let w = page.width as usize;
    let ink = page
        .pixels
        .chunks_exact(4)
        .map(|px| px[0] < 200 || px[1] < 200 || px[2] < 200)
        .collect();
    (ink, w)
}

#[test]
fn an_annotation_appearance_is_painted() {
    // /Rect and /BBox agree at 10x10, so the appearance lands
    // one-to-one in the bottom-left quarter: 100 device pixels.
    let (ink, w) = annot_ink(
        "<< /Type /Annot /Subtype /Square /Rect [0 0 10 10] /AP << /N 6 0 R >> >>",
        appearance_object("/BBox [0 0 10 10]", "0 0 10 10 re f"),
    );
    assert_eq!(ink_count(&ink), 100, "the appearance should reach the page");
    assert!(ink[15 * w + 5], "it sits where /Rect puts it");
}

#[test]
fn an_annotation_without_an_appearance_is_skipped() {
    // No /AP: nothing is synthesised from the annotation's properties.
    let pdf = pdf_with_annots(
        "",
        "5 0 R",
        &["<< /Type /Annot /Subtype /Square /Rect [0 0 10 10] >>".to_string()],
    );
    let page = page_renderer::render(&pdf, 0, 72.0, false).expect("render");
    let inked = page.pixels.chunks_exact(4).filter(|px| px[0] < 200).count();
    assert_eq!(inked, 0, "an annotation with no /AP paints nothing");
}

#[test]
fn the_hidden_and_noview_flags_keep_an_annotation_off_the_page() {
    let ap = || appearance_object("/BBox [0 0 10 10]", "0 0 10 10 re f");
    let hidden = annot_ink(
        "<< /Type /Annot /Subtype /Square /Rect [0 0 10 10] /F 2 /AP << /N 6 0 R >> >>",
        ap(),
    );
    let no_view = annot_ink(
        "<< /Type /Annot /Subtype /Square /Rect [0 0 10 10] /F 32 /AP << /N 6 0 R >> >>",
        ap(),
    );
    // /F 4 is Print — it says nothing about viewing, so it still paints.
    let printable = annot_ink(
        "<< /Type /Annot /Subtype /Square /Rect [0 0 10 10] /F 4 /AP << /N 6 0 R >> >>",
        ap(),
    );
    assert_eq!(ink_count(&hidden.0), 0, "Hidden (bit 2) suppresses the appearance");
    assert_eq!(ink_count(&no_view.0), 0, "NoView (bit 6) suppresses it too");
    assert_eq!(ink_count(&printable.0), 100, "other flags must not suppress it");
}

#[test]
fn the_appearance_state_selects_the_stream() {
    // /N is a dictionary of states; /AS names the one in force.
    let states = "<< /Type /Annot /Subtype /Widget /Rect [0 0 10 10] /AS /Yes \
                  /AP << /N << /Off 6 0 R /Yes 7 0 R >> >> >>";
    let pdf = pdf_with_annots(
        "",
        "5 0 R",
        &[
            states.to_string(),
            appearance_object("/BBox [0 0 10 10]", "0 0 5 5 re f"),
            appearance_object("/BBox [0 0 10 10]", "0 0 10 10 re f"),
        ],
    );
    let page = page_renderer::render(&pdf, 0, 72.0, false).expect("render");
    let inked = page.pixels.chunks_exact(4).filter(|px| px[0] < 200).count();
    assert_eq!(inked, 100, "/AS /Yes should pick the 10x10 stream, not the 5x5 one");
}

#[test]
fn the_appearance_box_is_fitted_to_the_annotation_rect() {
    // A 10x10 /BBox stretched onto a 20x20 /Rect doubles the content.
    let (ink, _) = annot_ink(
        "<< /Type /Annot /Subtype /Square /Rect [0 0 20 20] /AP << /N 6 0 R >> >>",
        appearance_object("/BBox [0 0 10 10]", "0 0 10 10 re f"),
    );
    assert_eq!(ink_count(&ink), 400, "the appearance should cover the whole page");
}

#[test]
fn the_fit_is_measured_after_the_appearance_matrix() {
    // /Matrix shifts the box to x 5..15; the fit must undo that shift
    // to land on /Rect, so the mark still sits at x 0..10 (§12.5.5).
    let (ink, w) = annot_ink(
        "<< /Type /Annot /Subtype /Square /Rect [0 0 10 10] /AP << /N 6 0 R >> >>",
        appearance_object("/BBox [0 0 10 10] /Matrix [1 0 0 1 5 0]", "0 0 10 10 re f"),
    );
    assert_eq!(ink_count(&ink), 100);
    assert!(ink[15 * w + 2], "the fit accounted for the matrix");
    assert!(!ink[15 * w + 12], "the appearance did not drift right with it");
}

#[test]
fn appearances_are_painted_over_the_page_content() {
    // The page fills red, the annotation paints black on top of part
    // of it: appearances come after the content stream.
    let pdf = pdf_with_annots(
        "1 0 0 rg 0 0 20 20 re f",
        "5 0 R",
        &[
            "<< /Type /Annot /Subtype /Square /Rect [0 0 10 10] /AP << /N 6 0 R >> >>"
                .to_string(),
            appearance_object("/BBox [0 0 10 10]", "0 0 10 10 re f"),
        ],
    );
    let page = page_renderer::render(&pdf, 0, 72.0, false).expect("render");
    let w = page.width as usize;
    let at = |x: usize, y: usize| {
        let i = (y * w + x) * 4;
        (page.pixels[i], page.pixels[i + 1], page.pixels[i + 2])
    };
    assert_eq!(at(5, 15), (0, 0, 0), "the appearance covers the content under it");
    assert_eq!(at(15, 5), (255, 0, 0), "and leaves the rest of the page alone");
}

// ============================================================
// Image masking (§11.6.5.2 soft masks, §8.9.6.3 stencils). The corpus
// fixtures `image_smask` and `image_mask_stencil` pin the rendered
// result against an oracle; these pin the sample-to-opacity rules,
// which is where the polarity mistakes live.
// ============================================================

/// A square page wholly covered by one image, which carries a second
/// stream — its /SMask or its /Mask — as object 6.
fn pdf_with_masked_image(
    page: u32,
    image_entries: &str,
    image: &[u8],
    mask_entries: &str,
    mask: &[u8],
) -> Vec<u8> {
    let content = format!("q {page} 0 0 {page} 0 0 cm /Im0 Do Q");
    let mut objects: Vec<Vec<u8>> = Vec::new();
    objects.push(b"<< /Type /Catalog /Pages 2 0 R >>".to_vec());
    objects.push(b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec());
    objects.push(
        format!(
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {page} {page}] \
             /Contents 4 0 R /Resources << /XObject << /Im0 5 0 R >> >> >>"
        )
        .into_bytes(),
    );
    let mut content_obj =
        format!("<< /Length {} >>\nstream\n", content.len()).into_bytes();
    content_obj.extend_from_slice(content.as_bytes());
    content_obj.extend_from_slice(b"\nendstream");
    objects.push(content_obj);
    for (entries, bytes) in [(image_entries, image), (mask_entries, mask)] {
        let mut obj =
            format!("<< {entries} /Length {} >>\nstream\n", bytes.len()).into_bytes();
        obj.extend_from_slice(bytes);
        obj.extend_from_slice(b"\nendstream");
        objects.push(obj);
    }

    let mut out = b"%PDF-1.7\n".to_vec();
    let mut offsets = Vec::with_capacity(objects.len());
    for (i, body) in objects.iter().enumerate() {
        offsets.push(out.len());
        out.extend_from_slice(format!("{} 0 obj\n", i + 1).as_bytes());
        out.extend_from_slice(body);
        out.extend_from_slice(b"\nendobj\n");
    }
    let startxref = out.len();
    out.extend_from_slice(format!("xref\n0 {}\n", objects.len() + 1).as_bytes());
    out.extend_from_slice(b"0000000000 65535 f \n");
    for off in &offsets {
        out.extend_from_slice(format!("{off:010} 00000 n \n").as_bytes());
    }
    out.extend_from_slice(
        format!(
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{startxref}\n%%EOF\n",
            objects.len() + 1
        )
        .as_bytes(),
    );
    out
}

/// The four corners of a 4x4 page — top-left, top-right, bottom-left,
/// bottom-right — as RGB triples. The image under test is 2x2, so one
/// source sample lands in each corner.
fn corners(pdf: &[u8]) -> [(u8, u8, u8); 4] {
    let page = page_renderer::render(pdf, 0, 72.0, false).expect("render");
    let w = page.width as usize;
    let at = |x: usize, y: usize| {
        let i = (y * w + x) * 4;
        (page.pixels[i], page.pixels[i + 1], page.pixels[i + 2])
    };
    [at(0, 0), at(3, 0), at(0, 3), at(3, 3)]
}

/// 2x2 all-red DeviceRGB samples, and the dictionary that describes
/// them. `extras` names the mask the image carries.
const RED_2X2: &[u8] = &[
    0xFF, 0x00, 0x00, 0xFF, 0x00, 0x00, 0xFF, 0x00, 0x00, 0xFF, 0x00, 0x00,
];

fn red_image_entries(extras: &str) -> String {
    format!(
        "/Type /XObject /Subtype /Image /Width 2 /Height 2 \
         /ColorSpace /DeviceRGB /BitsPerComponent 8 {extras}"
    )
}

const WHITE: (u8, u8, u8) = (255, 255, 255);
const RED: (u8, u8, u8) = (255, 0, 0);

#[test]
fn a_soft_mask_sample_is_the_images_opacity() {
    // Opaque on the diagonal, transparent off it.
    let pdf = pdf_with_masked_image(
        4,
        &red_image_entries("/SMask 6 0 R"),
        RED_2X2,
        "/Type /XObject /Subtype /Image /Width 2 /Height 2 \
         /ColorSpace /DeviceGray /BitsPerComponent 8",
        &[255, 0, 0, 255],
    );
    assert_eq!(corners(&pdf), [RED, WHITE, WHITE, RED]);
}

#[test]
fn a_soft_mask_blends_at_intermediate_samples() {
    let pdf = pdf_with_masked_image(
        4,
        &red_image_entries("/SMask 6 0 R"),
        RED_2X2,
        "/Type /XObject /Subtype /Image /Width 2 /Height 2 \
         /ColorSpace /DeviceGray /BitsPerComponent 8",
        &[128, 128, 128, 128],
    );
    let (r, g, b) = corners(&pdf)[0];
    assert_eq!(r, 255, "red over white stays saturated in red");
    assert!((124..=132).contains(&g), "half-opaque red over white, got g={g}");
    assert_eq!(g, b, "the blend is symmetric in green and blue");
}

#[test]
fn a_decode_array_inverts_a_soft_mask() {
    // The same fully-set mask means opaque by default and fully
    // transparent under /Decode [1 0].
    let plain = pdf_with_masked_image(
        4,
        &red_image_entries("/SMask 6 0 R"),
        RED_2X2,
        "/Type /XObject /Subtype /Image /Width 2 /Height 2 \
         /ColorSpace /DeviceGray /BitsPerComponent 8",
        &[255, 255, 255, 255],
    );
    let inverted = pdf_with_masked_image(
        4,
        &red_image_entries("/SMask 6 0 R"),
        RED_2X2,
        "/Type /XObject /Subtype /Image /Width 2 /Height 2 \
         /ColorSpace /DeviceGray /BitsPerComponent 8 /Decode [1 0]",
        &[255, 255, 255, 255],
    );
    assert_eq!(corners(&plain), [RED, RED, RED, RED]);
    assert_eq!(corners(&inverted), [WHITE, WHITE, WHITE, WHITE]);
}

#[test]
fn a_stencil_paints_where_its_sample_is_set() {
    // 1 bpc, one byte per row, MSB first: row 0 is 0 1, row 1 is 1 0.
    // Sample 0 paints under the default /Decode [0 1].
    let pdf = pdf_with_masked_image(
        4,
        &red_image_entries("/Mask 6 0 R"),
        RED_2X2,
        "/Type /XObject /Subtype /Image /Width 2 /Height 2 \
         /ImageMask true /BitsPerComponent 1",
        &[0b0100_0000, 0b1000_0000],
    );
    assert_eq!(corners(&pdf), [RED, WHITE, WHITE, RED]);
}

#[test]
fn a_decode_array_inverts_a_stencil() {
    let pdf = pdf_with_masked_image(
        4,
        &red_image_entries("/Mask 6 0 R"),
        RED_2X2,
        "/Type /XObject /Subtype /Image /Width 2 /Height 2 \
         /ImageMask true /BitsPerComponent 1 /Decode [1 0]",
        &[0b0100_0000, 0b1000_0000],
    );
    assert_eq!(corners(&pdf), [WHITE, RED, RED, WHITE]);
}

#[test]
fn a_colour_key_mask_is_left_unapplied() {
    // /Mask as an array is colour-key masking (§8.9.6.4), which this
    // renderer documents as out of scope: the image paints in full.
    let pdf = pdf_with_masked_image(
        4,
        &red_image_entries("/Mask [0 255 0 0 0 0]"),
        RED_2X2,
        "/Type /XObject /Subtype /Image /Width 2 /Height 2 \
         /ColorSpace /DeviceGray /BitsPerComponent 8",
        &[0, 0, 0, 0],
    );
    assert_eq!(corners(&pdf), [RED, RED, RED, RED]);
}

// ============================================================
// Shadings (§8.7.4). The corpus fixtures `axial_shading` and
// `shading_pattern` pin the rendered gradients against an oracle;
// these pin the rules around them — where a shading stops, which
// subfunction a stitch picks, and what an unsupported type does.
// ============================================================

/// A 20x20 page whose /Resources are `resources` verbatim — shading
/// and pattern dictionaries need no objects of their own.
fn pdf_with_resources(resources: &str, content: &str) -> Vec<u8> {
    let mut objects: Vec<Vec<u8>> = Vec::new();
    objects.push(b"<< /Type /Catalog /Pages 2 0 R >>".to_vec());
    objects.push(b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec());
    objects.push(
        format!(
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 20 20] /Contents 4 0 R \
             /Resources << {resources} >> >>"
        )
        .into_bytes(),
    );
    let mut stream = format!("<< /Length {} >>\nstream\n", content.len()).into_bytes();
    stream.extend_from_slice(content.as_bytes());
    stream.extend_from_slice(b"\nendstream");
    objects.push(stream);

    let mut out = b"%PDF-1.7\n".to_vec();
    let mut offsets = Vec::new();
    for (i, body) in objects.iter().enumerate() {
        offsets.push(out.len());
        out.extend_from_slice(format!("{} 0 obj\n", i + 1).as_bytes());
        out.extend_from_slice(body);
        out.extend_from_slice(b"\nendobj\n");
    }
    let startxref = out.len();
    out.extend_from_slice(format!("xref\n0 {}\n", objects.len() + 1).as_bytes());
    out.extend_from_slice(b"0000000000 65535 f \n");
    for off in &offsets {
        out.extend_from_slice(format!("{off:010} 00000 n \n").as_bytes());
    }
    out.extend_from_slice(
        format!(
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{startxref}\n%%EOF\n",
            objects.len() + 1
        )
        .as_bytes(),
    );
    out
}

/// Render such a page and hand back a pixel reader.
fn shaded(resources: &str, content: &str) -> impl Fn(usize, usize) -> (u8, u8, u8) {
    let page = page_renderer::render(&pdf_with_resources(resources, content), 0, 72.0, false)
        .expect("render");
    let w = page.width as usize;
    move |x: usize, y: usize| {
        let i = (y * w + x) * 4;
        (page.pixels[i], page.pixels[i + 1], page.pixels[i + 2])
    }
}

/// A red-to-blue exponential function over the whole domain.
const RED_TO_BLUE: &str =
    "/Function << /FunctionType 2 /Domain [0 1] /C0 [1 0 0] /C1 [0 0 1] /N 1 >>";

fn axial_pattern(coords: &str, extend: &str) -> String {
    format!(
        "/Pattern << /P0 << /PatternType 2 /Shading << /ShadingType 2 \
         /ColorSpace /DeviceRGB /Coords {coords} /Extend {extend} {RED_TO_BLUE} >> >> >>"
    )
}

#[test]
fn an_axial_pattern_paints_a_gradient_along_its_axis() {
    // The axis spans the page, so the parameter at device column x is
    // (x + 0.5) / 20: red at the left edge, blue at the right.
    let px = shaded(
        &axial_pattern("[0 0 20 0]", "[true true]"),
        "/Pattern cs /P0 scn 0 0 20 20 re f",
    );
    let (r0, _, b0) = px(0, 10);
    let (r9, _, b9) = px(9, 10);
    let (r19, _, b19) = px(19, 10);
    assert!(r0 > 240 && b0 < 15, "the axis starts red, got {:?}", px(0, 10));
    assert!(r19 < 15 && b19 > 240, "and ends blue, got {:?}", px(19, 10));
    assert!(r0 > r9 && r9 > r19, "red falls off along the axis");
    assert!(b0 < b9 && b9 < b19, "blue rises along it");
}

#[test]
fn an_unextended_axis_leaves_its_outside_unpainted() {
    // The axis covers user x 5..15 only, and /Extend is false at both
    // ends, so the rest of the fill takes no paint at all — the anchor
    // is explicit that it is not flooded black.
    let px = shaded(
        &axial_pattern("[5 0 15 0]", "[false false]"),
        "/Pattern cs /P0 scn 0 0 20 20 re f",
    );
    assert_eq!(px(1, 10), (255, 255, 255), "before the axis stays unpainted");
    assert_eq!(px(18, 10), (255, 255, 255), "so does after it");
    // Device column 10 samples at user x 10.5, which is 0.55 along an
    // axis running from 5 to 15.
    let (r, _, b) = px(10, 10);
    assert!((113..=117).contains(&r) && (138..=142).contains(&b),
            "between them the gradient paints, got {:?}", px(10, 10));
}

#[test]
fn extending_an_axis_floods_past_its_ends() {
    let px = shaded(
        &axial_pattern("[5 0 15 0]", "[true true]"),
        "/Pattern cs /P0 scn 0 0 20 20 re f",
    );
    assert_eq!(px(1, 10), (255, 0, 0), "before the axis takes the start colour");
    assert_eq!(px(18, 10), (0, 0, 255), "after it takes the end colour");
}

#[test]
fn a_radial_shading_paints_within_its_circles() {
    // Circles centred at user (10, 10) with radii 0 and 8. A point at
    // distance d from the centre solves to s = d / 8, and beyond the
    // outer circle the unextended shading does not reach.
    let resources = "/Pattern << /P0 << /PatternType 2 /Shading << /ShadingType 3 \
         /ColorSpace /DeviceRGB /Coords [10 10 0 10 10 8] /Extend [false false] \
         /Function << /FunctionType 2 /Domain [0 1] /C0 [1 0 0] /C1 [0 0 1] /N 1 >> \
         >> >> >>";
    let px = shaded(resources, "/Pattern cs /P0 scn 0 0 20 20 re f");
    let (r_in, _, b_in) = px(10, 10);
    let (r_out, _, b_out) = px(16, 10);
    assert!(r_in > 200 && b_in < 55, "the centre is the start colour, got {:?}", px(10, 10));
    assert!(r_out < 80 && b_out > 175, "the rim approaches the end colour, got {:?}", px(16, 10));
    assert_eq!(px(19, 10), (255, 255, 255), "outside the outer circle stays unpainted");
}

#[test]
fn a_stitching_function_picks_the_subfunction_for_the_parameter() {
    // Red to green over the first half of the axis, green to blue over
    // the second. Just past the middle the colour must be nearly pure
    // green — that only happens if the stitch selected the second
    // subfunction AND re-encoded the parameter into its domain.
    let resources = "/Pattern << /P0 << /PatternType 2 /Shading << /ShadingType 2 \
         /ColorSpace /DeviceRGB /Coords [0 0 20 0] /Extend [true true] \
         /Function << /FunctionType 3 /Domain [0 1] /Bounds [0.5] \
         /Encode [0 1 0 1] /Functions [ \
           << /FunctionType 2 /Domain [0 1] /C0 [1 0 0] /C1 [0 1 0] /N 1 >> \
           << /FunctionType 2 /Domain [0 1] /C0 [0 1 0] /C1 [0 0 1] /N 1 >> \
         ] >> >> >> >>";
    let px = shaded(resources, "/Pattern cs /P0 scn 0 0 20 20 re f");
    let (r, g, b) = px(10, 10);
    assert!(g > 200, "just past the stitch the colour is green, got {:?}", (r, g, b));
    assert!(r < 30 && b < 30, "with neither endpoint colour left, got {:?}", (r, g, b));
    let (r_mid, g_mid, _) = px(5, 10);
    assert!(r_mid > 80 && g_mid > 80, "the first half blends red into green");
}

#[test]
fn the_sh_operator_paints_the_current_clip() {
    // `sh` has no path: it covers the clip, in the current CTM.
    let resources = format!(
        "/Shading << /Sh0 << /ShadingType 2 /ColorSpace /DeviceRGB \
         /Coords [0 0 20 0] /Extend [true true] {RED_TO_BLUE} >> >>"
    );
    let px = shaded(&resources, "q 0 0 10 20 re W n /Sh0 sh Q");
    // Device column 5 samples at user x 5.5, so s = 5.5 / 20 = 0.275:
    // red 0.725 and blue 0.275 of full scale.
    let (r, _, b) = px(5, 10);
    assert!((183..=187).contains(&r) && (68..=72).contains(&b),
            "inside the clip the shading paints its own colour, got {:?}", px(5, 10));
    assert_eq!(px(15, 10), (255, 255, 255), "outside it nothing is painted");
}

#[test]
fn an_unsupported_shading_type_paints_nothing() {
    // ShadingType 1 (function-based) is out of scope, and the anchor
    // requires its area left alone rather than filled black.
    let resources = "/Pattern << /P0 << /PatternType 2 /Shading << /ShadingType 1 \
         /ColorSpace /DeviceRGB /Domain [0 1 0 1] \
         /Function << /FunctionType 2 /Domain [0 1] /C0 [1 0 0] /C1 [0 0 1] /N 1 >> \
         >> >> >>";
    let px = shaded(resources, "/Pattern cs /P0 scn 0 0 20 20 re f");
    assert_eq!(px(10, 10), (255, 255, 255));
}

// ============================================================
// Tiling patterns (§8.7.3). The corpus fixture `tiling_pattern` pins
// the rendered result against an oracle; these pin the grid — where
// the tiles land, what bounds them, and what is skipped.
//
// The page is 20x20 at 72 dpi, so one user unit is one device pixel
// and user y maps to device 20 - y.
// ============================================================

/// A 20x20 page whose /Pattern resource /P0 is the tiling pattern in
/// object 5, carrying `entries` and the cell content `cell`.
fn pdf_with_tiling(entries: &str, cell: &str, content: &str) -> Vec<u8> {
    let mut objects: Vec<Vec<u8>> = Vec::new();
    objects.push(b"<< /Type /Catalog /Pages 2 0 R >>".to_vec());
    objects.push(b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec());
    objects.push(
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 20 20] /Contents 4 0 R \
          /Resources << /Pattern << /P0 5 0 R >> >> >>"
            .to_vec(),
    );
    let mut stream = format!("<< /Length {} >>\nstream\n", content.len()).into_bytes();
    stream.extend_from_slice(content.as_bytes());
    stream.extend_from_slice(b"\nendstream");
    objects.push(stream);
    let mut pattern = format!(
        "<< /Type /Pattern /PatternType 1 {entries} /Length {} >>\nstream\n",
        cell.len()
    )
    .into_bytes();
    pattern.extend_from_slice(cell.as_bytes());
    pattern.extend_from_slice(b"\nendstream");
    objects.push(pattern);

    let mut out = b"%PDF-1.7\n".to_vec();
    let mut offsets = Vec::new();
    for (i, body) in objects.iter().enumerate() {
        offsets.push(out.len());
        out.extend_from_slice(format!("{} 0 obj\n", i + 1).as_bytes());
        out.extend_from_slice(body);
        out.extend_from_slice(b"\nendobj\n");
    }
    let startxref = out.len();
    out.extend_from_slice(format!("xref\n0 {}\n", objects.len() + 1).as_bytes());
    out.extend_from_slice(b"0000000000 65535 f \n");
    for off in &offsets {
        out.extend_from_slice(format!("{off:010} 00000 n \n").as_bytes());
    }
    out.extend_from_slice(
        format!(
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{startxref}\n%%EOF\n",
            objects.len() + 1
        )
        .as_bytes(),
    );
    out
}

/// Render a tiled page: a pixel reader plus the count of red pixels.
fn tiled(entries: &str, cell: &str, content: &str)
    -> (impl Fn(usize, usize) -> (u8, u8, u8), usize)
{
    let pdf = pdf_with_tiling(entries, cell, content);
    let page = page_renderer::render(&pdf, 0, 72.0, false).expect("render");
    let w = page.width as usize;
    let red = page
        .pixels
        .chunks_exact(4)
        .filter(|px| px[0] > 200 && px[1] < 55 && px[2] < 55)
        .count();
    let pixels = page.pixels;
    (
        move |x: usize, y: usize| {
            let i = (y * w + x) * 4;
            (pixels[i], pixels[i + 1], pixels[i + 2])
        },
        red,
    )
}

/// A 10x10 cell holding a red 5x5 square in its lower-left corner.
const RED_CELL: &str = "1 0 0 rg 0 0 5 5 re f";
const CELL_10: &str = "/PaintType 1 /TilingType 1 /BBox [0 0 10 10] \
                       /XStep 10 /YStep 10 /Resources << >>";

#[test]
fn a_tiling_pattern_repeats_its_cell_across_the_fill() {
    // Four 10x10 tiles cover the page; each paints 25 red pixels in
    // its lower-left quarter, at user y 0..5 and 10..15.
    let (px, red) = tiled(CELL_10, RED_CELL, "/Pattern cs /P0 scn 0 0 20 20 re f");
    assert_eq!(red, 100, "four tiles of 25 red pixels each");
    assert_eq!(px(2, 17), (255, 0, 0), "tile (0,0)");
    assert_eq!(px(12, 17), (255, 0, 0), "tile (1,0)");
    assert_eq!(px(2, 7), (255, 0, 0), "tile (0,1)");
    assert_eq!(px(12, 7), (255, 0, 0), "tile (1,1)");
    assert_eq!(px(7, 17), (255, 255, 255), "the rest of a cell stays empty");
    assert_eq!(px(2, 12), (255, 255, 255), "and so does the gap in y");
}

#[test]
fn the_fill_path_bounds_the_tiling() {
    // The same pattern through a half-width path: only the left column
    // of tiles may paint, and nothing crosses x = 10.
    let (px, red) = tiled(CELL_10, RED_CELL, "/Pattern cs /P0 scn 0 0 10 20 re f");
    assert_eq!(red, 50, "two tiles survive the path");
    assert_eq!(px(2, 17), (255, 0, 0), "inside the path the cell paints");
    assert_eq!(px(12, 17), (255, 255, 255), "outside it nothing does");
}

#[test]
fn the_pattern_matrix_shifts_the_grid() {
    // /Matrix moves pattern space 5 units right, so the squares sit at
    // user x 5..10 and 15..20 instead of 0..5 and 10..15.
    let entries = "/PaintType 1 /TilingType 1 /BBox [0 0 10 10] /XStep 10 /YStep 10 \
                   /Matrix [1 0 0 1 5 0] /Resources << >>";
    let (px, red) = tiled(entries, RED_CELL, "/Pattern cs /P0 scn 0 0 20 20 re f");
    assert_eq!(red, 100, "the shift must not change how much is painted");
    assert_eq!(px(7, 17), (255, 0, 0), "the grid moved with the matrix");
    assert_eq!(px(2, 17), (255, 255, 255), "and left its old phase");
}

#[test]
fn a_step_wider_than_the_cell_leaves_gaps() {
    // /XStep and /YStep of 20 put one tile on the page, not four.
    let entries = "/PaintType 1 /TilingType 1 /BBox [0 0 10 10] \
                   /XStep 20 /YStep 20 /Resources << >>";
    let (px, red) = tiled(entries, RED_CELL, "/Pattern cs /P0 scn 0 0 20 20 re f");
    assert_eq!(red, 25, "a single tile");
    assert_eq!(px(2, 17), (255, 0, 0));
    assert_eq!(px(12, 17), (255, 255, 255), "the step skipped past this tile");
    assert_eq!(px(2, 7), (255, 255, 255));
}

#[test]
fn an_uncoloured_pattern_is_skipped() {
    // PaintType 2 is stencilled in the caller's colour, which is out of
    // scope; it must paint nothing rather than something wrong. The
    // coloured twin shows the test is not just watching a blank page.
    let coloured = "/PaintType 1 /TilingType 1 /BBox [0 0 10 10] \
                    /XStep 10 /YStep 10 /Resources << >>";
    let uncoloured = "/PaintType 2 /TilingType 1 /BBox [0 0 10 10] \
                      /XStep 10 /YStep 10 /Resources << >>";
    let fill = "/Pattern cs /P0 scn 0 0 20 20 re f";
    let (px_on, red_on) = tiled(coloured, RED_CELL, fill);
    let (px_off, red_off) = tiled(uncoloured, RED_CELL, fill);
    assert_eq!((px_on(2, 17), red_on), ((255, 0, 0), 100), "PaintType 1 paints");
    assert_eq!((px_off(2, 17), red_off), ((255, 255, 255), 0), "PaintType 2 does not");
}

#[test]
fn a_self_referencing_pattern_terminates() {
    // The cell paints, then fills with the pattern it is itself part of
    // — the page's /P0, since the cell brings no resources of its own.
    // Without the depth bound this recurses until the stack gives out.
    let entries = "/PaintType 1 /TilingType 1 /BBox [0 0 10 10] /XStep 10 /YStep 10";
    let cell = "1 0 0 rg 0 0 5 5 re f /Pattern cs /P0 scn 0 0 10 10 re f";
    let (px, red) = tiled(entries, cell, "/Pattern cs /P0 scn 0 0 20 20 re f");
    assert!(red >= 100, "the cell painted at every level it reached, got {red}");
    assert_eq!(px(2, 17), (255, 0, 0), "and the outermost tile is there");
}

// ============================================================
// Composite (Type0 / CID) fonts (§9.7). The corpus fixture `cid_text`
// pins the rendered result against an oracle; these pin the plumbing —
// two-byte codes, the CID-to-glyph map, and the CID metrics.
//
// The probe font is `buhid.ttf`: 1000 units per em, glyph 5 has an
// outline and glyph 2 is blank, so "did the right glyph get drawn?"
// needs no assumption about shapes.
// ============================================================

const INKED_GID: u16 = 5;
const BLANK_GID: u16 = 2;

fn probe_font_program() -> Vec<u8> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("truetype")
        .join("buhid.ttf");
    std::fs::read(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()))
}

/// A 60x30 page carrying a Type0 font as /F0. `descendant_extra` adds
/// the descendant's own entries (/CIDToGIDMap, /W, /DW); `cid_map`, when
/// given, becomes the /CIDToGIDMap stream in object 9.
fn pdf_with_cid_font(
    encoding: &str,
    descendant_extra: &str,
    cid_map: Option<&[u8]>,
    content: &str,
) -> Vec<u8> {
    let program = probe_font_program();
    let mut objects: Vec<Vec<u8>> = Vec::new();
    objects.push(b"<< /Type /Catalog /Pages 2 0 R >>".to_vec());
    objects.push(b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec());
    objects.push(
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 60 30] /Contents 4 0 R \
          /Resources << /Font << /F0 5 0 R >> >> >>"
            .to_vec(),
    );
    let mut stream = format!("<< /Length {} >>\nstream\n", content.len()).into_bytes();
    stream.extend_from_slice(content.as_bytes());
    stream.extend_from_slice(b"\nendstream");
    objects.push(stream);
    objects.push(
        format!(
            "<< /Type /Font /Subtype /Type0 /BaseFont /Probe /Encoding {encoding} \
             /DescendantFonts [6 0 R] >>"
        )
        .into_bytes(),
    );
    objects.push(
        format!(
            "<< /Type /Font /Subtype /CIDFontType2 /BaseFont /Probe /FontDescriptor 7 0 R \
             /CIDSystemInfo << /Registry (Adobe) /Ordering (Identity) /Supplement 0 >> \
             {descendant_extra} >>"
        )
        .into_bytes(),
    );
    objects.push(
        b"<< /Type /FontDescriptor /FontName /Probe /FontFile2 8 0 R /Flags 4 \
          /FontBBox [0 0 1000 1000] /ItalicAngle 0 /Ascent 800 /Descent -200 \
          /CapHeight 700 /StemV 80 >>"
            .to_vec(),
    );
    let mut ff = format!(
        "<< /Length {} /Length1 {} >>\nstream\n",
        program.len(),
        program.len()
    )
    .into_bytes();
    ff.extend_from_slice(&program);
    ff.extend_from_slice(b"\nendstream");
    objects.push(ff);
    if let Some(map) = cid_map {
        let mut obj = format!("<< /Length {} >>\nstream\n", map.len()).into_bytes();
        obj.extend_from_slice(map);
        obj.extend_from_slice(b"\nendstream");
        objects.push(obj);
    }

    let mut out = b"%PDF-1.7\n".to_vec();
    let mut offsets = Vec::new();
    for (i, body) in objects.iter().enumerate() {
        offsets.push(out.len());
        out.extend_from_slice(format!("{} 0 obj\n", i + 1).as_bytes());
        out.extend_from_slice(body);
        out.extend_from_slice(b"\nendobj\n");
    }
    let startxref = out.len();
    out.extend_from_slice(format!("xref\n0 {}\n", objects.len() + 1).as_bytes());
    out.extend_from_slice(b"0000000000 65535 f \n");
    for off in &offsets {
        out.extend_from_slice(format!("{off:010} 00000 n \n").as_bytes());
    }
    out.extend_from_slice(
        format!(
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{startxref}\n%%EOF\n",
            objects.len() + 1
        )
        .as_bytes(),
    );
    out
}

/// Ink statistics of a CID page: how many pixels took ink, and the
/// leftmost and rightmost inked columns.
fn cid_ink(
    encoding: &str,
    descendant_extra: &str,
    cid_map: Option<&[u8]>,
    content: &str,
) -> (usize, Option<usize>, Option<usize>) {
    let pdf = pdf_with_cid_font(encoding, descendant_extra, cid_map, content);
    let page = page_renderer::render(&pdf, 0, 72.0, false).expect("render");
    let w = page.width as usize;
    let (mut count, mut left, mut right) = (0usize, None, None);
    for (i, px) in page.pixels.chunks_exact(4).enumerate() {
        if px[0] < 200 || px[1] < 200 || px[2] < 200 {
            count += 1;
            let x = i % w;
            left = Some(left.map_or(x, |l: usize| l.min(x)));
            right = Some(right.map_or(x, |r: usize| r.max(x)));
        }
    }
    (count, left, right)
}

/// A /CIDToGIDMap stream: two big-endian bytes per CID.
fn cid_map_bytes(entries: &[(u32, u16)]) -> Vec<u8> {
    let highest = entries.iter().map(|(cid, _)| *cid).max().unwrap_or(0);
    let mut out = vec![0u8; (highest as usize + 1) * 2];
    for (cid, gid) in entries {
        out[*cid as usize * 2..*cid as usize * 2 + 2].copy_from_slice(&gid.to_be_bytes());
    }
    out
}

#[test]
fn an_identity_map_makes_the_cid_its_own_glyph() {
    // The same two-byte code shape, once naming an inked glyph and once
    // a blank one. Only the CID differs, so only the map can explain it.
    let inked = cid_ink(
        "/Identity-H",
        "/CIDToGIDMap /Identity /DW 1000",
        None,
        &format!("BT /F0 20 Tf 5 5 Td <{INKED_GID:04X}> Tj ET"),
    );
    let blank = cid_ink(
        "/Identity-H",
        "/CIDToGIDMap /Identity /DW 1000",
        None,
        &format!("BT /F0 20 Tf 5 5 Td <{BLANK_GID:04X}> Tj ET"),
    );
    assert!(inked.0 > 0, "glyph {INKED_GID} has an outline and must paint");
    assert_eq!(blank.0, 0, "glyph {BLANK_GID} is empty and must paint nothing");
}

#[test]
fn a_cid_to_gid_stream_redirects_the_glyph() {
    // CID 1 is deliberately NOT a glyph index here: the map decides.
    let to_inked = cid_map_bytes(&[(1, INKED_GID)]);
    let to_blank = cid_map_bytes(&[(1, BLANK_GID)]);
    let content = "BT /F0 20 Tf 5 5 Td <0001> Tj ET";
    let via_inked = cid_ink("/Identity-H", "/CIDToGIDMap 9 0 R /DW 1000",
                            Some(&to_inked), content);
    let via_blank = cid_ink("/Identity-H", "/CIDToGIDMap 9 0 R /DW 1000",
                            Some(&to_blank), content);
    assert!(via_inked.0 > 0, "the map sent CID 1 to an inked glyph");
    assert_eq!(via_blank.0, 0, "the same CID now lands on a blank one");
}

#[test]
fn the_w_array_sets_the_advance_between_cids() {
    // Two of the same glyph. At 20 pt a width of 1000 advances 20 user
    // units and a width of 200 advances 4, so the second glyph — and
    // with it the rightmost ink — sits 16 pixels further right.
    let content = format!("BT /F0 20 Tf 5 5 Td <{INKED_GID:04X}{INKED_GID:04X}> Tj ET");
    let wide = cid_ink("/Identity-H",
                       &format!("/CIDToGIDMap /Identity /W [{INKED_GID} [1000]]"),
                       None, &content);
    let narrow = cid_ink("/Identity-H",
                         &format!("/CIDToGIDMap /Identity /W [{INKED_GID} [200]]"),
                         None, &content);
    let (Some(wide_right), Some(narrow_right)) = (wide.2, narrow.2) else {
        panic!("both renderings must put ink on the page");
    };
    assert_eq!(wide.1, narrow.1, "the first glyph starts in the same place");
    assert_eq!(wide_right - narrow_right, 16,
               "800/1000 of 20 points between the two advances");
}

#[test]
fn the_dw_default_applies_where_the_w_array_is_silent() {
    // No /W at all: every CID takes /DW.
    let content = format!("BT /F0 20 Tf 5 5 Td <{INKED_GID:04X}{INKED_GID:04X}> Tj ET");
    let wide = cid_ink("/Identity-H", "/CIDToGIDMap /Identity /DW 1000", None, &content);
    let narrow = cid_ink("/Identity-H", "/CIDToGIDMap /Identity /DW 200", None, &content);
    let (Some(wide_right), Some(narrow_right)) = (wide.2, narrow.2) else {
        panic!("both renderings must put ink on the page");
    };
    assert_eq!(wide_right - narrow_right, 16, "the default width drives the advance");
}

#[test]
fn word_spacing_never_applies_to_a_two_byte_code() {
    // §9.3.3: Tw applies to the single-byte code 32, never to a CID
    // that happens to be 32. The map sends CID 32 to an inked glyph so
    // the second glyph's position is visible.
    let map = cid_map_bytes(&[(32, INKED_GID)]);
    let content = |tw: i32| {
        format!("BT /F0 20 Tf {tw} Tw 5 5 Td <0020{INKED_GID:04X}> Tj ET")
    };
    let without = cid_ink("/Identity-H", "/CIDToGIDMap 9 0 R /DW 500",
                          Some(&map), &content(0));
    let with = cid_ink("/Identity-H", "/CIDToGIDMap 9 0 R /DW 500",
                       Some(&map), &content(10));
    assert!(without.0 > 0, "the CID-32 glyph paints");
    assert_eq!(without.2, with.2, "word spacing must not move the next glyph");
}

#[test]
fn an_unsupported_encoding_paints_nothing() {
    // A CMap this renderer does not read must not fall back to Latin
    // glyphs for what are CIDs. The Identity twin shows the test is not
    // merely watching a blank page.
    let content = format!("BT /F0 20 Tf 5 5 Td <{INKED_GID:04X}> Tj ET");
    let identity = cid_ink("/Identity-H", "/CIDToGIDMap /Identity /DW 1000", None, &content);
    let other = cid_ink("/UniJIS-UCS2-H", "/CIDToGIDMap /Identity /DW 1000", None, &content);
    assert!(identity.0 > 0, "Identity-H paints");
    assert_eq!(other.0, 0, "an unread CMap paints nothing");
}

// ============================================================
// Corpus fixtures the oracle does not cover
// ============================================================

/// Five fixtures ship in the shared corpus with no `.pixels`
/// expectation, so `lossy_oracle_agreement_over_corpus` — which walks
/// the expectations, not the PDFs — never looks at them. That silence
/// hid a blank page: `fontfile_truetype` puts a real sfnt under
/// `/FontFile`, the slot §9.8.1 reserves for Type1, and the loader read
/// only `/FontFile2`.
///
/// Nothing here can compare against an oracle, so the claim is the
/// weakest one worth making: the page renders, and it is not blank.
/// That is exactly the claim the silence was hiding.
const UNCHECKED_FIXTURES: &[&str] = &[
    "clip_rect_lines",
    "fontfile_truetype",
    "image_clip",
    "nested_forms",
    "type3_bitmap",
];

#[test]
fn the_fixtures_without_an_oracle_still_put_ink_on_the_page() {
    let mut blank = Vec::new();
    for name in UNCHECKED_FIXTURES {
        let path = corpus_dir().join(format!("{name}.pdf"));
        let bytes = std::fs::read(&path)
            .unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
        let page = page_renderer::render(&bytes, 0, 72.0, false)
            .unwrap_or_else(|e| panic!("{name} failed to render: {e}"));
        let inked = page
            .pixels
            .chunks_exact(4)
            .filter(|px| px[0] < 200 || px[1] < 200 || px[2] < 200)
            .count();
        if inked == 0 {
            blank.push(*name);
        }
    }
    assert!(blank.is_empty(), "these fixtures rendered blank: {blank:?}");
}

// ============================================================
// Inline images (§8.9.7), image masks (§8.9.6.2) and Type3 fonts
// (§9.6.5). The corpus fixture `type3_bitmap` needs all three at once —
// a procedural glyph that paints an inline stencil in the text colour —
// so these pin each of them on its own.
//
// Inline samples go through /AHx so the content stream stays text.
// ============================================================

/// The RGB triple at a device pixel of a 20x20 page built from
/// `content` alone.
fn content_pixel(content: &str, x: usize, y: usize) -> (u8, u8, u8) {
    let page = page_renderer::render(&pdf_with_content(content), 0, 72.0, false)
        .expect("render");
    let i = (y * page.width as usize + x) * 4;
    (page.pixels[i], page.pixels[i + 1], page.pixels[i + 2])
}

#[test]
fn an_inline_image_paints_its_samples() {
    // A 2x2 RGB image over the whole page: each sample covers a 10x10
    // quadrant, source row 0 at the top.
    let content = "q 20 0 0 20 0 0 cm \
                   BI /W 2 /H 2 /CS /RGB /BPC 8 /F /AHx ID \
                   ff000000ff000000ffffff00> EI Q";
    assert_eq!(content_pixel(content, 5, 5), (255, 0, 0), "top-left sample");
    assert_eq!(content_pixel(content, 15, 5), (0, 255, 0), "top-right sample");
    assert_eq!(content_pixel(content, 5, 15), (0, 0, 255), "bottom-left sample");
    assert_eq!(content_pixel(content, 15, 15), (255, 255, 0), "bottom-right sample");
}

#[test]
fn an_image_mask_paints_the_fill_colour_through_its_stencil() {
    // 1 bpc, one byte per row: row 0 is 0 1, row 1 is 1 0. Under the
    // default /Decode [0 1] a 0 sample paints, and what it paints is
    // the fill colour — not the grey the samples would decode to.
    let content = "1 0 0 rg q 20 0 0 20 0 0 cm \
                   BI /W 2 /H 2 /IM true /BPC 1 /F /AHx ID 4080> EI Q";
    assert_eq!(content_pixel(content, 5, 5), (255, 0, 0), "a set sample takes the fill");
    assert_eq!(content_pixel(content, 15, 5), (255, 255, 255), "a clear one stays empty");
    assert_eq!(content_pixel(content, 5, 15), (255, 255, 255));
    assert_eq!(content_pixel(content, 15, 15), (255, 0, 0));
}

#[test]
fn a_decode_array_inverts_an_image_mask() {
    let content = "1 0 0 rg q 20 0 0 20 0 0 cm \
                   BI /W 2 /H 2 /IM true /BPC 1 /D [1 0] /F /AHx ID 4080> EI Q";
    assert_eq!(content_pixel(content, 5, 5), (255, 255, 255), "the polarity flipped");
    assert_eq!(content_pixel(content, 15, 5), (255, 0, 0));
}

/// A 100x100 page carrying a Type3 font as /F1, whose only glyph /gA is
/// code 65 and runs `proc_body`.
fn pdf_with_type3(
    font_matrix: &str,
    widths: &str,
    proc_body: &str,
    content: &str,
) -> Vec<u8> {
    let mut objects: Vec<Vec<u8>> = Vec::new();
    objects.push(b"<< /Type /Catalog /Pages 2 0 R >>".to_vec());
    objects.push(b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec());
    objects.push(
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Contents 4 0 R \
          /Resources << /Font << /F1 5 0 R >> >> >>"
            .to_vec(),
    );
    let mut stream = format!("<< /Length {} >>\nstream\n", content.len()).into_bytes();
    stream.extend_from_slice(content.as_bytes());
    stream.extend_from_slice(b"\nendstream");
    objects.push(stream);
    objects.push(
        format!(
            "<< /Type /Font /Subtype /Type3 /FontBBox [0 0 1000 1000] \
             /FontMatrix {font_matrix} /CharProcs 6 0 R /Encoding 8 0 R \
             /FirstChar 65 /LastChar 65 /Widths {widths} /Resources << >> >>"
        )
        .into_bytes(),
    );
    objects.push(b"<< /gA 7 0 R >>".to_vec());
    let mut proc_obj =
        format!("<< /Length {} >>\nstream\n", proc_body.len()).into_bytes();
    proc_obj.extend_from_slice(proc_body.as_bytes());
    proc_obj.extend_from_slice(b"\nendstream");
    objects.push(proc_obj);
    objects.push(b"<< /Type /Encoding /Differences [65 /gA] >>".to_vec());

    let mut out = b"%PDF-1.7\n".to_vec();
    let mut offsets = Vec::new();
    for (i, body) in objects.iter().enumerate() {
        offsets.push(out.len());
        out.extend_from_slice(format!("{} 0 obj\n", i + 1).as_bytes());
        out.extend_from_slice(body);
        out.extend_from_slice(b"\nendobj\n");
    }
    let startxref = out.len();
    out.extend_from_slice(format!("xref\n0 {}\n", objects.len() + 1).as_bytes());
    out.extend_from_slice(b"0000000000 65535 f \n");
    for off in &offsets {
        out.extend_from_slice(format!("{off:010} 00000 n \n").as_bytes());
    }
    out.extend_from_slice(
        format!(
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{startxref}\n%%EOF\n",
            objects.len() + 1
        )
        .as_bytes(),
    );
    out
}

/// Ink count and the extreme inked columns of a Type3 page.
fn type3_ink(
    font_matrix: &str,
    widths: &str,
    proc_body: &str,
    content: &str,
) -> (usize, Option<usize>, Option<usize>) {
    let pdf = pdf_with_type3(font_matrix, widths, proc_body, content);
    let page = page_renderer::render(&pdf, 0, 72.0, false).expect("render");
    let w = page.width as usize;
    let (mut count, mut left, mut right) = (0usize, None, None);
    for (i, px) in page.pixels.chunks_exact(4).enumerate() {
        if px[0] < 200 || px[1] < 200 || px[2] < 200 {
            count += 1;
            let x = i % w;
            left = Some(left.map_or(x, |l: usize| l.min(x)));
            right = Some(right.map_or(x, |r: usize| r.max(x)));
        }
    }
    (count, left, right)
}

/// A glyph procedure that fills its whole 1000-unit cell.
const FILLED_CELL: &str = "1000 0 0 0 1000 1000 d1 0 0 1000 1000 re f";

#[test]
fn a_type3_glyph_runs_its_procedure() {
    // The cell is 1000 glyph units, /FontMatrix scales that to 1 unit of
    // text space, and 40 pt makes it a 40x40 square at (20, 20).
    let (ink, left, right) = type3_ink(
        "[0.001 0 0 0.001 0 0]",
        "[1000]",
        FILLED_CELL,
        "BT /F1 40 Tf 20 20 Td (A) Tj ET",
    );
    assert_eq!(ink, 1600, "a 40 by 40 square");
    assert_eq!((left, right), (Some(20), Some(59)), "placed where Td put it");
}

#[test]
fn the_font_matrix_scales_a_type3_glyph() {
    // Half the matrix, a quarter of the area.
    let (ink, left, _) = type3_ink(
        "[0.0005 0 0 0.0005 0 0]",
        "[1000]",
        FILLED_CELL,
        "BT /F1 40 Tf 20 20 Td (A) Tj ET",
    );
    assert_eq!(ink, 400, "a 20 by 20 square");
    assert_eq!(left, Some(20), "the origin does not move with the scale");
}

#[test]
fn a_type3_advance_goes_through_the_font_matrix() {
    // /Widths is in glyph space, so the advance is width *
    // FontMatrix.a * Tfs — NOT the 1/1000 a simple font uses. The two
    // only differ when the matrix is not 0.001, so the halved matrix
    // here is what gives the test its teeth: through the matrix the
    // 20-unit glyph advances 20 and the pair ends at column 59, while
    // a hard-coded 1/1000 would advance 40 and end at 79.
    let halved = type3_ink("[0.0005 0 0 0.0005 0 0]", "[1000]", FILLED_CELL,
                           "BT /F1 40 Tf 20 20 Td (AA) Tj ET");
    let plain = type3_ink("[0.001 0 0 0.001 0 0]", "[1000]", FILLED_CELL,
                          "BT /F1 40 Tf 20 20 Td (AA) Tj ET");
    assert_eq!(halved.1, Some(20), "both start where Td put them");
    assert_eq!(halved.2, Some(59), "20-unit glyphs, 20 units apart");
    assert_eq!(plain.1, Some(20));
    assert_eq!(plain.2, Some(99), "40-unit glyphs run to the page edge");
}

#[test]
fn a_type3_bitmap_glyph_paints_in_the_text_colour() {
    // The ordinary bitmap-font shape: the procedure declares itself a
    // shape-only glyph with d1 and paints an inline stencil. The colour
    // has to come from the text state — a stencil carries none.
    let body = "1000 0 0 0 1000 1000 d1 q 1000 0 0 1000 0 0 cm \
                BI /W 2 /H 2 /IM true /BPC 1 /D [1 0] /F /AHx ID c0c0> EI Q";
    let pdf = pdf_with_type3("[0.001 0 0 0.001 0 0]", "[1000]", body,
                             "BT 0 0 1 rg /F1 40 Tf 20 20 Td (A) Tj ET");
    let page = page_renderer::render(&pdf, 0, 72.0, false).expect("render");
    let at = |x: usize, y: usize| {
        let i = (y * page.width as usize + x) * 4;
        (page.pixels[i], page.pixels[i + 1], page.pixels[i + 2])
    };
    // /D [1 0] makes the set bits paint, and every bit of the left
    // column pair is set, so the glyph is a solid blue square.
    assert_eq!(at(30, 50), (0, 0, 255), "the glyph takes the text colour");
    assert_eq!(at(70, 50), (255, 255, 255), "and stays inside its cell");
}

/// A one-page PDF drawing text in a simple font with no embedded
/// program. `flags` is the `/FontDescriptor /Flags` value, whose bit 2
/// is the serif bit.
fn pdf_with_nonembedded_font(base_font: &str, flags: i32) -> Vec<u8> {
    let content = "BT /F1 48 Tf 10 40 Td (HHHH) Tj ET";
    let mut objects: Vec<Vec<u8>> = Vec::new();
    objects.push(b"<< /Type /Catalog /Pages 2 0 R >>".to_vec());
    objects.push(b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec());
    objects.push(
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 100] \
          /Contents 4 0 R /Resources << /Font << /F1 5 0 R >> >> >>"
            .to_vec(),
    );
    let mut content_obj = format!("<< /Length {} >>\nstream\n", content.len()).into_bytes();
    content_obj.extend_from_slice(content.as_bytes());
    content_obj.extend_from_slice(b"\nendstream");
    objects.push(content_obj);
    objects.push(
        format!(
            "<< /Type /Font /Subtype /TrueType /BaseFont /{base_font} \
             /FirstChar 32 /LastChar 126 /FontDescriptor 6 0 R >>"
        )
        .into_bytes(),
    );
    objects.push(
        format!(
            "<< /Type /FontDescriptor /FontName /{base_font} /Flags {flags} \
             /ItalicAngle 0 >>"
        )
        .into_bytes(),
    );

    let mut out = b"%PDF-1.7\n".to_vec();
    let mut offsets = Vec::with_capacity(objects.len());
    for (i, body) in objects.iter().enumerate() {
        offsets.push(out.len());
        out.extend_from_slice(format!("{} 0 obj\n", i + 1).as_bytes());
        out.extend_from_slice(body);
        out.extend_from_slice(b"\nendobj\n");
    }
    let startxref = out.len();
    out.extend_from_slice(format!("xref\n0 {}\n", objects.len() + 1).as_bytes());
    out.extend_from_slice(b"0000000000 65535 f \n");
    for off in &offsets {
        out.extend_from_slice(format!("{off:010} 00000 n \n").as_bytes());
    }
    out.extend_from_slice(
        format!(
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{startxref}\n%%EOF\n",
            objects.len() + 1
        )
        .as_bytes(),
    );
    out
}

/// How many pixels of the rendered page took ink. (`inked` above
/// answers for a content string; this one renders whole bytes.)
fn inked_pixels_of(pdf: &[u8]) -> usize {
    let page = page_renderer::render(pdf, 0, 72.0, false).expect("render");
    page.pixels
        .chunks_exact(4)
        .filter(|px| px[0] < 200 || px[1] < 200 || px[2] < 200)
        .count()
}

/// The renderer contract requires ANY non-embedded simple font to be
/// mapped onto the nearest Standard14 face, "so a non-Standard14
/// non-embedded font renders in a metric-class substitute instead of
/// blank". It was not: only the fourteen exact names resolved, so a
/// document's /Verdana or /Arial painted nothing at all.
#[test]
fn a_non_embedded_font_outside_the_standard_14_still_paints() {
    let standard = inked_pixels_of(&pdf_with_nonembedded_font("Helvetica", 32));
    assert!(standard > 0, "the Standard14 case has always worked");

    for name in ["Arial", "Verdana", "Calibri", "WhateverSans"] {
        let ink = inked_pixels_of(&pdf_with_nonembedded_font(name, 32));
        assert!(ink > 0, "/{name} must paint through its metric class");
    }

    // The serif bit picks the family when the name does not say, so the
    // two render through different faces and differ in ink.
    let sans = inked_pixels_of(&pdf_with_nonembedded_font("WhateverElse", 32));
    let serif = inked_pixels_of(&pdf_with_nonembedded_font("WhateverElse", 34));
    assert!(sans > 0 && serif > 0, "both families paint");
    assert_ne!(sans, serif, "the serif bit chooses a different face");
}

/// Symbol and ZapfDingbats keep the documented gap: substituting a text
/// face for a symbol font would paint the wrong glyphs rather than none.
#[test]
fn the_two_symbol_fonts_keep_their_documented_gap() {
    assert_eq!(inked_pixels_of(&pdf_with_nonembedded_font("Symbol", 4)), 0);
    assert_eq!(inked_pixels_of(&pdf_with_nonembedded_font("ZapfDingbats", 4)), 0);
}
