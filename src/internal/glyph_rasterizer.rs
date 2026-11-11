// Adaptive cubic Bezier flattening + delegation to the polygon rasterizer.
// PDF 32000-1:2008 §9 (text rendering); Adobe Type 2 charstring (rrcurveto
// cubic origin — c1 + c2 + end); Foley/van Dam §11.2 (De Casteljau
// midpoint subdivision).
//
// Outline-format-agnostic — the bridge between any outline producer
// (cff today; truetype glyf/loca v2; any future format) and the polygon
// rasterizer. This primitive owns curve flattening; outline extraction
// belongs to the caller.
//
// Composes:
//   * transform::{Matrix, Point, apply}
//   * colorspace::ColorRgb
//   * rasterizer::{Path, PathSegment, SegmentKind, Raster, FillRule, fill}
//
// Algorithm: each Curve segment carries (start = prior endpoint, c1, c2,
// end); De Casteljau midpoint subdivision recurses while the maximum
// perpendicular distance from c1 / c2 to the chord (start → end) exceeds
// FLATNESS = 0.25 device pixels, capped at MAX_DEPTH = 12. The CTM is
// applied at flattening time so the flatness threshold is in device
// space. Each terminal subdivision emits one Line in the rasterizer::Path,
// which we then hand to rasterizer::fill with FillRule::NonZero — the
// universal convention for fonts.

use std::fmt;

use super::colorspace::ColorRgb;
use super::rasterizer::{
    fill, FillRule, Path, PathSegment, Raster, SegmentKind,
};
use super::transform::{apply, Matrix, Point};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum OutlineOp {
    Move = 0,
    Line = 1,
    Curve = 2,
    Close = 3,
}

/// One outline segment. `pts[0]` is the endpoint for Move / Line, and
/// the first control point for Curve; `pts[1]` is the second control
/// point for Curve; `pts[2]` is the endpoint for Curve. Close ignores
/// all three.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OutlineSegment {
    pub op: OutlineOp,
    pub pts: [Point; 3],
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Outline {
    pub segments: Vec<OutlineSegment>,
}

/// Standard lossy-primitive return shape — mirrors
/// `rasterizer::DecodedImage` so the gate template drives both the same
/// way. `components` is always 4 (RGBA8).
#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(dead_code)] // retained for a future-phase public-API binding
pub struct DecodedImage {
    pub width: u32,
    pub height: u32,
    pub components: u8,
    pub pixels: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(dead_code)] // retained for a future-phase public-API binding
pub enum Error {
    Truncated,
    BadMagic,
    UnsupportedVersion(u8),
    BadSegmentKind(u8),
    ZeroDimension,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Truncated => write!(
                f, "glyph_rasterizer: truncated input"),
            Error::BadMagic => write!(
                f, "glyph_rasterizer: bad magic (expected 0xCB)"),
            Error::UnsupportedVersion(v) => write!(
                f, "glyph_rasterizer: unsupported version {v} (expected 1)"),
            Error::BadSegmentKind(v) => write!(
                f, "glyph_rasterizer: bad segment kind {v} (expected 0..=3)"),
            Error::ZeroDimension => write!(
                f, "glyph_rasterizer: width or height is zero"),
        }
    }
}

impl std::error::Error for Error {}

/// Maximum perpendicular distance from any control point to the chord
/// (start → end) below which a cubic is considered "flat enough" to emit
/// as a single line. 0.25 device pixels is sub-rasterizer-resolution
/// (the 4×4 super-sampler resolves at 1/4 pixel) and keeps PSNR ≥ 40 dB
/// on the gate corpus comfortably.
const FLATNESS: f64 = 0.25;

/// Recursion depth cap. At 12, a curve has been subdivided into 4096
/// pieces — adequate for any sane glyph outline; serves as a safety net
/// against pathological self-intersecting input.
const MAX_DEPTH: u32 = 12;

