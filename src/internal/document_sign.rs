//! Two-phase PDF digital signing + signature read-back. Builds a
//! signature field + `/Sig` dictionary (`adbe.pkcs7.detached`) into the
//! document, computes the `/ByteRange` digest, and embeds the
//! [`pkcs7_sign`](super::pkcs7_sign) detached PKCS#7 in `/Contents`.
//! Backs the `PdfFileSignature` facade.
//!
//! Non-anchored (composes the foundation primitives + the
//! `document_encrypt` serialiser helpers).

use std::collections::HashMap;

use super::document_encrypt::emit_obj;
use super::objects::{self, Value};
use super::{pages_tree, pkcs7_sign, trailer, xref};

/// The smallest `/Contents` placeholder this signer writes, in bytes.
/// A caller may ask for more; asking for less cannot shrink it below
/// what this signer's own PKCS#7 output needs.
const MIN_SIGNATURE_BYTES: usize = 8192;
/// Signer name — matches the bundled test certificate's CN.
pub(crate) const SIGNER_NAME: &str = "Aspose.PDF FOSS Test Signer";

/// Signing request.
pub(crate) struct SignParams {
    pub field_name: String,
    pub reason: String,
    pub contact: String,
    pub location: String,
    /// `/M`, the time of signing (§12.8.1 Table 252), already in PDF
    /// date form. `None` writes no `/M` at all rather than a date the
    /// caller never gave.
    pub date: Option<String>,
    /// How many bytes the `/Contents` placeholder must hold. Taken as a
    /// minimum — see `MIN_SIGNATURE_BYTES`.
    pub signature_bytes: usize,
}

impl SignParams {
    /// The placeholder width in hex characters: two per byte.
    fn contents_hex_len(&self) -> usize {
        self.signature_bytes.max(MIN_SIGNATURE_BYTES) * 2
    }
}

/// A signature recovered from a signed document.
pub(crate) struct SigInfo {
    pub name: String,
    pub signer_name: String,
    pub reason: String,
    pub location: String,
    pub contact: String,
    pub has_signature: bool,
    pub covers_whole: bool,
}

fn pdf_string(s: &str) -> Value {
    Value::String(s.as_bytes().to_vec())
}

