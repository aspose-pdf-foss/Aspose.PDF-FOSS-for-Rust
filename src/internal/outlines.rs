// Outline-tree walker (PDF 32000-1:2008 §12.3.3). Pure function
// from raw PDF bytes to a flat preorder list of (depth, title)
// outline items.
//
// Composes `trailer` (locate /Root), `xref` (enumerate in-use
// indirect objects), `objects::parse_object` (materialise each
// indirect value), `encoding_tables` + `agl` (PDFDocEncoding
// /Title decode). Strategy lock: from_scratch, std-only.
//
// v1 scope:
//   * Classical trailer only (inherited from the trailer
//     primitive's v1 scope).
//   * Title-only output. /Dest, /A, /Count sign-flag,
//     /SE / /C / /F are explicitly out of scope; v2 may add
//     them as a superset.
//   * Title decode: UTF-16BE BOM, UTF-8 BOM, PDFDocEncoding
//     fallback (via encoding_tables + agl).
//   * Recursion depth capped at 1000; adversarial deeper trees
//     surface as `Error::Cycle`.
//   * /Outlines absent on Catalog → empty Outline (NOT error).

use std::collections::BTreeMap;
use std::fmt;

use super::agl;
use super::encoding_tables::{self, Encoding};
use super::objects::{self, IndirectObject, Value};
use super::trailer;
use super::xref::{self, Entry};

#[derive(Debug, Clone, PartialEq)]
#[allow(dead_code)] // retained for a future-phase public-API binding
pub struct OutlineItem {
    pub depth: u32,
    pub title: String,
}

#[derive(Debug, Clone, PartialEq, Default)]
#[allow(dead_code)] // retained for a future-phase public-API binding
pub struct Outline {
    pub items: Vec<OutlineItem>,
}

#[derive(Debug, Clone, PartialEq)]
#[allow(dead_code)] // retained for a future-phase public-API binding
pub enum Error {
    Trailer(trailer::Error),
    Xref(xref::Error),
    Objects(objects::Error),
    CatalogIdZero,
    CatalogNotFound,
    CatalogNotDict,
    OutlinesValueWrongType,
    OutlinesRootNotFound,
    OutlinesRootNotDict,
    OutlineItemNotFound,
    OutlineItemNotDict,
    OutlineItemMissingTitle,
    OutlineItemTitleNotString,
    UnpairedSurrogate,
    Cycle,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Trailer(e) => write!(f, "outlines: {e}"),
            Error::Xref(e) => write!(f, "outlines: {e}"),
            Error::Objects(e) => write!(f, "outlines: {e}"),
            Error::CatalogIdZero =>
                write!(f, "outlines: catalog object id is zero"),
            Error::CatalogNotFound =>
                write!(f, "outlines: catalog object not found"),
            Error::CatalogNotDict =>
                write!(f, "outlines: catalog is not a dictionary"),
            Error::OutlinesValueWrongType =>
                write!(f, "outlines: /Outlines is neither a Ref nor a Dict"),
            Error::OutlinesRootNotFound =>
                write!(f, "outlines: /Outlines reference does not resolve"),
            Error::OutlinesRootNotDict =>
                write!(f, "outlines: /Outlines root is not a dictionary"),
            Error::OutlineItemNotFound =>
                write!(f, "outlines: outline item reference does not resolve"),
            Error::OutlineItemNotDict =>
                write!(f, "outlines: outline item is not a dictionary"),
            Error::OutlineItemMissingTitle =>
                write!(f, "outlines: outline item missing /Title"),
            Error::OutlineItemTitleNotString =>
                write!(f, "outlines: outline item /Title is not a string"),
            Error::UnpairedSurrogate =>
                write!(f, "outlines: unpaired UTF-16 surrogate in /Title"),
            Error::Cycle =>
                write!(f, "outlines: cycle or excessive depth in outline tree"),
        }
    }
}

impl std::error::Error for Error {}

impl From<trailer::Error> for Error {
    fn from(e: trailer::Error) -> Self { Error::Trailer(e) }
}
impl From<xref::Error> for Error {
    fn from(e: xref::Error) -> Self { Error::Xref(e) }
}
impl From<objects::Error> for Error {
    fn from(e: objects::Error) -> Self { Error::Objects(e) }
}

#[allow(dead_code)] // retained for a future-phase public-API binding
const DEPTH_LIMIT: u32 = 1000;

