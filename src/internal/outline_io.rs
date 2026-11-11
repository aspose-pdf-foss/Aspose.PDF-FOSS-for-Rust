//! Target-side helper: read / write the catalog's `/Outlines` document
//! outline (bookmark) tree. Backs the public `Document::outlines`
//! save-through. Non-anchored (composes the trailer / xref / objects /
//! pages_tree parsers + the incremental writer).
//!
//! Each bookmark is a doubly-linked indirect object (PDF §12.3.3):
//! `/Title`, `/Parent`, `/Prev`, `/Next`, `/First`, `/Last`, `/Count`,
//! `/F` (style flags), and a target `/Dest`. The tree carries
//! destinations as their canonical `ToString` form (`"1 XYZ 100 700 2"`,
//! page number 1-based) exactly like
//! [`named_dest_io`](super::named_dest_io); the open/closed state rides
//! the sign of `/Count`. Action targets (`/A`) are a follow-on; only
//! `/Dest` (and a `/A /GoTo` destination) round-trip here.

use std::collections::HashMap;

use super::objects::{self, Value};
use super::pdf_writer_incremental::{self, DirtyObject};
use super::{pages_tree, trailer, xref};

/// One bookmark, recursively. The serialisable mirror of the public
/// outline tree.
pub(crate) struct OutlineData {
    pub title: String,
    pub bold: bool,
    pub italic: bool,
    pub open: bool,
    /// Canonical destination string, or `None` for a title-only item.
    pub dest: Option<String>,
    /// The `/A` action dictionary (§12.6), or `None` when the item
    /// carries no action. An item may have a destination OR an action,
    /// not both — §12.3.3 says /A takes precedence, so the writer
    /// prefers it.
    pub action: Option<crate::internal::annotation_io::Entry>,
    /// The `/A` dictionary read off a loaded item, verbatim, when it is
    /// not a `GoTo` — a `GoTo` becomes `dest` instead.
    ///
    /// The whole `/Outlines` tree is rewritten whenever any bookmark
    /// changes, so an item re-emitted from a model that knows only
    /// titles and destinations would lose the action it was opened
    /// with: a bookmark that ran a URI or a script would stop running
    /// anything. Indirect references inside it stay valid, since the
    /// objects they name are not rewritten.
    pub carried_action: Option<Value>,
    pub children: Vec<OutlineData>,
}

// ---- Read ------------------------------------------------------------

/// Parse the document outline tree from `bytes`. Empty when the
/// document carries no `/Outlines`.
pub(crate) fn parse(bytes: &[u8]) -> Vec<OutlineData> {
    let Ok(tr) = trailer::parse(bytes) else {
        return Vec::new();
    };
    let objs = build_objs(bytes);
    let page_num_by_id = page_numbers(bytes);

    let Some(catalog) = objs.get(&tr.root_id) else {
        return Vec::new();
    };
    let Some(outlines) = dict_get(catalog, b"Outlines").and_then(|v| resolve(&objs, &v)) else {
        return Vec::new();
    };
    let first = match dict_get(&outlines, b"First") {
        Some(Value::Ref { id, .. }) => id,
        _ => return Vec::new(),
    };
    collect_chain(&objs, first, &page_num_by_id)
}

/// Follow the `/Next` chain starting at item `id`, recursing `/First`
/// for each item's children.
fn collect_chain(
    objs: &HashMap<u32, Value>,
    id: u32,
    page_num_by_id: &HashMap<u32, u64>,
) -> Vec<OutlineData> {
    let mut out = Vec::new();
    let mut cur = Some(id);
    while let Some(cid) = cur {
        let Some(dict) = objs.get(&cid) else { break };
        let title = match dict_get(dict, b"Title") {
            Some(Value::String(b)) => decode_text_string(&b),
            _ => String::new(),
        };
        let flags = match dict_get(dict, b"F") {
            Some(Value::Integer(n)) => n,
            _ => 0,
        };
        let italic = flags & 0x1 != 0;
        let bold = flags & 0x2 != 0;
        let open = match dict_get(dict, b"Count") {
            Some(Value::Integer(n)) => n >= 0,
            _ => true,
        };
        let dest = item_destination(objs, dict, page_num_by_id);
        let children = match dict_get(dict, b"First") {
            Some(Value::Ref { id, .. }) => collect_chain(objs, id, page_num_by_id),
            _ => Vec::new(),
        };
        // A GoTo action is read as `dest` above; any other kind is not
        // reconstructed into a typed action, so it is carried verbatim
        // instead of being dropped.
        let carried_action = match dict_get(dict, b"A").and_then(|v| resolve(objs, &v)) {
            Some(a) if !matches!(dict_get(&a, b"S"), Some(Value::Name(ref n)) if n == b"GoTo") => {
                Some(a)
            }
            _ => None,
        };
        out.push(OutlineData {
            title,
            bold,
            italic,
            open,
            dest,
            action: None,
            carried_action,
            children,
        });
        cur = match dict_get(dict, b"Next") {
            Some(Value::Ref { id, .. }) => Some(id),
            _ => None,
        };
    }
    out
}

