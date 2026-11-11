// Anti-aliased polygon fill for PDF page rendering — PDF 32000-1:2008
// §8.5.2 path construction (Move / Line / Rect / Close subset), §8.5.3.3.1
// path-painting (f / F / f*), §8.5.3.3.2 fill rules. v1 scope is polygons
// only; Bezier curves are deferred to v2 once the algorithmic core is
// proven against the gate corpus.
//
// Composes the foundation `transform` (Matrix + apply) and `colorspace`
// (ColorRgb + quantize) primitives. std-only — no third-party rasteriser
// dependency.
//
// Algorithm: 4×4 centred sub-pixel super-sampling. Path flattens to edges
// with the CTM applied at emission time; for each device pixel, sixteen
// sub-pixel samples compute the polygon's winding number, the fill rule
// reduces winding → "inside", coverage = (count_inside) / 16, and src-over
// compositing of (color, alpha * coverage) lands the final pixel back into
// the raster. Quantisation back to u8 routes through colorspace::quantize
// so the rounding mode matches every other lossy primitive.
//
// `fill` is total — there is no error condition for well-formed input.
// `decode` validates the binary fixture wire format documented in
// Feedstock/PDF/Spec/rasterizer.yaml's `fixture_format` block and rejects
// truncated or malformed input.

use std::fmt;

use super::colorspace::{quantize, ColorRgb};
use super::transform::{apply, Matrix, Point};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum FillRule {
    NonZero = 0,
    EvenOdd = 1,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum SegmentKind {
    Move = 0,
    Line = 1,
    Rect = 2,
    Close = 3,
}

/// One path segment. Move and Line use `pts[0]` as the endpoint; Rect
/// uses `pts[0]` = (xmin, ymin) and `pts[1]` = (xmax, ymax); Close
/// ignores both.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PathSegment {
    pub kind: SegmentKind,
    pub pts: [Point; 2],
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Path {
    pub segments: Vec<PathSegment>,
}

/// RGBA8 raster, row-major, no row-stride padding. `pixels.len()` must
/// equal `width * height * 4`. `fill` mutates in place and composites
/// src-over onto the existing buffer contents.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Raster {
    pub pixels: Vec<u8>,
    pub width: u32,
    pub height: u32,
}

/// Standard lossy-primitive return shape — mirrors
/// `png_decoder::DecodedImage` so the gate template drives both the
/// same way. `components` is always 4 (RGBA8).
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
    /// Input ended before a fixed-size header field or a per-segment
    /// payload was fully read.
    Truncated,
    /// First byte was not 0xCA.
    BadMagic,
    /// Version byte was not 1.
    UnsupportedVersion(u8),
    /// Fill rule byte was outside {0, 1}.
    BadFillRule(u8),
    /// Segment kind byte was outside {0, 1, 2, 3}.
    BadSegmentKind(u8),
    /// Width or height was zero.
    ZeroDimension,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Truncated => write!(f, "rasterizer: truncated input"),
            Error::BadMagic => write!(
                f, "rasterizer: bad magic (expected 0xCA)"),
            Error::UnsupportedVersion(v) => write!(
                f, "rasterizer: unsupported version {v} (expected 1)"),
            Error::BadFillRule(v) => write!(
                f, "rasterizer: bad fill rule {v} (expected 0 or 1)"),
            Error::BadSegmentKind(v) => write!(
                f, "rasterizer: bad segment kind {v} (expected 0..=3)"),
            Error::ZeroDimension => write!(
                f, "rasterizer: width or height is zero"),
        }
    }
}

impl std::error::Error for Error {}

/// Fill `path`, transformed by `ctm`, into `dst` using the given fill
/// rule and src-over compositing of (color, alpha) against the existing
/// raster. `color` is in linear [0, 1] floating-point per
/// foundation::colorspace; `alpha` is a coverage multiplier in [0, 1]
/// applied on top of the per-sub-pixel anti-alias coverage.
///
/// Total — there is no error condition for well-formed input. Out-of-
/// canvas paths contribute coverage only to the visible rectangle; the
/// sub-pixel sampling naturally handles partial overlap.
/// A clipping region in device pixels, half-open: a pixel `(px, py)` is
/// inside when `x0 <= px < x1` and `y0 <= py < y1`.
///
/// The rectangle carries an axis-aligned clip — the dominant `re W n`
/// form — exactly and on its own. A clip path that is not a rectangle
/// additionally carries `mask`, a per-device-pixel coverage buffer of
/// `mask_stride` bytes per row: the primitives multiply each pixel's
/// alpha by `mask[py * mask_stride + px] / 255` on top of honouring the
/// box, which stays the mask's bounding rectangle so scans still narrow.
/// Built by [`rasterize_clip_mask`].
#[derive(Debug, Clone, PartialEq)]
pub struct ClipRect {
    pub x0: i64,
    pub y0: i64,
    pub x1: i64,
    pub y1: i64,
    pub mask: Option<std::rc::Rc<Vec<u8>>>,
    pub mask_stride: usize,
}

