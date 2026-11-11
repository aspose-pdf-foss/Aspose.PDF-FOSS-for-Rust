//! Target-side helper: derive the PDF font metrics needed to embed a
//! TrueType program as a `/FontFile2`. Backs the public text save
//! pipeline's embedded-font path. Non-anchored (composes the `truetype`
//! primitive).
//!
//! The caller supplies the raw TrueType bytes; this module parses them
//! and projects the head / hhea / hmtx / cmap data into PDF glyph space
//! (1000 units/em): a `/Widths` array over the `WinAnsiEncoding` code
//! range plus the `/FontDescriptor` numerics. The caller assembles the
//! three indirect objects (font dict, descriptor, `/FontFile2` stream)
//! and wires the object ids.

use super::truetype;

/// The metrics needed to write a TrueType font dict + descriptor.
pub(crate) struct FontMetrics {
    pub ascent: i64,
    pub descent: i64,
    pub cap_height: i64,
    pub bbox: [i64; 4],
    pub first_char: u8,
    pub last_char: u8,
    /// One scaled advance width per code in `first_char..=last_char`.
    pub widths: Vec<i64>,
}

/// First / last `WinAnsiEncoding` codes the `/Widths` array covers.
const FIRST_CHAR: u8 = 32;
const LAST_CHAR: u8 = 255;

/// Parse `program` and project its metrics into PDF glyph space, or
/// `None` when it isn't a usable TrueType font.
pub(crate) fn metrics(program: &[u8]) -> Option<FontMetrics> {
    let ttf = truetype::parse(program).ok()?;
    if ttf.units_per_em == 0 {
        return None;
    }
    let scale = 1000.0 / ttf.units_per_em as f64;
    let s = |v: f64| (v * scale).round() as i64;

    // FontBBox from the glyph outlines (min/max over every point).
    let mut min_x = i32::MAX;
    let mut min_y = i32::MAX;
    let mut max_x = i32::MIN;
    let mut max_y = i32::MIN;
    for g in &ttf.glyphs {
        for p in &g.points {
            min_x = min_x.min(p.x);
            min_y = min_y.min(p.y);
            max_x = max_x.max(p.x);
            max_y = max_y.max(p.y);
        }
    }
    let bbox = if min_x <= max_x {
        [s(min_x as f64), s(min_y as f64), s(max_x as f64), s(max_y as f64)]
    } else {
        // No outlines (e.g. a metrics-only font) — a conservative box.
        [0, s(ttf.descent as f64), s(ttf.units_per_em as f64), s(ttf.ascent as f64)]
    };

    // Per-code advance widths via the cmap (code treated as a Unicode
    // scalar — exact for ASCII / Latin-1, approximate for WinAnsi
    // 0x80–0x9F).
    let mut widths = Vec::with_capacity((LAST_CHAR - FIRST_CHAR) as usize + 1);
    for code in FIRST_CHAR..=LAST_CHAR {
        let gid = gid_for(&ttf.cmap, code as u32);
        let advance = match gid {
            Some(g) => ttf.widths.get(g as usize).copied().unwrap_or(0),
            None => 0,
        };
        widths.push(s(advance as f64));
    }

    Some(FontMetrics {
        ascent: s(ttf.ascent as f64),
        descent: s(ttf.descent as f64),
        // No dedicated CapHeight in the parsed tables; the ascent is a
        // reasonable stand-in for nonsymbolic Latin fonts.
        cap_height: s(ttf.ascent as f64),
        bbox,
        first_char: FIRST_CHAR,
        last_char: LAST_CHAR,
        widths,
    })
}

/// Look up a glyph id for `code` in the sorted cmap.
fn gid_for(cmap: &[(u32, u16)], code: u32) -> Option<u16> {
    cmap.binary_search_by_key(&code, |(c, _)| *c).ok().map(|i| cmap[i].1)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn liberation_sans() -> Vec<u8> {
        std::fs::read(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/internal/standard14_outlines.fonts/LiberationSans-Regular.ttf"
        ))
        .unwrap()
    }

    #[test]
    fn metrics_scale_to_pdf_glyph_space() {
        let m = metrics(&liberation_sans()).expect("metrics");
        assert_eq!(m.first_char, 32);
        assert_eq!(m.last_char, 255);
        assert_eq!(m.widths.len(), 224);
        // Latin fonts have a positive ascent and negative descent in
        // PDF glyph space.
        assert!(m.ascent > 0);
        assert!(m.descent < 0);
        // The space (code 32) and a letter (code 'A' = 65) have
        // non-zero advances in a Latin font.
        assert!(m.widths[0] > 0, "space width");
        assert!(m.widths[(b'A' - 32) as usize] > 0, "'A' width");
        // A sane bounding box.
        assert!(m.bbox[2] > m.bbox[0] && m.bbox[3] > m.bbox[1]);
    }

    #[test]
    fn rejects_non_font_bytes() {
        assert!(metrics(b"not a font").is_none());
    }
}