/// Rasterise `outline` into `dst` using NonZero winding (the universal
/// convention for fonts). Total — there is no error condition for
/// well-formed input. Empty outlines and out-of-bounds outlines are
/// handled by `rasterizer::fill`'s per-pixel coverage check.
pub fn draw(
    dst: &mut Raster,
    outline: &Outline,
    ctm: Matrix,
    color: ColorRgb,
    alpha: f64,
    clip: Option<&super::rasterizer::ClipRect>,
) {
    let path = flatten_outline(outline, ctm);
    // CTM is already absorbed into the flattened path's coordinates.
    // Hand identity to `rasterizer::fill` so it does not double-apply.
    // The clip passes straight through: a glyph is bounded by the same
    // region as any other painting operator.
    fill(dst, &path, identity_matrix(), color, alpha, FillRule::NonZero, clip);
}

/// Test-only entry point used by the lossy_oracle_agreement freeze gate.
/// Parses `input` per the binary fixture wire format documented in
/// `Feedstock/PDF/Spec/glyph_rasterizer.yaml`, builds the typed Outline /
/// CTM / Color, runs `draw` on a freshly zeroed Raster, and returns the
/// result wrapped as a DecodedImage so the gate template treats it
/// uniformly alongside the other lossy primitives.
#[allow(dead_code)] // retained for a future-phase public-API binding
pub fn decode(input: &[u8]) -> Result<DecodedImage, Error> {
    let mut r = Reader::new(input);

    let magic = r.read_u8()?;
    if magic != 0xCB {
        return Err(Error::BadMagic);
    }
    let version = r.read_u8()?;
    if version != 1 {
        return Err(Error::UnsupportedVersion(version));
    }
    let _reserved0 = r.read_u8()?;
    let _reserved1 = r.read_u8()?;
    let width = r.read_u32_le()?;
    let height = r.read_u32_le()?;
    if width == 0 || height == 0 {
        return Err(Error::ZeroDimension);
    }
    let a = r.read_f64_le()?;
    let b = r.read_f64_le()?;
    let c = r.read_f64_le()?;
    let d = r.read_f64_le()?;
    let e = r.read_f64_le()?;
    let f = r.read_f64_le()?;
    let ctm = Matrix::new(a, b, c, d, e, f);

    let cr = r.read_u8()?;
    let cg = r.read_u8()?;
    let cb = r.read_u8()?;
    let ca = r.read_u8()?;
    let color = ColorRgb {
        r: cr as f64 / 255.0,
        g: cg as f64 / 255.0,
        b: cb as f64 / 255.0,
    };
    let alpha = ca as f64 / 255.0;

    let num_segments = r.read_u32_le()? as usize;
    let mut segments = Vec::with_capacity(num_segments);
    for _ in 0..num_segments {
        let kind_byte = r.read_u8()?;
        let segment = match kind_byte {
            0 => {
                let x = r.read_f64_le()?;
                let y = r.read_f64_le()?;
                OutlineSegment {
                    op: OutlineOp::Move,
                    pts: [
                        Point { x, y },
                        Point { x: 0.0, y: 0.0 },
                        Point { x: 0.0, y: 0.0 },
                    ],
                }
            }
            1 => {
                let x = r.read_f64_le()?;
                let y = r.read_f64_le()?;
                OutlineSegment {
                    op: OutlineOp::Line,
                    pts: [
                        Point { x, y },
                        Point { x: 0.0, y: 0.0 },
                        Point { x: 0.0, y: 0.0 },
                    ],
                }
            }
            2 => {
                let c1x = r.read_f64_le()?;
                let c1y = r.read_f64_le()?;
                let c2x = r.read_f64_le()?;
                let c2y = r.read_f64_le()?;
                let ex = r.read_f64_le()?;
                let ey = r.read_f64_le()?;
                OutlineSegment {
                    op: OutlineOp::Curve,
                    pts: [
                        Point { x: c1x, y: c1y },
                        Point { x: c2x, y: c2y },
                        Point { x: ex, y: ey },
                    ],
                }
            }
            3 => OutlineSegment {
                op: OutlineOp::Close,
                pts: [Point { x: 0.0, y: 0.0 }; 3],
            },
            other => return Err(Error::BadSegmentKind(other)),
        };
        segments.push(segment);
    }
    let outline = Outline { segments };

    let pixel_count = (width as usize)
        .checked_mul(height as usize)
        .and_then(|n| n.checked_mul(4))
        .ok_or(Error::Truncated)?;
    let mut dst = Raster {
        pixels: vec![0u8; pixel_count],
        width,
        height,
    };
    draw(&mut dst, &outline, ctm, color, alpha, None);

    Ok(DecodedImage {
        width: dst.width,
        height: dst.height,
        components: 4,
        pixels: dst.pixels,
    })
}