impl ClipRect {
    /// The intersection of two clips. The masks multiply per pixel, so
    /// nested clips narrow rather than replace one another.
    ///
    /// Reached from `page_renderer`, which mounts this module as part of
    /// the library; a gate driver that mounts it standalone sees no
    /// caller, hence the attribute.
    #[allow(dead_code)]
    pub fn intersect(&self, other: &ClipRect) -> ClipRect {
        let (x0, y0) = (self.x0.max(other.x0), self.y0.max(other.y0));
        let (x1, y1) = (self.x1.min(other.x1), self.y1.min(other.y1));
        let (mask, mask_stride) = match (&self.mask, &other.mask) {
            (None, None) => (None, 0),
            (Some(m), None) => (Some(m.clone()), self.mask_stride),
            (None, Some(m)) => (Some(m.clone()), other.mask_stride),
            (Some(a), Some(b)) => {
                let stride = self.mask_stride.max(other.mask_stride);
                let rows = if stride == 0 { 0 } else { a.len() / self.mask_stride.max(1) };
                let mut out = vec![0u8; stride * rows];
                for y in 0..rows {
                    for x in 0..stride {
                        let av = a.get(y * self.mask_stride + x).copied().unwrap_or(0) as u32;
                        let bv = b.get(y * other.mask_stride + x).copied().unwrap_or(0) as u32;
                        out[y * stride + x] = ((av * bv + 127) / 255) as u8;
                    }
                }
                (Some(std::rc::Rc::new(out)), stride)
            }
        };
        ClipRect { x0, y0, x1, y1, mask, mask_stride }
    }

    /// The clip's coverage at a device pixel, `0.0..=1.0`.
    pub fn coverage_at(&self, px: i64, py: i64) -> f64 {
        if px < self.x0 || px >= self.x1 || py < self.y0 || py >= self.y1 {
            return 0.0;
        }
        match &self.mask {
            None => 1.0,
            Some(mask) => {
                if self.mask_stride == 0 {
                    return 1.0;
                }
                let i = py as usize * self.mask_stride + px as usize;
                mask.get(i).copied().unwrap_or(0) as f64 / 255.0
            }
        }
    }
}

