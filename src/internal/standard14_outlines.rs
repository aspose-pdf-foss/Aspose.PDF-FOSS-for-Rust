// Standard14 PostScript font name → TrueType bytes resolver.
// PDF 32000-1:2008 §9.6.2.2 + Annex D.1 — these 14 fonts must
// always be available without being embedded; consumers
// reference them by /BaseFont.
//
// Resolution order:
//   0. Canonicalize the /BaseFont name onto the nearest Standard14
//      name, so ANY non-embedded simple font resolves to a
//      metric-class substitute rather than to nothing (Arial →
//      Helvetica, TimesNewRoman → Times-Roman, Courier New →
//      Courier, bold / italic read off the name, serif-vs-sans from
//      the /FontDescriptor /Flags serif bit when the family is
//      unknown).
//   1. Probe each search_dir for filename matches (system
//      Helvetica.ttf / Arial.ttf / etc.). First hit wins.
//   2. Fall back to the bundled Liberation TTF compiled into
//      the binary.
//   3. Return empty bytes if the name has no Liberation equivalent
//      (Symbol, ZapfDingbats — documented gap).
//
// Bundled bytes come from `include_bytes!` against TTF files
// committed at `src/internal/standard14_outlines.fonts/` —
// Liberation 2.1.5 release, OFL-licenced; AUTHORS + OFL.txt
// sit alongside them for the redistribution clause.

use std::fs;
use std::path::Path;

const FONT_DIR: &str = "standard14_outlines.fonts";

// Liberation Sans → Helvetica family.
const LIB_SANS_REGULAR: &[u8] = include_bytes!(
    "standard14_outlines.fonts/LiberationSans-Regular.ttf");
const LIB_SANS_BOLD: &[u8] = include_bytes!(
    "standard14_outlines.fonts/LiberationSans-Bold.ttf");
const LIB_SANS_ITALIC: &[u8] = include_bytes!(
    "standard14_outlines.fonts/LiberationSans-Italic.ttf");
const LIB_SANS_BOLDITALIC: &[u8] = include_bytes!(
    "standard14_outlines.fonts/LiberationSans-BoldItalic.ttf");

// Liberation Serif → Times family.
const LIB_SERIF_REGULAR: &[u8] = include_bytes!(
    "standard14_outlines.fonts/LiberationSerif-Regular.ttf");
const LIB_SERIF_BOLD: &[u8] = include_bytes!(
    "standard14_outlines.fonts/LiberationSerif-Bold.ttf");
const LIB_SERIF_ITALIC: &[u8] = include_bytes!(
    "standard14_outlines.fonts/LiberationSerif-Italic.ttf");
const LIB_SERIF_BOLDITALIC: &[u8] = include_bytes!(
    "standard14_outlines.fonts/LiberationSerif-BoldItalic.ttf");

// Liberation Mono → Courier family.
const LIB_MONO_REGULAR: &[u8] = include_bytes!(
    "standard14_outlines.fonts/LiberationMono-Regular.ttf");
const LIB_MONO_BOLD: &[u8] = include_bytes!(
    "standard14_outlines.fonts/LiberationMono-Bold.ttf");
const LIB_MONO_ITALIC: &[u8] = include_bytes!(
    "standard14_outlines.fonts/LiberationMono-Italic.ttf");
const LIB_MONO_BOLDITALIC: &[u8] = include_bytes!(
    "standard14_outlines.fonts/LiberationMono-BoldItalic.ttf");

