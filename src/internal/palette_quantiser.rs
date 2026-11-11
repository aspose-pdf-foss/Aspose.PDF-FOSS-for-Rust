// Whole-image RGBA8 → indexed palette quantiser. The contract
// is the .NET BCL (System.Drawing.Imaging.ColorPalette +
// Bitmap.Clone with Format4bppIndexed / Format8bppIndexed) —
// the surface commercial Aspose.PDF .NET delegates to when
// producing palettised TIFFs / GIFs / PNGs from rendered pages.
// PDF itself doesn't pin a quantiser; this primitive's spec is
// a SHAPE contract (palette size, index range, exact-match
// self-identity, deterministic, alpha dropped before split,
// palette covers uniques when feasible) — not an algorithm.
//
// Algorithm: median-cut (Heckbert, SIGGRAPH 1982). Each bucket
// holds a slice of pixel indices into a flat RGB pixel array;
// at every step pick the splittable bucket with the largest
// single-axis range, sort that bucket by the chosen axis, and
// split at the median. Stop when bucket count == max_colors OR
// no bucket is splittable (all remaining buckets are
// homogeneous). Per-bucket palette entry is the centroid; final
// pass assigns each pixel to its nearest palette entry by
// squared Euclidean distance.
//
// Median-cut naturally satisfies
// palette_covers_unique_colors_when_feasible — when the input
// has K ≤ max_colors distinct colours, the algorithm halts with
// K homogeneous buckets and a palette of K exact input colours,
// no early-exit pass needed.

use std::cmp::Ordering;
use std::fmt;

/// Result of [`quantise`] — the input image's pixels remapped
/// to indices into a palette. `palette[i]` is an RGB byte triple
/// (alpha is dropped before quantisation; palette formats don't
/// carry alpha). `indices` is row-major, top-left origin, and
/// has length `width * height`.
#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(dead_code)] // test-oracle helper, reached by the matching tests/internal/ gate driver via #[path]
pub struct IndexedImage {
    pub palette: Vec<[u8; 3]>,
    pub indices: Vec<u8>,
    pub width: usize,
    pub height: usize,
}

/// Errors returned by [`quantise`] on out-of-contract inputs.
#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(dead_code)] // test-oracle helper, reached by the matching tests/internal/ gate driver via #[path]
pub enum Error {
    /// `max_colors` outside `[2, 256]`. The lower bound rejects
    /// the degenerate single-entry palette (every pixel maps to
    /// the same colour); the upper bound is the BCL
    /// `ColorPalette` cap.
    InvalidMaxColors { max_colors: usize },
    /// `rgba` slice shorter than `width * height * 4`.
    RgbaTooSmall { expected: usize, got: usize },
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::InvalidMaxColors { max_colors } => write!(
                f, "palette_quantiser: invalid max_colors {max_colors} (expected 2..=256)"),
            Error::RgbaTooSmall { expected, got } => write!(
                f, "palette_quantiser: rgba buffer too small (expected >= {expected} bytes, got {got})"),
        }
    }
}

impl std::error::Error for Error {}

#[allow(dead_code)] // test-oracle helper, reached by the matching tests/internal/ gate driver via #[path]
const MIN_MAX_COLORS: usize = 2;
#[allow(dead_code)] // test-oracle helper, reached by the matching tests/internal/ gate driver via #[path]
const MAX_MAX_COLORS: usize = 256;

/// Quantise an RGBA8 image to an indexed palette of at most
/// `max_colors` RGB entries. Alpha is dropped (palette formats
/// don't carry alpha); callers needing alpha keep RGBA8 / 24bpp.
#[allow(dead_code)] // test-oracle helper, reached by the matching tests/internal/ gate driver via #[path]
pub fn quantise(
    rgba: &[u8],
    width: usize,
    height: usize,
    max_colors: usize,
) -> Result<IndexedImage, Error> {
    if !(MIN_MAX_COLORS..=MAX_MAX_COLORS).contains(&max_colors) {
        return Err(Error::InvalidMaxColors { max_colors });
    }
    let pixel_count = width.saturating_mul(height);
    let expected = pixel_count.saturating_mul(4);
    if rgba.len() < expected {
        return Err(Error::RgbaTooSmall { expected, got: rgba.len() });
    }
    if pixel_count == 0 {
        // Out-of-contract per the spec ("the invariant gate
        // doesn't visit this case"). Spec permits an empty
        // result with a [0, 0, 0] sentinel palette.
        return Ok(IndexedImage {
            palette: vec![[0, 0, 0]],
            indices: Vec::new(),
            width,
            height,
        });
    }

    // 1. Drop alpha → flat array of RGB triples.
    let pixels: Vec<[u8; 3]> = (0..pixel_count)
        .map(|i| [rgba[4 * i], rgba[4 * i + 1], rgba[4 * i + 2]])
        .collect();

    // 2. Median-cut split into at most `max_colors` buckets.
    let buckets = median_cut(&pixels, max_colors);

    // 3. Per-bucket palette entry = centroid (or the bucket's
    // single colour if homogeneous, in which case the centroid
    // equals the input colour by construction).
    let palette: Vec<[u8; 3]> = buckets
        .iter()
        .map(|b| centroid(b, &pixels))
        .collect();

    // 4. Per-pixel nearest-neighbour assignment by squared
    // Euclidean distance. Strictly-less tie-break picks the
    // earliest palette entry on equal distance — and ensures
    // exact_match_self_identity (a zero-distance match always
    // wins).
    let indices = assign_indices(&pixels, &palette);

    Ok(IndexedImage { palette, indices, width, height })
}