pub fn fill(
    dst: &mut Raster,
    path: &Path,
    ctm: Matrix,
    color: ColorRgb,
    alpha: f64,
    rule: FillRule,
    clip: Option<&ClipRect>,
) {
    if dst.width == 0 || dst.height == 0 {
        return;
    }
    // §8.5.3.1: a fill implicitly closes every open subpath. The
    // winding number is only defined for closed contours, so
    // `m … l … l … f` must fill what `… h f` fills; without the
    // implicit edge a thin needle triangle authored with no `h`
    // rasterises as an open polyline and floods its whole bbox.
    // Closing is output-preserving for already-closed input — the
    // implicit edge is degenerate and drops out as horizontal — which
    // is why the clip path has always done the same.
    let edges = flatten(&closed(path), ctm);
    if edges.is_empty() {
        return;
    }

    // 4×4 centred sub-pixel grid: offsets are 1/8, 3/8, 5/8, 7/8.
    const SUBS: [f64; 4] = [
        0.125, 0.375, 0.625, 0.875,
    ];

    let width = dst.width as i64;
    let height = dst.height as i64;

    // Only the path's own bounding box can take ink: every sample
    // outside it has winding number zero (no edge crosses above or
    // below it, and the crossings to one side of a closed subpath
    // cancel). Sweeping the whole canvas per fill instead makes a
    // glyph cost as much as a full-page shape.
    let (mut min_x, mut min_y) = (f64::INFINITY, f64::INFINITY);
    let (mut max_x, mut max_y) = (f64::NEG_INFINITY, f64::NEG_INFINITY);
    for e in &edges {
        min_x = min_x.min(e.x0).min(e.x1);
        max_x = max_x.max(e.x0).max(e.x1);
        min_y = min_y.min(e.y0).min(e.y1);
        max_y = max_y.max(e.y0).max(e.y1);
    }
    if !(min_x.is_finite() && min_y.is_finite() && max_x.is_finite() && max_y.is_finite()) {
        return;
    }
    let mut x_start = min_x.floor().clamp(0.0, width as f64) as i64;
    let mut x_end = (max_x.ceil().clamp(0.0, width as f64) as i64).min(width - 1);
    let mut y_start = min_y.floor().clamp(0.0, height as f64) as i64;
    let mut y_end = (max_y.ceil().clamp(0.0, height as f64) as i64).min(height - 1);
    // The clip box narrows the scan the same way the path's own bounding
    // box does — pixels outside it cannot take ink.
    if let Some(c) = clip {
        x_start = x_start.max(c.x0);
        y_start = y_start.max(c.y0);
        x_end = x_end.min(c.x1 - 1);
        y_end = y_end.min(c.y1 - 1);
    }
    if x_start > x_end || y_start > y_end {
        return;
    }

    for py in y_start..=y_end {
        for px in x_start..=x_end {
            let mut count_inside = 0u32;
            for &j in &SUBS {
                let sy = py as f64 + j;
                for &i in &SUBS {
                    let sx = px as f64 + i;
                    let w = winding_at(&edges, sx, sy);
                    let inside = match rule {
                        FillRule::NonZero => w != 0,
                        FillRule::EvenOdd => (w & 1) != 0,
                    };
                    if inside {
                        count_inside += 1;
                    }
                }
            }
            if count_inside == 0 {
                continue;
            }
            let coverage = count_inside as f64 / 16.0;
            let clip_coverage = clip.map_or(1.0, |c| c.coverage_at(px, py));
            if clip_coverage <= 0.0 {
                continue;
            }
            let effective_alpha = alpha * coverage * clip_coverage;
            composite_src_over(dst, px as u32, py as u32,
                               color, effective_alpha);
        }
    }
}

/// Rasterize `path`'s coverage into a `width` × `height` byte buffer,
/// 0..255 per device pixel — the soft clip behind a `W` / `W*` whose
/// path is not a rectangle.
///
/// Coverage uses the same 4×4 super-sample as [`fill`], so a mask edge
/// is anti-aliased identically to a filled one. Every subpath is closed
/// first: a clip written `m l l l W n` without an `h` still covers the
/// whole polygon rather than a sliver. `prev`, when given, is the parent
/// clip's mask of the same dimensions and multiplies in, so nested clips
/// intersect.
///
/// Reached from `page_renderer` — see the note on [`ClipRect::intersect`].
#[allow(dead_code)]
pub fn rasterize_clip_mask(
    width: u32,
    height: u32,
    path: &Path,
    ctm: Matrix,
    rule: FillRule,
    prev: Option<&[u8]>,
) -> Vec<u8> {
    const SUBS: [f64; 4] = [0.125, 0.375, 0.625, 0.875];
    let stride = width as usize;
    let mut out = vec![0u8; stride * height as usize];
    let edges = flatten(&closed(path), ctm);
    if edges.is_empty() {
        return out;
    }
    for py in 0..height as i64 {
        for px in 0..width as i64 {
            let mut count_inside = 0u32;
            for &j in &SUBS {
                let sy = py as f64 + j;
                for &i in &SUBS {
                    let sx = px as f64 + i;
                    let w = winding_at(&edges, sx, sy);
                    let inside = match rule {
                        FillRule::NonZero => w != 0,
                        FillRule::EvenOdd => (w & 1) != 0,
                    };
                    if inside {
                        count_inside += 1;
                    }
                }
            }
            let mut value = (count_inside * 255 / 16) as u8;
            if let Some(prev) = prev {
                let p = prev.get(py as usize * stride + px as usize).copied().unwrap_or(0) as u32;
                value = ((value as u32 * p + 127) / 255) as u8;
            }
            out[py as usize * stride + px as usize] = value;
        }
    }
    out
}

/// `path` with every subpath explicitly closed — what §8.5.3.1 asks of
/// both a fill and a clip path.
fn closed(path: &Path) -> Path {
    let mut out = Path { segments: Vec::with_capacity(path.segments.len() + 4) };
    for segment in &path.segments {
        if segment.kind == SegmentKind::Move && !out.segments.is_empty() {
            out.segments.push(PathSegment { kind: SegmentKind::Close, ..*segment });
        }
        out.segments.push(*segment);
    }
    if !out.segments.is_empty() {
        let last = *out.segments.last().expect("non-empty");
        if last.kind != SegmentKind::Close {
            out.segments.push(PathSegment { kind: SegmentKind::Close, ..last });
        }
    }
    out
}

