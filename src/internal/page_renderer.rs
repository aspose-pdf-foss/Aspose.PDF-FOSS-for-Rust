// One-page software PDF rasteriser. Walks the content stream and
// dispatches every operator through the foundation primitives
// previously frozen on the Rust track:
//   * `objects` + `xref` + `trailer` + `lexer`  — document parse.
//   * `pages_tree`                              — leaf enumeration.
//   * `flate` / `lzw` / `asciihex` / `ascii85` / `runlength`
//                                               — content-stream
//                                                 filter chain.
//   * `content_stream`                          — operator parse.
//   * `transform`                               — CTM composition
//                                                 + viewport bootstrap.
//   * `colorspace`                              — ColorRgb fields.
//   * `rasterizer`                              — fill / stroke.
//   * `image_compositor`                        — /Do image placement.
//   * `dctdecode`                               — JPEG image XObjects.
//   * `truetype` + `glyph_rasterizer`           — text rendering.
//
// Scope per Feedstock/PDF/Spec/page_renderer.yaml: single-page software
// rasterisation with the operator subset documented there. Out-of-scope
// features (ICC/Pattern/DeviceN colour spaces, /SMask, /Form XObjects,
// Type 3 fonts, vertical writing, non-fill text rendering modes,
// advanced stroke styling, encrypted documents) are silently skipped
// at runtime; content depending on them renders as if those features
// were absent rather than throwing. Encrypted documents (/Encrypt in
// trailer) are the one exception — they reject with `Error::Encrypted`.

#![allow(clippy::needless_range_loop)] // index-arithmetic numeric kernels

use std::collections::BTreeMap;
use std::fmt;

use super::ascii85;
use super::asciihex;
use super::colorspace::{self, ColorRgb};
use super::content_stream::{self, Operation, Value as CsValue};
use super::dctdecode;
use super::flate;
use super::glyph_rasterizer;
use super::image_compositor;
use super::lzw;
use super::objects::{self, IndirectObject, Value};
use super::rasterizer::{self, FillRule, Path, PathSegment, Raster, SegmentKind};
use super::runlength;
use super::standard14_outlines;
use super::trailer;
use super::transform::{self, apply, compose, identity, Matrix, Point, Rect};
use super::truetype::{self, Glyph as TtGlyph, TrueType};
use super::xref::{self, Entry as XrefEntry};

// ============================================================
// Public API
// ============================================================

/// One rendered page's RGBA8 pixel buffer. Row-major, top-left
/// first (image-conventional, y-down), `width * height * 4` bytes,
/// no row-stride padding, channel order R G B A.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderedPage {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Error {
    Trailer(trailer::Error),
    Xref(xref::Error),
    Objects(objects::Error),
    ContentStream(content_stream::Error),
    Flate(flate::Error),
    Lzw(lzw::Error),
    AsciiHex(asciihex::Error),
    Ascii85(ascii85::Error),
    RunLength(runlength::Error),
    Truetype(truetype::Error),
    Encrypted,
    PageIndexOutOfRange { index: u64, page_count: usize },
    InvalidDpi(f64),
    /// Catalog object missing or malformed.
    CatalogInvalid,
    /// /Pages tree node not a dict, missing required entries, or
    /// contains a cycle / unresolvable reference.
    PagesTreeInvalid,
    /// Leaf has no /MediaBox in its inheritance chain.
    LeafMissingMediaBox,
    LeafNotFound(u32),
    #[allow(dead_code)] // retained for a future-phase public-API binding
    LeafNotDict,
    ContentsNotStreamOrArray,
    ContentsArrayElementNotStream,
    UnsupportedContentFilter(Vec<u8>),
    FilterNotNameOrArray,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Trailer(e) => write!(f, "page_renderer: {e}"),
            Error::Xref(e) => write!(f, "page_renderer: {e}"),
            Error::Objects(e) => write!(f, "page_renderer: {e}"),
            Error::ContentStream(e) => write!(f, "page_renderer: {e}"),
            Error::Flate(e) => write!(f, "page_renderer: {e}"),
            Error::Lzw(e) => write!(f, "page_renderer: {e}"),
            Error::AsciiHex(e) => write!(f, "page_renderer: {e}"),
            Error::Ascii85(e) => write!(f, "page_renderer: {e}"),
            Error::RunLength(e) => write!(f, "page_renderer: {e}"),
            Error::Truetype(e) => write!(f, "page_renderer: {e}"),
            Error::Encrypted => write!(
                f, "page_renderer: document declares /Encrypt; \
                    encrypted-on-render is reserved for a sister revision"),
            Error::PageIndexOutOfRange { index, page_count } => write!(
                f, "page_renderer: page_index={index} out of range \
                    (document has {page_count} pages)"),
            Error::InvalidDpi(v) => write!(
                f, "page_renderer: target_dpi must be > 0 and finite (got {v})"),
            Error::CatalogInvalid =>
                write!(f, "page_renderer: catalog object missing or malformed"),
            Error::PagesTreeInvalid =>
                write!(f, "page_renderer: /Pages tree node missing or malformed"),
            Error::LeafMissingMediaBox => write!(
                f, "page_renderer: leaf page has no /MediaBox in inheritance chain"),
            Error::LeafNotFound(id) => write!(
                f, "page_renderer: leaf page object {id} not found"),
            Error::LeafNotDict =>
                write!(f, "page_renderer: leaf page object is not a dictionary"),
            Error::ContentsNotStreamOrArray => write!(
                f, "page_renderer: /Contents resolves to neither Stream nor \
                    Array of Streams"),
            Error::ContentsArrayElementNotStream => write!(
                f, "page_renderer: /Contents array element is not a Stream"),
            Error::UnsupportedContentFilter(name) => write!(
                f, "page_renderer: unsupported content-stream /Filter /{}",
                String::from_utf8_lossy(name)),
            Error::FilterNotNameOrArray =>
                write!(f, "page_renderer: /Filter is neither a Name nor an Array"),
        }
    }
}

impl std::error::Error for Error {}

impl From<trailer::Error> for Error { fn from(e: trailer::Error) -> Self { Error::Trailer(e) } }
impl From<xref::Error> for Error { fn from(e: xref::Error) -> Self { Error::Xref(e) } }
impl From<objects::Error> for Error { fn from(e: objects::Error) -> Self { Error::Objects(e) } }
impl From<content_stream::Error> for Error { fn from(e: content_stream::Error) -> Self { Error::ContentStream(e) } }
impl From<flate::Error> for Error { fn from(e: flate::Error) -> Self { Error::Flate(e) } }
impl From<lzw::Error> for Error { fn from(e: lzw::Error) -> Self { Error::Lzw(e) } }
impl From<asciihex::Error> for Error { fn from(e: asciihex::Error) -> Self { Error::AsciiHex(e) } }
impl From<ascii85::Error> for Error { fn from(e: ascii85::Error) -> Self { Error::Ascii85(e) } }
impl From<runlength::Error> for Error { fn from(e: runlength::Error) -> Self { Error::RunLength(e) } }
impl From<truetype::Error> for Error { fn from(e: truetype::Error) -> Self { Error::Truetype(e) } }

const DEFAULT_FLATNESS: f64 = 0.25;
const MAX_FLATTEN_DEPTH: u32 = 12;

/// Render one page of a PDF document to an RGBA8 buffer. PDF user
/// space is 1/72 inch per unit, so `target_dpi=72.0` yields a 1:1
/// user-space-to-device-pixel ratio — the natural rendering DPI for
/// the freeze gate's mutool / pdftocairo oracle agreement.
///
/// `transparent_background` defaults to `false` (opaque white
/// initial canvas, matching the v1 default rendering). When
/// `true`, the canvas starts fully-transparent (0,0,0,0); page
/// content composites against transparent pixels — supports the
/// public-API `PngDevice.transparent_background` flag.
pub fn render(
    input: &[u8],
    page_index: u64,
    target_dpi: f64,
    transparent_background: bool,
) -> Result<RenderedPage, Error> {
    if !target_dpi.is_finite() || target_dpi <= 0.0 {
        return Err(Error::InvalidDpi(target_dpi));
    }
    let trailer_dict = trailer::parse(input)?;
    if trailer_dict.encrypt_id != 0 {
        return Err(Error::Encrypted);
    }

    let dump = build_dump(input);
    let ctx = Ctx { input, dump };
    let leaves = walk_pages_tree(&ctx, trailer_dict.root_id)?;
    if (page_index as usize) >= leaves.len() {
        return Err(Error::PageIndexOutOfRange {
            index: page_index,
            page_count: leaves.len(),
        });
    }
    let leaf = &leaves[page_index as usize];

    let scale = target_dpi / 72.0;
    let width_pdf = leaf.media_box[2] - leaf.media_box[0];
    let height_pdf = leaf.media_box[3] - leaf.media_box[1];
    let width = ((width_pdf * scale).round() as i64).max(1) as u32;
    let height = ((height_pdf * scale).round() as i64).max(1) as u32;

    let mut raster = Raster {
        pixels: vec![0u8; (width as usize) * (height as usize) * 4],
        width,
        height,
    };
    if !transparent_background {
        fill_opaque_white(&mut raster.pixels);
    }
    // else: pixel buffer is already zero-initialised (0,0,0,0 RGBA)
    // — a fully-transparent canvas. Page content composites in.

    // Bootstrap CTM: page user space (y-up, PDF units) → device
    // image space (y-down, pixels). Subsequent /cm operators
    // left-multiply onto this.
    let bootstrap = transform::viewport(
        Rect {
            x_min: leaf.media_box[0], y_min: leaf.media_box[1],
            x_max: leaf.media_box[2], y_max: leaf.media_box[3],
        },
        leaf.rotation, width, height,
    ).unwrap_or_else(|_| identity());

    let page_dict = ctx.get_dict(leaf.id, 0)
        .ok_or(Error::LeafNotFound(leaf.id))?;

    let resources = get_page_resources(&ctx, &page_dict);
    let ops = load_content_stream_ops(&ctx, &page_dict)?;
    if !ops.is_empty() {
        dispatch_operations(&mut raster, bootstrap, &ctx,
                            resources.as_deref(), &ops)?;
    }
    paint_annotation_appearances(&mut raster, bootstrap, &ctx,
                                 &page_dict, resources.as_deref())?;

    Ok(RenderedPage { width, height, pixels: raster.pixels })
}

// ============================================================
// Document context
// ============================================================

type Dump = BTreeMap<(u32, u16), IndirectObject>;

/// A stream as the renderer carries it around: its dictionary, and the
/// raw body still to be put through the filter chain.
type StreamParts = (Vec<(Vec<u8>, Value)>, Vec<u8>);

struct Ctx<'a> {
    #[allow(dead_code)] // retained borrow of the source buffer
    input: &'a [u8],
    dump: Dump,
}

impl<'a> Ctx<'a> {
    fn get(&self, id: u32, gen: u16) -> Option<&Value> {
        self.dump.get(&(id, gen)).map(|o| &o.value)
    }
    fn get_dict(&self, id: u32, gen: u16) -> Option<Vec<(Vec<u8>, Value)>> {
        match self.get(id, gen)? {
            Value::Dict(entries) => Some(entries.clone()),
            _ => None,
        }
    }
    fn resolve(&self, v: &Value) -> Value {
        let mut current = v.clone();
        for _ in 0..16 {
            if let Value::Ref { id, gen } = current {
                match self.dump.get(&(id, gen)) {
                    Some(obj) => current = obj.value.clone(),
                    None => return Value::Null,
                }
            } else {
                return current;
            }
        }
        Value::Null
    }
}

/// Build the (id, gen) → IndirectObject map. `objects::parse_object`
/// lexes from the object header to EOF; the lexer's `(...)` literal-
/// string scanner swallows bytes until the next `)`, so a stream
/// whose binary body contains unbalanced `(` (typical for JPEG /
/// JBIG2 / CCITT payloads AND for embedded TrueType programs whose
/// `name` table happens to carry `(` in a quoted byte) collapses
/// the framing detector. We try the standard parser first; on
/// failure we fall back to a tolerant header-only parser that hand-
/// scans the dict's `<<…>>` text, looks up /Length, and computes
/// body_offset by locating the EOL after `stream` — bypassing the
/// lex of the body entirely.
fn build_dump(input: &[u8]) -> Dump {
    let table = match xref::parse(input) {
        Ok(t) => t,
        Err(_) => return BTreeMap::new(),
    };
    let mut dump: Dump = BTreeMap::new();
    for entry in &table.entries {
        if let XrefEntry::InUse { id, gen, offset } = *entry {
            let off = offset as usize;
            if off >= input.len() { continue; }
            let obj = match objects::parse_object(input, off) {
                Ok(obj) => obj,
                Err(_) => match parse_stream_object_tolerant(input, off) {
                    Some(obj) => obj,
                    None => continue,
                },
            };
            dump.insert((id, gen), obj);
        }
    }
    dump
}

/// Fallback parser for stream-bearing objects whose binary body
/// confuses the lex-driven `objects::parse_object`. Returns `None`
/// when the input at `offset` is not a stream-shaped object (no
/// `<<…>>` followed by `stream`).
fn parse_stream_object_tolerant(
    input: &[u8],
    offset: usize,
) -> Option<IndirectObject> {
    // Header: `<id> <gen> obj` — scan three whitespace-delimited
    // tokens.
    let (id, gen, after_header) = read_obj_header(input, offset)?;
    // Locate the `<<` opening the dict.
    let dict_open = find_dict_open(input, after_header)?;
    // Walk `<<` / `>>` nesting depth to find the matching close.
    let dict_close = find_matching_dict_close(input, dict_open)?;
    // Synthetic envelope so we can reuse objects::parse_object on the
    // text-only dict slice (per feedback_rust_above_foundation_composition).
    let dict_bytes = &input[dict_open..dict_close + 2];
    let mut envelope: Vec<u8> = Vec::with_capacity(dict_bytes.len() + 32);
    envelope.extend_from_slice(b"1 0 obj\n");
    envelope.extend_from_slice(dict_bytes);
    envelope.extend_from_slice(b"\nendobj\n");
    let parsed = objects::parse_object(&envelope, 0).ok()?;
    let dict_entries = match parsed.value {
        Value::Dict(entries) => entries,
        _ => return None,
    };

    // After the dict close (`>>`), find the `stream` keyword.
    let stream_kw = find_stream_keyword(input, dict_close + 2)?;
    // PDF §7.3.8.1: `stream` must be followed by LF or CRLF — body
    // begins at the first byte after that EOL.
    let body_offset = skip_eol_after_stream(input, stream_kw + b"stream".len())?;
    // /Length entry must be an integer for us to materialise the
    // Stream variant. Indirect-ref /Length is rare in the v1 corpus
    // and we don't try to resolve it here.
    let body_length = find_length_in_dict(&dict_entries)?;
    if body_offset.checked_add(body_length)
        .map(|end| end > input.len())
        .unwrap_or(true)
    {
        return None;
    }

    let body = input[body_offset..body_offset + body_length].to_vec();
    Some(IndirectObject {
        id,
        generation: gen,
        value: Value::Stream {
            dict: dict_entries,
            body,
        },
    })
}

fn read_obj_header(input: &[u8], offset: usize) -> Option<(u32, u16, usize)> {
    let mut p = offset;
    p = skip_ws(input, p);
    let (id_str, p1) = read_decimal(input, p)?;
    let id: u32 = id_str.parse().ok()?;
    p = skip_ws(input, p1);
    let (gen_str, p2) = read_decimal(input, p)?;
    let gen: u16 = gen_str.parse().ok()?;
    p = skip_ws(input, p2);
    if p + 3 > input.len() || &input[p..p + 3] != b"obj" {
        return None;
    }
    Some((id, gen, p + 3))
}

fn read_decimal(input: &[u8], from: usize) -> Option<(&str, usize)> {
    let mut p = from;
    while p < input.len() && input[p].is_ascii_digit() {
        p += 1;
    }
    if p == from { return None; }
    std::str::from_utf8(&input[from..p]).ok().map(|s| (s, p))
}

fn skip_ws(input: &[u8], from: usize) -> usize {
    let mut p = from;
    while p < input.len() {
        match input[p] {
            b' ' | b'\t' | b'\r' | b'\n' | b'\x0c' => p += 1,
            _ => break,
        }
    }
    p
}

fn find_dict_open(input: &[u8], from: usize) -> Option<usize> {
    let mut p = from;
    while p + 1 < input.len() {
        if input[p] == b'<' && input[p + 1] == b'<' { return Some(p); }
        p += 1;
    }
    None
}

/// Walk `<<` / `>>` pairs at structural depth — text-only dicts
/// (no streams nested) so we can scan byte-by-byte without
/// worrying about literal-string `(...)` content escaping the
/// nesting count. PDF dict syntax does not allow `<<` / `>>` inside
/// literal strings without escaping, so depth tracking is safe.
fn find_matching_dict_close(input: &[u8], dict_open: usize) -> Option<usize> {
    let mut depth: i32 = 0;
    let mut p = dict_open;
    while p + 1 < input.len() {
        // Skip over `(...)` literal strings so `<<` inside a string
        // doesn't trip the counter. Standard PDF tokenisation.
        if input[p] == b'(' {
            let mut q = p + 1;
            let mut d: i32 = 1;
            while q < input.len() && d > 0 {
                let c = input[q];
                if c == b'\\' {
                    q += 1;
                    if q < input.len() { q += 1; }
                } else if c == b'(' { d += 1; q += 1; }
                else if c == b')' { d -= 1; q += 1; }
                else { q += 1; }
            }
            p = q;
            continue;
        }
        if input[p] == b'<' && input[p + 1] == b'<' {
            depth += 1;
            p += 2;
            continue;
        }
        if input[p] == b'>' && input[p + 1] == b'>' {
            depth -= 1;
            if depth == 0 { return Some(p); }
            p += 2;
            continue;
        }
        p += 1;
    }
    None
}

fn find_stream_keyword(input: &[u8], from: usize) -> Option<usize> {
    let kw = b"stream";
    let mut p = from;
    while p + kw.len() <= input.len() {
        if &input[p..p + kw.len()] == kw {
            // Must be at a token boundary — the byte before is
            // whitespace / EOL (or start of input).
            let before_ok = p == 0
                || matches!(input[p - 1],
                    b' ' | b'\t' | b'\r' | b'\n' | b'\x0c');
            // Byte after must be EOL (or EOF will be caught by the
            // caller).
            let after_ok = p + kw.len() == input.len()
                || matches!(input[p + kw.len()], b'\r' | b'\n');
            if before_ok && after_ok { return Some(p); }
        }
        p += 1;
    }
    None
}

fn skip_eol_after_stream(input: &[u8], from: usize) -> Option<usize> {
    let mut p = from;
    if p >= input.len() { return None; }
    if input[p] == b'\r' {
        p += 1;
        if p < input.len() && input[p] == b'\n' { p += 1; }
        return Some(p);
    }
    if input[p] == b'\n' {
        return Some(p + 1);
    }
    None
}

fn find_length_in_dict(dict: &[(Vec<u8>, Value)]) -> Option<usize> {
    for (k, v) in dict {
        if k == b"Length" {
            if let Value::Integer(n) = v {
                if *n >= 0 { return Some(*n as usize); }
            }
        }
    }
    None
}

#[derive(Clone)]
struct LeafInfo {
    id: u32,
    media_box: [f64; 4],
    rotation: i32,
}

