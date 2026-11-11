//! Target-side helper: produce a *subset* TrueType program that keeps
//! only the glyph outlines a document actually draws, so an embedded
//! `/FontFile2` is much smaller than the full program. Backs the public
//! `Font.is_subset` save path. Non-anchored (composes the `truetype`
//! primitive plus a direct re-read of the raw `glyf` / `loca` tables).
//!
//! Strategy — **glyph-stripping** (glyph IDs stay stable):
//!
//!   * Map each used WinAnsi char code → glyph id via the font cmap.
//!     gid 0 (`.notdef`) is always kept. For any kept composite glyph,
//!     its component gids are kept too (recursively), since a composite
//!     references its parts by glyph index.
//!   * Rewrite `glyf` so kept glyphs keep their original outline bytes
//!     and every other glyph becomes a zero-length entry. Rebuild `loca`
//!     (short or long per `head.indexToLocFormat`) to match.
//!   * Keep every other table (cmap, hmtx, hhea, head, maxp, name, …)
//!     byte-for-byte, so `numGlyphs` / cmap / advances are unchanged and
//!     no glyph renumbering is needed.
//!   * Recompute the checksum of each changed table directory entry and
//!     fix `head.checkSumAdjustment`.
//!
//! Returns `None` on any parse problem so the caller falls back to a
//! full embed rather than emitting a corrupt font.
//!
//! References: OpenType 1.9 / Apple TrueType Reference Manual — sfnt
//! offset table + table directory, `head`, `loca`, `glyf` (composite
//! component header), table checksums.

use super::truetype;

const MAX_COMPOSITE_DEPTH: u32 = 16;

/// Build a subset of `program` keeping only the glyphs reachable from
/// `used_codes` (plus `.notdef`). `None` on any parse / layout problem.
pub(crate) fn subset(program: &[u8], used_codes: &[u8]) -> Option<Vec<u8>> {
    let ttf = truetype::parse(program).ok()?;
    let dir = parse_directory(program)?;

    // Raw loca + glyf tables (we rewrite these at the byte level).
    let head = dir.table(program, b"head")?;
    if head.len() < 54 {
        return None;
    }
    let index_to_loc_format = i16_be(&head[50..52]);
    let loca_bytes = dir.table(program, b"loca")?;
    let glyf = dir.table(program, b"glyf")?;
    let num_glyphs = ttf.num_glyphs;

    let loca = parse_loca(loca_bytes, num_glyphs, index_to_loc_format, glyf.len())?;

    // ---- Determine the kept glyph set ------------------------------
    let mut keep = vec![false; num_glyphs as usize];
    keep[0] = true; // .notdef
    for &code in used_codes {
        if let Some(gid) = gid_for(&ttf.cmap, code as u32) {
            mark_with_components(gid, glyf, &loca, num_glyphs, &mut keep, 0);
        }
    }

    // ---- Rebuild glyf (kept glyphs verbatim, others empty) ---------
    let mut new_glyf: Vec<u8> = Vec::with_capacity(glyf.len());
    let mut new_offsets: Vec<u32> = Vec::with_capacity(num_glyphs as usize + 1);
    new_offsets.push(0);
    for gid in 0..num_glyphs as usize {
        if keep[gid] {
            let begin = loca[gid] as usize;
            let end = loca[gid + 1] as usize;
            if end > glyf.len() || begin > end {
                return None;
            }
            new_glyf.extend_from_slice(&glyf[begin..end]);
        }
        // dropped glyphs contribute zero bytes
        new_offsets.push(new_glyf.len() as u32);
    }
    // glyf table length must be a multiple of 4 (long-aligned). Pad.
    while new_glyf.len() % 4 != 0 {
        new_glyf.push(0);
    }

    // ---- Rebuild loca to match the new glyf offsets ----------------
    let new_loca = build_loca(&new_offsets, index_to_loc_format);

    // ---- Reassemble the sfnt with the two rewritten tables ---------
    Some(reassemble(program, &dir, &new_loca, &new_glyf))
}

/// Look up a glyph id for `code` in the sorted cmap.
fn gid_for(cmap: &[(u32, u16)], code: u32) -> Option<u16> {
    cmap.binary_search_by_key(&code, |(c, _)| *c).ok().map(|i| cmap[i].1)
}

