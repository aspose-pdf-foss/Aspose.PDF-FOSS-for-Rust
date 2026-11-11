//! Target-side helper: read / write a page's `/Resources /XObject`
//! image XObjects. Backs the public `Page::resources().images()`
//! save-through. Non-anchored (composes the trailer / xref / objects
//! parsers + the incremental writer + the image codecs).
//!
//! Embedding rules: a JPEG is stored verbatim under `/DCTDecode`
//! (lossless, byte-exact round-trip); a PNG / BMP / raster bitmap is
//! decoded to `RGB8` samples and stored under `/FlateDecode`. GIF and
//! other formats without a decoder are skipped (and reported by the
//! caller). On read, `/DCTDecode` images return their JPEG bytes and
//! `/FlateDecode` images return expanded `RGBA8` samples.

use std::collections::HashMap;

use super::objects::{self, Value};
use super::pdf_writer_incremental::{self, DirtyObject};
use super::{flate, trailer, xref};

/// Whether an [`ImageEntry`]'s `raw` is an encoded blob or raster
/// samples — mirrors the public `XImage` kind.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Kind {
    /// `raw` is an encoded image file (PNG / JPEG / GIF / BMP).
    Encoded,
    /// `raw` is `width * height * 4` `RGBA8` samples.
    RasterRgba8,
}

/// A PDF dictionary as `(key, value)` entries.
type DictEntries = Vec<(Vec<u8>, Value)>;

/// One image to embed.
pub(crate) struct ImageEntry {
    pub name: String,
    pub kind: Kind,
    pub raw: Vec<u8>,
    pub width: i32,
    pub height: i32,
}

/// One image parsed from a page's resources.
pub(crate) struct LoadedImage {
    pub name: String,
    pub width: i32,
    pub height: i32,
    pub raw: Vec<u8>,
    /// `true` when `raw` is encoded JPEG; `false` when `raw` is
    /// expanded `RGBA8` raster.
    pub encoded: bool,
    pub has_alpha: bool,
    /// The base `/ColorSpace` token (`DeviceGray` / `DeviceRGB` /
    /// `ICCBased` / `Indexed` / …), or `None` when absent — used to
    /// classify the image's [`ColorType`](crate::ColorType) even after
    /// a grayscale sample stream is widened to `RGBA8` on read.
    pub color_space: Option<Vec<u8>>,
    /// The declared `/BitsPerComponent` (0 when absent) — a value of 1
    /// marks a bilevel image.
    pub bits_per_component: i32,
    /// The declared `/ImageMask` flag — a stencil mask is bilevel.
    pub image_mask: bool,
}

// ---- Read ------------------------------------------------------------

/// Read the image XObjects in page `leaf_id`'s `/Resources /XObject`.
pub(crate) fn parse_page_images(bytes: &[u8], leaf_id: u32) -> Vec<LoadedImage> {
    let objs = build_objs(bytes);
    let Some(page) = objs.get(&leaf_id) else {
        return Vec::new();
    };
    let Some(resources) = dict_get(page, b"Resources").and_then(|v| resolve(&objs, &v)) else {
        return Vec::new();
    };
    let Some(xobject) = dict_get(&resources, b"XObject").and_then(|v| resolve(&objs, &v)) else {
        return Vec::new();
    };
    let Value::Dict(entries) = xobject else {
        return Vec::new();
    };

    let mut out = Vec::new();
    for (name, value) in &entries {
        let Some(stream) = resolve(&objs, value) else { continue };
        if !matches!(dict_get(&stream, b"Subtype"), Some(Value::Name(n)) if n == b"Image") {
            continue;
        }
        let Value::Stream { body, .. } = &stream else { continue };
        let width = int_field(&stream, b"Width").unwrap_or(0) as i32;
        let height = int_field(&stream, b"Height").unwrap_or(0) as i32;
        if width <= 0 || height <= 0 {
            continue;
        }
        let name = String::from_utf8_lossy(name).into_owned();
        let color_space = colorspace_token(&dict_get(&stream, b"ColorSpace"));
        let bits_per_component = int_field(&stream, b"BitsPerComponent").unwrap_or(0) as i32;
        let image_mask = matches!(dict_get(&stream, b"ImageMask"), Some(Value::Bool(true)));
        match filter_name(&dict_get(&stream, b"Filter")) {
            Some(f) if f == b"DCTDecode" => {
                out.push(LoadedImage {
                    name,
                    width,
                    height,
                    raw: body.clone(),
                    encoded: true,
                    has_alpha: false,
                    color_space,
                    bits_per_component,
                    image_mask,
                });
            }
            Some(f) if f == b"FlateDecode" => {
                if let Ok(samples) = flate::decode(body) {
                    let components = color_components(&dict_get(&stream, b"ColorSpace"));
                    let rgba = to_rgba8(&samples, width, height, components);
                    out.push(LoadedImage {
                        name,
                        width,
                        height,
                        raw: rgba,
                        encoded: false,
                        has_alpha: false,
                        color_space,
                        bits_per_component,
                        image_mask,
                    });
                }
            }
            _ => {}
        }
    }
    out
}