fn walk_pages_tree(ctx: &Ctx<'_>, root_id: u32) -> Result<Vec<LeafInfo>, Error> {
    if root_id == 0 {
        return Err(Error::CatalogInvalid);
    }
    let catalog = ctx.get_dict(root_id, 0).ok_or(Error::CatalogInvalid)?;
    let pages_v = lookup(&catalog, b"Pages").ok_or(Error::CatalogInvalid)?;
    let mut leaves: Vec<LeafInfo> = Vec::new();
    let mut visited: std::collections::BTreeSet<(u32, u16)> =
        std::collections::BTreeSet::new();
    walk_node(ctx, pages_v, None, 0, &mut leaves, &mut visited)?;
    Ok(leaves)
}

fn walk_node(
    ctx: &Ctx<'_>,
    node_v: &Value,
    inherited_media: Option<[f64; 4]>,
    inherited_rotation: i32,
    out: &mut Vec<LeafInfo>,
    visited: &mut std::collections::BTreeSet<(u32, u16)>,
) -> Result<(), Error> {
    let (resolved, owning_id_gen) = match node_v {
        Value::Ref { id, gen } => {
            if !visited.insert((*id, *gen)) {
                return Err(Error::PagesTreeInvalid);
            }
            let v = ctx.get(*id, *gen).cloned()
                .ok_or(Error::PagesTreeInvalid)?;
            (v, Some((*id, *gen)))
        }
        v => (v.clone(), None),
    };
    let dict = match resolved {
        Value::Dict(entries) => entries,
        _ => return Err(Error::PagesTreeInvalid),
    };

    let mut local_media = inherited_media;
    if let Some(mb_v) = lookup(&dict, b"MediaBox") {
        let mb_resolved = ctx.resolve(mb_v);
        if let Value::Array(items) = mb_resolved {
            if items.len() == 4 {
                let mut arr = [0.0f64; 4];
                for (i, it) in items.iter().enumerate() {
                    arr[i] = match ctx.resolve(it) {
                        Value::Integer(n) => n as f64,
                        Value::Real(x) => x,
                        _ => return Err(Error::PagesTreeInvalid),
                    };
                }
                local_media = Some(arr);
            }
        }
    }
    let mut local_rotation = inherited_rotation;
    if let Some(rot_v) = lookup(&dict, b"Rotate") {
        let resolved_rot = ctx.resolve(rot_v);
        let raw = match resolved_rot {
            Value::Integer(n) => Some(n),
            Value::Real(x) => Some(x.round() as i64),
            _ => None,
        };
        if let Some(r) = raw {
            // Snap to nearest 90, reduce to [0, 360).
            let snapped = ((r as f64) / 90.0).round() * 90.0;
            local_rotation = snapped.rem_euclid(360.0) as i32;
        }
    }

    // Default node type is /Page when /Type is absent (matches
    // pages_tree's interpretation).
    let node_type: &[u8] = match lookup(&dict, b"Type") {
        Some(Value::Name(n)) => n.as_slice(),
        _ => b"Page",
    };
    match node_type {
        b"Pages" => {
            let kids_v = lookup(&dict, b"Kids").ok_or(Error::PagesTreeInvalid)?;
            let kids_resolved = ctx.resolve(kids_v);
            let kids = match kids_resolved {
                Value::Array(items) => items,
                _ => return Err(Error::PagesTreeInvalid),
            };
            for kid in &kids {
                walk_node(ctx, kid, local_media, local_rotation, out, visited)?;
            }
            if let Some(r) = owning_id_gen {
                visited.remove(&r);
            }
            Ok(())
        }
        b"Page" => {
            let media = local_media.ok_or(Error::LeafMissingMediaBox)?;
            let id = owning_id_gen.map(|(i, _)| i).unwrap_or(0);
            out.push(LeafInfo {
                id,
                media_box: [
                    media[0].min(media[2]),
                    media[1].min(media[3]),
                    media[0].max(media[2]),
                    media[1].max(media[3]),
                ],
                rotation: local_rotation,
            });
            if let Some(r) = owning_id_gen {
                visited.remove(&r);
            }
            Ok(())
        }
        _ => Err(Error::PagesTreeInvalid),
    }
}

fn lookup<'a>(dict: &'a [(Vec<u8>, Value)], key: &[u8]) -> Option<&'a Value> {
    dict.iter().find(|(k, _)| k == key).map(|(_, v)| v)
}

// ============================================================
// /Contents loading + filter chain
// ============================================================

fn load_content_stream_ops(
    ctx: &Ctx<'_>,
    page_dict: &[(Vec<u8>, Value)],
) -> Result<Vec<Operation>, Error> {
    let contents_v = match lookup(page_dict, b"Contents") {
        Some(v) => v,
        None => return Ok(Vec::new()),
    };
    let resolved = ctx.resolve(contents_v);
    let mut chunks: Vec<Vec<u8>> = Vec::new();
    match resolved {
        Value::Stream { dict, body } => {
            chunks.push(decode_stream_payload(ctx, &dict, body)?);
        }
        Value::Array(items) => {
            for item in &items {
                let r = ctx.resolve(item);
                match r {
                    Value::Stream { dict, body } => {
                        chunks.push(decode_stream_payload(ctx, &dict, body)?);
                    }
                    _ => return Err(Error::ContentsArrayElementNotStream),
                }
            }
        }
        _ => return Err(Error::ContentsNotStreamOrArray),
    }
    if chunks.is_empty() {
        return Ok(Vec::new());
    }
    // PDF §7.8.2: successive content streams join as if concatenated.
    // Insert a single LF between adjacent chunks so an operator at
    // the end of one doesn't merge with operands at the start of
    // the next; content_stream's lexer treats LF as whitespace.
    let mut total = chunks.iter().map(|c| c.len()).sum::<usize>();
    if chunks.len() > 1 {
        total += chunks.len() - 1;
    }
    let mut merged = Vec::with_capacity(total);
    for (i, c) in chunks.iter().enumerate() {
        if i > 0 { merged.push(b'\n'); }
        merged.extend_from_slice(c);
    }
    Ok(content_stream::parse(&merged)?)
}

fn decode_stream_payload(
    ctx: &Ctx<'_>,
    dict: &[(Vec<u8>, Value)],
    body: Vec<u8>,
) -> Result<Vec<u8>, Error> {
    let names = match lookup(dict, b"Filter") {
        None => return Ok(body),
        Some(v) => filter_chain_names(&ctx.resolve(v))?,
    };
    let mut data = body;
    for name in &names {
        data = apply_content_filter(&data, name)?;
    }
    Ok(data)
}

fn filter_chain_names(v: &Value) -> Result<Vec<Vec<u8>>, Error> {
    match v {
        Value::Name(n) => Ok(vec![n.clone()]),
        Value::Array(items) => {
            let mut out = Vec::with_capacity(items.len());
            for item in items {
                match item {
                    Value::Name(n) => out.push(n.clone()),
                    _ => return Err(Error::FilterNotNameOrArray),
                }
            }
            Ok(out)
        }
        _ => Err(Error::FilterNotNameOrArray),
    }
}

fn apply_content_filter(data: &[u8], name: &[u8]) -> Result<Vec<u8>, Error> {
    match name {
        b"FlateDecode" | b"Fl" => Ok(flate::decode(data)?),
        b"LZWDecode" | b"LZW" => Ok(lzw::decode(data)?),
        b"ASCIIHexDecode" | b"AHx" => Ok(asciihex::decode(data)?),
        b"ASCII85Decode" | b"A85" => Ok(ascii85::decode(data)?),
        b"RunLengthDecode" | b"RL" => Ok(runlength::decode(data)?),
        _ => Err(Error::UnsupportedContentFilter(name.to_vec())),
    }
}

fn get_page_resources(
    ctx: &Ctx<'_>,
    page_dict: &[(Vec<u8>, Value)],
) -> Option<Vec<(Vec<u8>, Value)>> {
    let v = lookup(page_dict, b"Resources")?;
    match ctx.resolve(v) {
        Value::Dict(entries) => Some(entries),
        _ => None,
    }
}

// ============================================================
// Graphics + text state
// ============================================================

#[derive(Clone)]
struct GraphicsState {
    ctm: Matrix,
    fill_color: ColorRgb,
    stroke_color: ColorRgb,
    line_width: f64,
    /// Cache key into the per-page font cache; resolved through the
    /// cache map at glyph-show time.
    current_font_key: Option<String>,
    tfs: f64,
    tc: f64,
    tw: f64,
    /// The clipping region every painting operator is bounded by, in
    /// device pixels. `None` is the whole page. Snapshotted by `q` / `Q`
    /// with the rest of the graphics state (§8.5.4).
    clip: Option<rasterizer::ClipRect>,
    /// Constant alpha for non-stroking paint — fills, glyphs, images
    /// (`/ca`, §11.6.4.4). Multiplies the painted coverage until changed
    /// or popped by `Q`.
    fill_alpha: f64,
    /// Constant alpha for stroking paint (`/CA`).
    stroke_alpha: f64,
    /// The line dash pattern in user-space units, alternating on and off
    /// lengths and starting "on" (§8.4.3.6). Empty strokes solid.
    dash: Vec<f64>,
    /// How far into the dash cycle a subpath starts.
    dash_phase: f64,
    /// Text rendering mode (`Tr`, §9.3.6). The anchor maps 1 and 2
    /// (stroke, fill+stroke) onto a plain fill and has every other
    /// non-fill mode leave the glyph unpainted — the glyph still
    /// advances the text matrix, so invisible text stays laid out.
    text_render_mode: i64,
    /// The /Pattern resource a later fill paints with, named by `scn`.
    /// `None` is the ordinary case: fills take `fill_color`.
    fill_pattern: Option<String>,
}

impl GraphicsState {
    fn new(ctm: Matrix) -> Self {
        Self {
            ctm,
            fill_color: ColorRgb { r: 0.0, g: 0.0, b: 0.0 },
            stroke_color: ColorRgb { r: 0.0, g: 0.0, b: 0.0 },
            line_width: 1.0,
            current_font_key: None,
            tfs: 0.0,
            tc: 0.0,
            tw: 0.0,
            clip: None,
            fill_alpha: 1.0,
            stroke_alpha: 1.0,
            dash: Vec::new(),
            dash_phase: 0.0,
            text_render_mode: 0,
            fill_pattern: None,
        }
    }
}

struct TextState {
    tm: Matrix,
    tlm: Matrix,
    tl: f64,
}

impl TextState {
    fn new() -> Self {
        Self { tm: identity(), tlm: identity(), tl: 0.0 }
    }
}

struct LoadedFont {
    #[allow(dead_code)] // retained for a future-phase public-API binding
    subtype: Vec<u8>,
    tt: Option<TrueType>,
    pdf_widths: Option<Vec<f64>>,
    first_char: i32,
    last_char: i32,
    /// Set for a composite (Type0) font, whose codes are two bytes and
    /// whose widths are keyed by CID rather than by byte.
    cid: Option<CidFont>,
    /// Set for a Type3 font, whose glyphs are procedures rather than
    /// outlines.
    type3: Option<Type3Font>,
}

/// A Type3 font (§9.6.5): each code names a procedure in /CharProcs,
/// and the glyph is whatever that procedure paints. The font brings its
/// own resources and its own space, given by /FontMatrix.
struct Type3Font {
    font_matrix: Matrix,
    /// Code to the procedure's already-parsed content stream.
    procedures: BTreeMap<u8, Vec<Operation>>,
    resources: Option<Vec<(Vec<u8>, Value)>>,
}

/// What a composite font needs beyond the glyph program (§9.7): how a
/// CID reaches a glyph, and the widths that go with it.
struct CidFont {
    cid_to_gid: CidToGid,
    default_width: f64,
    widths: BTreeMap<u32, f64>,
}

enum CidToGid {
    /// `/CIDToGIDMap /Identity` — the CID is the glyph index.
    Identity,
    /// A stream of two-byte big-endian GIDs, one per CID.
    Map(Vec<u8>),
}

impl CidFont {
    fn gid_for(&self, cid: u32) -> u16 {
        match &self.cid_to_gid {
            CidToGid::Identity => cid.min(u16::MAX as u32) as u16,
            CidToGid::Map(bytes) => {
                let at = cid as usize * 2;
                match (bytes.get(at), bytes.get(at + 1)) {
                    (Some(hi), Some(lo)) => u16::from_be_bytes([*hi, *lo]),
                    // Past the end of the map the CID has no glyph.
                    _ => 0,
                }
            }
        }
    }

    fn width_for(&self, cid: u32) -> f64 {
        *self.widths.get(&cid).unwrap_or(&self.default_width)
    }
}

#[derive(Clone, Copy)]
enum PathOp {
    Move,
    Line,
    Curve,
    Close,
    Rect,
}

#[derive(Clone, Copy)]
struct UserPathSegment {
    op: PathOp,
    p0: Point,
    p1: Point,
    p2: Point,
}

struct CurrentPath {
    segments: Vec<UserPathSegment>,
    current_point: Point,
    subpath_start: Point,
    has_subpath: bool,
}

impl Default for CurrentPath {
    fn default() -> Self {
        Self {
            segments: Vec::new(),
            current_point: Point { x: 0.0, y: 0.0 },
            subpath_start: Point { x: 0.0, y: 0.0 },
            has_subpath: false,
        }
    }
}

impl CurrentPath {
    fn move_to(&mut self, x: f64, y: f64) {
        let p = Point { x, y };
        self.segments.push(UserPathSegment {
            op: PathOp::Move, p0: p,
            p1: Point { x: 0.0, y: 0.0 },
            p2: Point { x: 0.0, y: 0.0 },
        });
        self.current_point = p;
        self.subpath_start = p;
        self.has_subpath = true;
    }
    fn line_to(&mut self, x: f64, y: f64) {
        let p = Point { x, y };
        self.segments.push(UserPathSegment {
            op: PathOp::Line, p0: p,
            p1: Point { x: 0.0, y: 0.0 },
            p2: Point { x: 0.0, y: 0.0 },
        });
        self.current_point = p;
    }
    fn curve_to(&mut self, c1x: f64, c1y: f64,
                c2x: f64, c2y: f64, x: f64, y: f64) {
        let c1 = Point { x: c1x, y: c1y };
        let c2 = Point { x: c2x, y: c2y };
        let p = Point { x, y };
        self.segments.push(UserPathSegment {
            op: PathOp::Curve, p0: c1, p1: c2, p2: p,
        });
        self.current_point = p;
    }
    fn curve_to_v(&mut self, c2x: f64, c2y: f64, x: f64, y: f64) {
        let cp = self.current_point;
        self.curve_to(cp.x, cp.y, c2x, c2y, x, y);
    }
    fn curve_to_y(&mut self, c1x: f64, c1y: f64, x: f64, y: f64) {
        self.curve_to(c1x, c1y, x, y, x, y);
    }
    fn rect(&mut self, x: f64, y: f64, w: f64, h: f64) {
        let p0 = Point { x, y };
        let p1 = Point { x: x + w, y: y + h };
        self.segments.push(UserPathSegment {
            op: PathOp::Rect, p0, p1,
            p2: Point { x: 0.0, y: 0.0 },
        });
        self.current_point = p0;
        self.subpath_start = p0;
        self.has_subpath = true;
    }
    fn close(&mut self) {
        if !self.has_subpath { return; }
        self.segments.push(UserPathSegment {
            op: PathOp::Close,
            p0: Point { x: 0.0, y: 0.0 },
            p1: Point { x: 0.0, y: 0.0 },
            p2: Point { x: 0.0, y: 0.0 },
        });
        self.current_point = self.subpath_start;
    }
    fn reset(&mut self) {
        self.segments.clear();
        self.current_point = Point { x: 0.0, y: 0.0 };
        self.subpath_start = Point { x: 0.0, y: 0.0 };
        self.has_subpath = false;
    }
}

// ============================================================
// Operator dispatch
// ============================================================

/// How deep Form XObjects may nest before execution stops. The anchor
/// sets this deliberately deeper than the pattern limit: XFA forms wrap
/// every subform in a form of its own, so the labels and boxes of a real
/// form sit several levels down and a shallow limit would blank the page.
const MAX_FORM_DEPTH: u32 = 16;

/// How deep tiling patterns may nest. Far shallower than forms on
/// purpose: a pattern cell is executed once per tile, so the cost of a
/// cycle multiplies by the tile count at every level.
const MAX_PATTERN_DEPTH: u32 = 4;

/// How deep the interpreter already is, counted separately per kind
/// because the two limits differ by an order of magnitude.
#[derive(Clone, Copy, Default)]
struct Nesting {
    form: u32,
    pattern: u32,
}

/// What a painting operator needs beyond the graphics state: the
/// document, the resources in force, the space this content stream
/// started in, and how deep we already are.
struct PaintCtx<'a> {
    ctx: &'a Ctx<'a>,
    resources: Option<&'a [(Vec<u8>, Value)]>,
    base_ctm: Matrix,
    nesting: Nesting,
}

fn dispatch_operations(
    raster: &mut Raster,
    bootstrap: Matrix,
    ctx: &Ctx<'_>,
    resources: Option<&[(Vec<u8>, Value)]>,
    ops: &[Operation],
) -> Result<(), Error> {
    execute_content(raster, GraphicsState::new(bootstrap), ctx, resources, ops,
                    Nesting::default())
}