// =====================  Median-cut  =========================

/// One bucket — slice of pixel indices into the `pixels` array.
#[allow(dead_code)] // test-oracle helper, reached by the matching tests/internal/ gate driver via #[path]
type Bucket = Vec<usize>;

#[allow(dead_code)] // test-oracle helper, reached by the matching tests/internal/ gate driver via #[path]
fn median_cut(pixels: &[[u8; 3]], max_colors: usize) -> Vec<Bucket> {
    let mut buckets: Vec<Bucket> = vec![(0..pixels.len()).collect()];
    while buckets.len() < max_colors {
        // Pick the splittable bucket with the largest single-
        // axis range. Tie-break by lowest bucket index for
        // determinism.
        let mut pick: Option<(usize, u8, u8)> = None;  // (idx, range, axis)
        for (i, b) in buckets.iter().enumerate() {
            if let Some((range, axis)) = bucket_range(b, pixels) {
                let better = match pick {
                    None => true,
                    Some((_, r, _)) => range > r,
                };
                if better {
                    pick = Some((i, range, axis));
                }
            }
        }
        let Some((idx, _, axis)) = pick else { break; };
        // Sort the bucket by the chosen axis with full-RGB
        // tie-break — total ordering, deterministic.
        buckets[idx].sort_by(|&a, &b| {
            let pa = pixels[a];
            let pb = pixels[b];
            match pa[axis as usize].cmp(&pb[axis as usize]) {
                Ordering::Equal => pa.cmp(&pb),
                o => o,
            }
        });
        // Split at the median. With len >= 2 and >=2 distinct
        // colours along the chosen axis, mid in 1..len.
        let mid = buckets[idx].len() / 2;
        let right = buckets[idx].split_off(mid);
        buckets.push(right);
    }
    buckets
}

/// Returns `Some((range, axis))` if the bucket has a non-zero
/// span on at least one axis (i.e., is splittable). Axis is
/// 0=R / 1=G / 2=B. Ties prefer R then G then B for a stable
/// choice, matching the natural channel order.
#[allow(dead_code)] // test-oracle helper, reached by the matching tests/internal/ gate driver via #[path]
fn bucket_range(b: &Bucket, pixels: &[[u8; 3]]) -> Option<(u8, u8)> {
    if b.is_empty() {
        return None;
    }
    let mut mins = [255u8; 3];
    let mut maxs = [0u8; 3];
    for &i in b {
        let p = pixels[i];
        for c in 0..3 {
            if p[c] < mins[c] { mins[c] = p[c]; }
            if p[c] > maxs[c] { maxs[c] = p[c]; }
        }
    }
    let r0 = maxs[0] - mins[0];
    let r1 = maxs[1] - mins[1];
    let r2 = maxs[2] - mins[2];
    let range = r0.max(r1).max(r2);
    if range == 0 { return None; }
    let axis: u8 = if r0 == range { 0 }
                   else if r1 == range { 1 }
                   else { 2 };
    Some((range, axis))
}

/// Component-wise integer mean of the bucket's pixels. For a
/// homogeneous bucket the mean is exactly the bucket's colour
/// (pure unsigned integer division of N copies of the same
/// value), which is what
/// palette_covers_unique_colors_when_feasible relies on.
#[allow(dead_code)] // test-oracle helper, reached by the matching tests/internal/ gate driver via #[path]
fn centroid(b: &Bucket, pixels: &[[u8; 3]]) -> [u8; 3] {
    debug_assert!(!b.is_empty());
    let mut sum = [0u32; 3];
    for &i in b {
        let p = pixels[i];
        sum[0] += p[0] as u32;
        sum[1] += p[1] as u32;
        sum[2] += p[2] as u32;
    }
    let n = b.len() as u32;
    [(sum[0] / n) as u8, (sum[1] / n) as u8, (sum[2] / n) as u8]
}

// =====================  Nearest-neighbour assignment  =======