/// Sign `bytes`, returning the signed PDF.
pub(crate) fn sign(bytes: &[u8], p: &SignParams) -> Option<Vec<u8>> {
    let table = xref::parse(bytes).ok()?;
    let tr = trailer::parse(bytes).ok()?;
    let tree = pages_tree::parse(bytes).ok()?;
    let page1_id = tree.leaves.first()?.id;
    let root_id = tr.root_id;

    let mut objs: Vec<(u32, u16, Value)> = Vec::new();
    let mut max_id = 0u32;
    for entry in &table.entries {
        if let xref::Entry::InUse { id, gen, offset } = *entry {
            if let Ok(o) = objects::parse_object(bytes, offset as usize) {
                max_id = max_id.max(id);
                objs.push((id, gen, o.value));
            }
        }
    }
    if objs.is_empty() {
        return None;
    }
    let sig_field_id = max_id + 1;
    let sig_value_id = max_id + 2;

    // Inject /AcroForm into the catalog + /Annots into the first page.
    for (id, _, v) in objs.iter_mut() {
        if *id == root_id {
            if let Value::Dict(entries) = v {
                let acro = Value::Dict(vec![
                    (b"Fields".to_vec(), Value::Array(vec![Value::Ref { id: sig_field_id, gen: 0 }])),
                    (b"SigFlags".to_vec(), Value::Integer(3)),
                ]);
                set_entry(entries, b"AcroForm", acro);
            }
        }
        if *id == page1_id {
            if let Value::Dict(entries) = v {
                let annot = Value::Ref { id: sig_field_id, gen: 0 };
                match entries.iter_mut().find(|(k, _)| k.as_slice() == b"Annots") {
                    Some((_, Value::Array(a))) => a.push(annot),
                    _ => set_entry(entries, b"Annots", Value::Array(vec![annot])),
                }
            }
        }
    }

    // The signature field (widget annotation).
    let sig_field = Value::Dict(vec![
        (b"Type".to_vec(), Value::Name(b"Annot".to_vec())),
        (b"Subtype".to_vec(), Value::Name(b"Widget".to_vec())),
        (b"FT".to_vec(), Value::Name(b"Sig".to_vec())),
        (b"T".to_vec(), pdf_string(&p.field_name)),
        (b"V".to_vec(), Value::Ref { id: sig_value_id, gen: 0 }),
        (
            b"Rect".to_vec(),
            Value::Array(vec![Value::Integer(0), Value::Integer(0), Value::Integer(0), Value::Integer(0)]),
        ),
        (b"F".to_vec(), Value::Integer(132)),
        (b"P".to_vec(), Value::Ref { id: page1_id, gen: 0 }),
    ]);
    objs.push((sig_field_id, 0, sig_field));

    // Serialise everything, emitting the /Sig value object RAW so the
    // /ByteRange + /Contents placeholders keep a fixed width.
    let mut out: Vec<u8> = Vec::new();
    out.extend_from_slice(b"%PDF-1.7\n%\xE2\xE3\xCF\xD3\n");
    let size = sig_value_id + 1;
    let mut offsets: Vec<Option<usize>> = vec![None; size as usize];
    for (id, gen, v) in &objs {
        emit_obj(&mut out, *id, *gen, v, &mut offsets);
    }
    offsets[sig_value_id as usize] = Some(out.len());
    out.extend_from_slice(build_sig_object(sig_value_id, p).as_bytes());
    let contents_hex_len = p.contents_hex_len();

    // Cross-reference + trailer.
    let xref_at = out.len();
    out.extend_from_slice(format!("xref\n0 {size}\n").as_bytes());
    out.extend_from_slice(b"0000000000 65535 f \n");
    for id in 1..size {
        match offsets[id as usize] {
            Some(off) => out.extend_from_slice(format!("{off:010} 00000 n \n").as_bytes()),
            None => out.extend_from_slice(b"0000000000 65535 f \n"),
        }
    }
    out.extend_from_slice(b"trailer\n<< /Size ");
    out.extend_from_slice(size.to_string().as_bytes());
    out.extend_from_slice(format!(" /Root {root_id} 0 R").as_bytes());
    if tr.info_id != 0 {
        out.extend_from_slice(format!(" /Info {} 0 R", tr.info_id).as_bytes());
    }
    out.extend_from_slice(b" >>\nstartxref\n");
    out.extend_from_slice(xref_at.to_string().as_bytes());
    out.extend_from_slice(b"\n%%EOF\n");

    // ---- Phase 2: patch /ByteRange, digest, sign, patch /Contents ----
    let lt = find(&out, b"/Contents <")? + b"/Contents ".len(); // offset of '<'
    let a = lt;
    let b = lt + 1 + contents_hex_len + 1; // after the closing '>'
    let c = out.len() - b;

    // Patch the fixed-width /ByteRange placeholder.
    let br_at = find(&out, b"/ByteRange [0 ")? + b"/ByteRange [0 ".len();
    let real = format!("{a:010} {b:010} {c:010}");
    out[br_at..br_at + real.len()].copy_from_slice(real.as_bytes());

    // Digest over the signed ranges, sign, embed.
    let mut content = Vec::with_capacity(a + c);
    content.extend_from_slice(&out[..a]);
    content.extend_from_slice(&out[b..]);
    let der = pkcs7_sign::sign_detached(&content);
    let mut sig_hex = bytes_to_hex(&der);
    // A PKCS#7 blob too long for the placeholder cannot be written:
    // truncating it leaves a signature that looks well-formed and
    // fails to verify, which is worse than refusing to sign.
    if sig_hex.len() > contents_hex_len {
        return None;
    }
    while sig_hex.len() < contents_hex_len {
        sig_hex.push('0');
    }
    out[lt + 1..lt + 1 + contents_hex_len].copy_from_slice(sig_hex.as_bytes());

    Some(out)
}