/// Run one content stream — the page's, or a Form XObject's — starting
/// from `initial_state`. `nesting` carries how deep the interpreter
/// already is; see [`MAX_FORM_DEPTH`] and [`MAX_PATTERN_DEPTH`].
fn execute_content(
    raster: &mut Raster,
    initial_state: GraphicsState,
    ctx: &Ctx<'_>,
    resources: Option<&[(Vec<u8>, Value)]>,
    ops: &[Operation],
    nesting: Nesting,
) -> Result<(), Error> {
    // §8.7.3.1: a pattern's matrix maps its space onto the space this
    // content stream started in, not onto the CTM in force where the
    // fill happens — so the entry CTM has to be kept.
    let base_ctm = initial_state.ctm;
    let paint = PaintCtx { ctx, resources, base_ctm, nesting };
    let mut state = initial_state;
    let mut stack: Vec<GraphicsState> = Vec::new();
    let mut path = CurrentPath::default();
    let mut pending_clip: Option<FillRule> = None;
    let mut text: Option<TextState> = None;
    let mut font_cache: BTreeMap<String, Option<LoadedFont>> = BTreeMap::new();

    for op in ops {
        match op.op.as_str() {
            // §8.4.3.6: the dash pattern. An empty array — and, per the
            // anchor, one that is all zero or negative — strokes solid.
            "d" => {
                if let Some(CsValue::Array(items)) = op.operands.first() {
                    let pattern: Vec<f64> = items.iter().map(cs_d).collect();
                    state.dash = if pattern.iter().any(|v| *v > 0.0) {
                        pattern
                    } else {
                        Vec::new()
                    };
                    state.dash_phase = op.operands.get(1).map(cs_d).unwrap_or(0.0).max(0.0);
                }
            }

            // §8.4.5: apply a named graphics-state dictionary. Only the
            // constant-alpha entries are honoured here; /BM, /SMask,
            // /LW, /D, /Font and the rest are ignored by design.
            "gs" => {
                if let (Some(CsValue::Name(name)), Some(res)) =
                    (op.operands.first(), resources)
                {
                    apply_ext_gstate(&mut state, ctx, res, name);
                }
            }

            "q" => stack.push(state.clone()),
            "Q" => { if let Some(s) = stack.pop() { state = s; } }
            "cm" => {
                if op.operands.len() >= 6 {
                    let m = Matrix::new(
                        cs_d(&op.operands[0]), cs_d(&op.operands[1]),
                        cs_d(&op.operands[2]), cs_d(&op.operands[3]),
                        cs_d(&op.operands[4]), cs_d(&op.operands[5]));
                    // §8.4.4: cm LEFT-multiplies. compose(m, ctm) per
                    // row-vector convention.
                    state.ctm = compose(m, state.ctm);
                }
            }

            "rg" => if op.operands.len() >= 3 {
                state.fill_pattern = None;
                state.fill_color = ColorRgb {
                    r: clamp01(cs_d(&op.operands[0])),
                    g: clamp01(cs_d(&op.operands[1])),
                    b: clamp01(cs_d(&op.operands[2])),
                };
            }
            "RG" => if op.operands.len() >= 3 {
                state.stroke_color = ColorRgb {
                    r: clamp01(cs_d(&op.operands[0])),
                    g: clamp01(cs_d(&op.operands[1])),
                    b: clamp01(cs_d(&op.operands[2])),
                };
            }
            "g" => if !op.operands.is_empty() {
                state.fill_pattern = None;
                let v = clamp01(cs_d(&op.operands[0]));
                state.fill_color = ColorRgb { r: v, g: v, b: v };
            }
            "G" => if !op.operands.is_empty() {
                let v = clamp01(cs_d(&op.operands[0]));
                state.stroke_color = ColorRgb { r: v, g: v, b: v };
            }
            "k" => if op.operands.len() >= 4 {
                state.fill_pattern = None;
                let c = clamp01(cs_d(&op.operands[0]));
                let m = clamp01(cs_d(&op.operands[1]));
                let y = clamp01(cs_d(&op.operands[2]));
                let k = clamp01(cs_d(&op.operands[3]));
                let one_k = 1.0 - k;
                state.fill_color = ColorRgb {
                    r: (1.0 - c) * one_k,
                    g: (1.0 - m) * one_k,
                    b: (1.0 - y) * one_k,
                };
            }
            "K" => if op.operands.len() >= 4 {
                let c = clamp01(cs_d(&op.operands[0]));
                let m = clamp01(cs_d(&op.operands[1]));
                let y = clamp01(cs_d(&op.operands[2]));
                let k = clamp01(cs_d(&op.operands[3]));
                let one_k = 1.0 - k;
                state.stroke_color = ColorRgb {
                    r: (1.0 - c) * one_k,
                    g: (1.0 - m) * one_k,
                    b: (1.0 - y) * one_k,
                };
            }
            "w" => if !op.operands.is_empty() {
                state.line_width = cs_d(&op.operands[0]);
            }

            "m" => if op.operands.len() >= 2 {
                path.move_to(cs_d(&op.operands[0]), cs_d(&op.operands[1]));
            }
            "l" => if op.operands.len() >= 2 {
                path.line_to(cs_d(&op.operands[0]), cs_d(&op.operands[1]));
            }
            "c" => if op.operands.len() >= 6 {
                path.curve_to(
                    cs_d(&op.operands[0]), cs_d(&op.operands[1]),
                    cs_d(&op.operands[2]), cs_d(&op.operands[3]),
                    cs_d(&op.operands[4]), cs_d(&op.operands[5]));
            }
            "v" => if op.operands.len() >= 4 {
                path.curve_to_v(
                    cs_d(&op.operands[0]), cs_d(&op.operands[1]),
                    cs_d(&op.operands[2]), cs_d(&op.operands[3]));
            }
            "y" => if op.operands.len() >= 4 {
                path.curve_to_y(
                    cs_d(&op.operands[0]), cs_d(&op.operands[1]),
                    cs_d(&op.operands[2]), cs_d(&op.operands[3]));
            }
            "re" => if op.operands.len() >= 4 {
                path.rect(
                    cs_d(&op.operands[0]), cs_d(&op.operands[1]),
                    cs_d(&op.operands[2]), cs_d(&op.operands[3]));
            }
            "h" => path.close(),

            // §8.5.4: W / W* do not clip on their own — they mark the
            // current path, and the painting operator that follows
            // commits it into the graphics state.
            "W" => pending_clip = Some(FillRule::NonZero),
            "W*" => pending_clip = Some(FillRule::EvenOdd),

            "f" | "F" => {
                fill_path(raster, &path, &state, FillRule::NonZero, &paint)?;
                commit_clip(&mut state, &path, &mut pending_clip, raster.width, raster.height);
                path.reset();
            }
            "f*" => {
                fill_path(raster, &path, &state, FillRule::EvenOdd, &paint)?;
                commit_clip(&mut state, &path, &mut pending_clip, raster.width, raster.height);
                path.reset();
            }
            "S" => {
                stroke_path(raster, &path, &state);
                commit_clip(&mut state, &path, &mut pending_clip, raster.width, raster.height);
                path.reset();
            }
            "B" => {
                fill_path(raster, &path, &state, FillRule::NonZero, &paint)?;
                stroke_path(raster, &path, &state);
                commit_clip(&mut state, &path, &mut pending_clip, raster.width, raster.height);
                path.reset();
            }
            "B*" => {
                fill_path(raster, &path, &state, FillRule::EvenOdd, &paint)?;
                stroke_path(raster, &path, &state);
                commit_clip(&mut state, &path, &mut pending_clip, raster.width, raster.height);
                path.reset();
            }
            "b" => {
                path.close();
                fill_path(raster, &path, &state, FillRule::NonZero, &paint)?;
                stroke_path(raster, &path, &state);
                commit_clip(&mut state, &path, &mut pending_clip, raster.width, raster.height);
                path.reset();
            }
            "b*" => {
                path.close();
                fill_path(raster, &path, &state, FillRule::EvenOdd, &paint)?;
                stroke_path(raster, &path, &state);
                commit_clip(&mut state, &path, &mut pending_clip, raster.width, raster.height);
                path.reset();
            }
            "s" => {
                path.close();
                stroke_path(raster, &path, &state);
                commit_clip(&mut state, &path, &mut pending_clip, raster.width, raster.height);
                path.reset();
            }
            "n" => {
                commit_clip(&mut state, &path, &mut pending_clip, raster.width, raster.height);
                path.reset();
            }

            "BT" => text = Some(TextState::new()),
            "ET" => text = None,

            "Tf" => {
                if op.operands.len() >= 2 {
                    if let CsValue::Name(name) = &op.operands[0] {
                        let key = String::from_utf8_lossy(name).to_string();
                        if !font_cache.contains_key(&key) {
                            let loaded = resolve_font(ctx, resources, name);
                            font_cache.insert(key.clone(), loaded);
                        }
                        state.current_font_key = Some(key);
                    }
                    state.tfs = cs_d(&op.operands[1]);
                }
            }
            "Tr" => if !op.operands.is_empty() {
                state.text_render_mode = cs_d(&op.operands[0]) as i64;
            }
            "Tc" => if !op.operands.is_empty() {
                state.tc = cs_d(&op.operands[0]);
            }
            "Tw" => if !op.operands.is_empty() {
                state.tw = cs_d(&op.operands[0]);
            }

            "Tm" => if let Some(ts) = text.as_mut() {
                if op.operands.len() >= 6 {
                    let m = Matrix::new(
                        cs_d(&op.operands[0]), cs_d(&op.operands[1]),
                        cs_d(&op.operands[2]), cs_d(&op.operands[3]),
                        cs_d(&op.operands[4]), cs_d(&op.operands[5]));
                    ts.tm = m;
                    ts.tlm = m;
                }
            }
            "Td" => if let Some(ts) = text.as_mut() {
                if op.operands.len() >= 2 {
                    let tx = cs_d(&op.operands[0]);
                    let ty = cs_d(&op.operands[1]);
                    let t = Matrix::new(1.0, 0.0, 0.0, 1.0, tx, ty);
                    ts.tlm = compose(t, ts.tlm);
                    ts.tm = ts.tlm;
                }
            }
            "TD" => if let Some(ts) = text.as_mut() {
                if op.operands.len() >= 2 {
                    let tx = cs_d(&op.operands[0]);
                    let ty = cs_d(&op.operands[1]);
                    ts.tl = -ty;
                    let t = Matrix::new(1.0, 0.0, 0.0, 1.0, tx, ty);
                    ts.tlm = compose(t, ts.tlm);
                    ts.tm = ts.tlm;
                }
            }
            "TL" => if !op.operands.is_empty() {
                if let Some(ts) = text.as_mut() {
                    ts.tl = cs_d(&op.operands[0]);
                }
            }
            "T*" => if let Some(ts) = text.as_mut() {
                let t = Matrix::new(1.0, 0.0, 0.0, 1.0, 0.0, -ts.tl);
                ts.tlm = compose(t, ts.tlm);
                ts.tm = ts.tlm;
            }

            "Tj" => if let Some(ts) = text.as_mut() {
                if let Some(CsValue::String(bytes)) = op.operands.first() {
                    show_string(raster, &state, ts, &font_cache, bytes, &paint);
                }
            }
            "TJ" => if let Some(ts) = text.as_mut() {
                if let Some(CsValue::Array(items)) = op.operands.first() {
                    show_kerned(raster, &state, ts, &font_cache, items, &paint);
                }
            }
            "'" => if let Some(ts) = text.as_mut() {
                let t = Matrix::new(1.0, 0.0, 0.0, 1.0, 0.0, -ts.tl);
                ts.tlm = compose(t, ts.tlm);
                ts.tm = ts.tlm;
                if let Some(CsValue::String(bytes)) = op.operands.first() {
                    show_string(raster, &state, ts, &font_cache, bytes, &paint);
                }
            }
            "\"" => if let Some(ts) = text.as_mut() {
                if op.operands.len() >= 3 {
                    state.tw = cs_d(&op.operands[0]);
                    state.tc = cs_d(&op.operands[1]);
                    let t = Matrix::new(1.0, 0.0, 0.0, 1.0, 0.0, -ts.tl);
                    ts.tlm = compose(t, ts.tlm);
                    ts.tm = ts.tlm;
                    if let CsValue::String(bytes) = &op.operands[2] {
                        show_string(raster, &state, ts, &font_cache, bytes, &paint);
                    }
                }
            }

            // §8.6.8: select a colour space. A /Pattern space takes its
            // colour from a later scn; for a space this renderer does
            // not model the anchor keeps the colour already in force.
            "cs" => {
                if let Some(CsValue::Name(name)) = op.operands.first() {
                    if name.as_slice() != b"Pattern" {
                        state.fill_pattern = None;
                    }
                }
            }
            // §8.6.8.2: a trailing name means a pattern, numbers mean a
            // colour in the space cs selected.
            "sc" | "scn" => match op.operands.last() {
                Some(CsValue::Name(name)) => {
                    state.fill_pattern = Some(String::from_utf8_lossy(name).to_string());
                }
                _ => {
                    state.fill_pattern = None;
                    if let Some(c) = color_from_components(&op.operands) {
                        state.fill_color = c;
                    }
                }
            },

            // §8.7.4.3: paint a shading across the current clip under
            // the CURRENT ctm — no path, no pattern matrix.
            "sh" => {
                if let (Some(CsValue::Name(name)), Some(res)) =
                    (op.operands.first(), resources)
                {
                    if let Some(shading) = resolve_named_shading(ctx, res, name) {
                        paint_shading(raster, &shading, state.ctm,
                                      state.clip.as_ref(), state.fill_alpha);
                    }
                }
            }

            // §8.9.7: an inline image arrives whole, as one operation
            // whose operand is the parameter dictionary plus the
            // samples. Its keys are abbreviated; expanding them lets it
            // take the same path as an image XObject.
            content_stream::INLINE_IMAGE => {
                if let Some(CsValue::Stream { header, body }) = op.operands.first() {
                    let expanded = expand_inline_image_keys(header);
                    draw_image_xobject(raster, &state, ctx, &expanded, body);
                }
            }

            "Do" => {
                if let (Some(CsValue::Name(name)), Some(res)) =
                    (op.operands.first(), resources)
                {
                    match resolve_xobject(ctx, res, name) {
                        Some(XObject::Image { header, body }) => {
                            draw_image_xobject(raster, &state, ctx, &header, &body);
                        }
                        // §8.10: a form is EXECUTED, not composited.
                        Some(XObject::Form { header, body }) => {
                            run_form(raster, &state, ctx, resources,
                                     &header, body, nesting)?;
                        }
                        None => {}
                    }
                }
            }

            // Out-of-scope ops are silently ignored per the spec
            // anchor's "any other op — silently ignored" rule.
            _ => {}
        }
    }
    Ok(())
}

fn cs_d(v: &CsValue) -> f64 {
    match v {
        CsValue::Integer(n) => *n as f64,
        CsValue::Real(x) => *x,
        _ => 0.0,
    }
}

#[allow(clippy::manual_clamp, clippy::if_same_then_else)]
fn clamp01(v: f64) -> f64 {
    if v.is_nan() { 0.0 } else if v < 0.0 { 0.0 }
    else if v > 1.0 { 1.0 } else { v }
}

fn fill_opaque_white(pixels: &mut [u8]) {
    for b in pixels.iter_mut() { *b = 255; }
}

// ============================================================
// Path painting
// ============================================================

/// Commit a pending `W` / `W*` into the graphics state, if one is
/// waiting. §8.5.4 puts the clip into effect only after the painting
/// operator that follows the `W`, which is why this runs at the end of
/// each of them rather than at the `W` itself.
///
/// An axis-aligned rectangle — `re W n`, or the same shape traced with
/// lines, which vector output emits constantly — is carried exactly by
/// its device bounding box and needs no mask. That is a performance
/// requirement as much as a fidelity one: a page carrying thousands of
/// full-page rectangular clips would otherwise build a whole-page mask
/// for each. Anything else gets a per-pixel mask under the operator's
/// winding rule, intersected with whatever clip is already in force.
fn commit_clip(
    state: &mut GraphicsState,
    path: &CurrentPath,
    pending: &mut Option<FillRule>,
    width: u32,
    height: u32,
) {
    let Some(rule) = pending.take() else { return };
    if path.segments.is_empty() {
        return;
    }
    let rp = to_raster_path(path);
    state.clip = Some(
        clip_from_path(path, &rp, state.ctm, rule, state.clip.as_ref(), width, height)
            // An empty region: nothing can paint through it.
            .unwrap_or(rasterizer::ClipRect {
                x0: 0, y0: 0, x1: 0, y1: 0, mask: None, mask_stride: 0,
            }),
    );
}

/// The device region a path covers under `ctm`, narrowed by whatever
/// clip is already in force. An axis-aligned rectangle needs no mask,
/// which is the common case and much the cheaper one. `None` when the
/// path has no extent on the page.
fn clip_from_path(
    path: &CurrentPath,
    rp: &Path,
    ctm: Matrix,
    rule: FillRule,
    prev: Option<&rasterizer::ClipRect>,
    width: u32,
    height: u32,
) -> Option<rasterizer::ClipRect> {
    let (x0, y0, x1, y1) = device_bounds(rp, ctm, width, height)?;
    let mut clip = rasterizer::ClipRect { x0, y0, x1, y1, mask: None, mask_stride: 0 };
    if !is_axis_aligned_rectangle(path, ctm) {
        let mask = rasterizer::rasterize_clip_mask(width, height, rp, ctm, rule, None);
        clip.mask = Some(std::rc::Rc::new(mask));
        clip.mask_stride = width as usize;
    }
    Some(match prev {
        Some(p) => p.intersect(&clip),
        None => clip,
    })
}

/// The device-pixel bounding box of `path` under `ctm`, clamped to the
/// raster. `None` when the path has no extent on the page.
fn device_bounds(
    path: &Path,
    ctm: Matrix,
    width: u32,
    height: u32,
) -> Option<(i64, i64, i64, i64)> {
    let (mut min_x, mut min_y) = (f64::INFINITY, f64::INFINITY);
    let (mut max_x, mut max_y) = (f64::NEG_INFINITY, f64::NEG_INFINITY);
    let mut note = |p: Point| {
        let d = apply(ctm, p);
        min_x = min_x.min(d.x);
        max_x = max_x.max(d.x);
        min_y = min_y.min(d.y);
        max_y = max_y.max(d.y);
    };
    for seg in &path.segments {
        match seg.kind {
            SegmentKind::Move | SegmentKind::Line => note(seg.pts[0]),
            SegmentKind::Rect => {
                let (a, b) = (seg.pts[0], seg.pts[1]);
                note(a);
                note(Point { x: b.x, y: a.y });
                note(b);
                note(Point { x: a.x, y: b.y });
            }
            SegmentKind::Close => {}
        }
    }
    if !(min_x.is_finite() && min_y.is_finite() && max_x.is_finite() && max_y.is_finite()) {
        return None;
    }
    let x0 = (min_x.floor().max(0.0) as i64).min(width as i64);
    let y0 = (min_y.floor().max(0.0) as i64).min(height as i64);
    let x1 = (max_x.ceil().max(0.0) as i64).min(width as i64);
    let y1 = (max_y.ceil().max(0.0) as i64).min(height as i64);
    if x0 >= x1 || y0 >= y1 {
        return None;
    }
    Some((x0, y0, x1, y1))
}

/// Whether the path is one axis-aligned rectangle once `ctm` is applied,
/// in which case its bounding box carries the clip exactly.
///
/// Two shapes qualify: a single `re`, and a rectangle traced with lines —
/// one non-degenerate subpath of at most six vertices, every vertex on a
/// corner of the bounding box, every edge (including the implicit close)
/// horizontal or vertical.
fn is_axis_aligned_rectangle(path: &CurrentPath, ctm: Matrix) -> bool {
    // A rotation or skew would turn any of it into a diamond.
    const EPS: f64 = 1e-9;
    let axis_aligned_ctm = (ctm.b.abs() < EPS && ctm.c.abs() < EPS)
        || (ctm.a.abs() < EPS && ctm.d.abs() < EPS);
    if !axis_aligned_ctm {
        return false;
    }

    let mut rects = 0;
    let mut vertices: Vec<Point> = Vec::new();
    let mut subpaths = 0;
    for seg in &path.segments {
        match seg.op {
            PathOp::Rect => {
                rects += 1;
                subpaths += 1;
            }
            PathOp::Move => {
                subpaths += 1;
                vertices.push(seg.p0);
            }
            PathOp::Line => vertices.push(seg.p0),
            PathOp::Curve => return false,
            PathOp::Close => {}
        }
    }
    if rects == 1 && vertices.is_empty() && subpaths == 1 {
        return true;
    }
    if rects > 0 || subpaths != 1 || vertices.len() < 4 || vertices.len() > 6 {
        return false;
    }

    // A lone trailing vertex that draws nothing does not disqualify the
    // shape; drop a repeat of the first point.
    if vertices.len() > 4 && same_point(vertices[0], *vertices.last().expect("checked")) {
        vertices.pop();
    }
    if vertices.len() != 4 {
        return false;
    }
    let (mut min_x, mut min_y) = (f64::INFINITY, f64::INFINITY);
    let (mut max_x, mut max_y) = (f64::NEG_INFINITY, f64::NEG_INFINITY);
    for v in &vertices {
        min_x = min_x.min(v.x);
        max_x = max_x.max(v.x);
        min_y = min_y.min(v.y);
        max_y = max_y.max(v.y);
    }
    // Every vertex on a corner, and every edge axis-parallel.
    for v in &vertices {
        let on_x = (v.x - min_x).abs() < EPS || (v.x - max_x).abs() < EPS;
        let on_y = (v.y - min_y).abs() < EPS || (v.y - max_y).abs() < EPS;
        if !(on_x && on_y) {
            return false;
        }
    }
    for i in 0..vertices.len() {
        let a = vertices[i];
        let b = vertices[(i + 1) % vertices.len()];
        if (a.x - b.x).abs() >= EPS && (a.y - b.y).abs() >= EPS {
            return false;
        }
    }
    true
}