#[allow(dead_code)] // test-oracle helper, reached by the matching tests/internal/ gate driver via #[path]
fn assign_indices(pixels: &[[u8; 3]], palette: &[[u8; 3]]) -> Vec<u8> {
    debug_assert!(!palette.is_empty());
    debug_assert!(palette.len() <= 256);
    let mut indices = Vec::with_capacity(pixels.len());
    for &p in pixels {
        let mut best_d: u32 = u32::MAX;
        let mut best: u8 = 0;
        for (i, &q) in palette.iter().enumerate() {
            let d = sq_dist(p, q);
            if d < best_d {
                best_d = d;
                best = i as u8;
                if d == 0 { break; }   // exact match — can't beat 0.
            }
        }
        indices.push(best);
    }
    indices
}

#[inline]
#[allow(dead_code)] // test-oracle helper, reached by the matching tests/internal/ gate driver via #[path]
fn sq_dist(a: [u8; 3], b: [u8; 3]) -> u32 {
    let dr = a[0] as i32 - b[0] as i32;
    let dg = a[1] as i32 - b[1] as i32;
    let db = a[2] as i32 - b[2] as i32;
    (dr * dr + dg * dg + db * db) as u32
}

// =====================  Tests  ==============================

#[cfg(test)]
mod tests {
    use super::*;

    fn rgba(pixels: &[[u8; 4]]) -> Vec<u8> {
        let mut v = Vec::with_capacity(pixels.len() * 4);
        for p in pixels { v.extend_from_slice(p); }
        v
    }

    fn distinct_rgb(rgba_buf: &[u8], w: usize, h: usize) -> std::collections::BTreeSet<[u8; 3]> {
        (0..w * h)
            .map(|i| [rgba_buf[4 * i], rgba_buf[4 * i + 1], rgba_buf[4 * i + 2]])
            .collect()
    }

    // ------ Error paths.
    #[test]
    fn invalid_max_colors_below_range() {
        assert_eq!(
            quantise(&[0u8; 16], 2, 2, 1).unwrap_err(),
            Error::InvalidMaxColors { max_colors: 1 });
    }

    #[test]
    fn invalid_max_colors_above_range() {
        assert_eq!(
            quantise(&[0u8; 16], 2, 2, 257).unwrap_err(),
            Error::InvalidMaxColors { max_colors: 257 });
    }

    #[test]
    fn rgba_buffer_too_small() {
        // 2x2 image needs 16 bytes; pass 15.
        assert_eq!(
            quantise(&[0u8; 15], 2, 2, 16).unwrap_err(),
            Error::RgbaTooSmall { expected: 16, got: 15 });
    }

    // ------ palette_size_bound floor — single pixel.
    #[test]
    fn single_pixel_yields_single_entry_palette() {
        let img = quantise(&rgba(&[[0xC8, 0x32, 0x96, 0xFF]]), 1, 1, 16).unwrap();
        assert_eq!(img.palette.len(), 1);
        assert_eq!(img.palette[0], [0xC8, 0x32, 0x96]);
        assert_eq!(img.indices, vec![0]);
    }

    // ------ palette_covers_unique_colors_when_feasible — flat
    // K=1 input.
    #[test]
    fn flat_color_yields_one_entry() {
        let buf = rgba(&[[40, 200, 80, 0xFF]; 64]);   // 8x8 of one colour
        let img = quantise(&buf, 8, 8, 16).unwrap();
        assert_eq!(img.palette, vec![[40, 200, 80]]);
        assert!(img.indices.iter().all(|&i| i == 0));
    }

    // ------ palette_covers_unique_colors_when_feasible — K=2,
    // budget 256: every distinct input colour must appear in the
    // palette at zero distance.
    #[test]
    fn two_colours_with_large_budget_cover_exactly() {
        let mut buf = Vec::new();
        for _ in 0..4 {
            for x in 0..4 {
                let p = if x % 2 == 0 { [255, 0, 0, 0xFF] } else { [0, 0, 255, 0xFF] };
                buf.extend_from_slice(&p);
            }
        }
        let img = quantise(&buf, 4, 4, 256).unwrap();
        assert!(img.palette.len() <= 2);
        let palette_set: std::collections::BTreeSet<[u8; 3]> =
            img.palette.iter().copied().collect();
        for distinct in distinct_rgb(&buf, 4, 4) {
            assert!(palette_set.contains(&distinct),
                    "input colour {distinct:?} missing from palette");
        }
    }

    // ------ palette_size_bound + indices_in_range upper bound
    // when K > max_colors.
    #[test]
    fn k_greater_than_max_forces_quantisation() {
        // 17 distinct colours in a 17x1 image, budget 16.
        let mut buf = Vec::new();
        for i in 0..17u8 {
            buf.extend_from_slice(&[i.wrapping_mul(15),
                                    i.wrapping_mul(31),
                                    i.wrapping_mul(47),
                                    0xFF]);
        }
        let img = quantise(&buf, 17, 1, 16).unwrap();
        assert!(!img.palette.is_empty() && img.palette.len() <= 16);
        assert_eq!(img.indices.len(), 17);
        for &i in &img.indices {
            assert!((i as usize) < img.palette.len());
        }
    }

