//! Target-side helper: read / write the catalog's `/Names /Dests` name
//! tree (named destinations). Reused by the public `Document` named-
//! destination save-through. Non-anchored (composes the trailer / xref /
//! objects / pages_tree parsers + the incremental writer).
//!
//! Destinations are carried as their canonical `ToString` form
//! (`"1 XYZ 100 700 2"`): page number (1-based), verb, then numeric
//! parameters. On write the page number resolves to the page leaf's
//! object reference; on read the reference resolves back to the page
//! number.

use std::collections::HashMap;

use super::objects::{self, Value};
use super::pdf_writer_incremental::{self, DirtyObject};
use super::{pages_tree, trailer, xref};

// ---- Read ------------------------------------------------------------

/// Parse the `(name, destination-string)` pairs from `bytes`, in name-
/// tree order. Empty when the document carries no named destinations.
pub(crate) fn parse(bytes: &[u8]) -> Vec<(String, String)> {
    let Ok(tr) = trailer::parse(bytes) else {
        return Vec::new();
    };
    let objs = build_objs(bytes);
    let page_num_by_id = page_numbers(bytes);

    let Some(catalog) = objs.get(&tr.root_id) else {
        return Vec::new();
    };
    let Some(names) = dict_get(catalog, b"Names").and_then(|v| resolve(&objs, &v)) else {
        return Vec::new();
    };
    let Some(dests) = dict_get(&names, b"Dests").and_then(|v| resolve(&objs, &v)) else {
        return Vec::new();
    };

    let mut out = Vec::new();
    collect_tree(&objs, &dests, &page_num_by_id, &mut out);
    out
}

fn collect_tree(
    objs: &HashMap<u32, Value>,
    node: &Value,
    page_num_by_id: &HashMap<u32, u64>,
    out: &mut Vec<(String, String)>,
) {
    if let Some(Value::Array(kids)) = dict_get(node, b"Kids") {
        for kid in &kids {
            if let Some(resolved) = resolve(objs, kid) {
                collect_tree(objs, &resolved, page_num_by_id, out);
            }
        }
    }
    if let Some(Value::Array(names)) = dict_get(node, b"Names") {
        let mut i = 0;
        while i + 1 < names.len() {
            if let Value::String(key) = &names[i] {
                if let Some(arr) = resolve(objs, &names[i + 1]) {
                    if let Some(s) = array_to_dest_string(objs, &arr, page_num_by_id) {
                        out.push((String::from_utf8_lossy(key).into_owned(), s));
                    }
                }
            }
            i += 2;
        }
    }
}

/// A destination may be the array directly, or a dict `<< /D [...] >>`.
fn array_to_dest_string(
    objs: &HashMap<u32, Value>,
    dest: &Value,
    page_num_by_id: &HashMap<u32, u64>,
) -> Option<String> {
    let arr = match dest {
        Value::Array(a) => a.clone(),
        Value::Dict(_) => match dict_get(dest, b"D").and_then(|v| resolve(objs, &v)) {
            Some(Value::Array(a)) => a,
            _ => return None,
        },
        _ => return None,
    };
    if arr.len() < 2 {
        return None;
    }
    let page = match &arr[0] {
        Value::Ref { id, .. } => *page_num_by_id.get(id)? as i64,
        Value::Integer(n) => *n + 1, // explicit 0-based page index
        _ => return None,
    };
    let verb = match &arr[1] {
        Value::Name(n) => String::from_utf8_lossy(n).into_owned(),
        _ => return None,
    };
    let mut s = format!("{page} {verb}");
    for p in &arr[2..] {
        s.push(' ');
        s.push_str(&number_token(p));
    }
    Some(s)
}

fn number_token(v: &Value) -> String {
    match v {
        Value::Integer(n) => n.to_string(),
        Value::Real(r) => format!("{r}"),
        Value::Null => "null".to_string(),
        _ => "0".to_string(),
    }
}

// ---- Write -----------------------------------------------------------

/// Append an incremental update writing `dests` (name → destination
/// string) into the catalog's `/Names /Dests` name tree. Returns `None`
/// when the document can't be parsed.
pub(crate) fn append(bytes: &[u8], dests: &[(String, String)]) -> Option<Vec<u8>> {
    let tr = trailer::parse(bytes).ok()?;
    let objs = build_objs(bytes);
    let tree = pages_tree::parse(bytes).ok()?;
    let leaf_id_for_page = |page: usize| tree.leaves.get(page.checked_sub(1)?).map(|l| l.id);

    let mut catalog = match objs.get(&tr.root_id)? {
        Value::Dict(d) => d.clone(),
        _ => return None,
    };

    // Sorted name-tree leaf: /Names [(key) [array] ...] with keys in
    // lexicographic order (PDF §7.9.6).
    let mut sorted: Vec<&(String, String)> = dests.iter().collect();
    sorted.sort_by(|a, b| a.0.cmp(&b.0));
    let mut names_array: Vec<Value> = Vec::with_capacity(sorted.len() * 2);
    for (name, dest_str) in sorted {
        let arr = dest_string_to_array(dest_str, &leaf_id_for_page)?;
        names_array.push(Value::String(name.clone().into_bytes()));
        names_array.push(Value::Array(arr));
    }
    let dest_tree = Value::Dict(vec![(b"Names".to_vec(), Value::Array(names_array))]);
    let names_dict = Value::Dict(vec![(b"Dests".to_vec(), dest_tree)]);
    set_entry(&mut catalog, b"Names", names_dict);

    let dirty = [DirtyObject { id: tr.root_id, generation: 0, value: Value::Dict(catalog) }];
    pdf_writer_incremental::append_incremental(bytes, &dirty).ok()
}

fn dest_string_to_array(
    dest_str: &str,
    leaf_id_for_page: &impl Fn(usize) -> Option<u32>,
) -> Option<Vec<Value>> {
    let mut toks = dest_str.split_whitespace();
    let page: usize = toks.next()?.parse().ok()?;
    let verb = toks.next()?;
    let leaf_id = leaf_id_for_page(page)?;
    let mut arr = vec![
        Value::Ref { id: leaf_id, gen: 0 },
        Value::Name(verb.as_bytes().to_vec()),
    ];
    for t in toks {
        arr.push(parse_number(t));
    }
    Some(arr)
}

fn parse_number(t: &str) -> Value {
    if t == "null" {
        Value::Null
    } else if let Ok(n) = t.parse::<i64>() {
        Value::Integer(n)
    } else if let Ok(r) = t.parse::<f64>() {
        Value::Real(r)
    } else {
        Value::Null
    }
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

fn page_numbers(bytes: &[u8]) -> HashMap<u32, u64> {
    let mut m = HashMap::new();
    if let Ok(tree) = pages_tree::parse(bytes) {
        for leaf in &tree.leaves {
            m.insert(leaf.id, leaf.pagenum);
        }
    }
    m
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