/// Apply the `/ExtGState` entry `name` from `resources` to `state`.
///
/// Only `/ca` (non-stroking) and `/CA` (stroking) are read, each only
/// when the dictionary carries it, each clamped to `[0, 1]`
/// (§11.6.4.4). Every other key is ignored: blend modes, soft-mask
/// groups, line width, dash and font are out of this renderer's scope,
/// and silently skipping them leaves the page as it would have been.
fn apply_ext_gstate(
    state: &mut GraphicsState,
    ctx: &Ctx<'_>,
    resources: &[(Vec<u8>, Value)],
    name: &[u8],
) {
    let Some(dicts) = lookup(resources, b"ExtGState") else { return };
    let Value::Dict(dicts) = ctx.resolve(dicts) else { return };
    let Some(entry) = lookup(&dicts, name) else { return };
    let Value::Dict(gs) = ctx.resolve(entry) else { return };

    if let Some(v) = lookup(&gs, b"ca") {
        if let Some(a) = number_of(&ctx.resolve(v)) {
            state.fill_alpha = clamp01(a);
        }
    }
    if let Some(v) = lookup(&gs, b"CA") {
        if let Some(a) = number_of(&ctx.resolve(v)) {
            state.stroke_alpha = clamp01(a);
        }
    }
}

/// A numeric object's value, whether it was written as an integer or a
/// real.
fn number_of(v: &Value) -> Option<f64> {
    match v {
        Value::Integer(n) => Some(*n as f64),
        Value::Real(x) => Some(*x),
        _ => None,
    }
}

fn fill_path(
    raster: &mut Raster,
    path: &CurrentPath,
    state: &GraphicsState,
    rule: FillRule,
    paint: &PaintCtx<'_>,
) -> Result<(), Error> {
    if path.segments.is_empty() { return Ok(()); }
    let rp = to_raster_path(path);
    if let Some(name) = &state.fill_pattern {
        // A pattern has no single colour, so the path stops being a
        // coverage mask for one and becomes the REGION the pattern
        // paints through — a gradient per pixel, or a cell tiled over
        // it. Nothing paints outside that region.
        let Some(region) = clip_from_path(
            path, &rp, state.ctm, rule, state.clip.as_ref(),
            raster.width, raster.height)
        else {
            return Ok(());
        };
        match paint.resources.and_then(|res| {
            resolve_pattern(paint.ctx, res, name.as_bytes(), paint.base_ctm)
        }) {
            Some(Pattern::Shading { shading, to_device }) => {
                paint_shading(raster, &shading, to_device, Some(&region),
                              state.fill_alpha);
            }
            Some(Pattern::Tiling(tiling)) => {
                paint_tiling(raster, &tiling, state, &region, paint)?;
            }
            // A pattern this renderer does not paint — an uncoloured
            // tiling one, or a mesh shading — leaves the path alone
            // rather than flooding it with whatever solid colour was
            // last in force.
            None => {}
        }
        return Ok(());
    }
    rasterizer::fill(
        raster, &rp, state.ctm, state.fill_color, state.fill_alpha, rule,
        state.clip.as_ref());
    Ok(())
}

/// The components of `sc` / `scn` read as a colour: one is grey, three
/// are RGB, four are CMYK (§8.6.8.2). Any other count names a space
/// this renderer does not model.
fn color_from_components(operands: &[CsValue]) -> Option<ColorRgb> {
    let v: Vec<f64> = operands.iter().map(cs_d).collect();
    let family = match v.len() {
        1 => colorspace::Family::DeviceGray,
        3 => colorspace::Family::DeviceRGB,
        4 => colorspace::Family::DeviceCMYK,
        _ => return None,
    };
    colorspace::to_srgb(family, &v).ok()
}

/// The current path in the rasterizer's own segment form, with curves
/// flattened.
fn to_raster_path(path: &CurrentPath) -> Path {
    let mut rp = Path { segments: Vec::new() };
    let mut cursor = Point { x: 0.0, y: 0.0 };
    for seg in &path.segments {
        match seg.op {
            PathOp::Move => {
                rp.segments.push(PathSegment {
                    kind: SegmentKind::Move,
                    pts: [seg.p0, Point { x: 0.0, y: 0.0 }],
                });
                cursor = seg.p0;
            }
            PathOp::Line => {
                rp.segments.push(PathSegment {
                    kind: SegmentKind::Line,
                    pts: [seg.p0, Point { x: 0.0, y: 0.0 }],
                });
                cursor = seg.p0;
            }
            PathOp::Rect => {
                rp.segments.push(PathSegment {
                    kind: SegmentKind::Rect,
                    pts: [seg.p0, seg.p1],
                });
                cursor = seg.p0;
            }
            PathOp::Curve => {
                flatten_cubic(&mut rp, cursor, seg.p0, seg.p1, seg.p2, 0);
                cursor = seg.p2;
            }
            PathOp::Close => {
                rp.segments.push(PathSegment {
                    kind: SegmentKind::Close,
                    pts: [Point { x: 0.0, y: 0.0 },
                          Point { x: 0.0, y: 0.0 }],
                });
            }
        }
    }
    rp
}

fn flatten_cubic(
    rp: &mut Path,
    p0: Point, c1: Point, c2: Point, p3: Point,
    depth: u32,
) {
    if depth >= MAX_FLATTEN_DEPTH {
        rp.segments.push(PathSegment {
            kind: SegmentKind::Line,
            pts: [p3, Point { x: 0.0, y: 0.0 }],
        });
        return;
    }
    let chord_x = p3.x - p0.x;
    let chord_y = p3.y - p0.y;
    let chord_len_sq = chord_x * chord_x + chord_y * chord_y;
    if chord_len_sq < 1e-12 {
        rp.segments.push(PathSegment {
            kind: SegmentKind::Line,
            pts: [p3, Point { x: 0.0, y: 0.0 }],
        });
        return;
    }
    let chord_len = chord_len_sq.sqrt();
    let d1 = ((c1.x - p0.x) * chord_y - (c1.y - p0.y) * chord_x).abs() / chord_len;
    let d2 = ((c2.x - p0.x) * chord_y - (c2.y - p0.y) * chord_x).abs() / chord_len;
    if d1 <= DEFAULT_FLATNESS && d2 <= DEFAULT_FLATNESS {
        rp.segments.push(PathSegment {
            kind: SegmentKind::Line,
            pts: [p3, Point { x: 0.0, y: 0.0 }],
        });
        return;
    }
    let q0 = mid(p0, c1);
    let q1 = mid(c1, c2);
    let q2 = mid(c2, p3);
    let r0 = mid(q0, q1);
    let r1 = mid(q1, q2);
    let s = mid(r0, r1);
    flatten_cubic(rp, p0, q0, r0, s, depth + 1);
    flatten_cubic(rp, s, r1, q2, p3, depth + 1);
}

fn mid(a: Point, b: Point) -> Point {
    Point { x: (a.x + b.x) * 0.5, y: (a.y + b.y) * 0.5 }
}

// ============================================================
// Stroke painting — parallel-offset polylines
// ============================================================

struct StrokeSubpath {
    vertices: Vec<Point>,
    closed: bool,
}

fn stroke_path(
    raster: &mut Raster,
    path: &CurrentPath,
    state: &GraphicsState,
) {
    if path.segments.is_empty() { return; }
    // Negated `>` (not `<=`) so a NaN line width also returns.
    #[allow(clippy::neg_cmp_op_on_partial_ord)]
    if !(state.line_width > 0.0) { return; }
    let subpaths = collect_stroke_subpaths(path);
    if subpaths.is_empty() { return; }
    let half_w = state.line_width * 0.5;
    let mut rp = Path { segments: Vec::new() };
    for sp in &subpaths {
        if state.dash.is_empty() {
            emit_stroke_region(&mut rp, &sp.vertices, sp.closed, half_w);
            continue;
        }
        // The pattern restarts at the start of each subpath, and each
        // "on" span becomes its own open, butt-capped stroke.
        for span in dash_spans(&sp.vertices, sp.closed, &state.dash, state.dash_phase) {
            emit_stroke_region(&mut rp, &span, false, half_w);
        }
    }
    if rp.segments.is_empty() { return; }
    rasterizer::fill(raster, &rp, state.ctm, state.stroke_color, state.stroke_alpha,
                     FillRule::NonZero, state.clip.as_ref());
}

/// Split a subpath's polyline into the dash pattern's "on" spans,
/// measured by arc length in user space (§8.4.3.6).
///
/// `phase` is how far into the cycle the subpath starts. A closed
/// subpath is walked with its closing edge included. Caps stay butt:
/// round and projecting dash caps travel with `J`, which is out of
/// scope here.
fn dash_spans(
    vertices: &[Point],
    closed: bool,
    pattern: &[f64],
    phase: f64,
) -> Vec<Vec<Point>> {
    let cycle: f64 = pattern.iter().sum();
    if vertices.len() < 2 || cycle <= 0.0 {
        return Vec::new();
    }

    // Walk the pattern to where `phase` leaves us.
    let mut index = 0usize;
    let mut remaining = pattern[0];
    let mut on = true;
    let mut left = phase % cycle;
    while left > 0.0 {
        if left < remaining {
            remaining -= left;
            break;
        }
        left -= remaining;
        index = (index + 1) % pattern.len();
        remaining = pattern[index];
        on = !on;
    }

    let mut spans: Vec<Vec<Point>> = Vec::new();
    let mut current: Vec<Point> = Vec::new();
    if on {
        current.push(vertices[0]);
    }

    let walk = |from: Point, to: Point,
                    spans: &mut Vec<Vec<Point>>,
                    current: &mut Vec<Point>,
                    index: &mut usize,
                    remaining: &mut f64,
                    on: &mut bool| {
        let (dx, dy) = (to.x - from.x, to.y - from.y);
        let len = (dx * dx + dy * dy).sqrt();
        if len <= 0.0 {
            return;
        }
        let mut walked = 0.0;
        while len - walked > *remaining {
            walked += *remaining;
            let t = walked / len;
            let cut = Point { x: from.x + dx * t, y: from.y + dy * t };
            if *on {
                current.push(cut);
                if current.len() >= 2 {
                    spans.push(std::mem::take(current));
                } else {
                    current.clear();
                }
            } else {
                current.clear();
                current.push(cut);
            }
            *on = !*on;
            *index = (*index + 1) % pattern.len();
            *remaining = pattern[*index];
            // A zero-length entry would spin here forever.
            if *remaining <= 0.0 {
                *remaining = f64::MAX;
            }
        }
        *remaining -= len - walked;
        if *on {
            current.push(to);
        }
    };

    for pair in vertices.windows(2) {
        walk(pair[0], pair[1], &mut spans, &mut current, &mut index, &mut remaining, &mut on);
    }
    if closed {
        walk(
            *vertices.last().expect("non-empty"),
            vertices[0],
            &mut spans,
            &mut current,
            &mut index,
            &mut remaining,
            &mut on,
        );
    }
    if current.len() >= 2 {
        spans.push(current);
    }
    spans
}

fn collect_stroke_subpaths(path: &CurrentPath) -> Vec<StrokeSubpath> {
    let mut result: Vec<StrokeSubpath> = Vec::new();
    let mut cur: Option<StrokeSubpath> = None;
    let mut cursor = Point { x: 0.0, y: 0.0 };
    let mut sub_start = Point { x: 0.0, y: 0.0 };

    let flush = |c: &mut Option<StrokeSubpath>, r: &mut Vec<StrokeSubpath>| {
        if let Some(s) = c.take() {
            if s.vertices.len() >= 2 {
                r.push(s);
            }
        }
    };

    for seg in &path.segments {
        match seg.op {
            PathOp::Move => {
                flush(&mut cur, &mut result);
                let mut s = StrokeSubpath { vertices: Vec::new(), closed: false };
                s.vertices.push(seg.p0);
                cursor = seg.p0;
                sub_start = seg.p0;
                cur = Some(s);
            }
            PathOp::Line => {
                if let Some(s) = cur.as_mut() {
                    s.vertices.push(seg.p0);
                    cursor = seg.p0;
                }
            }
            PathOp::Curve => {
                if let Some(s) = cur.as_mut() {
                    let mut edges: Vec<(Point, Point)> = Vec::new();
                    flatten_cubic_edges(&mut edges, cursor,
                                        seg.p0, seg.p1, seg.p2, 0);
                    for (_a, b) in &edges {
                        s.vertices.push(*b);
                    }
                    cursor = seg.p2;
                }
            }
            PathOp::Rect => {
                flush(&mut cur, &mut result);
                let bl = seg.p0;
                let tr = seg.p1;
                let br = Point { x: tr.x, y: bl.y };
                let tl = Point { x: bl.x, y: tr.y };
                let mut rect = StrokeSubpath {
                    vertices: Vec::with_capacity(4),
                    closed: true,
                };
                rect.vertices.push(bl);
                rect.vertices.push(br);
                rect.vertices.push(tr);
                rect.vertices.push(tl);
                result.push(rect);
                cursor = bl;
                sub_start = bl;
                cur = None;
            }
            PathOp::Close => {
                if let Some(s) = cur.as_mut() {
                    s.closed = true;
                    cursor = sub_start;
                }
                flush(&mut cur, &mut result);
            }
        }
    }
    flush(&mut cur, &mut result);
    result
}

fn flatten_cubic_edges(
    out: &mut Vec<(Point, Point)>,
    p0: Point, c1: Point, c2: Point, p3: Point,
    depth: u32,
) {
    if depth >= MAX_FLATTEN_DEPTH {
        out.push((p0, p3));
        return;
    }
    let chord_x = p3.x - p0.x;
    let chord_y = p3.y - p0.y;
    let chord_len_sq = chord_x * chord_x + chord_y * chord_y;
    if chord_len_sq < 1e-12 {
        out.push((p0, p3));
        return;
    }
    let chord_len = chord_len_sq.sqrt();
    let d1 = ((c1.x - p0.x) * chord_y - (c1.y - p0.y) * chord_x).abs() / chord_len;
    let d2 = ((c2.x - p0.x) * chord_y - (c2.y - p0.y) * chord_x).abs() / chord_len;
    if d1 <= DEFAULT_FLATNESS && d2 <= DEFAULT_FLATNESS {
        out.push((p0, p3));
        return;
    }
    let q0 = mid(p0, c1);
    let q1 = mid(c1, c2);
    let q2 = mid(c2, p3);
    let r0 = mid(q0, q1);
    let r1 = mid(q1, q2);
    let s = mid(r0, r1);
    flatten_cubic_edges(out, p0, q0, r0, s, depth + 1);
    flatten_cubic_edges(out, s, r1, q2, p3, depth + 1);
}

fn emit_stroke_region(
    rp: &mut Path,
    vertices: &[Point],
    closed: bool,
    half_w: f64,
) {
    let mut n = vertices.len();
    if n < 2 { return; }
    if closed && same_point(vertices[0], vertices[n - 1]) {
        n -= 1;
        if n < 2 { return; }
    }

    let mut outer = vec![Point { x: 0.0, y: 0.0 }; n];
    let mut inner = vec![Point { x: 0.0, y: 0.0 }; n];

    for i in 0..n {
        let (has_in, prev) = if i > 0 { (true, i - 1) }
            else if closed { (true, n - 1) } else { (false, 0) };
        let (has_out, next) = if i + 1 < n { (true, i + 1) }
            else if closed { (true, 0) } else { (false, 0) };

        let (pix, piy) = if has_in {
            unit_perpendicular(vertices[prev], vertices[i])
        } else { (0.0, 0.0) };
        let (pox, poy) = if has_out {
            unit_perpendicular(vertices[i], vertices[next])
        } else { (0.0, 0.0) };

        let (off_x, off_y);
        if has_in && has_out {
            let sum_x = pix + pox;
            let sum_y = piy + poy;
            let dot = pix * pox + piy * poy;
            let denom = 1.0 + dot;
            if denom.abs() < 1e-6 {
                off_x = pix * half_w;
                off_y = piy * half_w;
            } else {
                off_x = sum_x / denom * half_w;
                off_y = sum_y / denom * half_w;
            }
        } else if has_in {
            off_x = pix * half_w;
            off_y = piy * half_w;
        } else {
            off_x = pox * half_w;
            off_y = poy * half_w;
        }

        outer[i] = Point { x: vertices[i].x + off_x, y: vertices[i].y + off_y };
        inner[i] = Point { x: vertices[i].x - off_x, y: vertices[i].y - off_y };
    }

    if closed {
        emit_closed_polygon(rp, &outer, true);
        emit_closed_polygon(rp, &inner, false);
    } else {
        rp.segments.push(PathSegment {
            kind: SegmentKind::Move,
            pts: [outer[0], Point { x: 0.0, y: 0.0 }],
        });
        for i in 1..n {
            rp.segments.push(PathSegment {
                kind: SegmentKind::Line,
                pts: [outer[i], Point { x: 0.0, y: 0.0 }],
            });
        }
        for i in (0..n).rev() {
            rp.segments.push(PathSegment {
                kind: SegmentKind::Line,
                pts: [inner[i], Point { x: 0.0, y: 0.0 }],
            });
        }
        rp.segments.push(PathSegment {
            kind: SegmentKind::Close,
            pts: [Point { x: 0.0, y: 0.0 }, Point { x: 0.0, y: 0.0 }],
        });
    }
}

fn emit_closed_polygon(rp: &mut Path, verts: &[Point], forward: bool) {
    let n = verts.len();
    if n < 2 { return; }
    if forward {
        rp.segments.push(PathSegment {
            kind: SegmentKind::Move,
            pts: [verts[0], Point { x: 0.0, y: 0.0 }],
        });
        for i in 1..n {
            rp.segments.push(PathSegment {
                kind: SegmentKind::Line,
                pts: [verts[i], Point { x: 0.0, y: 0.0 }],
            });
        }
    } else {
        rp.segments.push(PathSegment {
            kind: SegmentKind::Move,
            pts: [verts[n - 1], Point { x: 0.0, y: 0.0 }],
        });
        for i in (0..n - 1).rev() {
            rp.segments.push(PathSegment {
                kind: SegmentKind::Line,
                pts: [verts[i], Point { x: 0.0, y: 0.0 }],
            });
        }
    }
    rp.segments.push(PathSegment {
        kind: SegmentKind::Close,
        pts: [Point { x: 0.0, y: 0.0 }, Point { x: 0.0, y: 0.0 }],
    });
}

fn unit_perpendicular(a: Point, b: Point) -> (f64, f64) {
    let dx = b.x - a.x;
    let dy = b.y - a.y;
    let len = (dx * dx + dy * dy).sqrt();
    if len < 1e-12 { return (0.0, 0.0); }
    (-dy / len, dx / len)
}