// ---------------------------------------------------------------------
// Outline → rasterizer::Path (flattening + CTM applied).
// ---------------------------------------------------------------------

fn flatten_outline(outline: &Outline, ctm: Matrix) -> Path {
    let mut segments: Vec<PathSegment> = Vec::with_capacity(
        outline.segments.len() * 2);
    let mut cur: Option<Point> = None;
    let mut sub_start: Option<Point> = None;

    for seg in &outline.segments {
        match seg.op {
            OutlineOp::Move => {
                let p = apply(ctm, seg.pts[0]);
                segments.push(PathSegment {
                    kind: SegmentKind::Move,
                    pts: [p, Point { x: 0.0, y: 0.0 }],
                });
                cur = Some(p);
                sub_start = Some(p);
            }
            OutlineOp::Line => {
                if let Some(_c) = cur {
                    let p = apply(ctm, seg.pts[0]);
                    segments.push(PathSegment {
                        kind: SegmentKind::Line,
                        pts: [p, Point { x: 0.0, y: 0.0 }],
                    });
                    cur = Some(p);
                }
            }
            OutlineOp::Curve => {
                if let Some(c) = cur {
                    let c1 = apply(ctm, seg.pts[0]);
                    let c2 = apply(ctm, seg.pts[1]);
                    let end = apply(ctm, seg.pts[2]);
                    let mut pts: Vec<Point> = Vec::new();
                    flatten_cubic(c, c1, c2, end, &mut pts, 0);
                    let mut last = c;
                    for p in pts {
                        // Skip exact duplicates — the recursion can emit
                        // a final endpoint that equals the prior point on
                        // a degenerate (zero-length) input curve. Empty
                        // segments don't break correctness but avoid
                        // bloat in the path.
                        if p.x == last.x && p.y == last.y {
                            continue;
                        }
                        segments.push(PathSegment {
                            kind: SegmentKind::Line,
                            pts: [p, Point { x: 0.0, y: 0.0 }],
                        });
                        last = p;
                    }
                    cur = Some(end);
                }
            }
            OutlineOp::Close => {
                if let (Some(_c), Some(s)) = (cur, sub_start) {
                    segments.push(PathSegment {
                        kind: SegmentKind::Close,
                        pts: [Point { x: 0.0, y: 0.0 }; 2],
                    });
                    // Close moves the cursor back to the subpath start —
                    // a subsequent Curve continues from there. See
                    // semantic_traps.curve_after_close_subpath_start.
                    cur = Some(s);
                }
            }
        }
    }

    Path { segments }
}

fn flatten_cubic(
    p0: Point,
    p1: Point,
    p2: Point,
    p3: Point,
    out: &mut Vec<Point>,
    depth: u32,
) {
    let d1 = perpendicular_distance(p0, p1, p3);
    let d2 = perpendicular_distance(p0, p2, p3);
    if (d1.max(d2) <= FLATNESS) || depth >= MAX_DEPTH {
        out.push(p3);
        return;
    }
    let q0 = midpoint(p0, p1);
    let q1 = midpoint(p1, p2);
    let q2 = midpoint(p2, p3);
    let r0 = midpoint(q0, q1);
    let r1 = midpoint(q1, q2);
    let s = midpoint(r0, r1);
    flatten_cubic(p0, q0, r0, s, out, depth + 1);
    flatten_cubic(s, r1, q2, p3, out, depth + 1);
}

