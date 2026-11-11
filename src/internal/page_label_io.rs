//! Target-side helper: read / write the catalog's `/PageLabels` number
//! tree (page-label ranges). Backs the public `Document::page_labels`
//! save-through. Non-anchored (composes the trailer / xref / objects
//! parsers + the incremental writer).
//!
//! A label range is carried as `(start_index, /S style name, /St
//! starting value, /P prefix)`; the public layer maps the `/S` name to
//! and from [`NumberingStyle`](crate::public::numbering_style::NumberingStyle).

use std::collections::HashMap;

use super::objects::{self, Value};
use super::pdf_writer_incremental::{self, DirtyObject};
use super::{trailer, xref};

/// One label range's serialisable fields.
pub(crate) struct LabelData {
    pub index: i32,
    pub style: Option<String>,
    pub starting: i32,
    pub prefix: String,
}

// ---- Read ------------------------------------------------------------

/// Parse the page-label ranges from `bytes`, in `/Nums` order.
pub(crate) fn parse(bytes: &[u8]) -> Vec<LabelData> {
    let Ok(tr) = trailer::parse(bytes) else {
        return Vec::new();
    };
    let objs = build_objs(bytes);
    let Some(catalog) = objs.get(&tr.root_id) else {
        return Vec::new();
    };
    let Some(labels) = dict_get(catalog, b"PageLabels").and_then(|v| resolve(&objs, &v)) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    collect_tree(&objs, &labels, &mut out);
    out
}

fn collect_tree(objs: &HashMap<u32, Value>, node: &Value, out: &mut Vec<LabelData>) {
    if let Some(Value::Array(kids)) = dict_get(node, b"Kids") {
        for kid in &kids {
            if let Some(resolved) = resolve(objs, kid) {
                collect_tree(objs, &resolved, out);
            }
        }
    }
    if let Some(Value::Array(nums)) = dict_get(node, b"Nums") {
        let mut i = 0;
        while i + 1 < nums.len() {
            if let Value::Integer(idx) = nums[i] {
                if let Some(dict) = resolve(objs, &nums[i + 1]) {
                    let style = match dict_get(&dict, b"S") {
                        Some(Value::Name(n)) => Some(String::from_utf8_lossy(&n).into_owned()),
                        _ => None,
                    };
                    let starting = match dict_get(&dict, b"St") {
                        Some(Value::Integer(n)) => n as i32,
                        _ => 1,
                    };
                    let prefix = match dict_get(&dict, b"P") {
                        Some(Value::String(b)) => String::from_utf8_lossy(&b).into_owned(),
                        _ => String::new(),
                    };
                    out.push(LabelData { index: idx as i32, style, starting, prefix });
                }
            }
            i += 2;
        }
    }
}

// ---- Write -----------------------------------------------------------

/// Append an incremental update writing `labels` into the catalog's
/// `/PageLabels /Nums` number tree. Returns `None` when the document
/// can't be parsed.
pub(crate) fn append(bytes: &[u8], labels: &[LabelData]) -> Option<Vec<u8>> {
    let tr = trailer::parse(bytes).ok()?;
    let objs = build_objs(bytes);
    let mut catalog = match objs.get(&tr.root_id)? {
        Value::Dict(d) => d.clone(),
        _ => return None,
    };

    // /Nums entries in ascending index order (PDF §7.9.7).
    let mut sorted: Vec<&LabelData> = labels.iter().collect();
    sorted.sort_by_key(|l| l.index);
    let mut nums: Vec<Value> = Vec::with_capacity(sorted.len() * 2);
    for l in sorted {
        let mut dict: Vec<(Vec<u8>, Value)> = Vec::new();
        if let Some(s) = &l.style {
            dict.push((b"S".to_vec(), Value::Name(s.clone().into_bytes())));
        }
        if l.starting != 1 {
            dict.push((b"St".to_vec(), Value::Integer(l.starting as i64)));
        }
        if !l.prefix.is_empty() {
            dict.push((b"P".to_vec(), Value::String(l.prefix.clone().into_bytes())));
        }
        nums.push(Value::Integer(l.index as i64));
        nums.push(Value::Dict(dict));
    }
    let labels_value = Value::Dict(vec![(b"Nums".to_vec(), Value::Array(nums))]);
    set_entry(&mut catalog, b"PageLabels", labels_value);

    let dirty = [DirtyObject { id: tr.root_id, generation: 0, value: Value::Dict(catalog) }];
    pdf_writer_incremental::append_incremental(bytes, &dirty).ok()
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

fn set_entry(entries: &mut Vec<(Vec<u8>, Value)>, key: &[u8], value: Value) {
    if let Some(slot) = entries.iter_mut().find(|(k, _)| k.as_slice() == key) {
        slot.1 = value;
    } else {
        entries.push((key.to_vec(), value));
    }
}