// Per-PostScript-name candidate filename list. Each name's
// candidates are probed against every `search_dir` in order;
// first hit wins. Liberation filenames come first (the
// metric-equivalent reference), then OS-native names commonly
// found on macOS / Windows / Linux.
const CANDIDATES: &[(&str, &[&str], &[u8])] = &[
    // Helvetica family
    ("Helvetica", &[
        "LiberationSans-Regular.ttf",
        "Arial.ttf", "ArialMT.ttf", "DejaVuSans.ttf",
    ], LIB_SANS_REGULAR),
    ("Helvetica-Bold", &[
        "LiberationSans-Bold.ttf",
        "Arial Bold.ttf", "Arial-Bold.ttf", "DejaVuSans-Bold.ttf",
    ], LIB_SANS_BOLD),
    ("Helvetica-Oblique", &[
        "LiberationSans-Italic.ttf",
        "Arial Italic.ttf", "Arial-Italic.ttf", "DejaVuSans-Oblique.ttf",
    ], LIB_SANS_ITALIC),
    ("Helvetica-BoldOblique", &[
        "LiberationSans-BoldItalic.ttf",
        "Arial Bold Italic.ttf", "Arial-BoldItalic.ttf",
        "DejaVuSans-BoldOblique.ttf",
    ], LIB_SANS_BOLDITALIC),

    // Times-Roman family
    ("Times-Roman", &[
        "LiberationSerif-Regular.ttf",
        "Times New Roman.ttf", "TimesNewRomanPSMT.ttf",
        "DejaVuSerif.ttf",
    ], LIB_SERIF_REGULAR),
    ("Times-Bold", &[
        "LiberationSerif-Bold.ttf",
        "Times New Roman Bold.ttf", "TimesNewRomanPS-BoldMT.ttf",
        "DejaVuSerif-Bold.ttf",
    ], LIB_SERIF_BOLD),
    ("Times-Italic", &[
        "LiberationSerif-Italic.ttf",
        "Times New Roman Italic.ttf",
        "TimesNewRomanPS-ItalicMT.ttf",
        "DejaVuSerif-Italic.ttf",
    ], LIB_SERIF_ITALIC),
    ("Times-BoldItalic", &[
        "LiberationSerif-BoldItalic.ttf",
        "Times New Roman Bold Italic.ttf",
        "TimesNewRomanPS-BoldItalicMT.ttf",
        "DejaVuSerif-BoldItalic.ttf",
    ], LIB_SERIF_BOLDITALIC),

    // Courier family
    ("Courier", &[
        "LiberationMono-Regular.ttf",
        "Courier New.ttf", "CourierNewPSMT.ttf",
        "DejaVuSansMono.ttf",
    ], LIB_MONO_REGULAR),
    ("Courier-Bold", &[
        "LiberationMono-Bold.ttf",
        "Courier New Bold.ttf", "CourierNewPS-BoldMT.ttf",
        "DejaVuSansMono-Bold.ttf",
    ], LIB_MONO_BOLD),
    ("Courier-Oblique", &[
        "LiberationMono-Italic.ttf",
        "Courier New Italic.ttf", "CourierNewPS-ItalicMT.ttf",
        "DejaVuSansMono-Oblique.ttf",
    ], LIB_MONO_ITALIC),
    ("Courier-BoldOblique", &[
        "LiberationMono-BoldItalic.ttf",
        "Courier New Bold Italic.ttf",
        "CourierNewPS-BoldItalicMT.ttf",
        "DejaVuSansMono-BoldOblique.ttf",
    ], LIB_MONO_BOLDITALIC),
    // Symbol and ZapfDingbats: no Liberation equivalent. The two
    // entries below carry no candidate list and an empty bundle;
    // the resolver returns empty for both.
];

// Strip a leading '/' so callers can pass `/Helvetica` or
// `Helvetica` interchangeably (PDF font dicts spell /BaseFont
// with the slash; lexer-stripped names arrive without).
fn strip_slash(name: &str) -> &str {
    name.strip_prefix('/').unwrap_or(name)
}

/// Map an arbitrary `/BaseFont` name onto the nearest of the twelve
/// resolvable Standard14 names, so a non-embedded font that is not
/// itself a Standard14 still substitutes to a metric-class outline
/// instead of rendering nothing. `prefer_serif` is the caller's
/// `/FontDescriptor /Flags` serif bit and decides a name whose family
/// nothing in it names.
///
/// Always answers one of the twelve. Symbol and ZapfDingbats are not
/// among them and are not meant to be: `resolve` keeps its documented
/// gap for those two before it gets here, because substituting a text
/// face for a symbol font paints the wrong glyphs rather than none.
pub fn canonicalize(base_font_name: &str, prefer_serif: bool) -> String {
    let bare = strip_slash(base_font_name);
    // 1. A subset prefix names the same face: `ABCDEF+Arial` → `Arial`.
    let bare = bare.split_once('+').map_or(bare, |(_, n)| n);
    if CANDIDATES.iter().any(|(n, _, _)| *n == bare) {
        return bare.to_string();
    }

    let lower = bare.to_ascii_lowercase();
    let any = |tokens: &[&str]| tokens.iter().any(|t| lower.contains(t));

    // 2. Family from name tokens, monospace first, then sans, then
    //    serif; a name naming none follows the serif bit.
    enum Family {
        Sans,
        Serif,
        Mono,
    }
    let family = if any(&["courier", "mono", "consol"]) {
        Family::Mono
    } else if any(&[
        "arial", "helvetica", "sans", "verdana", "tahoma", "calibri", "segoe", "trebuchet",
        "gothic",
    ]) {
        Family::Sans
    } else if any(&["times", "serif", "roman", "georgia", "garamond", "palatino"]) || prefer_serif
    {
        // A name that names no family follows the serif bit.
        Family::Serif
    } else {
        Family::Sans
    };

    // 3. Weight and slope from the same tokens.
    let bold = any(&["bold", "black", "heavy", "semibold"]);
    let italic = any(&["italic", "oblique"]);

    // 4. Compose, in each family's own spelling of the variants.
    match family {
        Family::Sans => match (bold, italic) {
            (true, true) => "Helvetica-BoldOblique",
            (true, false) => "Helvetica-Bold",
            (false, true) => "Helvetica-Oblique",
            (false, false) => "Helvetica",
        },
        Family::Serif => match (bold, italic) {
            (true, true) => "Times-BoldItalic",
            (true, false) => "Times-Bold",
            (false, true) => "Times-Italic",
            (false, false) => "Times-Roman",
        },
        Family::Mono => match (bold, italic) {
            (true, true) => "Courier-BoldOblique",
            (true, false) => "Courier-Bold",
            (false, true) => "Courier-Oblique",
            (false, false) => "Courier",
        },
    }
    .to_string()
}

