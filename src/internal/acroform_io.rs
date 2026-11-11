//! Target-side helper: read / write the catalog's `/AcroForm /Fields`
//! interactive form. Backs the public `Document::form` save-through.
//! Non-anchored (composes the trailer / xref / objects / pages_tree
//! parsers + the incremental writer).
//!
//! Each field is written as a merged field / widget object (PDF
//! §12.7.3.1): a single dictionary carrying both the field entries
//! (`/FT`, `/T`, `/TU`, `/TM`, `/V`, `/Ff`) and the widget annotation entries
//! (`/Subtype /Widget`, `/Rect`, `/P`). The object is referenced from
//! `/AcroForm /Fields` and from its page's `/Annots`. `/V` is written as
//! a text string (correct for `/Tx`; button / choice values round-trip
//! their text). `/NeedAppearances true` asks the viewer to regenerate
//! field appearances from `/DA`.

use std::collections::HashMap;

use super::objects::{self, Value};
use super::pdf_writer_incremental::{self, DirtyObject};
use super::{pages_tree, trailer, xref};

/// One field parsed from `/AcroForm /Fields`.
use crate::internal::annotation_io::Entry;

pub(crate) struct FieldData {
    pub ft: String,
    pub partial_name: String,
    /// `/TU` — the name shown to the user, empty when absent.
    pub alternate_name: String,
    /// `/TM` — the name used when the form is exported, empty when
    /// absent.
    pub mapping_name: String,
    pub value: String,
    pub rect: [f64; 4],
    /// 1-based page number the widget sits on, or 0 when unknown.
    pub page_number: i32,
    pub read_only: bool,
    pub required: bool,
    pub exportable: bool,
    /// The `/Ff` bits beyond the three of Table 226, so a field loaded
    /// from a document keeps the ones this crate's model does not name
    /// when the form is written back.
    pub other_flags: i64,
}

