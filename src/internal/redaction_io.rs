//! Target-side helper: burn redaction regions out of a page's content
//! stream(s). Backs the public `RedactionAnnotation::redact` save-through.
//! Non-anchored (composes the trailer / xref / objects parsers, the
//! content-stream parser, the flate codec, and the incremental writer).
//!
//! A redaction is a genuine removal, not a cover: every text-showing
//! operator (`Tj`, `TJ`, `'`, `"`) whose text origin falls inside a
//! redaction rectangle is dropped from the re-serialised content. The
//! rewritten content stream overrides the original object id in the
//! incremental update, emitted uncompressed (the `/Filter` is removed).

use std::collections::HashMap;

use super::content_stream::{self, Operation, Value as CsValue};
use super::flate;
use super::objects::{self, Value};
use super::pdf_writer_incremental::{self, DirtyObject};
use super::{trailer, xref};

/// One redaction request: remove text inside `rect` on page `leaf_id`.
pub(crate) struct RedactRequest {
    pub leaf_id: u32,
    /// `[llx, lly, urx, ury]` in PDF user space.
    pub rect: [f64; 4],
}

/// Apply every redaction request as an incremental update over `bytes`,
/// rewriting the affected content streams with the in-region text removed.
/// Returns `None` when the document can't be parsed (forgiving posture
/// matching the other `*_io` composers).
pub(crate) fn apply(bytes: &[u8], requests: &[RedactRequest]) -> Option<Vec<u8>> {
    if requests.is_empty() {
        return Some(bytes.to_vec());
    }
    let tr = trailer::parse(bytes).ok()?;
    let _ = tr; // trailer presence is the parse-validity gate
    let objs = build_objs(bytes);

    // Group rects by page leaf.
    let mut by_page: HashMap<u32, Vec<[f64; 4]>> = HashMap::new();
    for r in requests {
        by_page.entry(r.leaf_id).or_default().push(r.rect);
    }

    // Collect rewritten content streams keyed by their object id so two
    // pages sharing a stream (rare) don't double-emit.
    let mut dirty: HashMap<u32, DirtyObject> = HashMap::new();

    for (leaf_id, rects) in &by_page {
        let page_dict = match objs.get(leaf_id) {
            Some(Value::Dict(d)) => d,
            _ => continue,
        };
        // The page's fonts (for glyph advances in the position pass).
        let fonts = resolve_fonts(&objs, page_dict);

        // Resolve /Contents to a list of (object id, gen) content streams.
        let stream_ids = content_stream_ids(page_dict);
        if stream_ids.is_empty() {
            continue;
        }

        for (sid, sgen) in stream_ids {
            let (sdict, raw_body) = match objs.get(&sid) {
                Some(Value::Stream { dict, body }) => (dict.clone(), body.clone()),
                _ => continue,
            };
            let decoded = match decode_stream(&sdict, &raw_body) {
                Some(d) => d,
                None => continue, // unsupported filter — leave untouched
            };
            let ops = match content_stream::parse(&decoded) {
                Ok(o) => o,
                Err(_) => continue,
            };
            let (rewritten, changed) = redact_ops(&ops, rects, fonts.as_ref());
            if !changed {
                continue;
            }
            let new_body = content_stream::to_canonical(&rewritten);
            // Drop /Filter + /Length: the rewritten body is uncompressed
            // and the writer re-injects /Length.
            let mut new_dict: Vec<(Vec<u8>, Value)> = sdict
                .iter()
                .filter(|(k, _)| k.as_slice() != b"Filter" && k.as_slice() != b"Length")
                .cloned()
                .collect();
            // Strip /DecodeParms too (it only describes a now-absent filter).
            new_dict.retain(|(k, _)| k.as_slice() != b"DecodeParms" && k.as_slice() != b"DP");
            dirty.insert(
                sid,
                DirtyObject {
                    id: sid,
                    generation: sgen,
                    value: Value::Stream { dict: new_dict, body: new_body },
                },
            );
        }
    }

    if dirty.is_empty() {
        return Some(bytes.to_vec());
    }
    let list: Vec<DirtyObject> = dirty.into_values().collect();
    pdf_writer_incremental::append_incremental(bytes, &list).ok()
}

// ---- /Contents + /Resources resolution -------------------------------