fn same_point(a: Point, b: Point) -> bool {
    (a.x - b.x).abs() < 1e-9 && (a.y - b.y).abs() < 1e-9
}

// ============================================================
// Text dispatch
// ============================================================

fn resolve_font(
    ctx: &Ctx<'_>,
    resources: Option<&[(Vec<u8>, Value)]>,
    font_name: &[u8],
) -> Option<LoadedFont> {
    let res = resources?;
    let fonts_v = lookup(res, b"Font")?;
    let fonts_dict = match ctx.resolve(fonts_v) {
        Value::Dict(entries) => entries,
        _ => return None,
    };
    let entry_v = lookup(&fonts_dict, font_name)?;
    let font_dict = match ctx.resolve(entry_v) {
        Value::Dict(entries) => entries,
        _ => return None,
    };
    let subtype_bytes = match lookup(&font_dict, b"Subtype") {
        Some(Value::Name(s)) => s.clone(),
        _ => return None,
    };
    let mut loaded = LoadedFont {
        subtype: subtype_bytes.clone(),
        tt: None,
        pdf_widths: None,
        first_char: 0,
        last_char: -1,
        cid: None,
        type3: None,
    };
    if let Some(v) = lookup(&font_dict, b"FirstChar") {
        if let Value::Integer(n) = ctx.resolve(v) {
            loaded.first_char = n as i32;
        }
    }
    if let Some(v) = lookup(&font_dict, b"LastChar") {
        if let Value::Integer(n) = ctx.resolve(v) {
            loaded.last_char = n as i32;
        }
    }
    if let Some(v) = lookup(&font_dict, b"Widths") {
        if let Value::Array(items) = ctx.resolve(v) {
            let mut ws = Vec::with_capacity(items.len());
            for it in &items {
                let r = ctx.resolve(it);
                ws.push(match r {
                    Value::Integer(n) => n as f64,
                    Value::Real(x) => x,
                    _ => 0.0,
                });
            }
            loaded.pdf_widths = Some(ws);
        }
    }
    if subtype_bytes == b"Type0" {
        // A composite font is a different animal: two-byte codes, a
        // descendant carrying the program, and widths keyed by CID.
        // If any of that is missing or out of scope, the font resolves
        // to nothing rather than painting Latin glyphs for CIDs.
        let (cid, tt) = load_composite_font(ctx, &font_dict)?;
        loaded.cid = Some(cid);
        loaded.tt = Some(tt);
        return Some(loaded);
    }
    if subtype_bytes == b"Type3" {
        // A Type3 font has no glyph program at all, so there is nothing
        // to fall back to: without its procedures it must not resolve.
        // A Standard14 substitute would paint full-size Latin glyphs at
        // the large size a bitmap Type3 font asks for — black blobs.
        loaded.type3 = Some(load_type3_font(ctx, &font_dict)?);
        return Some(loaded);
    }
    if subtype_bytes == b"TrueType" {
        loaded.tt = load_embedded_truetype(ctx, &font_dict);
    }
    if loaded.tt.is_none() {
        loaded.tt = load_standard14_substitute(ctx, &font_dict);
    }
    Some(loaded)
}

/// A simple font with no embedded program renders through the Standard 14
/// substitute its `/BaseFont` names (PDF 32000-1:2008 §9.6.2.2 + Annex
/// D.1): `standard14_outlines` canonicalizes the name onto the nearest
/// Standard14 face (the `Canonicalize` step of its own contract),
/// probes the platform font directories and falls back to the bundled
/// metric-class faces. That is why a `/Helvetica`
/// reference paints — and why a `/Arial` or `/Verdana` one does too,
/// where before the name matched nothing and the text came out blank.
/// The family for a name that says nothing about its own comes from the
/// `/FontDescriptor /Flags` serif bit (bit 2, §9.8.2 Table 123).
/// Symbol and ZapfDingbats have no substitute and resolve to nothing.
fn load_standard14_substitute(
    ctx: &Ctx<'_>,
    font_dict: &[(Vec<u8>, Value)],
) -> Option<TrueType> {
    let base_font = match ctx.resolve(lookup(font_dict, b"BaseFont")?) {
        Value::Name(name) => name,
        _ => return None,
    };
    let name = String::from_utf8_lossy(&base_font).into_owned();
    let serif = descriptor_is_serif(ctx, font_dict);
    let dirs = standard14_outlines::default_search_dirs();
    let dirs: Vec<&str> = dirs.iter().map(String::as_str).collect();
    let program = standard14_outlines::resolve(&name, &dirs, serif);
    if program.is_empty() {
        return None;
    }
    truetype::parse(&program).ok()
}

/// Whether the font's `/FontDescriptor` says it is serif — bit 2 of
/// `/Flags` (§9.8.2 Table 123), the 1-based numbering making it the
/// value 2. A font with no descriptor says nothing, which reads as
/// sans.
fn descriptor_is_serif(ctx: &Ctx<'_>, font_dict: &[(Vec<u8>, Value)]) -> bool {
    let Some(descriptor) = lookup(font_dict, b"FontDescriptor") else {
        return false;
    };
    let entries = match ctx.resolve(descriptor) {
        Value::Dict(entries) => entries,
        _ => return false,
    };
    match lookup(&entries, b"Flags").map(|v| ctx.resolve(v)) {
        Some(Value::Integer(flags)) => flags & 0x2 != 0,
        _ => false,
    }
}

/// Load a Type3 font: /Encoding /Differences names a procedure per
/// code, /CharProcs holds them, and /FontMatrix maps the space they
/// draw in onto text space.
fn load_type3_font(
    ctx: &Ctx<'_>,
    font_dict: &[(Vec<u8>, Value)],
) -> Option<Type3Font> {
    let matrix = matrix_entry(ctx, font_dict, b"FontMatrix")?;
    let char_procs = match ctx.resolve(lookup(font_dict, b"CharProcs")?) {
        Value::Dict(entries) => entries,
        _ => return None,
    };
    let mut procedures = BTreeMap::new();
    for (code, name) in type3_differences(ctx, font_dict) {
        let Some(entry) = lookup(&char_procs, &name) else { continue };
        let Value::Stream { dict, body } = ctx.resolve(entry) else { continue };
        let Ok(data) = decode_stream_payload(ctx, &dict, body) else { continue };
        let Ok(ops) = content_stream::parse(&data) else { continue };
        procedures.insert(code, ops);
    }
    let resources = match lookup(font_dict, b"Resources").map(|v| ctx.resolve(v)) {
        Some(Value::Dict(entries)) => Some(entries),
        _ => None,
    };
    Some(Type3Font { font_matrix: matrix, procedures, resources })
}

/// The code-to-name pairs of /Encoding /Differences (§9.6.6.1): each
/// number restarts the run, and each name that follows takes the next
/// code. A Type3 font has no base encoding to fall back on — the names
/// are its own — so only what /Differences lists is reachable.
fn type3_differences(
    ctx: &Ctx<'_>,
    font_dict: &[(Vec<u8>, Value)],
) -> Vec<(u8, Vec<u8>)> {
    let mut out = Vec::new();
    let Some(encoding) = lookup(font_dict, b"Encoding").map(|v| ctx.resolve(v)) else {
        return out;
    };
    let Value::Dict(encoding) = encoding else { return out };
    let Some(Value::Array(items)) = lookup(&encoding, b"Differences").map(|v| ctx.resolve(v))
    else {
        return out;
    };
    let mut code: i64 = 0;
    for item in &items {
        match ctx.resolve(item) {
            Value::Integer(n) => code = n,
            Value::Real(x) => code = x as i64,
            Value::Name(name) => {
                if (0..=255).contains(&code) {
                    out.push((code as u8, name));
                }
                code += 1;
            }
            _ => {}
        }
    }
    out
}

/// Load a Type0 font (§9.7): the /Encoding must be Identity, the
/// descendant carries the glyph program and the CID metrics, and the
/// CID reaches a glyph through /CIDToGIDMap.
///
/// Only CIDFontType2 — a TrueType program in /FontFile2 — is loaded.
/// CIDFontType0 puts a CFF program in /FontFile3, and this renderer has
/// no CFF outline path for any font, simple or composite.
fn load_composite_font(
    ctx: &Ctx<'_>,
    font_dict: &[(Vec<u8>, Value)],
) -> Option<(CidFont, TrueType)> {
    // Identity-H and Identity-V both make the two-byte code the CID
    // itself. Any other encoding is a CMap this renderer does not read,
    // and guessing would paint the wrong glyphs.
    match ctx.resolve(lookup(font_dict, b"Encoding")?) {
        Value::Name(name) if name == b"Identity-H" || name == b"Identity-V" => {}
        _ => return None,
    }
    let descendants = match ctx.resolve(lookup(font_dict, b"DescendantFonts")?) {
        Value::Array(items) => items,
        _ => return None,
    };
    let descendant = match ctx.resolve(descendants.first()?) {
        Value::Dict(entries) => entries,
        _ => return None,
    };
    let tt = load_embedded_truetype(ctx, &descendant)?;

    let cid_to_gid = match lookup(&descendant, b"CIDToGIDMap").map(|v| ctx.resolve(v)) {
        // The name form, and the default when the entry is absent.
        None | Some(Value::Name(_)) => CidToGid::Identity,
        Some(Value::Stream { dict, body }) => {
            CidToGid::Map(decode_stream_payload(ctx, &dict, body).ok()?)
        }
        _ => CidToGid::Identity,
    };
    Some((
        CidFont {
            cid_to_gid,
            default_width: number_entry(ctx, &descendant, b"DW").unwrap_or(1000.0),
            widths: parse_cid_widths(ctx, &descendant),
        },
        tt,
    ))
}

/// The /W array (§9.7.4.3), which interleaves two forms: `c [w ...]`
/// gives consecutive CIDs from `c` their own widths, and `c_first
/// c_last w` gives a whole range one width.
fn parse_cid_widths(
    ctx: &Ctx<'_>,
    descendant: &[(Vec<u8>, Value)],
) -> BTreeMap<u32, f64> {
    let mut out = BTreeMap::new();
    let items = match lookup(descendant, b"W").map(|v| ctx.resolve(v)) {
        Some(Value::Array(items)) => items,
        _ => return out,
    };
    let mut i = 0;
    while i < items.len() {
        let Some(first) = number_of(&ctx.resolve(&items[i])) else { break };
        let Some(next) = items.get(i + 1).map(|v| ctx.resolve(v)) else { break };
        match next {
            Value::Array(widths) => {
                for (offset, w) in widths.iter().enumerate() {
                    if let Some(width) = number_of(&ctx.resolve(w)) {
                        out.insert(first as u32 + offset as u32, width);
                    }
                }
                i += 2;
            }
            _ => {
                let Some(last) = number_of(&next) else { break };
                let Some(width) = items.get(i + 2).and_then(|v| number_of(&ctx.resolve(v)))
                else {
                    break;
                };
                // A malformed range must not spin: the count is bounded
                // by the CID space either way.
                let (lo, hi) = (first as i64, last as i64);
                if hi >= lo && hi - lo <= u16::MAX as i64 {
                    for cid in lo..=hi {
                        out.insert(cid as u32, width);
                    }
                }
                i += 3;
            }
        }
    }
    out
}

fn load_embedded_truetype(
    ctx: &Ctx<'_>,
    font_dict: &[(Vec<u8>, Value)],
) -> Option<TrueType> {
    let fdv = lookup(font_dict, b"FontDescriptor")?;
    let fd = match ctx.resolve(fdv) {
        Value::Dict(entries) => entries,
        _ => return None,
    };
    // §9.8.1 Table 122 puts a TrueType program in /FontFile2, and that
    // is where one is looked for first. The other two keys are tried
    // after it because files do put an sfnt in the Type1 slot — the
    // shared corpus has one — and `truetype::parse` is the arbiter:
    // whatever is not an sfnt it can read simply does not parse, so a
    // Type1 or CFF program in its proper slot is passed over rather
    // than mistaken for a TrueType one.
    for key in [b"FontFile2".as_slice(), b"FontFile3".as_slice(), b"FontFile".as_slice()] {
        let Some(ffv) = lookup(&fd, key) else { continue };
        let (dict, body) = match ctx.resolve(ffv) {
            Value::Stream { dict, body } => (dict, body),
            _ => continue,
        };
        let Ok(program) = decode_stream_payload(ctx, &dict, body) else { continue };
        if let Ok(font) = truetype::parse(&program) {
            return Some(font);
        }
    }
    None
}

fn show_string(
    raster: &mut Raster,
    state: &GraphicsState,
    text: &mut TextState,
    cache: &BTreeMap<String, Option<LoadedFont>>,
    bytes: &[u8],
    paint: &PaintCtx<'_>,
) {
    let key = match &state.current_font_key {
        Some(k) => k,
        None => return,
    };
    let font = match cache.get(key) {
        Some(Some(f)) => f,
        _ => return,
    };
    show_bytes(raster, state, text, font, bytes, paint);
}

/// Paint a string's glyphs and advance the text matrix past them. A
/// simple font takes one byte per glyph; a composite font takes two,
/// big-endian, and each pair IS the CID (§9.4.3).
fn show_bytes(
    raster: &mut Raster,
    state: &GraphicsState,
    text: &mut TextState,
    font: &LoadedFont,
    bytes: &[u8],
    paint: &PaintCtx<'_>,
) {
    if let Some(type3) = &font.type3 {
        for &b in bytes {
            draw_type3_glyph(raster, state, text, type3, b, paint);
            let adv = type3_advance(font, type3, b, state);
            advance_tm(text, adv);
        }
        return;
    }
    let Some(cid_font) = &font.cid else {
        for &b in bytes {
            draw_one_glyph(raster, state, text, font, b);
            let adv = compute_advance(font, b, state, 0.0);
            advance_tm(text, adv);
        }
        return;
    };
    for pair in bytes.chunks(2) {
        // A trailing odd byte is not a code; there is nothing to show.
        let [hi, lo] = pair else { break };
        let cid = u16::from_be_bytes([*hi, *lo]) as u32;
        if let Some(tt) = &font.tt {
            draw_glyph_by_gid(raster, state, text, tt, cid_font.gid_for(cid));
        }
        advance_tm(text, cid_advance(cid_font, cid, state, 0.0));
    }
}

/// Run one Type3 glyph procedure. §9.6.5 renders it like a nested form
/// under `FontMatrix ∘ [Tfs 0 0 Tfs 0 0] ∘ Tm ∘ CTM`, with the font's
/// own resources and the text fill colour already in force — that last
/// part is what makes a `d1` image-mask glyph, the ordinary bitmap-font
/// case, paint in the colour the text asked for.
///
/// Horizontal scaling and text rise are absent from the matrix because
/// this renderer does not track `Tz` or `Ts` anywhere, including on the
/// outline path.
fn draw_type3_glyph(
    raster: &mut Raster,
    state: &GraphicsState,
    text: &TextState,
    type3: &Type3Font,
    code: u8,
    paint: &PaintCtx<'_>,
) {
    if !paints_glyphs(state) { return; }
    let Some(ops) = type3.procedures.get(&code) else { return };
    if paint.nesting.form >= MAX_FORM_DEPTH {
        return;
    }
    let size = Matrix::new(state.tfs, 0.0, 0.0, state.tfs, 0.0, 0.0);
    let glyph_to_device = compose(
        type3.font_matrix,
        compose(size, compose(text.tm, state.ctm)),
    );
    let mut glyph_state = GraphicsState::new(glyph_to_device);
    glyph_state.fill_color = state.fill_color;
    glyph_state.stroke_color = state.stroke_color;
    glyph_state.fill_alpha = state.fill_alpha;
    glyph_state.stroke_alpha = state.stroke_alpha;
    glyph_state.clip = state.clip.clone();
    // A procedure that cannot run leaves the glyph unpainted; one bad
    // glyph must not take the page down.
    let _ = execute_content(
        raster,
        glyph_state,
        paint.ctx,
        type3.resources.as_deref().or(paint.resources),
        ops,
        Nesting { form: paint.nesting.form + 1, ..paint.nesting },
    );
}

/// A Type3 advance is /Widths in glyph space, so it goes through
/// /FontMatrix rather than the 1/1000 of a simple font (§9.6.5).
fn type3_advance(
    font: &LoadedFont,
    type3: &Type3Font,
    code: u8,
    state: &GraphicsState,
) -> f64 {
    let index = code as i32 - font.first_char;
    let glyph_width = font
        .pdf_widths
        .as_ref()
        .filter(|_| index >= 0)
        .and_then(|ws| ws.get(index as usize))
        .copied()
        .unwrap_or(0.0);
    let mut tx = glyph_width * type3.font_matrix.a * state.tfs + state.tc;
    if code == 0x20 {
        tx += state.tw;
    }
    tx
}

/// §9.3.3: word spacing applies to the single-byte code 32 and never to
/// a two-byte CID code, even when that code happens to be 32.
fn cid_advance(
    cid_font: &CidFont,
    cid: u32,
    state: &GraphicsState,
    tj_shift: f64,
) -> f64 {
    (cid_font.width_for(cid) - tj_shift) / 1000.0 * state.tfs + state.tc
}

fn show_kerned(
    raster: &mut Raster,
    state: &GraphicsState,
    text: &mut TextState,
    cache: &BTreeMap<String, Option<LoadedFont>>,
    items: &[CsValue],
    paint: &PaintCtx<'_>,
) {
    let key = match &state.current_font_key {
        Some(k) => k,
        None => return,
    };
    let font = match cache.get(key) {
        Some(Some(f)) => f,
        _ => return,
    };
    for item in items {
        match item {
            CsValue::String(bytes) => {
                show_bytes(raster, state, text, font, bytes, paint);
            }
            CsValue::Integer(n) => {
                advance_tm(text, -(*n as f64) / 1000.0 * state.tfs);
            }
            CsValue::Real(x) => {
                advance_tm(text, -x / 1000.0 * state.tfs);
            }
            _ => {}
        }
    }
}

fn draw_one_glyph(
    raster: &mut Raster,
    state: &GraphicsState,
    text: &TextState,
    font: &LoadedFont,
    b: u8,
) {
    let tt = match &font.tt { Some(tt) => tt, None => return };
    let gid = map_byte_to_truetype_glyph(tt, b);
    draw_glyph_by_gid(raster, state, text, tt, gid);
}

/// Paint one glyph named by its index in the program. A composite font
/// reaches glyphs this way — its embedded program carries no cmap, so
/// there is no character code to look up.
/// Whether the current text rendering mode paints glyphs at all
/// (§9.3.6). Modes 1 and 2 fall back to a fill; 3 and above — invisible
/// and the clip-only modes — paint nothing, though the text still
/// advances.
fn paints_glyphs(state: &GraphicsState) -> bool {
    matches!(state.text_render_mode, 0..=2)
}

fn draw_glyph_by_gid(
    raster: &mut Raster,
    state: &GraphicsState,
    text: &TextState,
    tt: &TrueType,
    gid: u16,
) {
    if !paints_glyphs(state) { return; }
    if gid as usize >= tt.glyphs.len() { return; }
    let glyph = &tt.glyphs[gid as usize];
    if glyph.points.is_empty() { return; }

    let outline = truetype_glyph_to_outline(glyph);
    if outline.segments.is_empty() { return; }

    // Glyph design space → device pixels via:
    //   Scale(Tfs / upem)  (design → text)
    //   text.tm            (text  → user)
    //   state.ctm          (user  → device)
    let upem = if tt.units_per_em == 0 { 1000.0 } else { tt.units_per_em as f64 };
    let s = state.tfs / upem;
    let glyph_scale = Matrix::new(s, 0.0, 0.0, s, 0.0, 0.0);
    let glyph_to_user = compose(glyph_scale, text.tm);
    let glyph_to_device = compose(glyph_to_user, state.ctm);

    glyph_rasterizer::draw(
        raster, &outline, glyph_to_device, state.fill_color, state.fill_alpha,
        state.clip.as_ref());
}

