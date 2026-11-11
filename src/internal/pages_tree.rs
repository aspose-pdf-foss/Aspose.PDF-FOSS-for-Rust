// /Pages-tree walker (PDF 32000-1:2008 §7.7.3). Pure function
// from raw PDF bytes to a flat preorder list of leaf pages with
// resolved geometry (MediaBox width/height + Rotate).
//
// Composes `trailer` (to obtain /Root id), `xref` (to enumerate
// in-use indirect objects), and `objects::parse_object` (to
// materialise each object's value). Walks the /Pages tree
// preorder, inheriting /MediaBox + /Rotate down the chain;
// emits one LeafPage per /Type /Page node.
//
// v1 scope:
//   * Classical trailer only (inherited from the trailer
//     primitive's v1 scope). /XRef-stream trailers throw via
//     `trailer::Error::MissingTrailerKeyword`.
//   * Inheritance chain absent across the entire walk for
//     /MediaBox is an error (every leaf must resolve to a
//     well-defined geometry). /Rotate defaults to 0.
//   * Rotation values are snapped to the nearest multiple of
//     90 and reduced to [0, 360).
//   * Output exposes (id, 1-based pagenum, width, height,
//     rotation). /Resources, /CropBox, /BleedBox, /TrimBox,
//     /ArtBox, /Contents are intentionally not surfaced.

use std::collections::BTreeMap;
use std::collections::HashSet;
use std::fmt;

use super::objects::{self, IndirectObject, Value};
use super::trailer;
use super::xref::{self, Entry};