/// The object ids (and generations) of the page's content stream(s),
/// in order. Handles a single `/Contents` Ref and an array of Refs.
fn content_stream_ids(page_dict: &[(Vec<u8>, Value)]) -> Vec<(u32, u16)> {
    let mut out = Vec::new();
    let contents = match dict_get(page_dict, b"Contents") {
        Some(v) => v,
        None => return out,
    };
    match contents {
        Value::Ref { id, gen } => out.push((id, gen)),
        Value::Array(items) => {
            for it in items {
                if let Value::Ref { id, gen } = it {
                    out.push((id, gen));
                }
            }
        }
        _ => {}
    }
    out
}

/// Resolve the page's `/Resources /Font` dict, following `/Parent`
/// inheritance. Returns the font subdictionary (name → font dict / ref).
fn resolve_fonts(
    objs: &HashMap<u32, Value>,
    page_dict: &[(Vec<u8>, Value)],
) -> Option<Vec<(Vec<u8>, Value)>> {
    let mut current = page_dict.to_vec();
    for _ in 0..16 {
        if let Some(res_v) = dict_get(&current, b"Resources") {
            let res = resolve_dict(objs, &res_v)?;
            let font_v = dict_get(&res, b"Font")?;
            let fonts = resolve_dict(objs, &font_v)?;
            // Pre-resolve each font entry's Ref to its dict so the
            // advance pass sees concrete `/BaseFont` + `/Widths`.
            let resolved = fonts
                .into_iter()
                .map(|(name, v)| {
                    let dict = resolve_dict(objs, &v).map(Value::Dict).unwrap_or(v);
                    (name, dict)
                })
                .collect();
            return Some(resolved);
        }
        let parent_v = dict_get(&current, b"Parent")?;
        current = resolve_dict(objs, &parent_v)?;
    }
    None
}

// ---- Stream decode ---------------------------------------------------

/// Decode a content-stream body, applying a `/FlateDecode` filter when
/// present (single Name or all-Flate array). Returns `None` for any
/// other filter (the stream is then left untouched).
fn decode_stream(dict: &[(Vec<u8>, Value)], body: &[u8]) -> Option<Vec<u8>> {
    match dict_get(dict, b"Filter") {
        None => Some(body.to_vec()),
        Some(Value::Name(n)) => {
            if n == b"FlateDecode" {
                flate::decode(body).ok()
            } else {
                None
            }
        }
        Some(Value::Array(items)) => {
            let mut raw = body.to_vec();
            for f in &items {
                match f {
                    Value::Name(n) if n == b"FlateDecode" => {
                        raw = flate::decode(&raw).ok()?;
                    }
                    _ => return None,
                }
            }
            Some(raw)
        }
        _ => None,
    }
}

// ---- Redaction pass over the operator list ---------------------------

type Matrix = [f64; 6];
const IDENTITY: Matrix = [1.0, 0.0, 0.0, 1.0, 0.0, 0.0];

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

fn mat_apply(m: &Matrix, x: f64, y: f64) -> (f64, f64) {
    (m[0] * x + m[2] * y + m[4], m[1] * x + m[3] * y + m[5])
}

fn num(v: Option<&CsValue>) -> f64 {
    match v {
        Some(CsValue::Integer(n)) => *n as f64,
        Some(CsValue::Real(r)) => *r,
        _ => 0.0,
    }
}

/// Per-font advance metrics for the text-matrix update during the walk.
struct PosFont {
    base_font: String,
    first_char: i64,
    widths: Vec<u16>,
}

impl PosFont {
    fn glyph_width(&self, code: u8) -> f64 {
        let c = code as i64;
        if c >= self.first_char {
            let idx = (c - self.first_char) as usize;
            if idx < self.widths.len() {
                return self.widths[idx] as f64;
            }
        }
        if let Some(w) = super::standard14_metrics::width(&self.base_font, code) {
            if w != 0 {
                return w as f64;
            }
        }
        500.0
    }
}