fn compute_advance(
    font: &LoadedFont,
    b: u8,
    state: &GraphicsState,
    tj_shift: f64,
) -> f64 {
    let w0 = width_for_byte(font, b);
    let width_in_text = (w0 - tj_shift) / 1000.0 * state.tfs;
    let mut tx = width_in_text + state.tc;
    if b == 0x20 { tx += state.tw; }
    tx
}

fn width_for_byte(font: &LoadedFont, b: u8) -> f64 {
    if let Some(ws) = &font.pdf_widths {
        let bi = b as i32;
        if bi >= font.first_char && bi <= font.last_char {
            let idx = (bi - font.first_char) as usize;
            if idx < ws.len() {
                let w = ws[idx];
                if w > 0.0 { return w; }
            }
        }
    }
    let tt = match &font.tt { Some(t) => t, None => return 0.0 };
    let gid = map_byte_to_truetype_glyph(tt, b);
    if (gid as usize) >= tt.widths.len() { return 0.0; }
    let upem = tt.units_per_em as f64;
    if upem <= 0.0 { return 0.0; }
    tt.widths[gid as usize] as f64 / upem * 1000.0
}

fn map_byte_to_truetype_glyph(tt: &TrueType, b: u8) -> u16 {
    let code = b as u32;
    for (cc, gid) in &tt.cmap {
        if *cc == code { return *gid; }
    }
    0
}

fn advance_tm(text: &mut TextState, tx: f64) {
    let t = Matrix::new(1.0, 0.0, 0.0, 1.0, tx, 0.0);
    text.tm = compose(t, text.tm);
}

fn truetype_glyph_to_outline(glyph: &TtGlyph) -> glyph_rasterizer::Outline {
    let mut outline = glyph_rasterizer::Outline { segments: Vec::new() };
    if glyph.points.is_empty() { return outline; }

    let mut contour_start = 0usize;
    for &contour_end_u in &glyph.contour_ends {
        let contour_end = contour_end_u as usize;
        if contour_end < contour_start {
            contour_start = contour_end + 1;
            continue;
        }
        // Resolve implicit on-curve midpoints between consecutive
        // off-curve points (PDF §9.6 / TrueType implicit on-curve
        // rule).
        let raw = &glyph.points[contour_start..=contour_end];
        let mut resolved: Vec<(f64, f64, bool)> =
            Vec::with_capacity(raw.len() * 2);
        for i in 0..raw.len() {
            resolved.push((raw[i].x as f64, raw[i].y as f64, raw[i].on_curve));
            let next = (i + 1) % raw.len();
            if !raw[i].on_curve && !raw[next].on_curve {
                let mx = (raw[i].x + raw[next].x) as f64 * 0.5;
                let my = (raw[i].y + raw[next].y) as f64 * 0.5;
                resolved.push((mx, my, true));
            }
        }
        // Find an on-curve start point.
        let start_idx = match resolved.iter().position(|p| p.2) {
            Some(i) => i,
            None => { contour_start = contour_end + 1; continue; }
        };
        let first = Point { x: resolved[start_idx].0, y: resolved[start_idx].1 };
        outline.segments.push(glyph_rasterizer::OutlineSegment {
            op: glyph_rasterizer::OutlineOp::Move,
            pts: [first, Point { x: 0.0, y: 0.0 },
                  Point { x: 0.0, y: 0.0 }],
        });
        let mut prev_on = first;
        let mut k = 1usize;
        let count = resolved.len();
        while k <= count {
            let idx = (start_idx + k) % count;
            let pt = resolved[idx];
            if pt.2 {
                let p = Point { x: pt.0, y: pt.1 };
                outline.segments.push(glyph_rasterizer::OutlineSegment {
                    op: glyph_rasterizer::OutlineOp::Line,
                    pts: [p, Point { x: 0.0, y: 0.0 },
                          Point { x: 0.0, y: 0.0 }],
                });
                prev_on = p;
                k += 1;
            } else {
                let next_idx = (start_idx + k + 1) % count;
                let pt_n = resolved[next_idx];
                let cq = Point { x: pt.0, y: pt.1 };
                let p_end = Point { x: pt_n.0, y: pt_n.1 };
                let c1 = Point {
                    x: prev_on.x + 2.0 / 3.0 * (cq.x - prev_on.x),
                    y: prev_on.y + 2.0 / 3.0 * (cq.y - prev_on.y),
                };
                let c2 = Point {
                    x: p_end.x + 2.0 / 3.0 * (cq.x - p_end.x),
                    y: p_end.y + 2.0 / 3.0 * (cq.y - p_end.y),
                };
                outline.segments.push(glyph_rasterizer::OutlineSegment {
                    op: glyph_rasterizer::OutlineOp::Curve,
                    pts: [c1, c2, p_end],
                });
                prev_on = p_end;
                k += 2;
            }
        }
        outline.segments.push(glyph_rasterizer::OutlineSegment {
            op: glyph_rasterizer::OutlineOp::Close,
            pts: [Point { x: 0.0, y: 0.0 },
                  Point { x: 0.0, y: 0.0 },
                  Point { x: 0.0, y: 0.0 }],
        });
        contour_start = contour_end + 1;
    }
    outline
}

// ============================================================
// /Do — image XObjects
// ============================================================

/// The two XObject kinds `Do` handles. They cannot share a path: an
/// image is decoded and composited, a form is executed.
enum XObject {
    Image { header: Vec<(Vec<u8>, Value)>, body: Vec<u8> },
    Form { header: Vec<(Vec<u8>, Value)>, body: Vec<u8> },
}

fn resolve_xobject(
    ctx: &Ctx<'_>,
    resources: &[(Vec<u8>, Value)],
    name: &[u8],
) -> Option<XObject> {
    let xos_dict = match ctx.resolve(lookup(resources, b"XObject")?) {
        Value::Dict(entries) => entries,
        _ => return None,
    };
    let (header, body) = match ctx.resolve(lookup(&xos_dict, name)?) {
        Value::Stream { dict, body } => (dict, body),
        _ => return None,
    };
    let subtype = match lookup(&header, b"Subtype").map(|v| ctx.resolve(v)) {
        Some(Value::Name(n)) => n,
        _ => return None,
    };
    match subtype.as_slice() {
        b"Image" => Some(XObject::Image { header, body }),
        b"Form" => Some(XObject::Form { header, body }),
        _ => None,
    }
}

/// Execute a Form XObject the way `Do` defines it (PDF §8.10.2): the
/// form's /Matrix joins the CTM, the form's own /Resources take over —
/// falling back to the caller's when it brings none — and the content
/// runs from a COPY of the current graphics state, so what the form
/// changes does not leak back to the caller.
///
/// The form /BBox is deliberately NOT applied as a clip: the anchor
/// leaves that out of v1, and a flattened appearance stream stays
/// inside its box anyway.
fn run_form(
    raster: &mut Raster,
    state: &GraphicsState,
    ctx: &Ctx<'_>,
    caller_resources: Option<&[(Vec<u8>, Value)]>,
    header: &[(Vec<u8>, Value)],
    body: Vec<u8>,
    nesting: Nesting,
) -> Result<(), Error> {
    // A form may reference itself, directly or around a cycle; without
    // this the walk never returns.
    if nesting.form >= MAX_FORM_DEPTH {
        return Ok(());
    }
    let mut inner = state.clone();
    if let Some(m) = matrix_entry(ctx, header, b"Matrix") {
        inner.ctm = compose(m, inner.ctm);
    }
    let own = match lookup(header, b"Resources").map(|v| ctx.resolve(v)) {
        Some(Value::Dict(entries)) => Some(entries),
        _ => None,
    };
    let data = decode_stream_payload(ctx, header, body)?;
    let ops = content_stream::parse(&data)?;
    execute_content(raster, inner, ctx, own.as_deref().or(caller_resources), &ops,
                    Nesting { form: nesting.form + 1, ..nesting })
}

// ============================================================
// Shadings (§8.7.4) and the patterns that paint with them
// ============================================================

/// How finely a shading's /Function is sampled into a colour ramp.
/// The anchor is explicit that the function is evaluated ONCE per
/// fill rather than once per pixel: a shading-heavy page covers tens
/// of millions of pixels, and evaluating per pixel costs minutes for
/// a difference nobody can see.
const SHADING_RAMP_STEPS: usize = 256;

/// A shading ready to paint: the geometry that turns a point into the
/// parameter s, and the ramp that turns s into a colour.
struct PreparedShading {
    geometry: ShadingGeometry,
    ramp: Vec<ColorRgb>,
    extend_before: bool,
    extend_after: bool,
}

enum ShadingGeometry {
    /// §8.7.4.5.3: s projects the point onto the axis.
    Axial { x0: f64, y0: f64, x1: f64, y1: f64 },
    /// §8.7.4.5.4: s is a root of the two-circle quadratic.
    Radial { x0: f64, y0: f64, r0: f64, x1: f64, y1: f64, r1: f64 },
}

impl PreparedShading {
    /// The colour at a point in shading space, or `None` where the
    /// shading does not reach. Those points are left untouched — the
    /// anchor is explicit that they are not filled black.
    fn color_at(&self, p: Point) -> Option<ColorRgb> {
        let s = match self.geometry {
            ShadingGeometry::Axial { x0, y0, x1, y1 } => {
                let (dx, dy) = (x1 - x0, y1 - y0);
                let squared = dx * dx + dy * dy;
                if squared <= 0.0 {
                    return None;
                }
                self.bounded(((p.x - x0) * dx + (p.y - y0) * dy) / squared)?
            }
            ShadingGeometry::Radial { x0, y0, r0, x1, y1, r1 } => {
                let (dx, dy, dr) = (x1 - x0, y1 - y0, r1 - r0);
                let (px, py) = (p.x - x0, p.y - y0);
                let a = dx * dx + dy * dy - dr * dr;
                let b = 2.0 * (px * dx + py * dy + r0 * dr);
                let c = px * px + py * py - r0 * r0;
                let roots: [Option<f64>; 2] = if a.abs() < 1e-9 {
                    // Concentric circles of equal radius degenerate to
                    // a linear equation.
                    if b.abs() < 1e-9 { [None, None] } else { [Some(c / b), None] }
                } else {
                    let disc = b * b - 4.0 * a * c;
                    if disc < 0.0 {
                        [None, None]
                    } else {
                        let root = disc.sqrt();
                        let (s1, s2) = ((b + root) / (2.0 * a), (b - root) / (2.0 * a));
                        // The larger root wins: the circles are drawn
                        // in order of increasing s, so the later one
                        // covers the earlier.
                        [Some(s1.max(s2)), Some(s1.min(s2))]
                    }
                };
                let mut found = None;
                for candidate in roots.into_iter().flatten() {
                    // A circle of negative radius does not exist.
                    if r0 + candidate * dr < 0.0 {
                        continue;
                    }
                    if let Some(s) = self.bounded(candidate) {
                        found = Some(s);
                        break;
                    }
                }
                found?
            }
        };
        let last = self.ramp.len().checked_sub(1)?;
        let index = ((s * last as f64).round().max(0.0) as usize).min(last);
        Some(self.ramp[index])
    }

    /// Bring a raw parameter into [0, 1], or reject it where the
    /// shading is not extended past its start or end (§8.7.4.5.3).
    fn bounded(&self, s: f64) -> Option<f64> {
        if s.is_nan() {
            None
        } else if s < 0.0 {
            self.extend_before.then_some(0.0)
        } else if s > 1.0 {
            self.extend_after.then_some(1.0)
        } else {
            Some(s)
        }
    }
}

/// Read a shading dictionary and sample its ramp. `None` for the
/// shading types out of scope (1 and 4-7, function-based and mesh),
/// which leaves their area unpainted.
fn prepare_shading(
    ctx: &Ctx<'_>,
    dict: &[(Vec<u8>, Value)],
) -> Option<PreparedShading> {
    let coords = number_array(ctx, dict, b"Coords")?;
    let geometry = match (int_entry(ctx, dict, b"ShadingType")?, coords.len()) {
        (2, n) if n >= 4 => ShadingGeometry::Axial {
            x0: coords[0], y0: coords[1], x1: coords[2], y1: coords[3],
        },
        (3, n) if n >= 6 => ShadingGeometry::Radial {
            x0: coords[0], y0: coords[1], r0: coords[2],
            x1: coords[3], y1: coords[4], r1: coords[5],
        },
        _ => return None,
    };
    let family = shading_color_family(ctx, dict)?;
    let components = colorspace::component_count(family);
    let function = parse_function(ctx, lookup(dict, b"Function")?)?;
    let domain = number_array(ctx, dict, b"Domain")
        .filter(|d| d.len() >= 2)
        .unwrap_or_else(|| vec![0.0, 1.0]);

    let mut ramp = Vec::with_capacity(SHADING_RAMP_STEPS);
    for step in 0..SHADING_RAMP_STEPS {
        let s = step as f64 / (SHADING_RAMP_STEPS - 1) as f64;
        let values = eval_function(&function, domain[0] + s * (domain[1] - domain[0]));
        let color = if values.len() == components {
            colorspace::to_srgb(family, &values).unwrap_or(ColorRgb { r: 0.0, g: 0.0, b: 0.0 })
        } else {
            ColorRgb { r: 0.0, g: 0.0, b: 0.0 }
        };
        ramp.push(color);
    }

    let (extend_before, extend_after) = match lookup(dict, b"Extend").map(|v| ctx.resolve(v)) {
        Some(Value::Array(items)) if items.len() >= 2 => (
            matches!(ctx.resolve(&items[0]), Value::Bool(true)),
            matches!(ctx.resolve(&items[1]), Value::Bool(true)),
        ),
        _ => (false, false),
    };
    Some(PreparedShading { geometry, ramp, extend_before, extend_after })
}

fn shading_color_family(
    ctx: &Ctx<'_>,
    dict: &[(Vec<u8>, Value)],
) -> Option<colorspace::Family> {
    match ctx.resolve(lookup(dict, b"ColorSpace")?) {
        Value::Name(n) => match n.as_slice() {
            b"DeviceGray" | b"G" | b"CalGray" => Some(colorspace::Family::DeviceGray),
            b"DeviceRGB" | b"RGB" | b"CalRGB" => Some(colorspace::Family::DeviceRGB),
            b"DeviceCMYK" | b"CMYK" => Some(colorspace::Family::DeviceCMYK),
            _ => None,
        },
        _ => None,
    }
}

/// The /Function forms a shading may use (§7.10). Type 2 interpolates
/// between two colours; type 3 stitches a sequence of functions over
/// subdomains; an array supplies one function per colour component.
/// Sampled (type 0) and PostScript (type 4) functions are out of scope
/// — a shading built on one is left unpainted.
enum PdfFunction {
    Exponential { domain: [f64; 2], c0: Vec<f64>, c1: Vec<f64>, n: f64 },
    Stitching {
        domain: [f64; 2],
        functions: Vec<PdfFunction>,
        bounds: Vec<f64>,
        encode: Vec<f64>,
    },
    PerComponent(Vec<PdfFunction>),
}

fn parse_function(ctx: &Ctx<'_>, value: &Value) -> Option<PdfFunction> {
    let resolved = ctx.resolve(value);
    if let Value::Array(items) = &resolved {
        let mut parts = Vec::with_capacity(items.len());
        for item in items {
            parts.push(parse_function(ctx, item)?);
        }
        return Some(PdfFunction::PerComponent(parts));
    }
    let dict = match &resolved {
        Value::Dict(d) => d.clone(),
        Value::Stream { dict, .. } => dict.clone(),
        _ => return None,
    };
    let d = number_array(ctx, &dict, b"Domain")
        .filter(|d| d.len() >= 2)
        .unwrap_or_else(|| vec![0.0, 1.0]);
    let domain = [d[0], d[1]];
    match int_entry(ctx, &dict, b"FunctionType")? {
        2 => Some(PdfFunction::Exponential {
            domain,
            c0: number_array(ctx, &dict, b"C0").unwrap_or_else(|| vec![0.0]),
            c1: number_array(ctx, &dict, b"C1").unwrap_or_else(|| vec![1.0]),
            n: number_entry(ctx, &dict, b"N").unwrap_or(1.0),
        }),
        3 => {
            let items = match ctx.resolve(lookup(&dict, b"Functions")?) {
                Value::Array(items) => items,
                _ => return None,
            };
            let mut functions = Vec::with_capacity(items.len());
            for item in &items {
                functions.push(parse_function(ctx, item)?);
            }
            if functions.is_empty() {
                return None;
            }
            Some(PdfFunction::Stitching {
                domain,
                bounds: number_array(ctx, &dict, b"Bounds").unwrap_or_default(),
                encode: number_array(ctx, &dict, b"Encode").unwrap_or_default(),
                functions,
            })
        }
        _ => None,
    }
}

fn eval_function(function: &PdfFunction, t: f64) -> Vec<f64> {
    match function {
        PdfFunction::Exponential { domain, c0, c1, n } => {
            let x = t.clamp(domain[0], domain[1]);
            let factor = if *n == 1.0 { x } else { x.powf(*n) };
            c0.iter()
                .zip(c1.iter())
                .map(|(start, end)| start + factor * (end - start))
                .collect()
        }
        PdfFunction::Stitching { domain, functions, bounds, encode } => {
            let x = t.clamp(domain[0], domain[1]);
            // §7.10.3: the subdomain is the first whose upper bound the
            // input has not reached.
            let k = bounds
                .iter()
                .position(|bound| x < *bound)
                .unwrap_or(functions.len() - 1)
                .min(functions.len() - 1);
            let low = if k == 0 { domain[0] } else { bounds[k - 1] };
            let high = if k < bounds.len() { bounds[k] } else { domain[1] };
            let e0 = encode.get(2 * k).copied().unwrap_or(0.0);
            let e1 = encode.get(2 * k + 1).copied().unwrap_or(1.0);
            let encoded = if high > low {
                e0 + (x - low) / (high - low) * (e1 - e0)
            } else {
                e0
            };
            eval_function(&functions[k], encoded)
        }
        PdfFunction::PerComponent(parts) => {
            parts.iter().flat_map(|part| eval_function(part, t)).collect()
        }
    }
}

/// Paint a prepared shading through `clip`, which also bounds the
/// sweep — the anchor requires the work to follow the region painted,
/// not the page, since a page may carry hundreds of small fills.
fn paint_shading(
    raster: &mut Raster,
    shading: &PreparedShading,
    to_device: Matrix,
    clip: Option<&rasterizer::ClipRect>,
    alpha: f64,
) {
    let Some(inverse) = transform::inverse(to_device) else { return };
    let (width, height) = (raster.width as i64, raster.height as i64);
    let (x0, y0, x1, y1) = match clip {
        Some(c) => (c.x0.max(0), c.y0.max(0), c.x1.min(width), c.y1.min(height)),
        None => (0, 0, width, height),
    };
    for py in y0..y1 {
        for px in x0..x1 {
            let coverage = clip.map_or(1.0, |c| c.coverage_at(px, py));
            if coverage <= 0.0 {
                continue;
            }
            let point = apply(inverse, Point { x: px as f64 + 0.5, y: py as f64 + 0.5 });
            let Some(color) = shading.color_at(point) else { continue };
            rasterizer::composite_pixel(raster, px as u32, py as u32, color, coverage * alpha);
        }
    }
}