/// Mark `gid` kept, and — if it is a composite glyph — every component
/// glyph it references (recursively, capped at the spec recursion
/// limit).
fn mark_with_components(
    gid: u16,
    glyf: &[u8],
    loca: &[u32],
    num_glyphs: u16,
    keep: &mut [bool],
    depth: u32,
) {
    if depth > MAX_COMPOSITE_DEPTH || gid as usize >= keep.len() {
        return;
    }
    keep[gid as usize] = true;
    let begin = loca[gid as usize] as usize;
    let end = loca[gid as usize + 1] as usize;
    if begin >= end || end > glyf.len() || end - begin < 10 {
        return; // empty or too small for a header — no components
    }
    let body = &glyf[begin..end];
    let num_contours = i16_be(&body[0..2]);
    if num_contours >= 0 {
        return; // simple glyph
    }
    // Composite: walk the component records, harvesting each child gid.
    for child in composite_components(&body[10..], num_glyphs) {
        if !keep[child as usize] {
            mark_with_components(child, glyf, loca, num_glyphs, keep, depth + 1);
        }
    }
}

const C_ARG_1_AND_2_ARE_WORDS: u16 = 0x0001;
const C_WE_HAVE_A_SCALE: u16 = 0x0008;
const C_MORE_COMPONENTS: u16 = 0x0020;
const C_WE_HAVE_AN_X_AND_Y_SCALE: u16 = 0x0040;
const C_WE_HAVE_A_TWO_BY_TWO: u16 = 0x0080;

/// Collect the component glyph indices of a composite glyph from the
/// bytes that follow its 10-byte header. Stops at the first malformed
/// record (best-effort; the parser already validated the font).
fn composite_components(mut body: &[u8], num_glyphs: u16) -> Vec<u16> {
    let mut out = Vec::new();
    loop {
        if body.len() < 4 {
            break;
        }
        let flags = u16_be(&body[0..2]);
        let glyph_index = u16_be(&body[2..4]);
        if glyph_index < num_glyphs {
            out.push(glyph_index);
        }
        body = &body[4..];

        // Skip the argument words (1 or 2 bytes each).
        let arg_bytes = if flags & C_ARG_1_AND_2_ARE_WORDS != 0 { 4 } else { 2 };
        if body.len() < arg_bytes {
            break;
        }
        body = &body[arg_bytes..];

        // Skip the transform.
        let xform_bytes = if flags & C_WE_HAVE_A_TWO_BY_TWO != 0 {
            8
        } else if flags & C_WE_HAVE_AN_X_AND_Y_SCALE != 0 {
            4
        } else if flags & C_WE_HAVE_A_SCALE != 0 {
            2
        } else {
            0
        };
        if body.len() < xform_bytes {
            break;
        }
        body = &body[xform_bytes..];

        if flags & C_MORE_COMPONENTS == 0 {
            break;
        }
    }
    out
}

/// A parsed sfnt table directory: `(tag, offset, length)` per table,
/// plus the offset-table header bytes preserved verbatim.
struct Directory {
    entries: Vec<([u8; 4], usize, usize)>,
}

impl Directory {
    fn table<'a>(&self, program: &'a [u8], tag: &[u8; 4]) -> Option<&'a [u8]> {
        for (t, off, len) in &self.entries {
            if t == tag {
                return program.get(*off..*off + *len);
            }
        }
        None
    }
}

/// Parse the sfnt offset table + table directory.
fn parse_directory(program: &[u8]) -> Option<Directory> {
    if program.len() < 12 {
        return None;
    }
    if u32_be(&program[0..4]) != 0x0001_0000 {
        return None;
    }
    let num_tables = u16_be(&program[4..6]) as usize;
    let dir_end = 12usize.checked_add(num_tables.checked_mul(16)?)?;
    if dir_end > program.len() {
        return None;
    }
    let mut entries = Vec::with_capacity(num_tables);
    for i in 0..num_tables {
        let base = 12 + i * 16;
        let mut tag = [0u8; 4];
        tag.copy_from_slice(&program[base..base + 4]);
        let offset = u32_be(&program[base + 8..base + 12]) as usize;
        let length = u32_be(&program[base + 12..base + 16]) as usize;
        if offset.checked_add(length)? > program.len() {
            return None;
        }
        entries.push((tag, offset, length));
    }
    Some(Directory { entries })
}

