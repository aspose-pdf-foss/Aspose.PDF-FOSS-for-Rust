//! Target-side helper: locate where a page draws each image XObject.
//! Backs the public `ImagePlacementAbsorber`. Non-anchored (composes
//! the object / xref parsers, the content-stream parser, and the flate
//! codec).
//!
//! Walks a page's content stream tracking the current transformation
//! matrix (the `q`/`Q` stack and `cm` composition) and the current blend
//! mode (the `gs` operator resolving `/ExtGState … /BM`), and records, for
//! every `/Name Do` invocation, the operand name paired with the CTM and
//! blend mode in effect plus the image's compression filter and mask
//! state. The caller correlates those names against the page's image
//! XObjects (via [`super::ximage_io::parse_page_images`]) to keep only
//! image placements — Form-XObject `Do`s fall away because their names
//! never match an image.

use std::collections::HashMap;

use super::content_stream::{self, Value as CsValue};
use super::objects::{self, Value};
use super::{flate, xref};

/// Affine matrix `[a b c d e f]` (PDF row-vector convention:
/// `point' = point · M`).
pub(crate) type Matrix = [f64; 6];

const IDENTITY: Matrix = [1.0, 0.0, 0.0, 1.0, 0.0, 0.0];

/// Compose `m` then `n` (matrix product `m · n`).
fn mat_mul(m: &Matrix, n: &Matrix) -> Matrix {
    [
        m[0] * n[0] + m[1] * n[2],
        m[0] * n[1] + m[1] * n[3],
        m[2] * n[0] + m[3] * n[2],
        m[2] * n[1] + m[3] * n[3],
        m[4] * n[0] + m[5] * n[2] + n[4],
        m[4] * n[1] + m[5] * n[3] + n[5],
    ]
}

/// Compression / mask metadata of one image XObject, read from its
/// stream dictionary.
#[derive(Clone, Default)]
pub(crate) struct ImageMeta {
    /// The image codec named in `/Filter` (last recognised entry when an
    /// array), lower-cased of case as stored. `None` when unfiltered or
    /// unrecognised.
    pub filter: Option<Vec<u8>>,
    /// Whether the image carries a soft mask (`/SMask`) or a `/Mask`.
    pub is_masked: bool,
}

/// One `Do` invocation: the XObject resource name, the CTM and blend mode
/// active at that point, and the image's compression / mask metadata.
pub(crate) struct Placement {
    pub name: String,
    pub matrix: Matrix,
    /// The `/BM` name in effect (e.g. `Normal`, `Multiply`).
    pub blend: Vec<u8>,
    pub filter: Option<Vec<u8>>,
    pub is_masked: bool,
}

/// Every image `Do` on page `leaf_id`, in content-stream order. Names
/// that don't resolve to an image XObject are filtered out here.
pub(crate) fn parse_page_placements(bytes: &[u8], leaf_id: u32) -> Vec<Placement> {
    let objs = build_objs(bytes);
    let Some(page) = objs.get(&leaf_id) else {
        return Vec::new();
    };

    // Resource name → image-XObject metadata (filter / mask).
    let image_meta = image_xobject_meta(&objs, page);
    if image_meta.is_empty() {
        return Vec::new();
    }
    // Named ExtGState dicts, for resolving the `gs` operator's `/BM`.
    let ext_gstates = ext_gstate_dicts(&objs, page);

    let content = match assemble_contents(&objs, page) {
        Some(c) => c,
        None => return Vec::new(),
    };
    let ops = match content_stream::parse(&content) {
        Ok(o) => o,
        Err(_) => return Vec::new(),
    };

    let mut ctm = IDENTITY;
    let mut blend: Vec<u8> = b"Normal".to_vec();
    let mut stack: Vec<(Matrix, Vec<u8>)> = Vec::new();
    let mut out = Vec::new();
    for op in &ops {
        match op.op.as_str() {
            "q" => stack.push((ctm, blend.clone())),
            "Q" => {
                if let Some((m, b)) = stack.pop() {
                    ctm = m;
                    blend = b;
                }
            }
            "cm" => {
                if let Some(m) = matrix_operands(&op.operands) {
                    ctm = mat_mul(&m, &ctm);
                }
            }
            "gs" => {
                if let Some(CsValue::Name(n)) = op.operands.first() {
                    if let Some(gs) = ext_gstates.get(n.as_slice()) {
                        if let Some(bm) = blend_mode_name(gs) {
                            blend = bm;
                        }
                    }
                }
            }
            "Do" => {
                if let Some(CsValue::Name(n)) = op.operands.first() {
                    let name = String::from_utf8_lossy(n).into_owned();
                    if let Some(meta) = image_meta.get(name.as_str()) {
                        out.push(Placement {
                            name,
                            matrix: ctm,
                            blend: blend.clone(),
                            filter: meta.filter.clone(),
                            is_masked: meta.is_masked,
                        });
                    }
                }
            }
            _ => {}
        }
    }
    out
}

/// The resource names under `/Resources /XObject` whose `/Subtype` is
/// `/Image`, paired with each image's compression / mask metadata.
fn image_xobject_meta(objs: &HashMap<u32, Value>, page: &Value) -> HashMap<String, ImageMeta> {
    let mut metas = HashMap::new();
    let Some(resources) = dict_get(page, b"Resources").and_then(|v| resolve(objs, &v)) else {
        return metas;
    };
    let Some(Value::Dict(entries)) =
        dict_get(&resources, b"XObject").and_then(|v| resolve(objs, &v))
    else {
        return metas;
    };
    for (name, value) in &entries {
        let Some(stream) = resolve(objs, value) else { continue };
        if matches!(dict_get(&stream, b"Subtype"), Some(Value::Name(n)) if n == b"Image") {
            let is_masked = dict_get(&stream, b"SMask").is_some() || dict_get(&stream, b"Mask").is_some();
            let meta = ImageMeta { filter: image_codec(&dict_get(&stream, b"Filter")), is_masked };
            metas.insert(String::from_utf8_lossy(name).into_owned(), meta);
        }
    }
    metas
}