fn perpendicular_distance(p0: Point, p1: Point, p3: Point) -> f64 {
    let dx = p3.x - p0.x;
    let dy = p3.y - p0.y;
    let chord = (dx * dx + dy * dy).sqrt();
    if chord < 1e-12 {
        // Degenerate chord — fall back to ordinary distance from p0 to p1.
        let ex = p1.x - p0.x;
        let ey = p1.y - p0.y;
        return (ex * ex + ey * ey).sqrt();
    }
    ((p1.x - p0.x) * dy - (p1.y - p0.y) * dx).abs() / chord
}

fn midpoint(a: Point, b: Point) -> Point {
    Point {
        x: (a.x + b.x) / 2.0,
        y: (a.y + b.y) / 2.0,
    }
}

fn identity_matrix() -> Matrix {
    Matrix::new(1.0, 0.0, 0.0, 1.0, 0.0, 0.0)
}

// ---------------------------------------------------------------------
// Little-endian byte reader for the binary fixture format.
// ---------------------------------------------------------------------

#[allow(dead_code)] // retained for a future-phase public-API binding
struct Reader<'a> {
    buf: &'a [u8],
    pos: usize,
}

impl<'a> Reader<'a> {
    #[allow(dead_code)] // retained for a future-phase public-API binding
    fn new(buf: &'a [u8]) -> Self {
        Reader { buf, pos: 0 }
    }

    #[allow(dead_code)] // retained for a future-phase public-API binding
    fn read_u8(&mut self) -> Result<u8, Error> {
        if self.pos >= self.buf.len() {
            return Err(Error::Truncated);
        }
        let v = self.buf[self.pos];
        self.pos += 1;
        Ok(v)
    }

    #[allow(dead_code)] // retained for a future-phase public-API binding
    fn read_u32_le(&mut self) -> Result<u32, Error> {
        if self.pos + 4 > self.buf.len() {
            return Err(Error::Truncated);
        }
        let bytes: [u8; 4] = self.buf[self.pos..self.pos + 4]
            .try_into()
            .expect("4-byte slice");
        self.pos += 4;
        Ok(u32::from_le_bytes(bytes))
    }