/// One field to write. `page_leaf_id` is the page object the widget is
/// attached to.
pub(crate) struct FieldSpec {
    pub ft: String,
    pub partial_name: String,
    pub alternate_name: String,
    pub mapping_name: String,
    pub value: String,
    pub rect: [f64; 4],
    pub page_leaf_id: u32,
    pub read_only: bool,
    pub required: bool,
    /// Whether the field's value is exported with the form — NoExport
    /// is the third bit of §12.7.4 Table 226.
    pub exportable: bool,
    /// The annotation `/F` flags (§12.5.3). `Print` is what a form
    /// field wants by default — a field a viewer will not print is
    /// unusual — but a caller who says otherwise is obeyed.
    pub appearance_flags: i64,
    /// The `/Ff` bits the field type contributes — multiline, combo,
    /// radio and the rest of §12.7.4. ReadOnly and Required come from
    /// the two flags above.
    pub flags: i64,
    /// Entries beyond the common ones: `/MaxLen`, `/Q`.
    pub entries: Vec<(&'static str, Entry)>,
}

// ---- Read ------------------------------------------------------------

/// Parse the form fields from `bytes`, in `/Fields` order.
pub(crate) fn parse(bytes: &[u8]) -> Vec<FieldData> {
    let Ok(tr) = trailer::parse(bytes) else {
        return Vec::new();
    };
    let objs = build_objs(bytes);
    let page_num_by_id = page_numbers(bytes);

    let Some(catalog) = objs.get(&tr.root_id) else {
        return Vec::new();
    };
    let Some(acroform) = dict_get(catalog, b"AcroForm").and_then(|v| resolve(&objs, &v)) else {
        return Vec::new();
    };
    let Some(Value::Array(fields)) = dict_get(&acroform, b"Fields") else {
        return Vec::new();
    };

    let mut out = Vec::new();
    for f in &fields {
        let Some(dict) = resolve(&objs, f) else { continue };
        let ft = match dict_get(&dict, b"FT") {
            Some(Value::Name(n)) => String::from_utf8_lossy(&n).into_owned(),
            _ => continue,
        };
        let partial_name = string_or_name(&dict, b"T");
        let alternate_name = string_or_name(&dict, b"TU");
        let mapping_name = string_or_name(&dict, b"TM");
        let value = string_or_name(&dict, b"V");
        let rect = match dict_get(&dict, b"Rect") {
            Some(Value::Array(r)) if r.len() == 4 => [num(&r[0]), num(&r[1]), num(&r[2]), num(&r[3])],
            _ => [0.0; 4],
        };
        let page_number = match dict_get(&dict, b"P") {
            Some(Value::Ref { id, .. }) => *page_num_by_id.get(&id).unwrap_or(&0) as i32,
            _ => 0,
        };
        let flags = match dict_get(&dict, b"Ff") {
            Some(Value::Integer(n)) => n,
            _ => 0,
        };
        out.push(FieldData {
            ft,
            partial_name,
            alternate_name,
            mapping_name,
            value,
            rect,
            page_number,
            read_only: flags & 0x1 != 0,
            required: flags & 0x2 != 0,
            exportable: flags & 0x4 == 0,
            other_flags: flags & !0x7,
        });
    }
    out
}

// ---- Write -----------------------------------------------------------

/// Append an incremental update writing `fields` as the catalog's
/// `/AcroForm /Fields`, attaching each widget to its page's `/Annots`.
/// Returns `None` when the document can't be parsed.
pub(crate) fn append(bytes: &[u8], fields: &[FieldSpec]) -> Option<Vec<u8>> {
    let tr = trailer::parse(bytes).ok()?;
    let objs = build_objs(bytes);
    let mut catalog = match objs.get(&tr.root_id)? {
        Value::Dict(d) => d.clone(),
        _ => return None,
    };

    let mut next_id = tr.size as u32;
    let acroform_id = next_id;
    next_id += 1;

    let mut dirty: Vec<DirtyObject> = Vec::new();
    let mut field_refs: Vec<Value> = Vec::with_capacity(fields.len());
    // Widget refs to attach, grouped by page object.
    let mut by_page: HashMap<u32, Vec<Value>> = HashMap::new();

    for spec in fields {
        let id = next_id;
        next_id += 1;

        let flags = (spec.read_only as i64)
            | ((spec.required as i64) << 1)
            | ((!spec.exportable as i64) << 2)
            | spec.flags;
        let mut dict: Vec<(Vec<u8>, Value)> = vec![
            (b"Type".to_vec(), Value::Name(b"Annot".to_vec())),
            (b"Subtype".to_vec(), Value::Name(b"Widget".to_vec())),
            (b"FT".to_vec(), Value::Name(spec.ft.clone().into_bytes())),
            (b"T".to_vec(), Value::String(spec.partial_name.clone().into_bytes())),
            (
                b"Rect".to_vec(),
                Value::Array(spec.rect.iter().map(|c| Value::Real(*c)).collect()),
            ),
            (b"P".to_vec(), Value::Ref { id: spec.page_leaf_id, gen: 0 }),
            (b"F".to_vec(), Value::Integer(spec.appearance_flags)),
        ];
        // §12.7.4.1 Table 220: /TU is what a viewer shows the user —
        // a tooltip, or the label a screen reader reads — and /TM the
        // name the field exports under. Each is written when given.
        if !spec.alternate_name.is_empty() {
            dict.push((b"TU".to_vec(), Value::String(spec.alternate_name.clone().into_bytes())));
        }
        if !spec.mapping_name.is_empty() {
            dict.push((b"TM".to_vec(), Value::String(spec.mapping_name.clone().into_bytes())));
        }
        if !spec.value.is_empty() {
            // §12.7.4.2.3: a button field's value is the NAME of its
            // state — /Yes, /Off, an export value — while a text or
            // choice field's is a string. Writing a name as a string
            // leaves a checkbox reading as unchecked in every viewer.
            let value = if spec.ft == "Btn" {
                Value::Name(spec.value.clone().into_bytes())
            } else {
                Value::String(spec.value.clone().into_bytes())
            };
            dict.push((b"V".to_vec(), value));
        }
        if flags != 0 {
            dict.push((b"Ff".to_vec(), Value::Integer(flags)));
        }
        for (key, entry) in &spec.entries {
            dict.push((key.as_bytes().to_vec(), entry.to_value()));
        }
        dirty.push(DirtyObject { id, generation: 0, value: Value::Dict(dict) });
        field_refs.push(Value::Ref { id, gen: 0 });
        by_page.entry(spec.page_leaf_id).or_default().push(Value::Ref { id, gen: 0 });
    }

    // The /AcroForm dictionary.
    let acroform = Value::Dict(vec![
        (b"Fields".to_vec(), Value::Array(field_refs)),
        (b"NeedAppearances".to_vec(), Value::Bool(true)),
        (b"DA".to_vec(), Value::String(b"/Helv 0 Tf 0 g".to_vec())),
    ]);
    dirty.push(DirtyObject { id: acroform_id, generation: 0, value: acroform });

    set_entry(&mut catalog, b"AcroForm", Value::Ref { id: acroform_id, gen: 0 });
    dirty.push(DirtyObject { id: tr.root_id, generation: 0, value: Value::Dict(catalog) });

    // Attach each widget to its page's /Annots (preserving existing).
    for (leaf_id, widgets) in by_page {
        let mut page_dict = match objs.get(&leaf_id) {
            Some(Value::Dict(d)) => d.clone(),
            _ => continue,
        };
        let mut annots = match dict_get(&Value::Dict(page_dict.clone()), b"Annots")
            .and_then(|v| resolve(&objs, &v))
        {
            Some(Value::Array(a)) => a,
            _ => Vec::new(),
        };
        annots.extend(widgets);
        set_entry(&mut page_dict, b"Annots", Value::Array(annots));
        dirty.push(DirtyObject { id: leaf_id, generation: 0, value: Value::Dict(page_dict) });
    }

    pdf_writer_incremental::append_incremental(bytes, &dirty).ok()
}

/// Append an incremental update that removes the catalog's
/// `/AcroForm`, so the document carries no interactive form at all
/// (§12.7.2: the form IS that entry). Returns `None` when the document
/// cannot be parsed, or when there was no form to remove.
pub(crate) fn remove(bytes: &[u8]) -> Option<Vec<u8>> {
    let tr = trailer::parse(bytes).ok()?;
    let objs = build_objs(bytes);
    let mut catalog = match objs.get(&tr.root_id)? {
        Value::Dict(d) => d.clone(),
        _ => return None,
    };
    let before = catalog.len();
    catalog.retain(|(key, _)| key.as_slice() != b"AcroForm");
    if catalog.len() == before {
        return None;
    }
    let dirty = vec![DirtyObject { id: tr.root_id, generation: 0, value: Value::Dict(catalog) }];
    pdf_writer_incremental::append_incremental(bytes, &dirty).ok()
}

/// Resolve a 1-based page number to its page-object id.
pub(crate) fn page_leaf_for_number(bytes: &[u8], page_number: i32) -> Option<u32> {
    let tree = pages_tree::parse(bytes).ok()?;
    let idx = if page_number >= 1 { page_number as usize - 1 } else { 0 };
    tree.leaves.get(idx).map(|l| l.id)
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

fn string_or_name(v: &Value, key: &[u8]) -> String {
    match dict_get(v, key) {
        Some(Value::String(b)) => String::from_utf8_lossy(&b).into_owned(),
        Some(Value::Name(b)) => String::from_utf8_lossy(&b).into_owned(),
        _ => String::new(),
    }
}

fn num(v: &Value) -> f64 {
    match v {
        Value::Integer(n) => *n as f64,
        Value::Real(r) => *r,
        _ => 0.0,
    }
}
