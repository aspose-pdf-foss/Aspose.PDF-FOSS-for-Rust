//! Target-side helper: rewrite the `/Pages` tree to a new page order.
//! Backs the public `PageCollection` mutation save-through (Add / Insert /
//! Delete). Non-anchored (composes the trailer / xref / objects parsers +
//! the incremental writer).
//!
//! On save, the public layer hands a flat list of [`PageSpec`]s — one per
//! page in the current order. Each spec is either a *reference* to an
//! existing leaf object (a loaded or copied page, addressed by its source
//! `leaf_id`) or a *blank* page to synthesise. This module emits:
//!
//! * a fresh `/Type /Page` leaf object for every blank page (empty
//!   `/Contents`, empty `/Resources`, `/MediaBox`, `/Parent` the root);
//! * a rewritten `/Pages` root dict whose `/Kids` is the new ordered array
//!   of leaf references and whose `/Count` is the new page total. The root
//!   is FLATTENED — any intermediate `/Pages` nodes are bypassed, every
//!   leaf hangs directly off the root. (`/Parent` on reused leaves is
//!   re-pointed at the root.)
//!
//! All emitted objects go out as one incremental update (PDF
//! §7.5.6) over the working bytes.

use std::collections::HashMap;

use super::objects::{self, Value};
use super::pdf_writer_incremental::{self, DirtyObject};
use super::{trailer, xref};

/// One page in the new order.
pub(crate) enum PageSpec {
    /// An existing leaf object (loaded or copied page). Reuses its content
    /// / resources by reference; the save path re-points its `/Parent` at
    /// the rewritten root. `leaf_id` is the source object id.
    Reference { leaf_id: u32 },
    /// A blank page to synthesise (empty content + resources).
    Blank { media_box: [f64; 4] },
}

/// Rewrite the `/Pages` tree of `bytes` to the ordered `specs`. Returns
/// the new bytes plus the leaf object id emitted for each spec, in spec
/// order (`0` where a spec emitted nothing — a reference to a leaf the
/// source doesn't carry). The caller stamps the ids of synthesised pages
/// back onto their page handles, so edits recorded against a page created
/// in-session resolve to a real object. `None` when the document can't be
/// parsed (caller keeps the prior bytes).
pub(crate) fn append(bytes: &[u8], specs: &[PageSpec]) -> Option<(Vec<u8>, Vec<u32>)> {
    let tr = trailer::parse(bytes).ok()?;
    let objs = build_objs(bytes);

    // Resolve the /Pages root object id via the catalog.
    let catalog = objs.get(&tr.root_id)?;
    let pages_root_id = match dict_get(catalog, b"Pages")? {
        Value::Ref { id, .. } => id,
        _ => return None,
    };
    let mut root_dict = match objs.get(&pages_root_id) {
        Some(Value::Dict(d)) => d.clone(),
        _ => return None,
    };

    let mut next_id = tr.size as u32;
    let mut dirty: Vec<DirtyObject> = Vec::new();
    let mut kids: Vec<Value> = Vec::with_capacity(specs.len());
    let mut emitted: Vec<u32> = Vec::with_capacity(specs.len());

    for spec in specs {
        match spec {
            PageSpec::Reference { leaf_id } => {
                // Re-point the reused leaf's /Parent at the (flattened)
                // root and re-emit it so the reference resolves under the
                // new tree. Copied pages share a source leaf_id; emitting
                // the same id twice would collide, so a leaf already queued
                // is cloned into a fresh object that copies its /Contents +
                // /Resources by reference.
                let already = kids.iter().any(|k| matches!(k, Value::Ref { id, .. } if *id == *leaf_id));
                if already {
                    let mut copied = 0;
                    if let Some(Value::Dict(src)) = objs.get(leaf_id) {
                        let id = next_id;
                        next_id += 1;
                        let mut leaf = clone_leaf_shell(src);
                        set_entry(&mut leaf, b"Parent", Value::Ref { id: pages_root_id, gen: 0 });
                        dirty.push(DirtyObject { id, generation: 0, value: Value::Dict(leaf) });
                        kids.push(Value::Ref { id, gen: 0 });
                        copied = id;
                    }
                    emitted.push(copied);
                    continue;
                }
                emitted.push(match objs.get(leaf_id) {
                    Some(Value::Dict(_)) => *leaf_id,
                    _ => 0,
                });
                if let Some(Value::Dict(src)) = objs.get(leaf_id) {
                    let mut leaf = src.clone();
                    set_entry(&mut leaf, b"Parent", Value::Ref { id: pages_root_id, gen: 0 });
                    dirty.push(DirtyObject {
                        id: *leaf_id,
                        generation: 0,
                        value: Value::Dict(leaf),
                    });
                    kids.push(Value::Ref { id: *leaf_id, gen: 0 });
                }
            }
            PageSpec::Blank { media_box } => {
                let id = next_id;
                next_id += 1;
                let leaf = vec![
                    (b"Type".to_vec(), Value::Name(b"Page".to_vec())),
                    (b"Parent".to_vec(), Value::Ref { id: pages_root_id, gen: 0 }),
                    (
                        b"MediaBox".to_vec(),
                        Value::Array(media_box.iter().map(|c| Value::Real(*c)).collect()),
                    ),
                    (b"Resources".to_vec(), Value::Dict(Vec::new())),
                ];
                dirty.push(DirtyObject { id, generation: 0, value: Value::Dict(leaf) });
                kids.push(Value::Ref { id, gen: 0 });
                emitted.push(id);
            }
        }
    }

    set_entry(&mut root_dict, b"Kids", Value::Array(kids.clone()));
    set_entry(&mut root_dict, b"Count", Value::Integer(kids.len() as i64));
    dirty.push(DirtyObject {
        id: pages_root_id,
        generation: 0,
        value: Value::Dict(root_dict),
    });

    let out = pdf_writer_incremental::append_incremental(bytes, &dirty).ok()?;
    Some((out, emitted))
}

/// Build a fresh leaf dict that reuses `src`'s `/Contents`, `/Resources`,
/// `/MediaBox`, `/CropBox`, `/Rotate` (by reference / value) — enough to
/// render the same page — without duplicating the whole original dict.
fn clone_leaf_shell(src: &[(Vec<u8>, Value)]) -> Vec<(Vec<u8>, Value)> {
    let mut leaf: Vec<(Vec<u8>, Value)> =
        vec![(b"Type".to_vec(), Value::Name(b"Page".to_vec()))];
    for key in [
        b"Contents".as_slice(),
        b"Resources".as_slice(),
        b"MediaBox".as_slice(),
        b"CropBox".as_slice(),
        b"Rotate".as_slice(),
    ] {
        if let Some((_, v)) = src.iter().find(|(k, _)| k.as_slice() == key) {
            leaf.push((key.to_vec(), v.clone()));
        }
    }
    leaf
}

// ---- shared helpers --------------------------------------------------

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