/// Parse `loca` into `num_glyphs + 1` byte offsets into `glyf`.
fn parse_loca(
    bytes: &[u8],
    num_glyphs: u16,
    index_to_loc_format: i16,
    glyf_len: usize,
) -> Option<Vec<u32>> {
    let n = num_glyphs as usize + 1;
    let entries: Vec<u32> = if index_to_loc_format == 0 {
        if bytes.len() < n * 2 {
            return None;
        }
        (0..n).map(|i| u16_be(&bytes[i * 2..i * 2 + 2]) as u32 * 2).collect()
    } else if index_to_loc_format == 1 {
        if bytes.len() < n * 4 {
            return None;
        }
        (0..n).map(|i| u32_be(&bytes[i * 4..i * 4 + 4])).collect()
    } else {
        return None;
    };
    for w in entries.windows(2) {
        if w[1] < w[0] || w[1] as usize > glyf_len {
            return None;
        }
    }
    Some(entries)
}

/// Serialise the new `loca` offsets in the font's index-to-loc format.
fn build_loca(offsets: &[u32], index_to_loc_format: i16) -> Vec<u8> {
    let mut out = Vec::new();
    if index_to_loc_format == 0 {
        // short: offset/2 as uint16 (glyf is 4-aligned, so even).
        for &o in offsets {
            let half = (o / 2) as u16;
            out.extend_from_slice(&half.to_be_bytes());
        }
    } else {
        for &o in offsets {
            out.extend_from_slice(&o.to_be_bytes());
        }
    }
    out
}

/// Reassemble the font: copy every table, substituting the rewritten
/// `loca` / `glyf`, repacking tables 4-byte aligned with a freshly
/// built directory, recomputing per-table checksums and
/// `head.checkSumAdjustment`.
fn reassemble(program: &[u8], dir: &Directory, new_loca: &[u8], new_glyf: &[u8]) -> Vec<u8> {
    // Gather (tag, bytes) for every table, in directory order.
    let mut tables: Vec<([u8; 4], Vec<u8>)> = Vec::with_capacity(dir.entries.len());
    for (tag, off, len) in &dir.entries {
        let bytes = match tag {
            b"loca" => new_loca.to_vec(),
            b"glyf" => new_glyf.to_vec(),
            _ => program[*off..*off + *len].to_vec(),
        };
        tables.push((*tag, bytes));
    }

    let num_tables = tables.len();
    let dir_size = 12 + num_tables * 16;

    // Compute each table's data offset (4-byte aligned).
    let mut offsets: Vec<usize> = Vec::with_capacity(num_tables);
    let mut cursor = dir_size;
    for (_, bytes) in &tables {
        offsets.push(cursor);
        cursor += align4(bytes.len());
    }
    let total = cursor;

    // Build the output buffer: offset table + directory + table data.
    let mut out = vec![0u8; total];

    // ---- Offset table (sfnt header) --------------------------------
    out[0..4].copy_from_slice(&0x0001_0000u32.to_be_bytes());
    out[4..6].copy_from_slice(&(num_tables as u16).to_be_bytes());
    let (search_range, entry_selector, range_shift) = search_params(num_tables as u16);
    out[6..8].copy_from_slice(&search_range.to_be_bytes());
    out[8..10].copy_from_slice(&entry_selector.to_be_bytes());
    out[10..12].copy_from_slice(&range_shift.to_be_bytes());

    // ---- Copy each table's bytes into place ------------------------
    for (i, (_, bytes)) in tables.iter().enumerate() {
        let start = offsets[i];
        out[start..start + bytes.len()].copy_from_slice(bytes);
        // padding bytes already zero
    }

    // ---- Directory entries (with per-table checksums) --------------
    let mut head_dir_idx: Option<usize> = None;
    for (i, (tag, bytes)) in tables.iter().enumerate() {
        let entry_base = 12 + i * 16;
        out[entry_base..entry_base + 4].copy_from_slice(tag);
        let checksum = table_checksum(bytes);
        out[entry_base + 4..entry_base + 8].copy_from_slice(&checksum.to_be_bytes());
        out[entry_base + 8..entry_base + 12]
            .copy_from_slice(&(offsets[i] as u32).to_be_bytes());
        out[entry_base + 12..entry_base + 16]
            .copy_from_slice(&(bytes.len() as u32).to_be_bytes());
        if tag == b"head" {
            head_dir_idx = Some(i);
        }
    }

    // ---- head.checkSumAdjustment -----------------------------------
    // Must be zeroed when computing the whole-font checksum, then set
    // to 0xB1B0AFBA - (sum over whole file).
    if let Some(i) = head_dir_idx {
        let head_off = offsets[i];
        if head_off + 12 <= out.len() {
            // Zero the adjustment field (head bytes 8..12) before summing.
            out[head_off + 8..head_off + 12].copy_from_slice(&[0, 0, 0, 0]);
            let file_sum = table_checksum(&out);
            let adjustment = 0xB1B0_AFBAu32.wrapping_sub(file_sum);
            out[head_off + 8..head_off + 12].copy_from_slice(&adjustment.to_be_bytes());
        }
    }

    out
}