/// Parse `input` into a preorder flat list of outline items.
#[allow(dead_code)] // retained for a future-phase public-API binding
pub fn parse(input: &[u8]) -> Result<Outline, Error> {
    let trailer_dict = trailer::parse(input)?;
    if trailer_dict.root_id == 0 {
        return Err(Error::CatalogIdZero);
    }

    let dump = build_dump(input)?;

    // Catalog generation is universally 0 (same convention as
    // pages_tree).
    let catalog = dump.get(&(trailer_dict.root_id, 0))
        .ok_or(Error::CatalogNotFound)?;
    let catalog_dict = match &catalog.value {
        Value::Dict(entries) => entries,
        _ => return Err(Error::CatalogNotDict),
    };

    // /Outlines is OPTIONAL — its absence yields an empty
    // Outline, NOT an error (PDFs without bookmarks are valid).
    let outlines_value = match lookup(catalog_dict, b"Outlines") {
        Some(v) => v,
        None => return Ok(Outline::default()),
    };

    // /Outlines is typically a Ref but may be a direct Dict.
    let outlines_dict: &[(Vec<u8>, Value)] = match outlines_value {
        Value::Ref { id, gen } => {
            let obj = dump.get(&(*id, *gen))
                .ok_or(Error::OutlinesRootNotFound)?;
            match &obj.value {
                Value::Dict(entries) => entries.as_slice(),
                _ => return Err(Error::OutlinesRootNotDict),
            }
        }
        Value::Dict(entries) => entries.as_slice(),
        _ => return Err(Error::OutlinesValueWrongType),
    };

    // The outline root has /First → top-level items.
    let first_value = match lookup(outlines_dict, b"First") {
        Some(v) => v,
        None => return Ok(Outline::default()),
    };
    let (first_id, first_gen) = match first_value {
        Value::Ref { id, gen } => (*id, *gen),
        _ => return Ok(Outline::default()),
    };

    // visit_with_first walks /Next siblings itself at the same
    // depth; we only seed it with the root's /First.
    let mut items: Vec<OutlineItem> = Vec::new();
    visit_with_first(first_id, first_gen, 0, &dump, &mut items)?;

    Ok(Outline { items })
}

/// Emit one line per item, preorder, newline-terminated:
/// `<depth>\t<title>\n`.
#[allow(dead_code)] // retained for a future-phase public-API binding
pub fn to_canonical(outline: &Outline) -> Vec<u8> {
    let mut out = Vec::new();
    for item in &outline.items {
        out.extend_from_slice(
            format!("{}\t{}\n", item.depth, item.title).as_bytes(),
        );
    }
    out
}

// ============================================================
// Dump construction — enumerate in-use objects via xref
// ============================================================

#[allow(dead_code)] // retained for a future-phase public-API binding
type Dump = BTreeMap<(u32, u16), IndirectObject>;

#[allow(dead_code)] // retained for a future-phase public-API binding
fn build_dump(input: &[u8]) -> Result<Dump, Error> {
    let table = xref::parse(input)?;
    let mut dump: Dump = BTreeMap::new();
    for entry in &table.entries {
        if let Entry::InUse { id, gen, offset } = *entry {
            // v1 scope: skip /ObjStm-compressed objects. The
            // outlines fixture corpus is classical-trailer PDFs
            // without /ObjStm containers; compressed objects
            // would surface as OutlineItemNotFound (an honest
            // signal that the v1 scope was exceeded).
            let obj = objects::parse_object(input, offset as usize)?;
            dump.insert((id, gen), obj);
        }
    }
    Ok(dump)
}

// ============================================================
// Tree walk
// ============================================================

// Visit one outline item — emit it at `depth`, then recurse
// into /First (depth + 1) and walk siblings via /Next at the
// same depth, all under the per-call recursion depth guard.
#[allow(dead_code)] // retained for a future-phase public-API binding
fn visit_with_first(
    item_id: u32,
    item_gen: u16,
    depth: u32,
    dump: &Dump,
    out: &mut Vec<OutlineItem>,
) -> Result<(), Error> {
    if depth > DEPTH_LIMIT {
        return Err(Error::Cycle);
    }

    let mut cur_id = item_id;
    let mut cur_gen = item_gen;
    loop {
        let obj = dump.get(&(cur_id, cur_gen))
            .ok_or(Error::OutlineItemNotFound)?;
        let item_dict = match &obj.value {
            Value::Dict(entries) => entries,
            _ => return Err(Error::OutlineItemNotDict),
        };

        // /Title is required per §12.3.3 on every outline item
        // (the root has no /Title — but we only reach here from
        // /First, never from the root).
        let title_value = lookup(item_dict, b"Title")
            .ok_or(Error::OutlineItemMissingTitle)?;
        let title_bytes = match title_value {
            Value::String(b) => b.as_slice(),
            _ => return Err(Error::OutlineItemTitleNotString),
        };
        let title = decode_title(title_bytes)?;

        out.push(OutlineItem { depth, title });

        // Recurse into children via /First.
        if let Some(Value::Ref { id, gen }) = lookup(item_dict, b"First") {
            visit_with_first(*id, *gen, depth + 1, dump, out)?;
        }

        // Iterate siblings via /Next (same depth). When /Next
        // is absent or not a Ref, the sibling chain ends.
        match lookup(item_dict, b"Next") {
            Some(Value::Ref { id, gen }) => {
                cur_id = *id;
                cur_gen = *gen;
            }
            _ => break,
        }
    }

    Ok(())
}