/// Test-only entry point used by the lossy_oracle_agreement freeze gate.
/// Parses `input` per the binary fixture wire format documented in
/// `Feedstock/PDF/Spec/rasterizer.yaml`, builds the typed Path / CTM /
/// Color, runs `fill` on a freshly zeroed Raster, and returns the result
/// wrapped as a DecodedImage so the gate template treats it uniformly
/// alongside the other lossy primitives.
///
/// NOT part of the foundation surface intended for production callers
/// — production code calls `fill` directly with typed arguments.
#[allow(dead_code)] // retained for a future-phase public-API binding
pub fn decode(input: &[u8]) -> Result<DecodedImage, Error> {
    let mut r = Reader::new(input);

    let magic = r.read_u8()?;
    if magic != 0xCA {
        return Err(Error::BadMagic);
    }
    let version = r.read_u8()?;
    if version != 1 {
        return Err(Error::UnsupportedVersion(version));
    }
    let fill_rule_byte = r.read_u8()?;
    let rule = match fill_rule_byte {
        0 => FillRule::NonZero,
        1 => FillRule::EvenOdd,
        other => return Err(Error::BadFillRule(other)),
    };
    let _reserved = r.read_u8()?;
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
                PathSegment {
                    kind: SegmentKind::Move,
                    pts: [Point { x, y }, Point { x: 0.0, y: 0.0 }],
                }
            }
            1 => {
                let x = r.read_f64_le()?;
                let y = r.read_f64_le()?;
                PathSegment {
                    kind: SegmentKind::Line,
                    pts: [Point { x, y }, Point { x: 0.0, y: 0.0 }],
                }
            }
            2 => {
                let xmin = r.read_f64_le()?;
                let ymin = r.read_f64_le()?;
                let xmax = r.read_f64_le()?;
                let ymax = r.read_f64_le()?;
                PathSegment {
                    kind: SegmentKind::Rect,
                    pts: [
                        Point { x: xmin, y: ymin },
                        Point { x: xmax, y: ymax },
                    ],
                }
            }
            3 => PathSegment {
                kind: SegmentKind::Close,
                pts: [
                    Point { x: 0.0, y: 0.0 },
                    Point { x: 0.0, y: 0.0 },
                ],
            },
            other => return Err(Error::BadSegmentKind(other)),
        };
        segments.push(segment);
    }

    let path = Path { segments };
    let pixel_count = (width as usize)
        .checked_mul(height as usize)
        .and_then(|n| n.checked_mul(4))
        .ok_or(Error::Truncated)?;
    let mut dst = Raster {
        pixels: vec![0u8; pixel_count],
        width,
        height,
    };
    fill(&mut dst, &path, ctm, color, alpha, rule, None);

    Ok(DecodedImage {
        width: dst.width,
        height: dst.height,
        components: 4,
        pixels: dst.pixels,
    })
}

// ---------------------------------------------------------------------
// Path → polyline edges (CTM applied at emission time).
// ---------------------------------------------------------------------

#[derive(Debug, Clone, Copy)]
struct Edge {
    x0: f64,
    y0: f64,
    x1: f64,
    y1: f64,
}

fn flatten(path: &Path, ctm: Matrix) -> Vec<Edge> {
    let mut edges = Vec::with_capacity(path.segments.len() * 2);
    let mut cur: Option<Point> = None;
    let mut sub_start: Option<Point> = None;
    for segment in &path.segments {
        match segment.kind {
            SegmentKind::Move => {
                let p = apply(ctm, segment.pts[0]);
                cur = Some(p);
                sub_start = Some(p);
            }
            SegmentKind::Line => {
                if let Some(c) = cur {
                    let p = apply(ctm, segment.pts[0]);
                    edges.push(Edge { x0: c.x, y0: c.y, x1: p.x, y1: p.y });
                    cur = Some(p);
                }
            }
            SegmentKind::Rect => {
                let xmin = segment.pts[0].x;
                let ymin = segment.pts[0].y;
                let xmax = segment.pts[1].x;
                let ymax = segment.pts[1].y;
                let corners = [
                    apply(ctm, Point { x: xmin, y: ymin }),
                    apply(ctm, Point { x: xmax, y: ymin }),
                    apply(ctm, Point { x: xmax, y: ymax }),
                    apply(ctm, Point { x: xmin, y: ymax }),
                ];
                for i in 0..4 {
                    let a = corners[i];
                    let b = corners[(i + 1) % 4];
                    edges.push(Edge { x0: a.x, y0: a.y, x1: b.x, y1: b.y });
                }
                // Per §8.5.2.6 `re`, the rect's ending current point is its
                // origin. Both `cur` and `sub_start` advance to corners[0].
                cur = Some(corners[0]);
                sub_start = Some(corners[0]);
            }
            SegmentKind::Close => {
                if let (Some(c), Some(s)) = (cur, sub_start) {
                    if c.x != s.x || c.y != s.y {
                        edges.push(Edge {
                            x0: c.x, y0: c.y, x1: s.x, y1: s.y,
                        });
                    }
                    cur = Some(s);
                }
            }
        }
    }
    edges
}