/// Round `n` up to a multiple of 4.
fn align4(n: usize) -> usize {
    (n + 3) & !3
}

/// The sum of a table's bytes as big-endian uint32 words (zero-padded
/// to a 4-byte boundary), wrapping at 2^32 — the sfnt checksum.
fn table_checksum(bytes: &[u8]) -> u32 {
    let mut sum: u32 = 0;
    let mut i = 0;
    while i < bytes.len() {
        let mut word = [0u8; 4];
        let take = (bytes.len() - i).min(4);
        word[..take].copy_from_slice(&bytes[i..i + take]);
        sum = sum.wrapping_add(u32_be(&word));
        i += 4;
    }
    sum
}

/// `(searchRange, entrySelector, rangeShift)` for `n` tables, per the
/// sfnt offset-table spec.
fn search_params(n: u16) -> (u16, u16, u16) {
    let mut entry_selector = 0u16;
    let mut pow2 = 1u16;
    while pow2 * 2 <= n {
        pow2 *= 2;
        entry_selector += 1;
    }
    let search_range = pow2 * 16;
    let range_shift = n * 16 - search_range;
    (search_range, entry_selector, range_shift)
}

fn u16_be(b: &[u8]) -> u16 {
    ((b[0] as u16) << 8) | (b[1] as u16)
}
fn i16_be(b: &[u8]) -> i16 {
    u16_be(b) as i16
}
fn u32_be(b: &[u8]) -> u32 {
    ((b[0] as u32) << 24)
        | ((b[1] as u32) << 16)
        | ((b[2] as u32) << 8)
        | (b[3] as u32)
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
    fn subset_is_smaller_and_reparses() {
        let full = liberation_sans();
        // Draw only "Hi" → codes 'H' and 'i'.
        let sub = subset(&full, b"Hi").expect("subset");
        assert!(sub.len() < full.len(), "subset {} !< full {}", sub.len(), full.len());

        // The subset is still a valid TrueType font with the SAME glyph
        // count + cmap (glyph IDs are stable; only outlines stripped).
        let ttf_full = truetype::parse(&full).unwrap();
        let ttf_sub = truetype::parse(&sub).unwrap();
        assert_eq!(ttf_full.num_glyphs, ttf_sub.num_glyphs);
        assert_eq!(ttf_full.cmap, ttf_sub.cmap);
        assert_eq!(ttf_full.widths, ttf_sub.widths);

        // The kept glyphs (H, i) retain their outlines; a never-used
        // glyph is stripped to empty.
        let gid_h = gid_for(&ttf_sub.cmap, b'H' as u32).unwrap();
        assert!(!ttf_sub.glyphs[gid_h as usize].points.is_empty(), "H kept");
    }

    #[test]
    fn empty_used_set_keeps_only_notdef() {
        let full = liberation_sans();
        let sub = subset(&full, b"").expect("subset");
        assert!(sub.len() < full.len());
        let ttf = truetype::parse(&sub).unwrap();
        // Glyph count is unchanged; nearly all outlines are gone.
        let non_empty = ttf.glyphs.iter().filter(|g| !g.points.is_empty()).count();
        assert!(non_empty <= 1, "only .notdef may keep an outline, got {non_empty}");
    }

    #[test]
    fn rejects_non_font_bytes() {
        assert!(subset(b"not a font at all", b"Hi").is_none());
    }
}