// ============================================================
// PDF text-string decoder (§7.9.2.2)
// ============================================================

#[allow(dead_code)] // retained for a future-phase public-API binding
fn decode_title(bytes: &[u8]) -> Result<String, Error> {
    if bytes.len() >= 2 && bytes[0] == 0xFE && bytes[1] == 0xFF {
        return decode_utf16be(&bytes[2..]);
    }
    if bytes.len() >= 3 && bytes[0] == 0xEF && bytes[1] == 0xBB && bytes[2] == 0xBF {
        // UTF-8 BOM — strip and pass through as-is. We use
        // from_utf8_lossy to tolerate stray bytes (matches the
        // C++ body's "pass through bytes after BOM" behaviour).
        return Ok(String::from_utf8_lossy(&bytes[3..]).into_owned());
    }
    Ok(decode_pdf_doc_encoding(bytes))
}

#[allow(dead_code)] // retained for a future-phase public-API binding
fn decode_utf16be(input: &[u8]) -> Result<String, Error> {
    let mut out = String::with_capacity(input.len() / 2);
    let mut i = 0;
    while i + 1 < input.len() {
        let high = ((input[i] as u32) << 8) | (input[i + 1] as u32);
        i += 2;
        if (0xD800..=0xDBFF).contains(&high) {
            if i + 1 >= input.len() {
                return Err(Error::UnpairedSurrogate);
            }
            let low = ((input[i] as u32) << 8) | (input[i + 1] as u32);
            i += 2;
            if !(0xDC00..=0xDFFF).contains(&low) {
                return Err(Error::UnpairedSurrogate);
            }
            let cp = 0x10000 + ((high - 0xD800) << 10) + (low - 0xDC00);
            push_codepoint(&mut out, cp);
        } else if (0xDC00..=0xDFFF).contains(&high) {
            return Err(Error::UnpairedSurrogate);
        } else {
            push_codepoint(&mut out, high);
        }
    }
    Ok(out)
}

#[allow(dead_code)] // retained for a future-phase public-API binding
fn decode_pdf_doc_encoding(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len());
    for &b in bytes {
        let name = encoding_tables::byte_to_glyph_name(Encoding::PdfDoc, b);
        if name.is_empty() {
            // Unmapped slot → Latin-1 byte-as-char fallback
            // (same convention as text_extractor v2).
            push_codepoint(&mut out, b as u32);
            continue;
        }
        let cps = agl::lookup(name);
        if cps.is_empty() {
            push_codepoint(&mut out, b as u32);
        } else {
            for cp in cps {
                push_codepoint(&mut out, cp);
            }
        }
    }
    out
}

#[allow(dead_code)] // retained for a future-phase public-API binding
fn push_codepoint(out: &mut String, cp: u32) {
    if let Some(c) = char::from_u32(cp) {
        out.push(c);
    } else {
        out.push('\u{FFFD}');
    }
}