/// Standard non-zero winding-number computation at a point. Half-open
/// y-interval `[min(y0,y1), max(y0,y1))` excludes horizontal edges
/// implicitly (interval is empty when y0 == y1) and prevents sample
/// double-counting at shared vertices. The "left of edge" check is
/// strictly `sx > x_cross` — left-inclusive / right-exclusive — required
/// by the adjacent_rects fixture.
fn winding_at(edges: &[Edge], sx: f64, sy: f64) -> i32 {
    let mut w = 0i32;
    for e in edges {
        if e.y0 == e.y1 {
            continue;
        }
        if e.y0 <= sy && sy < e.y1 {
            let t = (sy - e.y0) / (e.y1 - e.y0);
            let x_cross = e.x0 + t * (e.x1 - e.x0);
            if sx > x_cross {
                w += 1;
            }
        } else if e.y1 <= sy && sy < e.y0 {
            let t = (sy - e.y0) / (e.y1 - e.y0);
            let x_cross = e.x0 + t * (e.x1 - e.x0);
            if sx > x_cross {
                w -= 1;
            }
        }
    }
    w
}

/// src-over compositing with linear (non-premultiplied) source plus a
/// separate effective_alpha coverage multiplier:
///   dst_chan = src_chan * effective_alpha
///            + dst_chan_norm * (1 - effective_alpha)
///   dst_alpha = effective_alpha
///             + dst_alpha_norm * (1 - effective_alpha)
/// The dst-side `(1 - effective_alpha)` factor is applied to the
/// EXISTING normalised destination, not after re-multiplying it by
/// coverage — see semantic_traps.srgb_premul_confusion in the spec.
/// Composite a single pixel. A gradient's colour changes from pixel to
/// pixel, so it cannot go through [`fill`], which paints one colour
/// through a coverage mask; this is the per-pixel entry point it uses
/// instead. Out-of-bounds coordinates and non-positive alpha are no-ops.
#[allow(dead_code)] // not reached from the standalone test mounts
pub fn composite_pixel(
    dst: &mut Raster,
    px: u32,
    py: u32,
    color: ColorRgb,
    effective_alpha: f64,
) {
    // NaN is spelled out rather than written as a negated
    // comparison: it must be a no-op, not a silent pass.
    if px >= dst.width || py >= dst.height
        || effective_alpha.is_nan() || effective_alpha <= 0.0
    {
        return;
    }
    composite_src_over(dst, px, py, color, effective_alpha);
}

