// PDF page-tree assembler, per `page_tree_assembler.yaml` and the Rust
// overlay beside it. Builds a new PDF whose /Pages tree carries a
// selection of pages drawn from one or more source PDFs, used by the
// PdfFileEditor facade (concatenate / extract / delete / append /
// split).
//
// Two deltas from the anchor, both recorded in the overlay: the
// `Compact` sweep is not implemented (its consumers, Optimize /
// OptimizeResources, are outside the shipped subset), and page
// selection is by document-order index rather than by page object id.
//
// Strategy: parse each source's in-use objects (via xref +
// objects::parse_object) and its page order (via pages_tree).
// Renumber every source's object ids onto a disjoint contiguous
// range (ids 1 / 2 are reserved for the new catalog / pages node),
// rewriting every Ref. Emit catalog + pages(Kids = selected pages,
// each re-parented) + the renumbered objects (gen 0 assumed — the
// classical fixtures are gen-0), with a byte-offset xref. Unselected
// pages become harmless orphans (readers ignore unreferenced
// objects); the page count is exact.

use super::objects::{self, Value};
use super::xref::{self, Entry};
use std::collections::BTreeMap;

/// A parsed source PDF: its in-use objects (by id, gen 0 assumed)
/// and its page object ids in /Pages-tree order.
pub(crate) struct ParsedDoc {
    objs: BTreeMap<u32, Value>,
    pages: Vec<u32>,
}

/// Parse a source PDF's objects + page order.
pub(crate) fn parse(bytes: &[u8]) -> Option<ParsedDoc> {
    let table = xref::parse(bytes).ok()?;
    let mut objs = BTreeMap::new();
    for entry in &table.entries {
        if let Entry::InUse { id, offset, .. } = *entry {
            if let Ok(obj) = objects::parse_object(bytes, offset as usize) {
                objs.insert(id, obj.value);
            }
        }
    }
    let tree = super::pages_tree::parse(bytes).ok()?;
    let pages = tree.leaves.iter().map(|l| l.id).collect();
    Some(ParsedDoc { objs, pages })
}

/// The number of pages in a parsed source.
pub(crate) fn page_count(doc: &ParsedDoc) -> usize {
    doc.pages.len()
}

/// Assemble a new PDF from `selection` — `(doc_index, page_index)`
/// pairs (0-based) drawn from `docs`, in output order.
pub(crate) fn assemble(docs: &[ParsedDoc], selection: &[(usize, usize)]) -> Vec<u8> {
    // Per-doc id offset so all sources occupy disjoint ranges above
    // the reserved ids 1 (catalog) and 2 (pages).
    let mut offsets = Vec::with_capacity(docs.len());
    let mut next = 3u32;
    for d in docs {
        offsets.push(next - 1); // old id 1 → `next`
        let max_id = d.objs.keys().copied().max().unwrap_or(0);
        next += max_id;
    }

    // Renumber every object of every doc, rewriting Refs, and skip
    // the structural Catalog / Pages nodes (replaced by ours).
    let mut out_objs: BTreeMap<u32, Value> = BTreeMap::new();
    let mut page_new_ids: Vec<Vec<u32>> = Vec::with_capacity(docs.len());
    for (di, d) in docs.iter().enumerate() {
        let off = offsets[di];
        for (&old_id, value) in &d.objs {
            if is_structural(value) {
                continue;
            }
            out_objs.insert(old_id + off, renumber(value, off));
        }
        page_new_ids.push(d.pages.iter().map(|p| p + off).collect());
    }

    // Selected page ids, in output order; re-parent each to id 2.
    let mut kids: Vec<u32> = Vec::with_capacity(selection.len());
    for &(di, pi) in selection {
        if let Some(new_id) = page_new_ids.get(di).and_then(|v| v.get(pi)).copied() {
            if let Some(Value::Dict(dict)) = out_objs.get_mut(&new_id) {
                set_entry(dict, b"Parent", Value::Ref { id: 2, gen: 0 });
            }
            kids.push(new_id);
        }
    }

    serialize(&out_objs, &kids)
}

fn is_structural(value: &Value) -> bool {
    if let Value::Dict(dict) = value {
        if let Some(Value::Name(t)) = lookup(dict, b"Type") {
            return t == b"Catalog" || t == b"Pages";
        }
    }
    false
}

fn lookup<'a>(dict: &'a [(Vec<u8>, Value)], key: &[u8]) -> Option<&'a Value> {
    dict.iter().find(|(k, _)| k == key).map(|(_, v)| v)
}

fn set_entry(dict: &mut Vec<(Vec<u8>, Value)>, key: &[u8], value: Value) {
    if let Some(slot) = dict.iter_mut().find(|(k, _)| k == key) {
        slot.1 = value;
    } else {
        dict.push((key.to_vec(), value));
    }
}

fn renumber(value: &Value, off: u32) -> Value {
    match value {
        Value::Ref { id, .. } => Value::Ref { id: id + off, gen: 0 },
        Value::Array(items) => {
            Value::Array(items.iter().map(|v| renumber(v, off)).collect())
        }
        Value::Dict(entries) => Value::Dict(
            entries.iter().map(|(k, v)| (k.clone(), renumber(v, off))).collect(),
        ),
        Value::Stream { dict, body } => Value::Stream {
            dict: dict.iter().map(|(k, v)| (k.clone(), renumber(v, off))).collect(),
            body: body.clone(),
        },
        other => other.clone(),
    }
}

