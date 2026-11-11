//! Target-side helper: read / write a page's `/MediaBox`, `/CropBox`
//! and `/Rotate`. Backs the public `Page::rect` / `Page::media_box` /
//! `Page::crop_box` / `Page::rotate` / `Page::set_page_size`
//! save-through. Non-anchored (composes the trailer / xref / objects
//! parsers + the incremental writer).
//!
//! `/MediaBox` and `/CropBox` are inheritable (PDF 32000-1:2008
//! §7.7.3.3); a leaf that lacks one walks its `/Parent` chain. The
//! resolved `/MediaBox` is the only box guaranteed present — `/CropBox`
//! defaults to it.

use std::collections::HashMap;

use super::objects::{self, Value};
use super::pdf_writer_incremental::{self, DirtyObject};
use super::{trailer, xref};

/// One page's resolved geometry boxes (`[llx, lly, urx, ury]`) plus its
/// `/Rotate` (already normalised into `0` / `90` / `180` / `270`).
pub(crate) struct PageGeometry {
    pub media_box: [f64; 4],
    pub crop_box: [f64; 4],
    /// §14.11.2 boxes. `None` means the entry is absent, which the
    /// caller reads as "same as the /CropBox". Unlike /MediaBox and
    /// /CropBox these are NOT inheritable (Table 30), so they are read
    /// from the leaf alone.
    pub art_box: Option<[f64; 4]>,
    pub bleed_box: Option<[f64; 4]>,
    pub trim_box: Option<[f64; 4]>,
    pub rotate: i32,
}

/// One page's edited geometry, addressed by its leaf object id.
pub(crate) struct GeometryEdit {
    pub leaf_id: u32,
    pub media_box: [f64; 4],
    pub crop_box: [f64; 4],
    /// Written only when set; an absent box stays absent rather than
    /// being materialised as a copy of the /CropBox.
    pub art_box: Option<[f64; 4]>,
    pub bleed_box: Option<[f64; 4]>,
    pub trim_box: Option<[f64; 4]>,
    pub rotate: i32,
}

// ---- Read ------------------------------------------------------------

/// Resolve the geometry of the page object `leaf_id`. `/MediaBox` and
/// `/CropBox` follow the `/Parent` inheritance chain; `/CropBox`
/// defaults to the resolved `/MediaBox`; `/Rotate` likewise inherits and
/// defaults to `0`.
pub(crate) fn parse_page_geometry(bytes: &[u8], leaf_id: u32) -> Option<PageGeometry> {
    let objs = build_objs(bytes);
    let media_box = inherited_rect(&objs, leaf_id, b"MediaBox")?;
    let crop_box = inherited_rect(&objs, leaf_id, b"CropBox").unwrap_or(media_box);
    let rotate = inherited_rotate(&objs, leaf_id).unwrap_or(0);
    Some(PageGeometry {
        media_box,
        crop_box,
        art_box: leaf_rect(&objs, leaf_id, b"ArtBox"),
        bleed_box: leaf_rect(&objs, leaf_id, b"BleedBox"),
        trim_box: leaf_rect(&objs, leaf_id, b"TrimBox"),
        rotate,
    })
}

/// The `key` rectangle on the leaf itself. The §14.11.2 boxes do not
/// inherit, so there is no `/Parent` walk here.
fn leaf_rect(objs: &HashMap<u32, Value>, leaf_id: u32, key: &[u8]) -> Option<[f64; 4]> {
    let node = objs.get(&leaf_id)?;
    match dict_get(node, key) {
        Some(Value::Array(a)) if a.len() == 4 => {
            Some([num(&a[0]), num(&a[1]), num(&a[2]), num(&a[3])])
        }
        _ => None,
    }
}

