// Adobe Glyph List (AGL) lookup — pure-data primitive backing
// PDF 32000-1:2008 §9.10.2 (the second arrow in the
// byte → glyph_name → Unicode fallback chain).
//
// Strategy: hand-authored §4 algorithm against a static AGL
// table generated from pdfminer.six's glyphname2unicode (which
// republishes Adobe's authoritative list). The data lives in the
// sibling `agl_data.rs` file; refresh it by running
// `Feedstock/PDF/Spec/targets/rust/agl.py`.
//
// No corpus freeze gate — the data IS the canonical reference,
// so unit tests spot-check ~30 representative entries (single-
// codepoint, multi-codepoint, uniHHHH, uHHHHHH, suffix strip,
// underscore split, surrogate rejection, unknown).

#[path = "agl_data.rs"]
mod data;

/// Look up a PostScript glyph name and return the Unicode
/// codepoint sequence it represents per the AGL §4 algorithm.
/// Returns an empty vector for an unknown / malformed name.
pub fn lookup(name: &str) -> Vec<u32> {
    // §4.1 — strip suffix from the first dot.
    let stripped = match name.find('.') {
        Some(idx) => &name[..idx],
        None => name,
    };
    // §4.2 — underscore-separated component name; recurse and
    // concatenate. This catches f_f_i (three-glyph ligature
    // assembled by the renderer); precomposed entries like "ffi"
    // (no underscore) hit the static table at §4.3 instead.
    if stripped.contains('_') {
        let mut out = Vec::new();
        for part in stripped.split('_') {
            out.extend(lookup_single(part));
        }
        return out;
    }
    lookup_single(stripped)
}

fn lookup_single(name: &str) -> Vec<u32> {
    if name.is_empty() {
        return Vec::new();
    }
    // §4.3 — static AGL table, binary search over the sorted
    // (name, codepoints) slice.
    if let Ok(idx) = data::AGL_TABLE.binary_search_by(
        |(k, _)| (*k).cmp(name)
    ) {
        return data::AGL_TABLE[idx].1.to_vec();
    }
    // §4.4 — uniHHHH... groups (multiple of 4 uppercase hex
    // digits, each a BMP codepoint; surrogates D800..=DFFF
    // rejected, so the whole name yields empty if any chunk is
    // a surrogate).
    if let Some(rest) = name.strip_prefix("uni") {
        if !rest.is_empty() && rest.len() % 4 == 0
            && rest.bytes().all(is_hex_upper)
        {
            let mut out = Vec::with_capacity(rest.len() / 4);
            for chunk in rest.as_bytes().chunks(4) {
                let h = parse_hex(chunk);
                if (0xD800..=0xDFFF).contains(&h) {
                    return Vec::new();
                }
                out.push(h);
            }
            return out;
        }
    }
    // §4.5 — uHHHHHH (4..=6 uppercase hex digits, single codepoint).
    if let Some(rest) = name.strip_prefix('u') {
        if (4..=6).contains(&rest.len())
            && rest.bytes().all(is_hex_upper)
        {
            let h = parse_hex(rest.as_bytes());
            if h < 0x110000 && !(0xD800..=0xDFFF).contains(&h) {
                return vec![h];
            }
        }
    }
    // §4.6 — unknown / malformed.
    Vec::new()
}

fn is_hex_upper(b: u8) -> bool {
    matches!(b, b'0'..=b'9' | b'A'..=b'F')
}