/// Walk the operators tracking the graphics + text state; drop every
/// text-show whose origin falls inside any redaction rect. Returns the
/// rewritten op list and whether anything was removed.
fn redact_ops(
    ops: &[Operation],
    rects: &[[f64; 4]],
    fonts: Option<&Vec<(Vec<u8>, Value)>>,
) -> (Vec<Operation>, bool) {
    let mut out: Vec<Operation> = Vec::with_capacity(ops.len());
    let mut changed = false;

    let mut ctm = IDENTITY;
    let mut ctm_stack: Vec<Matrix> = Vec::new();
    let mut tm = IDENTITY;
    let mut lm = IDENTITY;
    let mut font: Option<PosFont> = None;
    let mut font_size = 0.0_f64;
    let mut tc = 0.0_f64;
    let mut tw = 0.0_f64;
    let mut th = 100.0_f64;
    let mut leading = 0.0_f64;

    for op in ops {
        let a = &op.operands;
        match op.op.as_str() {
            "q" => ctm_stack.push(ctm),
            "Q" => {
                if let Some(m) = ctm_stack.pop() {
                    ctm = m;
                }
            }
            "cm" => {
                let m: Matrix = [
                    num(a.first()),
                    num(a.get(1)),
                    num(a.get(2)),
                    num(a.get(3)),
                    num(a.get(4)),
                    num(a.get(5)),
                ];
                ctm = mat_mul(&m, &ctm);
            }
            "BT" => {
                tm = IDENTITY;
                lm = IDENTITY;
            }
            "Tf" => {
                font = Some(configure_font(op, fonts));
                font_size = num(a.get(1));
            }
            "Tc" => tc = num(a.first()),
            "Tw" => tw = num(a.first()),
            "Tz" => th = num(a.first()),
            "TL" => leading = num(a.first()),
            "Td" => {
                let m = [1.0, 0.0, 0.0, 1.0, num(a.first()), num(a.get(1))];
                lm = mat_mul(&m, &lm);
                tm = lm;
            }
            "TD" => {
                leading = -num(a.get(1));
                let m = [1.0, 0.0, 0.0, 1.0, num(a.first()), num(a.get(1))];
                lm = mat_mul(&m, &lm);
                tm = lm;
            }
            "Tm" => {
                lm = [
                    num(a.first()),
                    num(a.get(1)),
                    num(a.get(2)),
                    num(a.get(3)),
                    num(a.get(4)),
                    num(a.get(5)),
                ];
                tm = lm;
            }
            "T*" => {
                let m = [1.0, 0.0, 0.0, 1.0, 0.0, -leading];
                lm = mat_mul(&m, &lm);
                tm = lm;
            }
            "'" | "\"" => {
                // Both advance to the next line first.
                let m = [1.0, 0.0, 0.0, 1.0, 0.0, -leading];
                lm = mat_mul(&m, &lm);
                tm = lm;
                if op.op == "\"" {
                    tw = num(a.first());
                    tc = num(a.get(1));
                }
                let str_idx = if op.op == "\"" { 2 } else { 0 };
                let origin = mat_apply(&mat_mul(&tm, &ctm), 0.0, 0.0);
                let inside = point_in_any(origin, font_size, rects);
                if let Some(CsValue::String(bytes)) = a.get(str_idx) {
                    advance_tm(&mut tm, bytes, &font, font_size, tc, tw, th);
                    if inside {
                        changed = true;
                        continue; // drop the show op entirely
                    }
                }
                out.push(op.clone());
                continue;
            }
            "Tj" => {
                let origin = mat_apply(&mat_mul(&tm, &ctm), 0.0, 0.0);
                let inside = point_in_any(origin, font_size, rects);
                if let Some(CsValue::String(bytes)) = a.first() {
                    advance_tm(&mut tm, bytes, &font, font_size, tc, tw, th);
                    if inside {
                        changed = true;
                        continue;
                    }
                }
                out.push(op.clone());
                continue;
            }
            "TJ" => {
                let origin = mat_apply(&mat_mul(&tm, &ctm), 0.0, 0.0);
                let inside = point_in_any(origin, font_size, rects);
                if let Some(CsValue::Array(items)) = a.first() {
                    advance_tm_tj(&mut tm, items, &font, font_size, tc, tw, th);
                    if inside {
                        changed = true;
                        continue;
                    }
                }
                out.push(op.clone());
                continue;
            }
            _ => {}
        }
        out.push(op.clone());
    }

    (out, changed)
}

/// Whether `origin` (a text baseline origin in user space) lies inside
/// any redaction rect. The y test is widened upward by the font size so a
/// baseline that sits below the glyph's cap height still matches a rect
/// drawn around the visible glyphs.
fn point_in_any(origin: (f64, f64), font_size: f64, rects: &[[f64; 4]]) -> bool {
    let (x, y) = origin;
    let pad = (font_size * 0.25).max(1.0);
    rects.iter().any(|r| {
        let (llx, lly, urx, ury) = (r[0].min(r[2]), r[1].min(r[3]), r[0].max(r[2]), r[1].max(r[3]));
        x >= llx - 0.01 && x <= urx + 0.01 && y >= lly - pad && y <= ury + pad
    })
}