/// Walk `leaf_id`'s `/Parent` chain looking for the first `key`
/// rectangle (a 4-element numeric array).
fn inherited_rect(objs: &HashMap<u32, Value>, leaf_id: u32, key: &[u8]) -> Option<[f64; 4]> {
    let mut id = leaf_id;
    let mut guard = 0;
    loop {
        let node = objs.get(&id)?;
        if let Some(Value::Array(a)) = dict_get(node, key) {
            if a.len() == 4 {
                return Some([num(&a[0]), num(&a[1]), num(&a[2]), num(&a[3])]);
            }
        }
        match dict_get(node, b"Parent") {
            Some(Value::Ref { id: pid, .. }) => id = pid,
            _ => return None,
        }
        guard += 1;
        if guard > 64 {
            return None;
        }
    }
}

/// Walk `leaf_id`'s `/Parent` chain looking for the first `/Rotate`,
/// normalised into `[0, 360)`.
fn inherited_rotate(objs: &HashMap<u32, Value>, leaf_id: u32) -> Option<i32> {
    let mut id = leaf_id;
    let mut guard = 0;
    loop {
        let node = objs.get(&id)?;
        match dict_get(node, b"Rotate") {
            Some(Value::Integer(n)) => return Some((n as i32).rem_euclid(360)),
            Some(Value::Real(r)) => return Some((r as i32).rem_euclid(360)),
            _ => {}
        }
        match dict_get(node, b"Parent") {
            Some(Value::Ref { id: pid, .. }) => id = pid,
            _ => return None,
        }
        guard += 1;
        if guard > 64 {
            return None;
        }
    }
}

// ---- Write -----------------------------------------------------------

/// Append an incremental update rewriting `/MediaBox`, `/CropBox` and
/// `/Rotate` on each given page leaf. Returns `None` when the document
/// can't be parsed.
pub(crate) fn append(bytes: &[u8], edits: &[GeometryEdit]) -> Option<Vec<u8>> {
    let _ = trailer::parse(bytes).ok()?;
    let objs = build_objs(bytes);
    let mut dirty: Vec<DirtyObject> = Vec::new();

    for e in edits {
        let mut page_dict = match objs.get(&e.leaf_id) {
            Some(Value::Dict(d)) => d.clone(),
            _ => continue,
        };
        set_entry(&mut page_dict, b"MediaBox", rect_value(&e.media_box));
        set_entry(&mut page_dict, b"CropBox", rect_value(&e.crop_box));
        for (key, box_) in [
            (b"ArtBox".as_slice(), &e.art_box),
            (b"BleedBox".as_slice(), &e.bleed_box),
            (b"TrimBox".as_slice(), &e.trim_box),
        ] {
            if let Some(rect) = box_ {
                set_entry(&mut page_dict, key, rect_value(rect));
            }
        }
        // `/Rotate 0` is the default — emit it explicitly so an edit
        // that resets a rotated page back to upright is durable.
        set_entry(
            &mut page_dict,
            b"Rotate",
            Value::Integer(e.rotate.rem_euclid(360) as i64),
        );
        dirty.push(DirtyObject {
            id: e.leaf_id,
            generation: 0,
            value: Value::Dict(page_dict),
        });
    }

    if dirty.is_empty() {
        return Some(bytes.to_vec());
    }
    pdf_writer_incremental::append_incremental(bytes, &dirty).ok()
}

fn rect_value(r: &[f64; 4]) -> Value {
    Value::Array(r.iter().map(|c| Value::Real(*c)).collect())
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

fn dict_get(v: &Value, key: &[u8]) -> Option<Value> {
    let entries = match v {
        Value::Dict(e) => e,
        Value::Stream { dict, .. } => dict,
        _ => return None,
    };
    entries
        .iter()
        .find(|(k, _)| k.as_slice() == key)
        .map(|(_, val)| val.clone())
}

fn set_entry(entries: &mut Vec<(Vec<u8>, Value)>, key: &[u8], value: Value) {
    if let Some(slot) = entries.iter_mut().find(|(k, _)| k.as_slice() == key) {
        slot.1 = value;
    } else {
        entries.push((key.to_vec(), value));
    }
}

fn num(v: &Value) -> f64 {
    match v {
        Value::Integer(n) => *n as f64,
        Value::Real(r) => *r,
        _ => 0.0,
    }
}