// ---- Write -----------------------------------------------------------

/// Append an incremental update embedding `pages`' images into each
/// page's `/Resources /XObject`. Returns `None` when the document can't
/// be parsed. Images whose format can't be encoded are skipped.
pub(crate) fn append(bytes: &[u8], pages: &[(u32, Vec<ImageEntry>)]) -> Option<Vec<u8>> {
    let tr = trailer::parse(bytes).ok()?;
    let objs = build_objs(bytes);
    let mut next_id = tr.size as u32;
    let mut dirty: Vec<DirtyObject> = Vec::new();

    for (leaf_id, images) in pages {
        let mut page_dict = match objs.get(leaf_id) {
            Some(Value::Dict(d)) => d.clone(),
            _ => continue,
        };

        // Start from the page's own /Resources (resolved), or a fresh
        // dict. Preserve every existing resource sub-entry.
        let mut resources = match dict_get(&Value::Dict(page_dict.clone()), b"Resources")
            .and_then(|v| resolve(&objs, &v))
        {
            Some(Value::Dict(d)) => d,
            _ => Vec::new(),
        };
        let mut xobject = match dict_get(&Value::Dict(resources.clone()), b"XObject")
            .and_then(|v| resolve(&objs, &v))
        {
            Some(Value::Dict(d)) => d,
            _ => Vec::new(),
        };

        // Drop every image XObject entry whose name is not in the page's
        // current image set, so deletions persist on save. Non-image
        // XObjects (forms, etc.) and any image whose name survives are
        // preserved / re-pointed below.
        let keep: std::collections::HashSet<&[u8]> =
            images.iter().map(|i| i.name.as_bytes()).collect();
        xobject.retain(|(name, value)| {
            if keep.contains(name.as_slice()) {
                return true;
            }
            // Keep non-image XObjects untouched; drop image entries that
            // are no longer in the collection.
            match resolve(&objs, value) {
                Some(stream) => !matches!(
                    dict_get(&stream, b"Subtype"),
                    Some(Value::Name(n)) if n == b"Image"
                ),
                None => true,
            }
        });

        for img in images {
            let Some((stream_dict, body)) = encode_image(img) else {
                continue; // undecodable format — skipped
            };
            let id = next_id;
            next_id += 1;
            dirty.push(DirtyObject {
                id,
                generation: 0,
                value: Value::Stream { dict: stream_dict, body },
            });
            set_entry(&mut xobject, img.name.as_bytes(), Value::Ref { id, gen: 0 });
        }

        set_entry(&mut resources, b"XObject", Value::Dict(xobject));
        set_entry(&mut page_dict, b"Resources", Value::Dict(resources));
        dirty.push(DirtyObject { id: *leaf_id, generation: 0, value: Value::Dict(page_dict) });
    }

    if dirty.is_empty() {
        return Some(bytes.to_vec());
    }
    pdf_writer_incremental::append_incremental(bytes, &dirty).ok()
}

/// Build the `(stream dict, body)` of an image XObject, or `None` when
/// the format can't be encoded.
fn encode_image(img: &ImageEntry) -> Option<(DictEntries, Vec<u8>)> {
    let (color_space, bits, filter, body): (&[u8], i64, &[u8], Vec<u8>) = match img.kind {
        Kind::Encoded => {
            if is_jpeg(&img.raw) {
                let cs: &[u8] = if jpeg_components(&img.raw) == 1 {
                    b"DeviceGray"
                } else {
                    b"DeviceRGB"
                };
                (cs, 8, b"DCTDecode", img.raw.clone())
            } else {
                let rgba = decode_to_rgba(&img.raw)?;
                (b"DeviceRGB", 8, b"FlateDecode", flate::encode(&rgb_from_rgba(&rgba)))
            }
        }
        Kind::RasterRgba8 => {
            (b"DeviceRGB", 8, b"FlateDecode", flate::encode(&rgb_from_rgba(&img.raw)))
        }
    };

    let dict = vec![
        (b"Type".to_vec(), Value::Name(b"XObject".to_vec())),
        (b"Subtype".to_vec(), Value::Name(b"Image".to_vec())),
        (b"Width".to_vec(), Value::Integer(img.width as i64)),
        (b"Height".to_vec(), Value::Integer(img.height as i64)),
        (b"ColorSpace".to_vec(), Value::Name(color_space.to_vec())),
        (b"BitsPerComponent".to_vec(), Value::Integer(bits)),
        (b"Filter".to_vec(), Value::Name(filter.to_vec())),
    ];
    Some((dict, body))
}