/// What a /Pattern resource turns out to be. Both kinds share the rule
/// that their matrix maps pattern space onto the space the content
/// stream started in — NOT onto the CTM in force where the fill
/// happens (§8.7.3.1).
enum Pattern {
    Shading { shading: PreparedShading, to_device: Matrix },
    Tiling(TilingPattern),
}

/// A tiling pattern's cell: the content to run, the resources to run it
/// against, the step between tiles in pattern space, and the matrix
/// onto the device.
struct TilingPattern {
    ops: Vec<Operation>,
    resources: Option<Vec<(Vec<u8>, Value)>>,
    x_step: f64,
    y_step: f64,
    to_device: Matrix,
}

fn resolve_pattern(
    ctx: &Ctx<'_>,
    resources: &[(Vec<u8>, Value)],
    name: &[u8],
    base_ctm: Matrix,
) -> Option<Pattern> {
    let patterns = match ctx.resolve(lookup(resources, b"Pattern")?) {
        Value::Dict(entries) => entries,
        _ => return None,
    };
    let entry = ctx.resolve(lookup(&patterns, name)?);
    let dict = match &entry {
        Value::Dict(d) => d.clone(),
        Value::Stream { dict, .. } => dict.clone(),
        _ => return None,
    };
    let to_device = compose(matrix_entry(ctx, &dict, b"Matrix").unwrap_or_else(identity), base_ctm);
    match int_entry(ctx, &dict, b"PatternType")? {
        1 => {
            // §8.7.3.3: PaintType 2 is an uncoloured pattern, stencilled
            // in the caller's colour. Out of scope, and skipped rather
            // than painted in some substitute colour.
            if int_entry(ctx, &dict, b"PaintType") != Some(1) {
                return None;
            }
            let body = match &entry {
                Value::Stream { body, .. } => body.clone(),
                _ => return None,
            };
            let bbox = bbox_of(ctx, &dict)?;
            // A missing step defaults to the cell's own extent, which
            // is what an unspaced tiling means.
            let x_step = number_entry(ctx, &dict, b"XStep")
                .filter(|v| v.abs() > 0.0)
                .unwrap_or(bbox.x_max - bbox.x_min);
            let y_step = number_entry(ctx, &dict, b"YStep")
                .filter(|v| v.abs() > 0.0)
                .unwrap_or(bbox.y_max - bbox.y_min);
            // Spelled out rather than negated: this must also reject a
            // non-finite step, which `abs() > 0.0` would let through and
            // which would make the tile-index range meaningless.
            if !x_step.is_finite() || !y_step.is_finite()
                || x_step == 0.0 || y_step == 0.0
            {
                return None;
            }
            let data = decode_stream_payload(ctx, &dict, body).ok()?;
            let ops = content_stream::parse(&data).ok()?;
            let own = match lookup(&dict, b"Resources").map(|v| ctx.resolve(v)) {
                Some(Value::Dict(entries)) => Some(entries),
                _ => None,
            };
            Some(Pattern::Tiling(TilingPattern {
                ops,
                resources: own,
                x_step: x_step.abs(),
                y_step: y_step.abs(),
                to_device,
            }))
        }
        2 => {
            let shading = match ctx.resolve(lookup(&dict, b"Shading")?) {
                Value::Dict(d) => prepare_shading(ctx, &d)?,
                Value::Stream { dict, .. } => prepare_shading(ctx, &dict)?,
                _ => return None,
            };
            Some(Pattern::Shading { shading, to_device })
        }
        _ => None,
    }
}

/// Tile a pattern cell across `region` (§8.7.3.2). The cell's content
/// stream is executed once per tile, translated by a whole number of
/// /XStep and /YStep in pattern space, and every tile paints through
/// `region` — so the path's coverage bounds the result and nothing
/// reaches the page outside it.
///
/// Only the tiles whose step lands inside the region are run: the
/// region's device box is mapped back into pattern space and the index
/// range read off it, so the work follows the fill rather than the page.
fn paint_tiling(
    raster: &mut Raster,
    pattern: &TilingPattern,
    state: &GraphicsState,
    region: &rasterizer::ClipRect,
    paint: &PaintCtx<'_>,
) -> Result<(), Error> {
    if paint.nesting.pattern >= MAX_PATTERN_DEPTH {
        return Ok(());
    }
    let Some(inverse) = transform::inverse(pattern.to_device) else { return Ok(()) };
    if region.x1 <= region.x0 || region.y1 <= region.y0 {
        return Ok(());
    }
    // The region's corners in pattern space. The matrix may rotate or
    // skew, so take the axis-aligned box of all four.
    let box_in_pattern = transform::apply_to_rect(inverse, Rect {
        x_min: region.x0 as f64, y_min: region.y0 as f64,
        x_max: region.x1 as f64, y_max: region.y1 as f64,
    });
    let bbox = bbox_of_cell(pattern);
    let i0 = ((box_in_pattern.x_min - bbox.x_max) / pattern.x_step).floor() as i64;
    let i1 = ((box_in_pattern.x_max - bbox.x_min) / pattern.x_step).ceil() as i64;
    let j0 = ((box_in_pattern.y_min - bbox.y_max) / pattern.y_step).floor() as i64;
    let j1 = ((box_in_pattern.y_max - bbox.y_min) / pattern.y_step).ceil() as i64;

    // A pathological matrix can ask for more tiles than could ever be
    // distinguishable; stop rather than grind.
    const MAX_TILES: i64 = 4096;
    if (i1 - i0 + 1).saturating_mul(j1 - j0 + 1) > MAX_TILES {
        return Ok(());
    }

    let nesting = Nesting { pattern: paint.nesting.pattern + 1, ..paint.nesting };
    for j in j0..=j1 {
        for i in i0..=i1 {
            let step = Matrix::new(1.0, 0.0, 0.0, 1.0,
                                   i as f64 * pattern.x_step,
                                   j as f64 * pattern.y_step);
            let mut cell = GraphicsState::new(compose(step, pattern.to_device));
            // The cell inherits what the fill was painting with, minus
            // the pattern itself — a cell that fills without setting a
            // colour is black, not a recursive pattern.
            cell.clip = Some(region.clone());
            cell.fill_alpha = state.fill_alpha;
            cell.stroke_alpha = state.stroke_alpha;
            execute_content(raster, cell, paint.ctx,
                            pattern.resources.as_deref().or(paint.resources),
                            &pattern.ops, nesting)?;
        }
    }
    Ok(())
}

/// A tiling cell's /BBox is already folded into its step defaults; this
/// is the extent used to decide which tiles can reach the region.
fn bbox_of_cell(pattern: &TilingPattern) -> Rect {
    Rect {
        x_min: 0.0, y_min: 0.0,
        x_max: pattern.x_step, y_max: pattern.y_step,
    }
}

/// The shading `sh` names in the /Shading resource dictionary.
fn resolve_named_shading(
    ctx: &Ctx<'_>,
    resources: &[(Vec<u8>, Value)],
    name: &[u8],
) -> Option<PreparedShading> {
    let shadings = match ctx.resolve(lookup(resources, b"Shading")?) {
        Value::Dict(entries) => entries,
        _ => return None,
    };
    match ctx.resolve(lookup(&shadings, name)?) {
        Value::Dict(d) => prepare_shading(ctx, &d),
        Value::Stream { dict, .. } => prepare_shading(ctx, &dict),
        _ => None,
    }
}

/// A dictionary entry read as an array of numbers.
fn number_array(
    ctx: &Ctx<'_>,
    dict: &[(Vec<u8>, Value)],
    key: &[u8],
) -> Option<Vec<f64>> {
    let items = match ctx.resolve(lookup(dict, key)?) {
        Value::Array(items) => items,
        _ => return None,
    };
    items.iter().map(|item| number_of(&ctx.resolve(item))).collect()
}

/// A dictionary entry read as a single number.
fn number_entry(ctx: &Ctx<'_>, dict: &[(Vec<u8>, Value)], key: &[u8]) -> Option<f64> {
    number_of(&ctx.resolve(lookup(dict, key)?))
}

// ============================================================
// Annotation appearances (§12.5.5)
// ============================================================

/// Paint the normal appearance of every visible annotation on top of
/// the finished page content. An appearance IS a Form XObject, so the
/// only work beyond executing it is choosing which stream to run and
/// fitting it onto the annotation's /Rect.
///
/// Annotations carrying no /AP are skipped: an appearance is never
/// synthesised from the annotation's own properties.
fn paint_annotation_appearances(
    raster: &mut Raster,
    bootstrap: Matrix,
    ctx: &Ctx<'_>,
    page_dict: &[(Vec<u8>, Value)],
    page_resources: Option<&[(Vec<u8>, Value)]>,
) -> Result<(), Error> {
    let annots = match lookup(page_dict, b"Annots").map(|v| ctx.resolve(v)) {
        Some(Value::Array(items)) => items,
        _ => return Ok(()),
    };
    for annot_v in &annots {
        let annot = match ctx.resolve(annot_v) {
            Value::Dict(entries) => entries,
            _ => continue,
        };
        if annotation_is_invisible(ctx, &annot) {
            continue;
        }
        let (header, body) = match normal_appearance(ctx, &annot) {
            Some(stream) => stream,
            None => continue,
        };
        let rect = match annotation_rect(ctx, &annot) {
            Some(r) => r,
            None => continue,
        };
        let placement = appearance_placement(ctx, &header, rect);
        let state = GraphicsState::new(compose(placement, bootstrap));
        run_form(raster, &state, ctx, page_resources, &header, body, Nesting::default())?;
    }
    Ok(())
}

/// §12.5.3: the Hidden (bit 2) and NoView (bit 6) flags of /F keep an
/// annotation off the rendered page. Every other flag is about
/// printing or interaction and does not affect us.
fn annotation_is_invisible(ctx: &Ctx<'_>, annot: &[(Vec<u8>, Value)]) -> bool {
    const HIDDEN: i64 = 1 << 1;
    const NO_VIEW: i64 = 1 << 5;
    match lookup(annot, b"F").map(|v| ctx.resolve(v)) {
        Some(Value::Integer(flags)) => flags & (HIDDEN | NO_VIEW) != 0,
        _ => false,
    }
}

/// The stream of /AP /N. It is either the appearance itself, or a
/// sub-dictionary of states — a checkbox's /Off and /Yes — from which
/// /AS names the current one.
fn normal_appearance(
    ctx: &Ctx<'_>,
    annot: &[(Vec<u8>, Value)],
) -> Option<StreamParts> {
    let ap = match ctx.resolve(lookup(annot, b"AP")?) {
        Value::Dict(entries) => entries,
        _ => return None,
    };
    let normal = ctx.resolve(lookup(&ap, b"N")?);
    let stream = match normal {
        Value::Stream { dict, body } => Some((dict, body)),
        Value::Dict(states) => {
            let state = match ctx.resolve(lookup(annot, b"AS")?) {
                Value::Name(n) => n,
                _ => return None,
            };
            match ctx.resolve(lookup(&states, &state)?) {
                Value::Stream { dict, body } => Some((dict, body)),
                _ => None,
            }
        }
        _ => None,
    };
    stream
}

/// The annotation's /Rect, normalised so the first corner is the lower
/// left one — the array is allowed to name the corners either way
/// round (§7.9.5).
fn annotation_rect(ctx: &Ctx<'_>, annot: &[(Vec<u8>, Value)]) -> Option<Rect> {
    let items = match ctx.resolve(lookup(annot, b"Rect")?) {
        Value::Array(items) => items,
        _ => return None,
    };
    if items.len() < 4 {
        return None;
    }
    let mut v = [0.0f64; 4];
    for (slot, item) in v.iter_mut().zip(items.iter()) {
        *slot = number_of(&ctx.resolve(item))?;
    }
    Some(Rect {
        x_min: v[0].min(v[2]),
        y_min: v[1].min(v[3]),
        x_max: v[0].max(v[2]),
        y_max: v[1].max(v[3]),
    })
}

/// The §12.5.5 appearance-placement matrix A: transform the form's
/// /BBox by its own /Matrix, then map the axis-aligned box of the
/// result onto /Rect by a scale and a translate. `run_form` re-applies
/// the /Matrix afterwards, which is the order the algorithm specifies.
///
/// A degenerate box — zero-width, zero-height, absent or malformed —
/// has nothing to fit, so that axis keeps unit scale rather than
/// dividing by zero.
fn appearance_placement(
    ctx: &Ctx<'_>,
    header: &[(Vec<u8>, Value)],
    rect: Rect,
) -> Matrix {
    let bbox = match bbox_of(ctx, header) {
        Some(b) => b,
        None => return identity(),
    };
    let matrix = matrix_entry(ctx, header, b"Matrix").unwrap_or_else(identity);
    let t = transform::apply_to_rect(matrix, bbox);
    let (tw, th) = (t.x_max - t.x_min, t.y_max - t.y_min);
    let sx = if tw > 0.0 { (rect.x_max - rect.x_min) / tw } else { 1.0 };
    let sy = if th > 0.0 { (rect.y_max - rect.y_min) / th } else { 1.0 };
    Matrix::new(sx, 0.0, 0.0, sy,
                rect.x_min - t.x_min * sx,
                rect.y_min - t.y_min * sy)
}

fn bbox_of(ctx: &Ctx<'_>, header: &[(Vec<u8>, Value)]) -> Option<Rect> {
    let items = match ctx.resolve(lookup(header, b"BBox")?) {
        Value::Array(items) => items,
        _ => return None,
    };
    if items.len() < 4 {
        return None;
    }
    let mut v = [0.0f64; 4];
    for (slot, item) in v.iter_mut().zip(items.iter()) {
        *slot = number_of(&ctx.resolve(item))?;
    }
    Some(Rect {
        x_min: v[0].min(v[2]),
        y_min: v[1].min(v[3]),
        x_max: v[0].max(v[2]),
        y_max: v[1].max(v[3]),
    })
}

/// A dictionary's six-number matrix entry: a form's /Matrix (§8.10.2),
/// a pattern's (§8.7.3.1), or a Type3 font's /FontMatrix (§9.6.5).
/// `None` when absent or malformed, which for the first two means
/// identity — the caller gets that by leaving the CTM alone.
fn matrix_entry(
    ctx: &Ctx<'_>,
    header: &[(Vec<u8>, Value)],
    key: &[u8],
) -> Option<Matrix> {
    let items = match ctx.resolve(lookup(header, key)?) {
        Value::Array(items) => items,
        _ => return None,
    };
    if items.len() < 6 {
        return None;
    }
    let mut m = [0.0f64; 6];
    for (slot, item) in m.iter_mut().zip(items.iter()) {
        *slot = number_of(&ctx.resolve(item))?;
    }
    Some(Matrix::new(m[0], m[1], m[2], m[3], m[4], m[5]))
}

/// Rewrite an inline image's abbreviated parameter keys and colour
/// space names into the forms the XObject path reads (§8.9.7 Table 93).
/// Keys already spelled in full are left alone, so a file that mixes
/// the two spellings still works.
fn expand_inline_image_keys(
    header: &[(Vec<u8>, CsValue)],
) -> Vec<(Vec<u8>, Value)> {
    let mut out = Vec::with_capacity(header.len() + 1);
    for (key, value) in header {
        let full: &[u8] = match key.as_slice() {
            b"W" => b"Width",
            b"H" => b"Height",
            b"BPC" => b"BitsPerComponent",
            b"CS" => b"ColorSpace",
            b"F" => b"Filter",
            b"DP" => b"DecodeParms",
            b"D" => b"Decode",
            b"IM" => b"ImageMask",
            b"I" => b"Interpolate",
            other => other,
        };
        let value = cs_value_to_object(value);
        let value = if full == b"ColorSpace" {
            expand_inline_color_space(value)
        } else {
            value
        };
        out.push((full.to_vec(), value));
    }
    out
}

fn expand_inline_color_space(value: Value) -> Value {
    match value {
        Value::Name(n) => Value::Name(match n.as_slice() {
            b"G" => b"DeviceGray".to_vec(),
            b"RGB" => b"DeviceRGB".to_vec(),
            b"CMYK" => b"DeviceCMYK".to_vec(),
            b"I" => b"Indexed".to_vec(),
            _ => n,
        }),
        other => other,
    }
}

/// An inline image's dictionary comes from the content lexer, so it
/// carries content-stream values; the image path reads document
/// objects. The shapes are the same, only the types differ.
fn cs_value_to_object(value: &CsValue) -> Value {
    match value {
        CsValue::Null => Value::Null,
        CsValue::Bool(b) => Value::Bool(*b),
        CsValue::Integer(n) => Value::Integer(*n),
        CsValue::Real(x) => Value::Real(*x),
        CsValue::Name(n) => Value::Name(n.clone()),
        CsValue::String(bytes) => Value::String(bytes.clone()),
        CsValue::Array(items) => {
            Value::Array(items.iter().map(cs_value_to_object).collect())
        }
        CsValue::Dict(entries) => Value::Dict(cs_entries_to_object(entries)),
        CsValue::Stream { header, body } => Value::Stream {
            dict: cs_entries_to_object(header),
            body: body.clone(),
        },
    }
}

fn cs_entries_to_object(entries: &[(Vec<u8>, CsValue)]) -> Vec<(Vec<u8>, Value)> {
    entries
        .iter()
        .map(|(k, v)| (k.clone(), cs_value_to_object(v)))
        .collect()
}

/// Decode an image XObject's body to RGBA, whichever filter chain it
/// carries. Shared by the base image and by the subsidiary images that
/// mask it, which arrive in exactly the same shapes.
fn decode_image_stream(
    ctx: &Ctx<'_>,
    header: &[(Vec<u8>, Value)],
    body: &[u8],
) -> (Option<Vec<u8>>, u32, u32) {
    let terminal_filter: Option<Vec<u8>> = match lookup(header, b"Filter") {
        Some(v) => match ctx.resolve(v) {
            Value::Name(n) => Some(n),
            Value::Array(items) => match items.last() {
                Some(Value::Name(n)) => Some(n.clone()),
                _ => None,
            },
            _ => None,
        },
        None => None,
    };
    match terminal_filter.as_deref() {
        Some(b"DCTDecode") | Some(b"DCT") => decode_dct_image(body),
        // Terminal image-encoding filters this body does not decode.
        // Best-effort per the anchor's error model: the single /Do is
        // skipped, the rest of the page still renders.
        Some(b"JPXDecode") | Some(b"JPX") | Some(b"CCITTFaxDecode") | Some(b"CCF")
        | Some(b"JBIG2Decode") => (None, 0, 0),
        // Everything else is a chain of general-purpose filters over
        // raw samples (and the no-filter case).
        _ => decode_sampled_image(ctx, header, body),
    }
}

