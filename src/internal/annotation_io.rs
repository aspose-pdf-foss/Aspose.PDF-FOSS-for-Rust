//! Target-side helper: read / write a page's `/Annots` array. Backs the
//! public `Page::annotations` save-through. Non-anchored (composes the
//! trailer / xref / objects parsers + the incremental writer).
//!
//! Annotations are carried as plain data (`/Subtype` name, `/Rect`,
//! `/Contents`, `/NM` name) — the public layer maps `/Subtype` to and
//! from [`AnnotationType`](crate::public::annotation_type::AnnotationType).

use std::collections::HashMap;

use super::objects::{self, Value};
use super::pdf_writer_incremental::{self, DirtyObject};
use super::{trailer, xref};

/// One annotation's serialisable fields.
pub(crate) struct AnnotData {
    pub subtype: String,
    pub rect: [f64; 4],
    pub contents: String,
    pub name: String,
    /// `/MN` — which printer mark this is (§12.5.6.21), empty for every
    /// other subtype. Five printer marks share one `/Subtype`, so this
    /// is the only thing that tells them apart. Read-only: on the way
    /// out `/MN` travels in `entries` like every other written entry.
    pub marks_name: String,
    /// Everything else the annotation dictionary carries (§12.5.2 for
    /// the common entries, §12.5.6 for the per-subtype ones). The
    /// public side names them; this module only writes them, so a new
    /// annotation property needs no change here.
    pub entries: Vec<(&'static str, Entry)>,
    /// Entries read off a loaded annotation that the model does not
    /// name, kept verbatim so a page rewrite does not strip them.
    ///
    /// A page's `/Annots` is written whole, so every annotation on a
    /// dirty page is re-emitted from the model — and a model that knows
    /// four keys would re-emit four keys. These are the rest: an
    /// appearance stream reference, a popup, a reply chain, whatever
    /// the producer wrote. Indirect references stay valid because the
    /// objects they point at are not rewritten.
    pub carried: Vec<(Vec<u8>, Value)>,
}

/// The keys the model owns and therefore writes itself. Anything else
/// on a loaded annotation is carried through untouched.
const MODELLED_KEYS: &[&[u8]] = &[b"Type", b"Subtype", b"Rect", b"P", b"Contents", b"NM"];

/// The value shapes an annotation entry can take. Kept on this side of
/// the boundary because the writer's vocabulary is what it is: the
/// public surface fills these in, and foundation code never reaches
/// back into the public API to learn about them.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Entry {
    /// An integer — `/F` flags.
    Int(i64),
    /// A real — `/CA` opacity.
    Real(f64),
    /// A boolean — `/Cap`, whether a line draws its caption.
    Bool(bool),
    /// A single name — `/RT` reply type.
    Name(String),
    /// A literal string — `/OverlayText`.
    Text(String),
    /// A flat number array — `/C`, `/L`, `/QuadPoints`, `/Vertices`.
    Reals(Vec<f64>),
    /// An array of number arrays — `/InkList`, one entry per stroke.
    RealGroups(Vec<Vec<f64>>),
    /// A two-name array — `/LE` carries the start and end styles.
    Names(Vec<String>),
    /// A nested dictionary — `/BS` carries its width and style inside
    /// one (§12.5.4).
    Dict(Vec<(&'static str, Entry)>),
    /// An array of anything — `/Opt` holds a text string or a
    /// two-string array per option (§12.7.4.4), `/I` holds integers.
    Array(Vec<Entry>),
}

impl Entry {
    pub(crate) fn to_value(&self) -> Value {
        match self {
            Entry::Int(n) => Value::Integer(*n),
            Entry::Real(x) => Value::Real(*x),
            Entry::Bool(b) => Value::Bool(*b),
            Entry::Name(s) => Value::Name(s.clone().into_bytes()),
            Entry::Text(s) => Value::String(s.clone().into_bytes()),
            Entry::Reals(v) => Value::Array(v.iter().map(|x| Value::Real(*x)).collect()),
            Entry::RealGroups(groups) => Value::Array(
                groups
                    .iter()
                    .map(|g| Value::Array(g.iter().map(|x| Value::Real(*x)).collect()))
                    .collect(),
            ),
            Entry::Names(v) => Value::Array(
                v.iter().map(|s| Value::Name(s.clone().into_bytes())).collect(),
            ),
            Entry::Array(items) => Value::Array(items.iter().map(|e| e.to_value()).collect()),
            Entry::Dict(entries) => Value::Dict(
                entries
                    .iter()
                    .map(|(k, v)| (k.as_bytes().to_vec(), v.to_value()))
                    .collect(),
            ),
        }
    }
}

// ---- Read ------------------------------------------------------------

/// Read the annotations on the page object `leaf_id`.
pub(crate) fn parse_page_annots(bytes: &[u8], leaf_id: u32) -> Vec<AnnotData> {
    let objs = build_objs(bytes);
    let Some(page) = objs.get(&leaf_id) else {
        return Vec::new();
    };
    let Some(Value::Array(annots)) = dict_get(page, b"Annots") else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for a in &annots {
        let Some(dict) = resolve(&objs, a) else { continue };
        let subtype = match dict_get(&dict, b"Subtype") {
            Some(Value::Name(n)) => String::from_utf8_lossy(&n).into_owned(),
            _ => continue,
        };
        let rect = match dict_get(&dict, b"Rect") {
            Some(Value::Array(r)) if r.len() == 4 => {
                [num(&r[0]), num(&r[1]), num(&r[2]), num(&r[3])]
            }
            _ => [0.0; 4],
        };
        let contents = string_field(&dict, b"Contents");
        let name = string_field(&dict, b"NM");
        let carried: Vec<(Vec<u8>, Value)> = match &dict {
            Value::Dict(entries) => entries
                .iter()
                .filter(|(key, _)| !MODELLED_KEYS.contains(&key.as_slice()))
                .cloned()
                .collect(),
            _ => Vec::new(),
        };
        let marks_name = match dict_get(&dict, b"MN") {
            Some(Value::Name(n)) => String::from_utf8_lossy(&n).into_owned(),
            _ => String::new(),
        };
        out.push(AnnotData {
            subtype,
            rect,
            contents,
            name,
            marks_name,
            entries: Vec::new(),
            carried,
        });
    }
    out
}

// ---- Write -----------------------------------------------------------

/// Append an incremental update rewriting the `/Annots` of each given
/// page (`leaf_id`) to the supplied annotation list. Returns `None` when
/// the document can't be parsed.
pub(crate) fn append(bytes: &[u8], pages: &[(u32, Vec<AnnotData>)]) -> Option<Vec<u8>> {
    let tr = trailer::parse(bytes).ok()?;
    let objs = build_objs(bytes);
    let mut next_id = tr.size as u32;
    let mut dirty: Vec<DirtyObject> = Vec::new();

    for (leaf_id, annots) in pages {
        let mut page_dict = match objs.get(leaf_id) {
            Some(Value::Dict(d)) => d.clone(),
            _ => continue,
        };
        let mut refs: Vec<Value> = Vec::with_capacity(annots.len());
        for a in annots {
            let id = next_id;
            next_id += 1;
            let mut dict: Vec<(Vec<u8>, Value)> = vec![
                (b"Type".to_vec(), Value::Name(b"Annot".to_vec())),
                (b"Subtype".to_vec(), Value::Name(a.subtype.clone().into_bytes())),
                (b"Rect".to_vec(), Value::Array(a.rect.iter().map(|c| Value::Real(*c)).collect())),
                (b"P".to_vec(), Value::Ref { id: *leaf_id, gen: 0 }),
            ];
            if !a.contents.is_empty() {
                dict.push((b"Contents".to_vec(), Value::String(a.contents.clone().into_bytes())));
            }
            if !a.name.is_empty() {
                dict.push((b"NM".to_vec(), Value::String(a.name.clone().into_bytes())));
            }
            for (key, entry) in &a.entries {
                dict.push((key.as_bytes().to_vec(), entry.to_value()));
            }
            // What the model knows wins; the rest of the original
            // dictionary follows, so a loaded annotation keeps the
            // entries this crate cannot name. A key already written is
            // not written twice — that would be an invalid dictionary.
            for (key, value) in &a.carried {
                if !dict.iter().any(|(written, _)| written == key) {
                    dict.push((key.clone(), value.clone()));
                }
            }
            dirty.push(DirtyObject { id, generation: 0, value: Value::Dict(dict) });
            refs.push(Value::Ref { id, gen: 0 });
        }
        set_entry(&mut page_dict, b"Annots", Value::Array(refs));
        dirty.push(DirtyObject { id: *leaf_id, generation: 0, value: Value::Dict(page_dict) });
    }

    if dirty.is_empty() {
        return Some(bytes.to_vec());
    }
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

fn num(v: &Value) -> f64 {
    match v {
        Value::Integer(n) => *n as f64,
        Value::Real(r) => *r,
        _ => 0.0,
    }
}

fn string_field(v: &Value, key: &[u8]) -> String {
    match dict_get(v, key) {
        Some(Value::String(b)) => String::from_utf8_lossy(&b).into_owned(),
        _ => String::new(),
    }
}