fn parse_hex(bytes: &[u8]) -> u32 {
    let mut h = 0u32;
    for &b in bytes {
        let nib = match b {
            b'0'..=b'9' => b - b'0',
            b'A'..=b'F' => b - b'A' + 10,
            _ => unreachable!("caller guarantees uppercase hex"),
        };
        h = (h << 4) | nib as u32;
    }
    h
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn single_codepoint_basic_latin() {
        assert_eq!(lookup("A"), vec![0x41]);
        assert_eq!(lookup("z"), vec![0x7A]);
        assert_eq!(lookup("space"), vec![0x20]);
    }

    #[test]
    fn single_codepoint_accented() {
        assert_eq!(lookup("Aacute"), vec![0xC1]);
        assert_eq!(lookup("eacute"), vec![0xE9]);
        assert_eq!(lookup("AE"), vec![0xC6]);
    }

    #[test]
    fn precomposed_ligature_in_table() {
        // "ffi" (no underscore) is a single AGL entry mapping
        // to the precomposed FB03; the underscore-split path
        // does not run.
        assert_eq!(lookup("ffi"), vec![0xFB03]);
        assert_eq!(lookup("ff"), vec![0xFB00]);
        assert_eq!(lookup("fi"), vec![0xFB01]);
    }

    #[test]
    fn underscore_split_compound() {
        // "f_f_i" splits into three components, each looked up
        // separately and concatenated.
        assert_eq!(lookup("f_f_i"), vec![0x66, 0x66, 0x69]);
    }

    #[test]
    fn underscore_split_with_unknown_yields_empty_for_part() {
        // Unknown component contributes nothing; known
        // components still emit. "A_unknownname_B" → A, B.
        assert_eq!(lookup("A_unknownname_B"), vec![0x41, 0x42]);
    }

    #[test]
    fn dot_suffix_stripped() {
        assert_eq!(lookup("A.alt"), vec![0x41]);
        assert_eq!(lookup("A.sc"), vec![0x41]);
        assert_eq!(lookup("eacute.smcp"), vec![0xE9]);
    }

    #[test]
    fn uni_single_4hex() {
        assert_eq!(lookup("uni0041"), vec![0x41]);
        assert_eq!(lookup("uni4E2D"), vec![0x4E2D]);
    }

    #[test]
    fn uni_multi_4hex_concatenates() {
        // §4.4: multiple 4-hex groups concatenated.
        assert_eq!(lookup("uni00410042"), vec![0x41, 0x42]);
    }

    #[test]
    fn uni_surrogate_rejected() {
        // Surrogates D800..=DFFF are not legal codepoints; the
        // whole name yields empty.
        assert_eq!(lookup("uniD800"), Vec::<u32>::new());
        assert_eq!(lookup("uni0041D800"), Vec::<u32>::new());
    }

    #[test]
    fn u_single_codepoint_4to6_hex() {
        assert_eq!(lookup("u0041"), vec![0x41]);
        assert_eq!(lookup("u1F600"), vec![0x1F600]);
        assert_eq!(lookup("u10FFFD"), vec![0x10FFFD]);
    }

    #[test]
    fn u_above_max_codepoint_rejected() {
        // > U+10FFFF is not a valid codepoint.
        assert_eq!(lookup("u110000"), Vec::<u32>::new());
    }

    #[test]
    fn u_lowercase_hex_rejected() {
        // §4 specifies uppercase hex; lowercase yields empty.
        assert_eq!(lookup("u00ab"), Vec::<u32>::new());
        assert_eq!(lookup("uni00ab"), Vec::<u32>::new());
    }

    #[test]
    fn uni_wrong_length_rejected() {
        // §4.4: each group is exactly 4 hex digits.
        assert_eq!(lookup("uni041"), Vec::<u32>::new());   // 3
        assert_eq!(lookup("uni00415"), Vec::<u32>::new()); // 5
    }

    #[test]
    fn unknown_returns_empty() {
        assert_eq!(lookup("notarealglyph"), Vec::<u32>::new());
        assert_eq!(lookup(""), Vec::<u32>::new());
    }

    #[test]
    fn multi_codepoint_table_entry() {
        // §1 includes 81 multi-codepoint entries (Hebrew + Arabic
        // compound names). "ffi" maps to a single FB03 (above);
        // pick a genuine multi-codepoint one.
        let result = lookup("dalethatafpatahhebrew");
        assert!(result.len() >= 2,
                "expected multi-codepoint, got {result:?}");
    }

    #[test]
    fn dot_then_underscore_strips_first() {
        // §4 ordering: suffix strip runs before underscore
        // split. "A.alt_B.alt" → strip → "A" → no underscore.
        assert_eq!(lookup("A.alt_B.alt"), vec![0x41]);
    }

    #[test]
    fn suffix_only_returns_empty() {
        // Empty name after suffix strip should not match
        // anything.
        assert_eq!(lookup(".alt"), Vec::<u32>::new());
    }

    #[test]
    fn table_is_sorted_for_binary_search() {
        // Defensive — the codegen guarantees this; the test
        // lives here so a hand-edit that breaks the ordering
        // surfaces immediately.
        for window in data::AGL_TABLE.windows(2) {
            assert!(window[0].0 < window[1].0,
                    "AGL_TABLE not sorted near {} / {}",
                    window[0].0, window[1].0);
        }
    }

    #[test]
    fn table_size_matches_pdfminer_baseline() {
        // pdfminer.six's table is ~4280 entries. Tight bounds
        // catch a regenerator that silently truncates / drops
        // entries; loose enough that an upstream
        // pdfminer minor-rev that adds a handful of names
        // doesn't break the gate.
        let n = data::AGL_TABLE.len();
        assert!((4200..=4400).contains(&n),
                "AGL_TABLE size {n} outside expected 4200..=4400");
    }
}