/// Apply an image's /SMask (§11.6.5.2): a subsidiary DeviceGray image
/// whose sample at each point IS the base image's opacity there. The
/// mask carries its own resolution and is sampled nearest-neighbour
/// onto the base grid.
///
/// A /Decode [1 0] on the mask inverts the sample-to-alpha mapping.
/// That is the ordinary case on a scanned mixed-raster foreground,
/// where the dark (1) bits are the ink and must come out OPAQUE —
/// read the other way round the layer floods the page.
fn apply_soft_mask(
    ctx: &Ctx<'_>,
    header: &[(Vec<u8>, Value)],
    rgba: &mut [u8],
    width: u32,
    height: u32,
) {
    let (mask_header, mask_body) = match lookup(header, b"SMask").map(|v| ctx.resolve(v)) {
        Some(Value::Stream { dict, body }) => (dict, body),
        _ => return,
    };
    let (mask, mask_w, mask_h) = decode_image_stream(ctx, &mask_header, &mask_body);
    let mask = match mask {
        Some(m) if mask_w > 0 && mask_h > 0 => m,
        _ => return,
    };
    let (width, height) = (width as usize, height as usize);
    let (mask_w, mask_h) = (mask_w as usize, mask_h as usize);
    let inverted = decode_array_inverts(ctx, &mask_header);
    for y in 0..height {
        // Sample at the centre of the base pixel, so a mask at the
        // same resolution maps one-to-one.
        let my = (((y * 2 + 1) * mask_h) / (2 * height)).min(mask_h - 1);
        for x in 0..width {
            let mx = (((x * 2 + 1) * mask_w) / (2 * width)).min(mask_w - 1);
            // The mask is DeviceGray, so any channel of the decoded
            // pixel carries the sample.
            let sample = mask[(my * mask_w + mx) * 4];
            let at = (y * width + x) * 4 + 3;
            if at < rgba.len() {
                rgba[at] = if inverted { 255 - sample } else { sample };
            }
        }
    }
}

/// Apply an image's /Mask when it references a 1-bpc /ImageMask
/// stencil (§8.9.6.3): the stencil says, per point, whether the base
/// image paints there at all. Under the default /Decode [0 1] a set
/// (0) sample paints and a clear (1) one masks out; /Decode [1 0]
/// swaps that. Like a soft mask the stencil has its own resolution —
/// commonly a higher one — and is sampled nearest-neighbour.
///
/// The colour-key form of /Mask (an array of sample ranges, §8.9.6.4)
/// is not applied, nor is a JBIG2-coded stencil, whose samples this
/// renderer cannot decode.
fn apply_stencil_mask(
    ctx: &Ctx<'_>,
    header: &[(Vec<u8>, Value)],
    rgba: &mut [u8],
    width: u32,
    height: u32,
) {
    let (mask_header, mask_body) = match lookup(header, b"Mask").map(|v| ctx.resolve(v)) {
        Some(Value::Stream { dict, body }) => (dict, body),
        _ => return,
    };
    if !matches!(lookup(&mask_header, b"ImageMask").map(|v| ctx.resolve(v)),
                 Some(Value::Bool(true))) {
        return;
    }
    let (mask, mask_w, mask_h) = decode_image_stream(ctx, &mask_header, &mask_body);
    let mask = match mask {
        Some(m) if mask_w > 0 && mask_h > 0 => m,
        _ => return,
    };
    let (width, height) = (width as usize, height as usize);
    let (mask_w, mask_h) = (mask_w as usize, mask_h as usize);
    let inverted = decode_array_inverts(ctx, &mask_header);
    for y in 0..height {
        let my = (((y * 2 + 1) * mask_h) / (2 * height)).min(mask_h - 1);
        for x in 0..width {
            let mx = (((x * 2 + 1) * mask_w) / (2 * width)).min(mask_w - 1);
            // decode_sampled_image renders a set stencil bit as 0.
            let set = mask[(my * mask_w + mx) * 4] < 128;
            let paints = if inverted { !set } else { set };
            let at = (y * width + x) * 4 + 3;
            if at < rgba.len() && !paints {
                rgba[at] = 0;
            }
        }
    }
}

/// Turn a decoded stencil into paint: every sample that marks the
/// region takes `color` at full opacity, every other sample becomes
/// transparent. Under the default /Decode [0 1] a set (0) sample
/// paints; /Decode [1 0] swaps that, which is what a bitmap glyph
/// whose bits are 1 relies on.
fn paint_as_stencil(
    ctx: &Ctx<'_>,
    header: &[(Vec<u8>, Value)],
    rgba: &mut [u8],
    color: ColorRgb,
) {
    let inverted = decode_array_inverts(ctx, header);
    let (r, g, b) = (
        colorspace::quantize(color.r),
        colorspace::quantize(color.g),
        colorspace::quantize(color.b),
    );
    for px in rgba.chunks_exact_mut(4) {
        // decode_sampled_image renders a set stencil bit as 0.
        let set = px[0] < 128;
        let paints = if inverted { !set } else { set };
        px[0] = r;
        px[1] = g;
        px[2] = b;
        px[3] = if paints { 255 } else { 0 };
    }
}

/// Whether an image's /Decode array reverses its samples — the [1 0]
/// form of a one-component image (§8.9.5.2).
fn decode_array_inverts(ctx: &Ctx<'_>, header: &[(Vec<u8>, Value)]) -> bool {
    let items = match lookup(header, b"Decode").map(|v| ctx.resolve(v)) {
        Some(Value::Array(items)) => items,
        _ => return false,
    };
    if items.len() < 2 {
        return false;
    }
    match (number_of(&ctx.resolve(&items[0])), number_of(&ctx.resolve(&items[1]))) {
        (Some(first), Some(second)) => first > second,
        _ => false,
    }
}

fn draw_image_xobject(
    raster: &mut Raster,
    state: &GraphicsState,
    ctx: &Ctx<'_>,
    header: &[(Vec<u8>, Value)],
    body: &[u8],
) {
    let (rgba, w, h) = decode_image_stream(ctx, header, body);
    let mut rgba = match rgba { Some(p) => p, None => return };
    if matches!(lookup(header, b"ImageMask").map(|v| ctx.resolve(v)), Some(Value::Bool(true))) {
        // §8.9.6.2: the image is not a picture but a stencil. Its set
        // samples paint the CURRENT FILL COLOUR and the rest of it is
        // transparent — decoded as grey it would paint black and white
        // squares instead.
        paint_as_stencil(ctx, header, &mut rgba, state.fill_color);
    } else {
        apply_soft_mask(ctx, header, &mut rgba, w, h);
        apply_stencil_mask(ctx, header, &mut rgba, w, h);
    }

    // PDF §8.9.5 places source row 0 at user (0, 1); image_compositor
    // wants source row 0 at v=0. Compose flip BEFORE the post-cm CTM.
    let flip_y = Matrix::new(1.0, 0.0, 0.0, -1.0, 0.0, 1.0);
    let src_to_device = compose(flip_y, state.ctm);
    let src = image_compositor::ImageSource {
        width: w, height: h, pixels: rgba,
    };
    image_compositor::composite(
        raster, &src, src_to_device, state.fill_alpha, state.clip.as_ref());
}

/// Decode an image XObject whose filter chain carries raw samples —
/// FlateDecode / LZWDecode / ASCIIHexDecode / ASCII85Decode /
/// RunLengthDecode, or no filter at all. The samples are then read per
/// `/Width`, `/Height`, `/BitsPerComponent` and `/ColorSpace`
/// (PDF 32000-1:2008 §8.9.5). Returns no pixels for anything it cannot
/// read, which the caller treats as "skip this image".
fn decode_sampled_image(
    ctx: &Ctx<'_>,
    header: &[(Vec<u8>, Value)],
    raw: &[u8],
) -> (Option<Vec<u8>>, u32, u32) {
    let width = match int_entry(ctx, header, b"Width") { Some(v) if v > 0 => v as u32, _ => return (None, 0, 0) };
    let height = match int_entry(ctx, header, b"Height") { Some(v) if v > 0 => v as u32, _ => return (None, 0, 0) };
    // §8.9.6.2: a stencil carries no /ColorSpace of its own — it is
    // one component of one bit, and decodes here to 0 for a set bit
    // and 255 for a clear one. What that means for the image it masks
    // is the caller's business.
    let stencil = matches!(lookup(header, b"ImageMask").map(|v| ctx.resolve(v)),
                           Some(Value::Bool(true)));
    let bpc = if stencil {
        1
    } else {
        int_entry(ctx, header, b"BitsPerComponent").unwrap_or(8)
    };
    if !matches!(bpc, 1 | 2 | 4 | 8 | 16) {
        return (None, 0, 0);
    }
    let comps = if stencil {
        1
    } else {
        match component_count_of(ctx, header) {
            Some(n) => n,
            None => return (None, 0, 0),
        }
    };

    let mut data = raw.to_vec();
    if let Some(v) = lookup(header, b"Filter") {
        let Ok(names) = filter_chain_names(&ctx.resolve(v)) else {
            return (None, 0, 0);
        };
        for name in &names {
            match apply_content_filter(&data, name) {
                Ok(next) => data = next,
                Err(_) => return (None, 0, 0),
            }
        }
    }
    if let Some(params) = decode_parms_dict(ctx, header) {
        data = reverse_predictor(ctx, &params, data, width, comps, bpc);
    }

    let Some(rgba) = samples_to_rgba(&data, width, height, bpc, comps) else {
        return (None, 0, 0);
    };
    (Some(rgba), width, height)
}

/// The number of colour components an image's `/ColorSpace` carries, or
/// `None` for a space this body does not read. `/ICCBased` is honoured
/// through its `/N`, and the CIE-based greys and RGBs fall back to their
/// device equivalents (§8.6.5.2-3 — colour management is out of scope).
fn component_count_of(ctx: &Ctx<'_>, header: &[(Vec<u8>, Value)]) -> Option<usize> {
    let cs = ctx.resolve(lookup(header, b"ColorSpace")?);
    match cs {
        Value::Name(n) => match n.as_slice() {
            b"DeviceGray" | b"G" | b"CalGray" => Some(1),
            b"DeviceRGB" | b"RGB" | b"CalRGB" => Some(3),
            b"DeviceCMYK" | b"CMYK" => Some(4),
            _ => None,
        },
        Value::Array(items) => {
            let family = match items.first().map(|v| ctx.resolve(v)) {
                Some(Value::Name(n)) => n,
                _ => return None,
            };
            match family.as_slice() {
                b"CalGray" => Some(1),
                b"CalRGB" | b"Lab" => Some(3),
                b"ICCBased" => {
                    let stream = ctx.resolve(items.get(1)?);
                    let dict = match stream {
                        Value::Stream { dict, .. } => dict,
                        Value::Dict(d) => d,
                        _ => return None,
                    };
                    match int_entry(ctx, &dict, b"N") {
                        Some(1) => Some(1),
                        Some(3) => Some(3),
                        Some(4) => Some(4),
                        _ => None,
                    }
                }
                _ => None,
            }
        }
        _ => None,
    }
}

/// The `/DecodeParms` dictionary, taking the last element when the entry
/// is an array (it parallels the filter chain, and the predictor belongs
/// to the terminal general-purpose filter).
fn decode_parms_dict(ctx: &Ctx<'_>, header: &[(Vec<u8>, Value)]) -> Option<Vec<(Vec<u8>, Value)>> {
    let v = lookup(header, b"DecodeParms").or_else(|| lookup(header, b"DP"))?;
    match ctx.resolve(v) {
        Value::Dict(d) => Some(d),
        Value::Array(items) => items.iter().rev().find_map(|item| match ctx.resolve(item) {
            Value::Dict(d) => Some(d),
            _ => None,
        }),
        _ => None,
    }
}

/// Reverse a `/DecodeParms` predictor over decoded samples (§7.4.4.4):
/// TIFF horizontal differencing (2) or the PNG per-row filter family
/// (10–15). The step is byte-wise, so both the row stride and the
/// per-pixel step come from the sample depth — a sub-byte image packs to
/// `ceil(columns · colors · bpc / 8)` bytes per row, and using an 8-bit
/// stride would leave each row's filter byte in the pixels and shear the
/// image. Returns the input untouched for `Predictor <= 1`.
fn reverse_predictor(
    ctx: &Ctx<'_>,
    parms: &[(Vec<u8>, Value)],
    data: Vec<u8>,
    width: u32,
    comps: usize,
    image_bpc: i64,
) -> Vec<u8> {
    let predictor = int_entry(ctx, parms, b"Predictor").unwrap_or(1);
    if predictor <= 1 {
        return data;
    }
    let colors = int_entry(ctx, parms, b"Colors").unwrap_or(comps as i64).max(1) as usize;
    let columns = int_entry(ctx, parms, b"Columns").unwrap_or(width as i64).max(1) as usize;
    let bpc = int_entry(ctx, parms, b"BitsPerComponent").unwrap_or(image_bpc).max(1) as usize;

    let row_len = (columns * colors * bpc).div_ceil(8);
    if row_len == 0 {
        return data;
    }
    // Byte step between a sample and the one left of it; sub-byte
    // depths pack several samples per byte, so the step is one.
    let step = (colors * bpc).div_ceil(8).max(1);

    if predictor == 2 {
        // TIFF horizontal differencing. Defined per component, so only
        // the byte-aligned depths are reversible byte-wise.
        if bpc != 8 {
            return data;
        }
        let mut out = data;
        for row in out.chunks_mut(row_len) {
            for i in step..row.len() {
                row[i] = row[i].wrapping_add(row[i - step]);
            }
        }
        return out;
    }

    // PNG family: each row is preceded by a filter-type byte.
    let mut out = Vec::with_capacity(data.len());
    let mut prev = vec![0u8; row_len];
    let mut cur = vec![0u8; row_len];
    for chunk in data.chunks(row_len + 1) {
        if chunk.len() < 2 {
            break;
        }
        let kind = chunk[0];
        let src = &chunk[1..];
        cur.iter_mut().for_each(|b| *b = 0);
        for i in 0..src.len().min(row_len) {
            let left = if i >= step { cur[i - step] } else { 0 };
            let up = prev[i];
            let up_left = if i >= step { prev[i - step] } else { 0 };
            cur[i] = match kind {
                0 => src[i],
                1 => src[i].wrapping_add(left),
                2 => src[i].wrapping_add(up),
                3 => src[i].wrapping_add(((left as u16 + up as u16) / 2) as u8),
                4 => src[i].wrapping_add(paeth_byte(left, up, up_left)),
                // An unknown filter type means the stream is not what
                // /Predictor claimed; keep the bytes rather than
                // corrupting them further.
                _ => src[i],
            };
        }
        out.extend_from_slice(&cur[..src.len().min(row_len)]);
        prev.copy_from_slice(&cur);
    }
    out
}

/// The PNG Paeth predictor (RFC 2083 §6.6), computed in signed
/// arithmetic so the comparison is exact.
fn paeth_byte(a: u8, b: u8, c: u8) -> u8 {
    let p = a as i32 + b as i32 - c as i32;
    let (pa, pb, pc) = ((p - a as i32).abs(), (p - b as i32).abs(), (p - c as i32).abs());
    if pa <= pb && pa <= pc {
        a
    } else if pb <= pc {
        b
    } else {
        c
    }
}

/// Read packed samples as RGBA8. Rows are byte-aligned (§8.9.5.2), and
/// each component is scaled from its `bpc` range to 0–255. A row the
/// data runs out on ends the image — a truncated stream paints what it
/// carries rather than nothing.
fn samples_to_rgba(
    data: &[u8],
    width: u32,
    height: u32,
    bpc: i64,
    comps: usize,
) -> Option<Vec<u8>> {
    let (w, h) = (width as usize, height as usize);
    let bpc = bpc as usize;
    let row_len = (w * comps * bpc).div_ceil(8);
    if row_len == 0 || data.len() < row_len {
        return None;
    }
    // 16-bit samples are narrowed to 8 by `read_sample`, so the
    // normalisation ladder tops out at 255 for them.
    let max = if bpc == 16 { 255.0 } else { ((1u32 << bpc) - 1) as f64 };
    let mut rgba = vec![0u8; w * h * 4];
    let mut channel = vec![0f64; comps];

    for y in 0..h {
        let row = match data.get(y * row_len..(y + 1) * row_len) {
            Some(r) => r,
            None => break,
        };
        let mut bit = 0usize;
        for x in 0..w {
            for c in channel.iter_mut() {
                *c = read_sample(row, bit, bpc) as f64 / max;
                bit += bpc;
            }
            let rgb = match comps {
                1 => ColorRgb { r: channel[0], g: channel[0], b: channel[0] },
                3 => ColorRgb { r: channel[0], g: channel[1], b: channel[2] },
                _ => match colorspace::to_srgb(colorspace::Family::DeviceCMYK, &channel) {
                    Ok(c) => c,
                    Err(_) => return None,
                },
            };
            let d = (y * w + x) * 4;
            rgba[d] = colorspace::quantize(rgb.r);
            rgba[d + 1] = colorspace::quantize(rgb.g);
            rgba[d + 2] = colorspace::quantize(rgb.b);
            rgba[d + 3] = 255;
        }
    }
    Some(rgba)
}

/// One `bits`-wide sample starting at `bit_offset` within `row`,
/// MSB-first. Reads past the row's end as zero.
fn read_sample(row: &[u8], bit_offset: usize, bits: usize) -> u32 {
    let mut value = 0u32;
    for i in 0..bits {
        let bit = bit_offset + i;
        let byte = row.get(bit / 8).copied().unwrap_or(0);
        let set = (byte >> (7 - (bit % 8))) & 1;
        value = (value << 1) | set as u32;
    }
    if bits == 16 {
        // Scale 16-bit samples down to the 8-bit ladder the caller
        // normalises against.
        value >>= 8;
    }
    value
}

/// An integer dictionary entry, resolved through an indirect reference.
fn int_entry(ctx: &Ctx<'_>, dict: &[(Vec<u8>, Value)], key: &[u8]) -> Option<i64> {
    match ctx.resolve(lookup(dict, key)?) {
        Value::Integer(n) => Some(n),
        Value::Real(x) => Some(x as i64),
        _ => None,
    }
}

fn decode_dct_image(raw: &[u8]) -> (Option<Vec<u8>>, u32, u32) {
    let dec = match dctdecode::decode(raw) { Ok(d) => d, Err(_) => return (None, 0, 0) };
    to_rgba8(&dec.pixels, dec.width, dec.height, dec.components)
}

fn to_rgba8(pixels: &[u8], w: u32, h: u32, comps: u8) -> (Option<Vec<u8>>, u32, u32) {
    let n = (w as usize) * (h as usize);
    let expected = n * (comps as usize);
    if pixels.len() < expected { return (None, 0, 0); }
    if comps == 4 {
        return (Some(pixels.to_vec()), w, h);
    }
    let mut rgba = vec![0u8; n * 4];
    if comps == 3 {
        let mut s = 0;
        let mut d = 0;
        for _ in 0..n {
            rgba[d] = pixels[s];
            rgba[d + 1] = pixels[s + 1];
            rgba[d + 2] = pixels[s + 2];
            rgba[d + 3] = 255;
            s += 3;
            d += 4;
        }
    } else if comps == 1 {
        for i in 0..n {
            let v = pixels[i];
            rgba[i * 4] = v;
            rgba[i * 4 + 1] = v;
            rgba[i * 4 + 2] = v;
            rgba[i * 4 + 3] = 255;
        }
    } else {
        return (None, 0, 0);
    }
    (Some(rgba), w, h)
}

// Reference unused symbols so the linker / lint passes are happy
// even when a particular call site is gated out.
#[allow(dead_code)]
fn _unused_apply_marker() {
    let _ = apply;
}