fn composite_src_over(
    dst: &mut Raster,
    px: u32,
    py: u32,
    color: ColorRgb,
    effective_alpha: f64,
) {
    let stride = (dst.width as usize) * 4;
    let i = (py as usize) * stride + (px as usize) * 4;
    let dr = dst.pixels[i] as f64 / 255.0;
    let dg = dst.pixels[i + 1] as f64 / 255.0;
    let db = dst.pixels[i + 2] as f64 / 255.0;
    let da = dst.pixels[i + 3] as f64 / 255.0;
    let one_minus = 1.0 - effective_alpha;
    let or = color.r * effective_alpha + dr * one_minus;
    let og = color.g * effective_alpha + dg * one_minus;
    let ob = color.b * effective_alpha + db * one_minus;
    let oa = effective_alpha + da * one_minus;
    dst.pixels[i] = quantize(or);
    dst.pixels[i + 1] = quantize(og);
    dst.pixels[i + 2] = quantize(ob);
    dst.pixels[i + 3] = quantize(oa);
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

    fn identity_ctm() -> Matrix {
        Matrix::new(1.0, 0.0, 0.0, 1.0, 0.0, 0.0)
    }

    fn opaque_white() -> ColorRgb {
        ColorRgb { r: 1.0, g: 1.0, b: 1.0 }
    }

    #[test]
    fn empty_path_leaves_canvas_zeroed() {
        let mut r = solid(4, 4);
        fill(&mut r, &Path::default(), identity_ctm(),
             opaque_white(), 1.0, FillRule::NonZero, None);
        assert!(r.pixels.iter().all(|&p| p == 0));
    }

    #[test]
    fn solid_full_canvas_rect_fills_every_pixel_to_255() {
        // Rect covering the full canvas — coverage is 1 everywhere.
        let mut r = solid(4, 4);
        let path = Path {
            segments: vec![PathSegment {
                kind: SegmentKind::Rect,
                pts: [Point { x: 0.0, y: 0.0 },
                      Point { x: 4.0, y: 4.0 }],
            }],
        };
        fill(&mut r, &path, identity_ctm(),
             opaque_white(), 1.0, FillRule::NonZero, None);
        assert!(r.pixels.iter().all(|&p| p == 255),
                "expected solid white, got {:?}", &r.pixels[..16]);
    }

    #[test]
    fn close_without_move_is_noop() {
        let mut r = solid(4, 4);
        let path = Path {
            segments: vec![PathSegment {
                kind: SegmentKind::Close,
                pts: [Point { x: 0.0, y: 0.0 },
                      Point { x: 0.0, y: 0.0 }],
            }],
        };
        fill(&mut r, &path, identity_ctm(),
             opaque_white(), 1.0, FillRule::NonZero, None);
        assert!(r.pixels.iter().all(|&p| p == 0));
    }

    #[test]
    fn decode_rejects_bad_magic() {
        let mut bytes = vec![0u8; 68];
        bytes[0] = 0xAB;
        bytes[1] = 1;
        assert_eq!(decode(&bytes), Err(Error::BadMagic));
    }

    #[test]
    fn decode_rejects_unsupported_version() {
        let mut bytes = vec![0u8; 68];
        bytes[0] = 0xCA;
        bytes[1] = 2;
        assert_eq!(decode(&bytes), Err(Error::UnsupportedVersion(2)));
    }

    #[test]
    fn decode_rejects_bad_fill_rule() {
        let mut bytes = vec![0u8; 68];
        bytes[0] = 0xCA;
        bytes[1] = 1;
        bytes[2] = 9;
        assert_eq!(decode(&bytes), Err(Error::BadFillRule(9)));
    }

    #[test]
    fn decode_rejects_truncated_header() {
        let bytes = vec![0xCA, 0x01];
        assert_eq!(decode(&bytes), Err(Error::Truncated));
    }

    #[test]
    fn decode_rejects_zero_dimension() {
        // magic=0xCA, version=1, fill=NonZero, reserved=0, w=0, h=4
        let mut bytes = vec![0xCA, 0x01, 0x00, 0x00];
        bytes.extend_from_slice(&0u32.to_le_bytes());
        bytes.extend_from_slice(&4u32.to_le_bytes());
        bytes.extend_from_slice(&[0u8; 48]);
        bytes.extend_from_slice(&[0u8, 0u8, 0u8, 0u8]);
        bytes.extend_from_slice(&0u32.to_le_bytes());
        assert_eq!(decode(&bytes), Err(Error::ZeroDimension));
    }

    #[test]
    fn decode_empty_segments_returns_zeroed_image() {
        let mut bytes = vec![0xCA, 0x01, 0x00, 0x00];
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
        assert_eq!(img.pixels.len(), 64);
        assert!(img.pixels.iter().all(|&p| p == 0));
    }

    #[test]
    fn winding_horizontal_edge_contributes_zero() {
        let edges = vec![Edge { x0: 0.0, y0: 5.0, x1: 10.0, y1: 5.0 }];
        assert_eq!(winding_at(&edges, 5.0, 5.0), 0);
        assert_eq!(winding_at(&edges, 5.0, 4.5), 0);
    }

    #[test]
    fn winding_left_inclusive_right_exclusive() {
        // Vertical edge from (5, 0) up to (5, 10). Sample at (5.0, 1.0):
        // sx > x_cross (5.0 > 5.0) is false — sample is NOT inside.
        let edges = vec![Edge { x0: 5.0, y0: 0.0, x1: 5.0, y1: 10.0 }];
        assert_eq!(winding_at(&edges, 5.0, 1.0), 0,
                   "x_sample == x_edge counts as outside");
        // Just to the right: sx > x_cross (5.001 > 5.0) is true.
        assert_eq!(winding_at(&edges, 5.001, 1.0), 1);
    }
}