fn lookup(name: &str) -> Option<(&'static [&'static str], &'static [u8])> {
    let bare = strip_slash(name);
    if bare.is_empty() {
        return None;
    }
    for (n, cands, bytes) in CANDIDATES {
        if *n == bare {
            return Some((cands, bytes));
        }
    }
    None
}

/// Resolve a Standard14 PostScript font name to a TrueType
/// byte buffer. Returns an empty `Vec` when the name is
/// unknown or has no bundled fallback (Symbol, ZapfDingbats).
pub fn resolve(base_font_name: &str, search_dirs: &[&str], serif: bool) -> Vec<u8> {
    // The two documented gaps are decided before canonicalizing, which
    // would otherwise map them into a text family: there is no
    // metric-class substitute for a symbol font.
    let bare = strip_slash(base_font_name);
    let bare = bare.split_once('+').map_or(bare, |(_, n)| n);
    if bare.is_empty() || bare == "Symbol" || bare == "ZapfDingbats" {
        return Vec::new();
    }
    let canonical = canonicalize(base_font_name, serif);
    let Some((cands, bundle)) = lookup(&canonical) else {
        return Vec::new();
    };
    // Probe each (dir, candidate) — top-level scan only, no
    // recursion, paralleling the cpp / typescript strategy.
    for dir in search_dirs {
        if dir.is_empty() {
            continue;
        }
        for fn_ in cands.iter() {
            let path = Path::new(dir).join(fn_);
            if let Ok(bytes) = fs::read(&path) {
                if !bytes.is_empty() {
                    return bytes;
                }
            }
        }
    }
    // Fall through: bundled Liberation. Symbol / ZapfDingbats
    // never reach here because `lookup` returns None for them.
    bundle.to_vec()
}

/// Per-OS default font-search directories. Mirrors cpp's
/// per-OS ifdef blocks and csharp's `OperatingSystem.*` ladder.
pub fn default_search_dirs() -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    if cfg!(target_os = "macos") {
        out.push("/System/Library/Fonts".to_string());
        out.push("/Library/Fonts".to_string());
        if let Ok(home) = std::env::var("HOME") {
            out.push(format!("{home}/Library/Fonts"));
        }
    } else if cfg!(target_os = "windows") {
        let windir = std::env::var("WINDIR")
            .unwrap_or_else(|_| "C:\\Windows".to_string());
        out.push(format!("{windir}\\Fonts"));
    } else {
        // Linux / *BSD.
        out.push("/usr/share/fonts".to_string());
        out.push("/usr/local/share/fonts".to_string());
        if let Ok(home) = std::env::var("HOME") {
            out.push(format!("{home}/.fonts"));
            out.push(format!("{home}/.local/share/fonts"));
        }
    }
    out
}

/// Bundled TTF bytes for `base_font_name`. Empty slice when
/// the name isn't in the 12-entry bundle (Symbol,
/// ZapfDingbats, or any non-Standard14 name).
// Reached only by the freeze-gate driver, which mounts this module
// standalone; the renderer goes through `resolve`.
#[allow(dead_code)]
pub fn bundled(base_font_name: &str) -> &'static [u8] {
    lookup(base_font_name)
        .map(|(_, bytes)| bytes)
        .unwrap_or(&[])
}