fn serialize(objs: &BTreeMap<u32, Value>, kids: &[u32]) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(b"%PDF-1.7\n");

    // id → byte offset, for the xref.
    let mut offsets: BTreeMap<u32, usize> = BTreeMap::new();

    let mut write_obj = |out: &mut Vec<u8>, id: u32, body: &[u8]| {
        offsets.insert(id, out.len());
        out.extend_from_slice(format!("{id} 0 obj\n").as_bytes());
        out.extend_from_slice(body);
        out.extend_from_slice(b"\nendobj\n");
    };

    // 1: catalog, 2: pages.
    write_obj(&mut out, 1, b"<</Type/Catalog/Pages 2 0 R>>");
    let kids_str: Vec<String> = kids.iter().map(|k| format!("{k} 0 R")).collect();
    let pages_body =
        format!("<</Type/Pages/Kids[{}]/Count {}>>", kids_str.join(" "), kids.len());
    write_obj(&mut out, 2, pages_body.as_bytes());

    // The imported objects.
    for (&id, value) in objs {
        let mut body = Vec::new();
        serialize_value(&mut body, value);
        write_obj(&mut out, id, &body);
    }

    // xref: a single subsection 0..max_id+1 (free for missing ids).
    let max_id = objs.keys().copied().max().unwrap_or(2).max(2);
    let xref_off = out.len();
    out.extend_from_slice(format!("xref\n0 {}\n", max_id + 1).as_bytes());
    out.extend_from_slice(b"0000000000 65535 f \n");
    for id in 1..=max_id {
        match offsets.get(&id) {
            Some(off) => {
                out.extend_from_slice(format!("{off:010} 00000 n \n").as_bytes())
            }
            None => out.extend_from_slice(b"0000000000 65535 f \n"),
        }
    }
    out.extend_from_slice(
        format!(
            "trailer\n<</Size {}/Root 1 0 R>>\nstartxref\n{xref_off}\n%%EOF\n",
            max_id + 1
        )
        .as_bytes(),
    );
    out
}

/// Serialize one object value (mirrors the incremental writer's
/// encoding; streams get an auto `/Length`).
fn serialize_value(out: &mut Vec<u8>, v: &Value) {
    match v {
        Value::Null => out.extend_from_slice(b"null"),
        Value::Bool(b) => out.extend_from_slice(if *b { b"true" } else { b"false" }),
        Value::Integer(n) => out.extend_from_slice(n.to_string().as_bytes()),
        Value::Real(r) => out.extend_from_slice(format!("{r}").as_bytes()),
        Value::Name(n) => {
            out.push(b'/');
            out.extend_from_slice(n);
        }
        Value::String(s) => {
            out.push(b'(');
            for &b in s {
                if matches!(b, b'(' | b')' | b'\\') {
                    out.push(b'\\');
                }
                out.push(b);
            }
            out.push(b')');
        }
        Value::Array(items) => {
            out.push(b'[');
            for (i, it) in items.iter().enumerate() {
                if i > 0 {
                    out.push(b' ');
                }
                serialize_value(out, it);
            }
            out.push(b']');
        }
        Value::Dict(entries) => serialize_dict(out, entries),
        Value::Ref { id, gen } => {
            out.extend_from_slice(format!("{id} {gen} R").as_bytes())
        }
        Value::Stream { dict, body } => {
            let mut d = dict.clone();
            set_entry(&mut d, b"Length", Value::Integer(body.len() as i64));
            serialize_dict(out, &d);
            out.extend_from_slice(b"\nstream\n");
            out.extend_from_slice(body);
            out.extend_from_slice(b"\nendstream");
        }
    }
}

fn serialize_dict(out: &mut Vec<u8>, entries: &[(Vec<u8>, Value)]) {
    out.extend_from_slice(b"<<");
    for (k, v) in entries {
        out.push(b'/');
        out.extend_from_slice(k);
        out.push(b' ');
        serialize_value(out, v);
    }
    out.extend_from_slice(b">>");
}

#[cfg(test)]
mod tests {
    use super::*;

    fn read(name: &str) -> Vec<u8> {
        std::fs::read(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("tests/fixtures/text_extractor")
                .join(name),
        )
        .unwrap()
    }

    fn count_pages(bytes: &[u8]) -> usize {
        page_count(&parse(bytes).unwrap())
    }

    #[test]
    fn concatenate_two_sources() {
        let a = read("hello_world.pdf");
        let b = read("two_pages.pdf");
        let na = count_pages(&a);
        let nb = count_pages(&b);
        let da = parse(&a).unwrap();
        let db = parse(&b).unwrap();
        let mut sel = Vec::new();
        for i in 0..na {
            sel.push((0, i));
        }
        for i in 0..nb {
            sel.push((1, i));
        }
        let merged = assemble(&[da, db], &sel);
        assert_eq!(count_pages(&merged), na + nb);
    }

    #[test]
    fn extract_single_page() {
        let b = read("two_pages.pdf");
        let db = parse(&b).unwrap();
        let merged = assemble(&[db], &[(0, 0)]);
        assert_eq!(count_pages(&merged), 1);
    }
}