/// Resolve a bookmark's destination from `/Dest` (array / dict) or a
/// `/A` `/GoTo` action's `/D`.
fn item_destination(
    objs: &HashMap<u32, Value>,
    dict: &Value,
    page_num_by_id: &HashMap<u32, u64>,
) -> Option<String> {
    if let Some(d) = dict_get(dict, b"Dest").and_then(|v| resolve(objs, &v)) {
        if let Some(s) = array_to_dest_string(objs, &d, page_num_by_id) {
            return Some(s);
        }
    }
    if let Some(a) = dict_get(dict, b"A").and_then(|v| resolve(objs, &v)) {
        let is_goto = matches!(dict_get(&a, b"S"), Some(Value::Name(n)) if n == b"GoTo");
        if is_goto {
            if let Some(d) = dict_get(&a, b"D").and_then(|v| resolve(objs, &v)) {
                return array_to_dest_string(objs, &d, page_num_by_id);
            }
        }
    }
    None
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

/// Decode a PDF text string (§7.9.2.2): UTF-16BE when prefixed by the
/// `FE FF` byte-order mark, otherwise the bytes verbatim (PDFDocEncoding
/// is ASCII-compatible for the characters this lib writes).
fn decode_text_string(b: &[u8]) -> String {
    if b.len() >= 2 && b[0] == 0xFE && b[1] == 0xFF {
        let mut units = Vec::with_capacity((b.len() - 2) / 2);
        let mut i = 2;
        while i + 1 < b.len() {
            units.push(u16::from_be_bytes([b[i], b[i + 1]]));
            i += 2;
        }
        String::from_utf16_lossy(&units)
    } else {
        String::from_utf8_lossy(b).into_owned()
    }
}

// ---- Write -----------------------------------------------------------

/// One outline object to emit, with its resolved sibling / child links.
struct Emit<'a> {
    id: u32,
    data: &'a OutlineData,
    parent: u32,
    prev: Option<u32>,
    next: Option<u32>,
    first_child: Option<u32>,
    last_child: Option<u32>,
}