/// Decode a PNG / BMP blob to `RGBA8`, or `None` for other formats.
fn decode_to_rgba(raw: &[u8]) -> Option<Vec<u8>> {
    if raw.len() >= 8 && raw[..8] == [0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A] {
        return super::png_decoder::decode(raw).ok().map(|d| d.pixels);
    }
    if raw.len() >= 2 && &raw[..2] == b"BM" {
        return super::bmp_decoder::decode(raw).ok().map(|d| d.pixels);
    }
    None
}

/// Drop the alpha channel from `RGBA8` samples.
fn rgb_from_rgba(rgba: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(rgba.len() / 4 * 3);
    for px in rgba.chunks_exact(4) {
        out.extend_from_slice(&px[..3]);
    }
    out
}

/// Expand decoded samples to `RGBA8` (gray / RGB widened, alpha 255).
fn to_rgba8(samples: &[u8], width: i32, height: i32, components: usize) -> Vec<u8> {
    let pixels = (width as usize) * (height as usize);
    let mut out = Vec::with_capacity(pixels * 4);
    match components {
        1 => {
            for &g in samples.iter().take(pixels) {
                out.extend_from_slice(&[g, g, g, 0xFF]);
            }
        }
        _ => {
            for px in samples.chunks_exact(3).take(pixels) {
                out.extend_from_slice(&[px[0], px[1], px[2], 0xFF]);
            }
        }
    }
    out
}

fn is_jpeg(b: &[u8]) -> bool {
    b.len() >= 2 && b[0] == 0xFF && b[1] == 0xD8
}

/// Number of components declared in a JPEG SOF marker (1 = grayscale,
/// 3 = YCbCr/RGB). Defaults to 3.
fn jpeg_components(b: &[u8]) -> u8 {
    let mut i = 2;
    while i + 9 <= b.len() {
        if b[i] != 0xFF {
            i += 1;
            continue;
        }
        let marker = b[i + 1];
        if (0xC0..=0xCF).contains(&marker) && !matches!(marker, 0xC4 | 0xC8 | 0xCC) {
            return *b.get(i + 9).unwrap_or(&3);
        }
        let seg = ((b.get(i + 2).copied().unwrap_or(0) as usize) << 8)
            | b.get(i + 3).copied().unwrap_or(0) as usize;
        if seg < 2 {
            break;
        }
        i += 2 + seg;
    }
    3
}

/// The number of colour components for a `/ColorSpace` name.
fn color_components(cs: &Option<Value>) -> usize {
    match cs {
        Some(Value::Name(n)) if n == b"DeviceGray" => 1,
        Some(Value::Name(n)) if n == b"DeviceCMYK" => 4,
        _ => 3,
    }
}

/// The base colour-space token from a `/ColorSpace`: a plain name, or
/// the leading family name of an array form (`Indexed` / `ICCBased` /
/// `CalRGB` / `CalGray` / `Separation` / `DeviceN` / …).
fn colorspace_token(cs: &Option<Value>) -> Option<Vec<u8>> {
    match cs {
        Some(Value::Name(n)) => Some(n.clone()),
        Some(Value::Array(a)) => a.first().and_then(|v| match v {
            Value::Name(n) => Some(n.clone()),
            _ => None,
        }),
        _ => None,
    }
}

/// The first filter name from a `/Filter` (name or single-element
/// array).
fn filter_name(filter: &Option<Value>) -> Option<Vec<u8>> {
    match filter {
        Some(Value::Name(n)) => Some(n.clone()),
        Some(Value::Array(a)) => a.first().and_then(|v| match v {
            Value::Name(n) => Some(n.clone()),
            _ => None,
        }),
        _ => None,
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

fn int_field(v: &Value, key: &[u8]) -> Option<i64> {
    match dict_get(v, key) {
        Some(Value::Integer(n)) => Some(n),
        _ => None,
    }
}

fn set_entry(entries: &mut Vec<(Vec<u8>, Value)>, key: &[u8], value: Value) {
    if let Some(slot) = entries.iter_mut().find(|(k, _)| k.as_slice() == key) {
        slot.1 = value;
    } else {
        entries.push((key.to_vec(), value));
    }
}