/// The `/Resources /ExtGState` sub-dictionaries, keyed by resource name.
fn ext_gstate_dicts(objs: &HashMap<u32, Value>, page: &Value) -> HashMap<Vec<u8>, Value> {
    let mut out = HashMap::new();
    let Some(resources) = dict_get(page, b"Resources").and_then(|v| resolve(objs, &v)) else {
        return out;
    };
    let Some(Value::Dict(entries)) =
        dict_get(&resources, b"ExtGState").and_then(|v| resolve(objs, &v))
    else {
        return out;
    };
    for (name, value) in &entries {
        if let Some(dict) = resolve(objs, value) {
            out.insert(name.clone(), dict);
        }
    }
    out
}

/// The `/BM` blend-mode name of an ExtGState dict — a single name, or the
/// first entry when `/BM` is an array of fallbacks.
fn blend_mode_name(gs: &Value) -> Option<Vec<u8>> {
    match dict_get(gs, b"BM") {
        Some(Value::Name(n)) => Some(n),
        Some(Value::Array(a)) => a.into_iter().find_map(|v| match v {
            Value::Name(n) => Some(n),
            _ => None,
        }),
        _ => None,
    }
}

/// The recognised image codec named in `/Filter` — the last recognised
/// entry when `/Filter` is a cascade array (the codec closest to the raw
/// samples). `None` when unfiltered or unrecognised.
fn image_codec(filter: &Option<Value>) -> Option<Vec<u8>> {
    let is_codec = |n: &[u8]| {
        matches!(
            n,
            b"DCTDecode" | b"JPXDecode" | b"FlateDecode" | b"CCITTFaxDecode"
        )
    };
    match filter {
        Some(Value::Name(n)) if is_codec(n) => Some(n.clone()),
        Some(Value::Array(a)) => a.iter().rev().find_map(|v| match v {
            Value::Name(n) if is_codec(n) => Some(n.clone()),
            _ => None,
        }),
        _ => None,
    }
}

/// Concatenate + filter-decode a page's `/Contents` (single stream or
/// an array of streams). Only `FlateDecode` and unfiltered streams are
/// handled, matching the corpus.
fn assemble_contents(objs: &HashMap<u32, Value>, page: &Value) -> Option<Vec<u8>> {
    let contents = dict_get(page, b"Contents").and_then(|v| resolve(objs, &v))?;
    match contents {
        Value::Stream { .. } => decode_stream(&contents),
        Value::Array(items) => {
            let mut out = Vec::new();
            for item in &items {
                if let Some(stream) = resolve(objs, item) {
                    if let Some(bytes) = decode_stream(&stream) {
                        out.extend_from_slice(&bytes);
                        out.push(b'\n');
                    }
                }
            }
            Some(out)
        }
        _ => None,
    }
}

/// Decode a content stream's body, applying `FlateDecode` when present.
fn decode_stream(stream: &Value) -> Option<Vec<u8>> {
    let Value::Stream { body, .. } = stream else { return None };
    match filter_name(&dict_get(stream, b"Filter")) {
        Some(f) if f == b"FlateDecode" => flate::decode(body).ok(),
        Some(_) => None, // unsupported filter — skip
        None => Some(body.clone()),
    }
}

/// Read six numeric operands as a matrix, or `None` if malformed.
fn matrix_operands(operands: &[CsValue]) -> Option<Matrix> {
    if operands.len() != 6 {
        return None;
    }
    let mut m = [0.0f64; 6];
    for (i, v) in operands.iter().enumerate() {
        m[i] = match v {
            CsValue::Integer(n) => *n as f64,
            CsValue::Real(r) => *r,
            _ => return None,
        };
    }
    Some(m)
}

// ---- Shared helpers --------------------------------------------------

fn build_objs(bytes: &[u8]) -> HashMap<u32, Value> {
    let mut objs = HashMap::new();
    if let Ok(table) = xref::parse(bytes) {
        for entry in &table.entries {
            if let xref::Entry::InUse { id, offset, .. } = *entry {
                if let Ok(o) = objects::parse_object(bytes, offset as usize) {
                    objs.insert(id, o.value);
                }
            }
        }
    }
    objs
}

fn resolve(objs: &HashMap<u32, Value>, v: &Value) -> Option<Value> {
    match v {
        Value::Ref { id, .. } => objs.get(id).cloned(),
        other => Some(other.clone()),
    }
}

fn dict_get(v: &Value, key: &[u8]) -> Option<Value> {
    let entries = match v {
        Value::Dict(e) => e,
        Value::Stream { dict, .. } => dict,
        _ => return None,
    };
    entries.iter().find(|(k, _)| k.as_slice() == key).map(|(_, val)| val.clone())
}

fn filter_name(filter: &Option<Value>) -> Option<Vec<u8>> {
    match filter {
        Some(Value::Name(n)) => Some(n.clone()),
        Some(Value::Array(a)) => a.first().and_then(|v| match v {
            Value::Name(n) => Some(n.clone()),
            _ => None,
        }),
        _ => None,
    }
}