// Reference the shared font-dir constant so the linker doesn't
// drop it. (Not used at runtime — kept for tooling that might
// inspect the source.)
#[allow(dead_code)]
const _FONT_DIR_NAME: &str = FONT_DIR;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn helvetica_bundle_fallback() {
        // Empty search_dirs forces the bundle path.
        let bytes = resolve("/Helvetica", &[], false);
        assert!(!bytes.is_empty(), "Helvetica should fall back to LiberationSans-Regular");
    }

    #[test]
    fn times_roman_bundle_fallback() {
        let bytes = resolve("/Times-Roman", &[], false);
        assert!(!bytes.is_empty(), "Times-Roman should fall back to LiberationSerif-Regular");
    }

    #[test]
    fn courier_bundle_fallback() {
        let bytes = resolve("/Courier", &[], false);
        assert!(!bytes.is_empty(), "Courier should fall back to LiberationMono-Regular");
    }

    #[test]
    fn symbol_documented_gap() {
        // No Liberation equivalent — empty bytes, by design.
        assert!(resolve("/Symbol", &[], false).is_empty());
    }

    #[test]
    fn zapfdingbats_documented_gap() {
        assert!(resolve("/ZapfDingbats", &[], false).is_empty());
    }

    #[test]
    fn an_unknown_name_resolves_to_its_metric_class() {
        // This used to return nothing, which is what made a document's
        // non-embedded Arial come out blank.
        assert!(!resolve("/NotAStandard14Name", &[], false).is_empty());
        assert_eq!(
            resolve("/NotAStandard14Name", &[], false).len(),
            resolve("/Helvetica", &[], false).len(),
            "a name that says nothing and claims no serif falls to Helvetica"
        );
        assert_eq!(
            resolve("/NotAStandard14Name", &[], true).len(),
            resolve("/Times-Roman", &[], false).len(),
            "and to Times when its descriptor says serif"
        );
    }

    #[test]
    fn canonicalize_maps_a_name_onto_the_nearest_standard14() {
        // Already Standard14: unchanged.
        assert_eq!(canonicalize("/Helvetica-Bold", false), "Helvetica-Bold");
        // Known families, by name.
        assert_eq!(canonicalize("Arial", false), "Helvetica");
        assert_eq!(canonicalize("Arial,Bold", false), "Helvetica-Bold");
        assert_eq!(canonicalize("ArialMT-BoldItalic", false), "Helvetica-BoldOblique");
        assert_eq!(canonicalize("TimesNewRoman", false), "Times-Roman");
        assert_eq!(canonicalize("TimesNewRomanPS-ItalicMT", false), "Times-Italic");
        assert_eq!(canonicalize("Courier New Bold", false), "Courier-Bold");
        assert_eq!(canonicalize("Consolas", false), "Courier");
        // The anchor's own examples, including the sans tokens a bare
        // serif-bit default would otherwise have decided.
        assert_eq!(canonicalize("Calibri", true), "Helvetica");
        assert_eq!(canonicalize("TrebuchetMS", true), "Helvetica");
        assert_eq!(canonicalize("Palatino", false), "Times-Roman");
        // A subset prefix names the same face.
        assert_eq!(canonicalize("ABCDEF+Arial-Bold", false), "Helvetica-Bold");
        // Unknown family: the serif bit decides.
        assert_eq!(canonicalize("Whatever", false), "Helvetica");
        assert_eq!(canonicalize("Whatever", true), "Times-Roman");
        assert_eq!(canonicalize("Whatever-Oblique", true), "Times-Italic");
        // Canonicalize always answers one of the twelve; the two
        // symbol fonts are kept out by `resolve`, not here.
        assert!(resolve("/Symbol", &[], false).is_empty());
        assert!(resolve("/ZapfDingbats", &[], true).is_empty());
    }

    #[test]
    fn default_search_dirs_non_empty() {
        let dirs = default_search_dirs();
        assert!(!dirs.is_empty(), "every supported OS has at least one system font dir");
    }

    #[test]
    fn slash_optional() {
        // With and without leading slash should resolve identically.
        assert_eq!(
            resolve("/Helvetica", &[], false).len(),
            resolve("Helvetica", &[], false).len(),
        );
    }

    #[test]
    fn helvetica_starts_with_ttf_magic() {
        // First 4 bytes of a TrueType file are 0x00 0x01 0x00 0x00.
        let bytes = resolve("/Helvetica", &[], false);
        assert!(bytes.len() > 4);
        assert_eq!(&bytes[..4], &[0x00, 0x01, 0x00, 0x00]);
    }
}