/// Append an incremental update writing `items` as the catalog's
/// `/Outlines` tree. Returns `None` when the document can't be parsed.
pub(crate) fn append(bytes: &[u8], items: &[OutlineData]) -> Option<Vec<u8>> {
    let tr = trailer::parse(bytes).ok()?;
    let objs = build_objs(bytes);
    let tree = pages_tree::parse(bytes).ok()?;
    let leaf_id_for_page = |page: usize| tree.leaves.get(page.checked_sub(1)?).map(|l| l.id);

    let mut catalog = match objs.get(&tr.root_id)? {
        Value::Dict(d) => d.clone(),
        _ => return None,
    };

    let mut next_id = tr.size as u32;
    let outlines_id = next_id;
    next_id += 1;

    // Flatten the tree, assigning ids + resolving the doubly-linked
    // sibling / child references.
    let mut emit: Vec<Emit> = Vec::new();
    let (first, last) = build_level(items, outlines_id, &mut next_id, &mut emit);

    let mut dirty: Vec<DirtyObject> = Vec::with_capacity(emit.len() + 2);
    for e in &emit {
        let mut dict: Vec<(Vec<u8>, Value)> = vec![
            (b"Title".to_vec(), Value::String(encode_text_string(&e.data.title))),
            (b"Parent".to_vec(), Value::Ref { id: e.parent, gen: 0 }),
        ];
        if let Some(p) = e.prev {
            dict.push((b"Prev".to_vec(), Value::Ref { id: p, gen: 0 }));
        }
        if let Some(n) = e.next {
            dict.push((b"Next".to_vec(), Value::Ref { id: n, gen: 0 }));
        }
        if let Some(fc) = e.first_child {
            dict.push((b"First".to_vec(), Value::Ref { id: fc, gen: 0 }));
        }
        if let Some(lc) = e.last_child {
            dict.push((b"Last".to_vec(), Value::Ref { id: lc, gen: 0 }));
        }
        if !e.data.children.is_empty() {
            let mag = visible_count(&e.data.children);
            let count = if e.data.open { mag } else { -mag };
            dict.push((b"Count".to_vec(), Value::Integer(count as i64)));
        }
        let flags = (e.data.italic as i64) | ((e.data.bold as i64) << 1);
        if flags != 0 {
            dict.push((b"F".to_vec(), Value::Integer(flags)));
        }
        // §12.3.3: an item may carry /Dest or /A, and /A wins when both
        // are present — so an item that was given an action writes the
        // action and leaves /Dest off.
        if let Some(action) = &e.data.action {
            dict.push((b"A".to_vec(), action.to_value()));
        } else if let Some(carried) = &e.data.carried_action {
            // The item was opened with an action this model does not
            // name; it goes back out as it came in.
            dict.push((b"A".to_vec(), carried.clone()));
        } else if let Some(dest) = &e.data.dest {
            if let Some(arr) = dest_string_to_array(dest, &leaf_id_for_page) {
                dict.push((b"Dest".to_vec(), Value::Array(arr)));
            }
        }
        dirty.push(DirtyObject { id: e.id, generation: 0, value: Value::Dict(dict) });
    }

    // The /Outlines root dictionary (PDF §12.3.3).
    let mut root_dict: Vec<(Vec<u8>, Value)> =
        vec![(b"Type".to_vec(), Value::Name(b"Outlines".to_vec()))];
    if let Some(f) = first {
        root_dict.push((b"First".to_vec(), Value::Ref { id: f, gen: 0 }));
    }
    if let Some(l) = last {
        root_dict.push((b"Last".to_vec(), Value::Ref { id: l, gen: 0 }));
    }
    root_dict.push((b"Count".to_vec(), Value::Integer(visible_count(items) as i64)));
    dirty.push(DirtyObject { id: outlines_id, generation: 0, value: Value::Dict(root_dict) });

    set_entry(&mut catalog, b"Outlines", Value::Ref { id: outlines_id, gen: 0 });
    dirty.push(DirtyObject { id: tr.root_id, generation: 0, value: Value::Dict(catalog) });

    pdf_writer_incremental::append_incremental(bytes, &dirty).ok()
}

/// Assign ids to one sibling level and recurse; returns the level's
/// `(first, last)` child ids for the parent's `/First` `/Last`.
fn build_level<'a>(
    siblings: &'a [OutlineData],
    parent_id: u32,
    next_id: &mut u32,
    emit: &mut Vec<Emit<'a>>,
) -> (Option<u32>, Option<u32>) {
    if siblings.is_empty() {
        return (None, None);
    }
    // Reserve every sibling's id first so /Prev /Next can resolve.
    let ids: Vec<u32> = siblings
        .iter()
        .map(|_| {
            let id = *next_id;
            *next_id += 1;
            id
        })
        .collect();
    for (i, item) in siblings.iter().enumerate() {
        let id = ids[i];
        let prev = if i > 0 { Some(ids[i - 1]) } else { None };
        let next = ids.get(i + 1).copied();
        let (first_child, last_child) = build_level(&item.children, id, next_id, emit);
        emit.push(Emit { id, data: item, parent: parent_id, prev, next, first_child, last_child });
    }
    (ids.first().copied(), ids.last().copied())
}

/// Visible-descendant count (PDF §12.3.3 `/Count` magnitude): each child
/// counts once, plus an open child's own visible descendants.
fn visible_count(items: &[OutlineData]) -> i32 {
    let mut total = 0;
    for it in items {
        total += 1;
        if it.open {
            total += visible_count(&it.children);
        }
    }
    total
}

/// Encode a title as a PDF text string: ASCII verbatim, otherwise
/// UTF-16BE with the `FE FF` byte-order mark (§7.9.2.2).
fn encode_text_string(s: &str) -> Vec<u8> {
    if s.is_ascii() {
        s.as_bytes().to_vec()
    } else {
        let mut out = vec![0xFE, 0xFF];
        for u in s.encode_utf16() {
            out.extend_from_slice(&u.to_be_bytes());
        }
        out
    }
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