#[derive(Debug, Clone, PartialEq)]
pub struct LeafPage {
    pub id: u32,
    pub pagenum: u64,
    pub width: f64,
    pub height: f64,
    pub rotation: i32,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Tree {
    pub leaves: Vec<LeafPage>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Error {
    Trailer(trailer::Error),
    Xref(xref::Error),
    Objects(objects::Error),
    CatalogIdZero,
    CatalogNotFound,
    CatalogNotDict,
    CatalogMissingPages,
    PagesNotRef,
    PagesRootNotFound,
    PagesRootNotDict,
    PagesRootWrongType,
    NodeNotDict,
    KidsNotArray,
    KidsEntryNotRef,
    KidNotFound,
    LeafMissingMediaBox,
    MediaBoxNotArray,
    MediaBoxElementNotNumeric,
    UnknownNodeType(Vec<u8>),
    Cycle,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Trailer(e) => write!(f, "pages_tree: {e}"),
            Error::Xref(e) => write!(f, "pages_tree: {e}"),
            Error::Objects(e) => write!(f, "pages_tree: {e}"),
            Error::CatalogIdZero =>
                write!(f, "pages_tree: catalog object id is zero"),
            Error::CatalogNotFound =>
                write!(f, "pages_tree: catalog object not found"),
            Error::CatalogNotDict =>
                write!(f, "pages_tree: catalog is not a dictionary"),
            Error::CatalogMissingPages =>
                write!(f, "pages_tree: catalog missing /Pages entry"),
            Error::PagesNotRef =>
                write!(f, "pages_tree: /Pages entry is not an indirect reference"),
            Error::PagesRootNotFound =>
                write!(f, "pages_tree: page-tree root object not found"),
            Error::PagesRootNotDict =>
                write!(f, "pages_tree: page-tree root is not a dictionary"),
            Error::PagesRootWrongType =>
                write!(f, "pages_tree: page-tree root /Type is not /Pages"),
            Error::NodeNotDict =>
                write!(f, "pages_tree: page tree node is not a dictionary"),
            Error::KidsNotArray =>
                write!(f, "pages_tree: /Kids is not an array"),
            Error::KidsEntryNotRef =>
                write!(f, "pages_tree: /Kids entry is not an indirect reference"),
            Error::KidNotFound =>
                write!(f, "pages_tree: page tree kid object not found"),
            Error::LeafMissingMediaBox =>
                write!(f, "pages_tree: leaf page has no /MediaBox in inheritance chain"),
            Error::MediaBoxNotArray =>
                write!(f, "pages_tree: /MediaBox is not a 4-element array"),
            Error::MediaBoxElementNotNumeric =>
                write!(f, "pages_tree: /MediaBox element is not numeric"),
            Error::UnknownNodeType(t) => {
                let s = String::from_utf8_lossy(t);
                write!(f, "pages_tree: unknown page tree node type: {s}")
            }
            Error::Cycle =>
                write!(f, "pages_tree: cycle detected in page tree"),
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

/// Parse `input` into a flat preorder list of leaf pages.
pub fn parse(input: &[u8]) -> Result<Tree, Error> {
    let trailer_dict = trailer::parse(input)?;
    if trailer_dict.root_id == 0 {
        return Err(Error::CatalogIdZero);
    }

    let dump = build_dump(input)?;

    // Catalog generation is universally 0; we look up by
    // (root_id, 0). PDF spec §7.5.5 doesn't pin the generation
    // explicitly, but every real-world Catalog is gen 0.
    let catalog = dump.get(&(trailer_dict.root_id, 0))
        .ok_or(Error::CatalogNotFound)?;
    let catalog_dict = match &catalog.value {
        Value::Dict(entries) => entries,
        _ => return Err(Error::CatalogNotDict),
    };

    let pages_value = lookup(catalog_dict, b"Pages")
        .ok_or(Error::CatalogMissingPages)?;
    let (pages_id, pages_gen) = match pages_value {
        Value::Ref { id, gen } => (*id, *gen),
        _ => return Err(Error::PagesNotRef),
    };

    let pages_root = dump.get(&(pages_id, pages_gen))
        .ok_or(Error::PagesRootNotFound)?;
    let pages_root_dict = match &pages_root.value {
        Value::Dict(entries) => entries,
        _ => return Err(Error::PagesRootNotDict),
    };
    match lookup(pages_root_dict, b"Type") {
        Some(Value::Name(name)) if name == b"Pages" => {}
        _ => return Err(Error::PagesRootWrongType),
    }

    let mut leaves = Vec::new();
    let mut next_pagenum: u64 = 1;
    let mut visited: HashSet<(u32, u16)> = HashSet::new();
    visited.insert((pages_id, pages_gen));
    walk(
        pages_root,
        &dump,
        &mut next_pagenum,
        &mut leaves,
        None,
        0,
        &mut visited,
    )?;
    visited.remove(&(pages_id, pages_gen));

    Ok(Tree { leaves })
}

/// Emit one line per leaf page, preorder, newline-terminated:
/// `page <pagenum> <width> <height> <rotation>\n`.
#[allow(dead_code)] // retained for a future-phase public-API binding
pub fn to_canonical(tree: &Tree) -> Vec<u8> {
    let mut out = Vec::new();
    for leaf in &tree.leaves {
        out.extend_from_slice(
            format!(
                "page {} {} {} {}\n",
                leaf.pagenum,
                format_number(leaf.width),
                format_number(leaf.height),
                leaf.rotation,
            ).as_bytes(),
        );
    }
    out
}

// ============================================================
// Dump construction — enumerate in-use objects via xref
// ============================================================

type Dump = BTreeMap<(u32, u16), IndirectObject>;

fn build_dump(input: &[u8]) -> Result<Dump, Error> {
    let table = xref::parse(input)?;
    let mut dump: Dump = BTreeMap::new();
    for entry in &table.entries {
        if let Entry::InUse { id, gen, offset } = *entry {
            // v1 scope: skip /ObjStm-compressed objects. The
            // pages_tree fixture corpus is classical-trailer
            // PDFs without /ObjStm containers; compressed
            // objects in the tree would surface as
            // PagesRootNotFound / KidNotFound (an honest
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

#[allow(clippy::too_many_arguments)]
fn walk(
    obj: &IndirectObject,
    dump: &Dump,
    next_pagenum: &mut u64,
    leaves: &mut Vec<LeafPage>,
    inherited_media: Option<[f64; 4]>,
    inherited_rotation: i32,
    visited: &mut HashSet<(u32, u16)>,
) -> Result<(), Error> {
    let dict = match &obj.value {
        Value::Dict(entries) => entries,
        _ => return Err(Error::NodeNotDict),
    };

    // Default node type per spec is /Page when /Type absent on a
    // leaf-shaped node; here the root is enforced /Pages by the
    // caller, so any node we reach inside the recursion that
    // lacks /Type is treated as /Page (matches C++/C# bodies).
    let node_type: &[u8] = match lookup(dict, b"Type") {
        Some(Value::Name(name)) => name.as_slice(),
        _ => b"Page",
    };

    // Merge declared /MediaBox + /Rotate into the cumulative
    // state for both /Pages-recursion AND /Page-emit branches.
    let mut local_media = inherited_media;
    if let Some(mb_value) = lookup(dict, b"MediaBox") {
        local_media = Some(parse_media_box(mb_value)?);
    }
    let mut local_rotation = inherited_rotation;
    if let Some(rot_value) = lookup(dict, b"Rotate") {
        if let Some(r) = read_integer(rot_value) {
            local_rotation = normalize_rotation(r);
        }
    }

    match node_type {
        b"Pages" => {
            let kids_value = lookup(dict, b"Kids")
                .ok_or(Error::KidsNotArray)?;
            let kids = match kids_value {
                Value::Array(items) => items,
                _ => return Err(Error::KidsNotArray),
            };
            for kid in kids {
                let (kid_id, kid_gen) = match kid {
                    Value::Ref { id, gen } => (*id, *gen),
                    _ => return Err(Error::KidsEntryNotRef),
                };
                if !visited.insert((kid_id, kid_gen)) {
                    return Err(Error::Cycle);
                }
                let kid_obj = dump.get(&(kid_id, kid_gen))
                    .ok_or(Error::KidNotFound)?;
                walk(
                    kid_obj,
                    dump,
                    next_pagenum,
                    leaves,
                    local_media,
                    local_rotation,
                    visited,
                )?;
                visited.remove(&(kid_id, kid_gen));
            }
            Ok(())
        }
        b"Page" => {
            let media = local_media.ok_or(Error::LeafMissingMediaBox)?;
            let width = media[2] - media[0];
            let height = media[3] - media[1];
            leaves.push(LeafPage {
                id: obj.id,
                pagenum: *next_pagenum,
                width,
                height,
                rotation: local_rotation,
            });
            *next_pagenum += 1;
            Ok(())
        }
        other => Err(Error::UnknownNodeType(other.to_vec())),
    }
}

fn parse_media_box(v: &Value) -> Result<[f64; 4], Error> {
    let items = match v {
        Value::Array(items) if items.len() == 4 => items,
        _ => return Err(Error::MediaBoxNotArray),
    };
    let mut out = [0.0f64; 4];
    for (i, item) in items.iter().enumerate() {
        out[i] = read_number(item)
            .ok_or(Error::MediaBoxElementNotNumeric)?;
    }
    Ok(out)
}

fn read_number(v: &Value) -> Option<f64> {
    match v {
        Value::Integer(n) => Some(*n as f64),
        Value::Real(x) => Some(*x),
        _ => None,
    }
}

fn read_integer(v: &Value) -> Option<i64> {
    match v {
        Value::Integer(n) => Some(*n),
        Value::Real(x) => Some(x.round() as i64),
        _ => None,
    }
}

fn normalize_rotation(raw: i64) -> i32 {
    // Snap to nearest multiple of 90, reduce to [0, 360).
    let snapped = ((raw as f64) / 90.0).round() * 90.0;
    let reduced = snapped.rem_euclid(360.0);
    reduced as i32
}

fn lookup<'a>(dict: &'a [(Vec<u8>, Value)], key: &[u8]) -> Option<&'a Value> {
    dict.iter().find(|(k, _)| k == key).map(|(_, v)| v)
}

#[allow(dead_code)] // retained for a future-phase public-API binding
fn format_number(value: f64) -> String {
    // Integer when exact-integer-valued; otherwise Rust's
    // shortest-round-trip Display form (matches `%g` on the
    // canonical fixture inputs).
    if value.is_finite()
        && value == value.trunc()
        && value.abs() < (i64::MAX as f64)
    {
        format!("{}", value as i64)
    } else {
        format!("{value}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Hand-built minimal PDF, classical trailer, no streams —
    // exercises the inheritance branch (/MediaBox declared on
    // a /Pages intermediate node, leaves omit it).
    fn pdf_with_inherited_mediabox() -> Vec<u8> {
        // 1: Catalog -> 2 (Pages root)
        // 2: Pages root, declares /MediaBox + /Kids [3 0 R]
        // 3: Pages intermediate, /Kids [4 0 R 5 0 R]
        // 4: Page (no MediaBox — inherits)
        // 5: Page (no MediaBox — inherits, /Rotate 270)
        let header = b"%PDF-1.4\n";
        let bodies: &[&[u8]] = &[
            b"<< /Type /Catalog /Pages 2 0 R >>",
            b"<< /Type /Pages /Kids [3 0 R] /Count 2 \
              /MediaBox [0 0 50 75] >>",
            b"<< /Type /Pages /Parent 2 0 R \
              /Kids [4 0 R 5 0 R] /Count 2 >>",
            b"<< /Type /Page /Parent 3 0 R >>",
            b"<< /Type /Page /Parent 3 0 R /Rotate 270 >>",
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
            format!("xref\n0 {}\n", bodies.len() + 1).as_bytes());
        out.extend_from_slice(b"0000000000 65535 f \n");
        for off in offsets {
            out.extend_from_slice(
                format!("{off:010} 00000 n \n").as_bytes());
        }
        out.extend_from_slice(
            format!(
                "trailer\n<< /Size {} /Root 1 0 R >>\n\
                 startxref\n{xref_pos}\n%%EOF\n",
                bodies.len() + 1,
            ).as_bytes(),
        );
        out
    }

    #[test]
    fn inherits_mediabox_from_intermediate() {
        let pdf = pdf_with_inherited_mediabox();
        let tree = parse(&pdf).expect("parse");
        let canon = String::from_utf8(to_canonical(&tree)).expect("utf8");
        assert_eq!(canon, "page 1 50 75 0\npage 2 50 75 270\n");
    }

    #[test]
    fn rotation_snapping() {
        assert_eq!(normalize_rotation(0), 0);
        assert_eq!(normalize_rotation(90), 90);
        assert_eq!(normalize_rotation(180), 180);
        assert_eq!(normalize_rotation(270), 270);
        assert_eq!(normalize_rotation(360), 0);
        assert_eq!(normalize_rotation(450), 90);
        assert_eq!(normalize_rotation(-90), 270);
        // Non-multiple snaps to nearest 90.
        assert_eq!(normalize_rotation(95), 90);
        assert_eq!(normalize_rotation(46), 90);
        assert_eq!(normalize_rotation(44), 0);
    }
}