    #[allow(dead_code)] // retained for a future-phase public-API binding
    fn read_f64_le(&mut self) -> Result<f64, Error> {
        if self.pos + 8 > self.buf.len() {
            return Err(Error::Truncated);
        }
        let bytes: [u8; 8] = self.buf[self.pos..self.pos + 8]
            .try_into()
            .expect("8-byte slice");
        self.pos += 8;
        Ok(f64::from_le_bytes(bytes))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn solid(width: u32, height: u32) -> Raster {
        Raster {
            pixels: vec![0u8; (width as usize) * (height as usize) * 4],
            width,
            height,
        }
    }

    #[test]
    fn empty_outline_leaves_canvas_zeroed() {
        let mut r = solid(4, 4);
        draw(&mut r, &Outline::default(), identity_matrix(),
             ColorRgb { r: 1.0, g: 1.0, b: 1.0 }, 1.0, None);
        assert!(r.pixels.iter().all(|&p| p == 0));
    }

    #[test]
    fn flatness_terminates_within_depth_cap() {
        // A wildly self-intersecting cubic — recursion still terminates.
        let mut out = Vec::new();
        flatten_cubic(
            Point { x: 0.0, y: 0.0 },
            Point { x: 1000.0, y: 1000.0 },
            Point { x: -1000.0, y: -1000.0 },
            Point { x: 1.0, y: 1.0 },
            &mut out, 0);
        // 2^MAX_DEPTH terminal segments at most.
        assert!(out.len() <= 1 << MAX_DEPTH);
    }

    #[test]
    fn perpendicular_distance_zero_on_chord_point() {
        let p0 = Point { x: 0.0, y: 0.0 };
        let p3 = Point { x: 10.0, y: 0.0 };
        // p1 ON the chord — distance 0.
        let p1 = Point { x: 5.0, y: 0.0 };
        assert!(perpendicular_distance(p0, p1, p3).abs() < 1e-12);
        // p1 perpendicular off the chord — distance = |y|.
        let p1 = Point { x: 5.0, y: 3.0 };
        assert!((perpendicular_distance(p0, p1, p3) - 3.0).abs() < 1e-12);
    }

    #[test]
    fn close_then_curve_uses_subpath_start_as_cursor() {
        // After Close the cursor should be back at the Move's endpoint.
        let outline = Outline {
            segments: vec![
                OutlineSegment {
                    op: OutlineOp::Move,
                    pts: [Point { x: 1.0, y: 2.0 },
                          Point { x: 0.0, y: 0.0 },
                          Point { x: 0.0, y: 0.0 }],
                },
                OutlineSegment {
                    op: OutlineOp::Line,
                    pts: [Point { x: 5.0, y: 5.0 },
                          Point { x: 0.0, y: 0.0 },
                          Point { x: 0.0, y: 0.0 }],
                },
                OutlineSegment {
                    op: OutlineOp::Close,
                    pts: [Point { x: 0.0, y: 0.0 }; 3],
                },
                // Curve continues from the post-Close cursor (= Move's
                // endpoint, (1, 2)). The first emitted line's start is
                // therefore (1, 2).
                OutlineSegment {
                    op: OutlineOp::Curve,
                    pts: [Point { x: 1.0, y: 4.0 },
                          Point { x: 3.0, y: 4.0 },
                          Point { x: 3.0, y: 2.0 }],
                },
            ],
        };
        let path = flatten_outline(&outline, identity_matrix());
        let kinds: Vec<_> = path.segments.iter().map(|s| s.kind).collect();
        let close_idx = kinds.iter().position(|k| *k == SegmentKind::Close)
            .expect("Close present");
        let after_close = &kinds[close_idx + 1..];
        // If `cur` is lost on Close, the OutlineOp::Curve branch skips
        // and `after_close` is empty. Correct behaviour produces at
        // least one Line emitted from flatten_cubic.
        assert!(!after_close.is_empty(),
                "curve was skipped — cur was lost on Close");
        assert!(after_close.contains(&SegmentKind::Line),
                "expected curve-flattened lines after Close");
    }

    #[test]
    fn decode_rejects_bad_magic() {
        let mut bytes = vec![0u8; 68];
        bytes[0] = 0xFF;
        bytes[1] = 1;
        assert_eq!(decode(&bytes), Err(Error::BadMagic));
    }

    #[test]
    fn decode_rejects_unsupported_version() {
        let mut bytes = vec![0u8; 68];
        bytes[0] = 0xCB;
        bytes[1] = 9;
        assert_eq!(decode(&bytes), Err(Error::UnsupportedVersion(9)));
    }

    #[test]
    fn decode_empty_segments_returns_zeroed_image() {
        let mut bytes = vec![0xCB, 0x01, 0x00, 0x00];
        bytes.extend_from_slice(&4u32.to_le_bytes());
        bytes.extend_from_slice(&4u32.to_le_bytes());
        for v in [1.0f64, 0.0, 0.0, 1.0, 0.0, 0.0] {
            bytes.extend_from_slice(&v.to_le_bytes());
        }
        bytes.extend_from_slice(&[255u8, 0, 0, 255]);
        bytes.extend_from_slice(&0u32.to_le_bytes());
        let img = decode(&bytes).unwrap();
        assert_eq!(img.width, 4);
        assert_eq!(img.height, 4);
        assert_eq!(img.components, 4);
        assert!(img.pixels.iter().all(|&p| p == 0));
    }
}