#[allow(dead_code)] // retained for a future-phase public-API binding
fn lookup<'a>(dict: &'a [(Vec<u8>, Value)], key: &[u8]) -> Option<&'a Value> {
    dict.iter().find(|(k, _)| k == key).map(|(_, v)| v)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pdf_with_flat_outline() -> Vec<u8> {
        // 1: Catalog -> Pages 2, Outlines 3
        // 2: Pages root (one page)
        // 3: Outline root /First 4 R /Last 5 R
        // 4: Item "Intro" /Next 5 R /Parent 3 R
        // 5: Item "Conclusion" /Prev 4 R /Parent 3 R
        // 6: One page (placeholder, unreferenced by outlines)
        let header = b"%PDF-1.4\n";
        let bodies: &[&[u8]] = &[
            b"<< /Type /Catalog /Pages 2 0 R /Outlines 3 0 R >>",
            b"<< /Type /Pages /Kids [6 0 R] /Count 1 >>",
            b"<< /Type /Outlines /First 4 0 R /Last 5 0 R /Count 2 >>",
            b"<< /Title (Intro) /Parent 3 0 R /Next 5 0 R >>",
            b"<< /Title (Conclusion) /Parent 3 0 R /Prev 4 0 R >>",
            b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] >>",
        ];
        let mut out = Vec::from(&header[..]);
        let mut offsets = Vec::new();
        for (i, body) in bodies.iter().enumerate() {
            offsets.push(out.len());
            out.extend_from_slice(format!("{} 0 obj\n", i + 1).as_bytes());
            out.extend_from_slice(body);
            out.extend_from_slice(b"\nendobj\n");
        }
        let xref_pos = out.len();
        out.extend_from_slice(
            format!("xref\n0 {}\n", bodies.len() + 1).as_bytes(),
        );
        out.extend_from_slice(b"0000000000 65535 f \n");
        for off in offsets {
            out.extend_from_slice(
                format!("{off:010} 00000 n \n").as_bytes(),
            );
        }
        out.extend_from_slice(
            format!(
                "trailer\n<< /Size {} /Root 1 0 R >>\n\
                 startxref\n{xref_pos}\n%%EOF\n",
                bodies.len() + 1,
            )
            .as_bytes(),
        );
        out
    }

    fn pdf_without_outlines() -> Vec<u8> {
        let header = b"%PDF-1.4\n";
        let bodies: &[&[u8]] = &[
            b"<< /Type /Catalog /Pages 2 0 R >>",
            b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
            b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] >>",
        ];
        let mut out = Vec::from(&header[..]);
        let mut offsets = Vec::new();
        for (i, body) in bodies.iter().enumerate() {
            offsets.push(out.len());
            out.extend_from_slice(format!("{} 0 obj\n", i + 1).as_bytes());
            out.extend_from_slice(body);
            out.extend_from_slice(b"\nendobj\n");
        }
        let xref_pos = out.len();
        out.extend_from_slice(
            format!("xref\n0 {}\n", bodies.len() + 1).as_bytes(),
        );
        out.extend_from_slice(b"0000000000 65535 f \n");
        for off in offsets {
            out.extend_from_slice(
                format!("{off:010} 00000 n \n").as_bytes(),
            );
        }
        out.extend_from_slice(
            format!(
                "trailer\n<< /Size {} /Root 1 0 R >>\n\
                 startxref\n{xref_pos}\n%%EOF\n",
                bodies.len() + 1,
            )
            .as_bytes(),
        );
        out
    }

    #[test]
    fn flat_outline_emits_two_top_level_items() {
        let pdf = pdf_with_flat_outline();
        let outline = parse(&pdf).expect("parse");
        let canon = String::from_utf8(to_canonical(&outline)).expect("utf8");
        assert_eq!(canon, "0\tIntro\n0\tConclusion\n");
    }

    #[test]
    fn missing_outlines_returns_empty() {
        let pdf = pdf_without_outlines();
        let outline = parse(&pdf).expect("parse");
        assert!(outline.items.is_empty());
        assert_eq!(to_canonical(&outline), Vec::<u8>::new());
    }

    #[test]
    fn utf16be_bom_decodes_supplementary() {
        // U+1F600 GRINNING FACE, encoded UTF-16BE as a surrogate
        // pair: 0xD83D 0xDE00.
        let bytes: &[u8] = &[0xFE, 0xFF, 0xD8, 0x3D, 0xDE, 0x00];
        let out = decode_title(bytes).unwrap();
        assert_eq!(out, "\u{1F600}");
    }

    #[test]
    fn utf8_bom_strips_and_passes_through() {
        let bytes: &[u8] = &[0xEF, 0xBB, 0xBF, b'C', b'a', b'f', b'\xC3', b'\xA9'];
        // "Café" in UTF-8 = 43 61 66 C3 A9
        let out = decode_title(bytes).unwrap();
        assert_eq!(out, "Café");
    }

    #[test]
    fn pdfdoc_encoding_decodes_eacute() {
        // PDFDocEncoding 0xE9 = eacute = U+00E9.
        let bytes: &[u8] = b"Caf\xe9";
        let out = decode_title(bytes).unwrap();
        assert_eq!(out, "Café");
    }

    #[test]
    fn unpaired_surrogate_errors() {
        // Lone high surrogate without low surrogate.
        let bytes: &[u8] = &[0xFE, 0xFF, 0xD8, 0x3D, 0x00, 0x41];
        assert_eq!(decode_title(bytes), Err(Error::UnpairedSurrogate));
    }
}