/// Advance the text matrix by a shown literal string's width.
fn advance_tm(
    tm: &mut Matrix,
    bytes: &[u8],
    font: &Option<PosFont>,
    font_size: f64,
    tc: f64,
    tw: f64,
    th: f64,
) {
    let mut adv = 0.0_f64;
    for &code in bytes {
        let gw = font.as_ref().map_or(500.0, |f| f.glyph_width(code));
        adv += (gw / 1000.0 * font_size + tc + if code == b' ' { tw } else { 0.0 }) * (th / 100.0);
    }
    let m = [1.0, 0.0, 0.0, 1.0, adv, 0.0];
    *tm = mat_mul(&m, tm);
}

/// Advance the text matrix by a `TJ` array's width.
fn advance_tm_tj(
    tm: &mut Matrix,
    items: &[CsValue],
    font: &Option<PosFont>,
    font_size: f64,
    tc: f64,
    tw: f64,
    th: f64,
) {
    let mut adv = 0.0_f64;
    for it in items {
        match it {
            CsValue::String(bytes) => {
                for &code in bytes {
                    let gw = font.as_ref().map_or(500.0, |f| f.glyph_width(code));
                    adv += (gw / 1000.0 * font_size
                        + tc
                        + if code == b' ' { tw } else { 0.0 })
                        * (th / 100.0);
                }
            }
            CsValue::Integer(n) => adv += -(*n as f64) / 1000.0 * font_size * (th / 100.0),
            CsValue::Real(r) => adv += -*r / 1000.0 * font_size * (th / 100.0),
            _ => {}
        }
    }
    let m = [1.0, 0.0, 0.0, 1.0, adv, 0.0];
    *tm = mat_mul(&m, tm);
}

/// Resolve a `Tf` operator's font into advance metrics. Only the
/// `/BaseFont`, `/FirstChar`, and `/Widths` are needed here.
fn configure_font(tf: &Operation, fonts: Option<&Vec<(Vec<u8>, Value)>>) -> PosFont {
    let mut pos = PosFont { base_font: String::new(), first_char: 0, widths: Vec::new() };
    let font_name = match tf.operands.first() {
        Some(CsValue::Name(n)) => n,
        _ => return pos,
    };
    let fonts = match fonts {
        Some(f) => f,
        None => return pos,
    };
    let entry = match fonts.iter().find(|(k, _)| k == font_name).map(|(_, v)| v) {
        Some(v) => v,
        None => return pos,
    };
    // The font entry may itself be a Ref; the caller pre-resolved the
    // /Font subdict but its values can still be indirect.
    let font_dict = match entry {
        Value::Dict(d) => d.clone(),
        _ => return pos,
    };
    if let Some(Value::Name(bf)) = dict_get(&font_dict, b"BaseFont") {
        pos.base_font = String::from_utf8_lossy(&bf).into_owned();
    }
    if let Some(Value::Integer(fc)) = dict_get(&font_dict, b"FirstChar") {
        pos.first_char = fc;
    }
    if let Some(Value::Array(items)) = dict_get(&font_dict, b"Widths") {
        pos.widths = items
            .iter()
            .map(|it| match it {
                Value::Integer(n) => (*n).clamp(0, u16::MAX as i64) as u16,
                Value::Real(r) => r.round().clamp(0.0, u16::MAX as f64) as u16,
                _ => 0,
            })
            .collect();
    }
    pos
}

// ---- Shared object helpers -------------------------------------------

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

fn dict_get(dict: &[(Vec<u8>, Value)], key: &[u8]) -> Option<Value> {
    dict.iter().find(|(k, _)| k.as_slice() == key).map(|(_, v)| v.clone())
}

/// Resolve a value to a dictionary's entry list, following one level of
/// indirection.
fn resolve_dict(objs: &HashMap<u32, Value>, v: &Value) -> Option<Vec<(Vec<u8>, Value)>> {
    let resolved = match v {
        Value::Ref { id, .. } => objs.get(id)?.clone(),
        other => other.clone(),
    };
    match resolved {
        Value::Dict(d) => Some(d),
        Value::Stream { dict, .. } => Some(dict),
        _ => None,
    }
}