    // ------ exact_match_self_identity: with K ≤ max_colors, an
    // input pixel whose RGB equals a palette entry must map to
    // that entry (zero distance always wins ties).
    #[test]
    fn exact_match_self_identity() {
        // 4 distinct colours, budget 16.
        let buf = rgba(&[
            [10, 20, 30, 0xFF],
            [200, 100, 50, 0xFF],
            [50, 200, 100, 0xFF],
            [100, 50, 200, 0xFF],
        ]);
        let img = quantise(&buf, 4, 1, 16).unwrap();
        let palette_set: std::collections::BTreeSet<[u8; 3]> =
            img.palette.iter().copied().collect();
        for (px_i, p) in [
            [10u8, 20, 30],
            [200, 100, 50],
            [50, 200, 100],
            [100, 50, 200],
        ].iter().enumerate() {
            assert!(palette_set.contains(p),
                    "input pixel {p:?} missing from palette");
            // The pixel's index must point at THAT entry.
            let idx = img.indices[px_i] as usize;
            assert_eq!(img.palette[idx], *p);
        }
    }

    // ------ Alpha drop: K_rgb=1 even if K_rgba=many.
    #[test]
    fn alpha_dropped_before_quantisation() {
        // 1x4 image, same RGB, varying alpha.
        let buf = rgba(&[
            [180, 60, 240, 0x00],
            [180, 60, 240, 0x55],
            [180, 60, 240, 0xAA],
            [180, 60, 240, 0xFF],
        ]);
        let img = quantise(&buf, 1, 4, 256).unwrap();
        assert_eq!(img.palette, vec![[180, 60, 240]]);
        assert_eq!(img.indices, vec![0; 4]);
    }

    // ------ K == max_colors boundary.
    #[test]
    fn k_equals_max_colors_palette_covers_all() {
        let mut buf = Vec::new();
        for i in 0..16u8 {
            buf.extend_from_slice(&[
                i.wrapping_mul(16),
                255u8.wrapping_sub(i.wrapping_mul(16)),
                i.wrapping_mul(32),
                0xFF,
            ]);
        }
        let img = quantise(&buf, 16, 1, 16).unwrap();
        assert!(img.palette.len() <= 16);
        let distinct = distinct_rgb(&buf, 16, 1);
        let palette_set: std::collections::BTreeSet<[u8; 3]> =
            img.palette.iter().copied().collect();
        for c in distinct {
            assert!(palette_set.contains(&c));
        }
    }

    // ------ Determinism: two calls produce identical output.
    #[test]
    fn deterministic_repeat() {
        let mut buf = Vec::new();
        for y in 0..16u8 {
            for x in 0..16u8 {
                buf.extend_from_slice(&[x * 16, y * 16, 0x80, 0xFF]);
            }
        }
        let a = quantise(&buf, 16, 16, 16).unwrap();
        let b = quantise(&buf, 16, 16, 16).unwrap();
        assert_eq!(a, b);
    }

    // ------ Smallest budget (max_colors=2): palette must have
    // at most 2 entries; every pixel must map into [0, 1].
    #[test]
    fn min_budget_two() {
        let palette_in = [[0xFF, 0xFF, 0xFF], [0x00, 0x00, 0x00]];
        let mut buf = Vec::new();
        for x in 0..32 {
            let p = palette_in[x % 2];
            buf.extend_from_slice(&[p[0], p[1], p[2], 0xFF]);
        }
        let img = quantise(&buf, 32, 1, 2).unwrap();
        assert!(img.palette.len() <= 2);
        for &i in &img.indices {
            assert!((i as usize) < img.palette.len());
        }
    }

    // ------ Row-major layout: 1-wide column where indexing
    // mistakes (width/height swap) would surface.
    #[test]
    fn tall_thin_indexing() {
        let mut buf = Vec::new();
        for i in 0..16u8 {
            buf.extend_from_slice(&[i * 16, 0x40, 255 - i * 16, 0xFF]);
        }
        let img = quantise(&buf, 1, 16, 256).unwrap();
        assert_eq!(img.indices.len(), 16);
        assert_eq!(img.width, 1);
        assert_eq!(img.height, 16);
        // Each row's pixel must match an exact palette entry.
        for i in 0..16 {
            let p = [(i as u8) * 16, 0x40, 255 - (i as u8) * 16];
            let idx = img.indices[i] as usize;
            assert_eq!(img.palette[idx], p);
        }
    }
}