fn build_sig_object(id: u32, p: &SignParams) -> String {
    let zeros = "0".repeat(p.contents_hex_len());
    // §12.8.1 Table 252: /M is optional, so a signature with no date
    // given carries none — the alternative is stamping every signed
    // document with a time the caller never stated.
    let m = match &p.date {
        Some(date) => format!("/M ({}) ", escape(date)),
        None => String::new(),
    };
    let head = format!(
        "{id} 0 obj\n<< /Type /Sig /Filter /Adobe.PPKLite /SubFilter /adbe.pkcs7.detached ",
    );
    format!(
        "{head}/Name ({name}) /Reason ({reason}) /Location ({location}) \
         /ContactInfo ({contact}) {m}\
         /ByteRange [0 0000000000 0000000000 0000000000] \
         /Contents <{zeros}> >>\nendobj\n",
        name = escape(SIGNER_NAME),
        reason = escape(&p.reason),
        location = escape(&p.location),
        contact = escape(&p.contact),
    )
}

fn escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        if matches!(ch, '(' | ')' | '\\') {
            out.push('\\');
        }
        out.push(ch);
    }
    out
}

fn bytes_to_hex(b: &[u8]) -> String {
    let mut s = String::with_capacity(b.len() * 2);
    for x in b {
        s.push_str(&format!("{x:02x}"));
    }
    s
}

fn find(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack.windows(needle.len()).position(|w| w == needle)
}

fn set_entry(entries: &mut Vec<(Vec<u8>, Value)>, key: &[u8], value: Value) {
    if let Some(slot) = entries.iter_mut().find(|(k, _)| k.as_slice() == key) {
        slot.1 = value;
    } else {
        entries.push((key.to_vec(), value));
    }
}

// ---- Read-back -------------------------------------------------------

/// Read all signature fields back from a (possibly signed) document.
pub(crate) fn read_signatures(bytes: &[u8]) -> Vec<SigInfo> {
    let Ok(table) = xref::parse(bytes) else {
        return Vec::new();
    };
    let Ok(tr) = trailer::parse(bytes) else {
        return Vec::new();
    };
    let mut objs: HashMap<u32, Value> = HashMap::new();
    for entry in &table.entries {
        if let xref::Entry::InUse { id, offset, .. } = *entry {
            if let Ok(o) = objects::parse_object(bytes, offset as usize) {
                objs.insert(id, o.value);
            }
        }
    }

    let resolve = |v: &Value| -> Option<Value> {
        match v {
            Value::Ref { id, .. } => objs.get(id).cloned(),
            other => Some(other.clone()),
        }
    };

    let mut out = Vec::new();
    let Some(catalog) = objs.get(&tr.root_id) else {
        return out;
    };
    let Some(acroform) = dict_get(catalog, b"AcroForm").and_then(|v| resolve(&v)) else {
        return out;
    };
    let Some(Value::Array(fields)) = dict_get(&acroform, b"Fields") else {
        return out;
    };
    for fref in &fields {
        let Some(field) = resolve(fref) else { continue };
        if dict_get(&field, b"FT") != Some(Value::Name(b"Sig".to_vec())) {
            continue;
        }
        let name = dict_string(&field, b"T");
        let mut info = SigInfo {
            name,
            signer_name: String::new(),
            reason: String::new(),
            location: String::new(),
            contact: String::new(),
            has_signature: false,
            covers_whole: false,
        };
        if let Some(sig) = dict_get(&field, b"V").and_then(|v| resolve(&v)) {
            let has_contents = dict_get(&sig, b"Contents").is_some();
            info.has_signature = has_contents;
            info.signer_name = dict_string(&sig, b"Name");
            info.reason = dict_string(&sig, b"Reason");
            info.location = dict_string(&sig, b"Location");
            info.contact = dict_string(&sig, b"ContactInfo");
            if let Some(Value::Array(br)) = dict_get(&sig, b"ByteRange") {
                if let [Value::Integer(a0), _, Value::Integer(b2), Value::Integer(c3)] = br.as_slice() {
                    info.covers_whole = *a0 == 0 && (*b2 + *c3) as usize == bytes.len();
                }
            }
        }
        out.push(info);
    }
    out
}

fn dict_get(v: &Value, key: &[u8]) -> Option<Value> {
    let entries = match v {
        Value::Dict(e) => e,
        Value::Stream { dict, .. } => dict,
        _ => return None,
    };
    entries.iter().find(|(k, _)| k.as_slice() == key).map(|(_, val)| val.clone())
}

fn dict_string(v: &Value, key: &[u8]) -> String {
    match dict_get(v, key) {
        Some(Value::String(b)) => String::from_utf8_lossy(&b).into_owned(),
        _ => String::new(),
    }
}
