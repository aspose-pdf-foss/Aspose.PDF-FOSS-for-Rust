//! Foundation primitives — parsers, decoders, codecs,
//! rasterisers. Crate-private; backs the public API but is never
//! exposed directly.
//!
//! Each submodule maps to one anchor in `Feedstock/PDF/Spec/`
//! (shared) merged with `Feedstock/PDF/Spec/targets/rust/<name>.yaml`
//! (Rust overlay). SpecDev composes bodies into these submodules;
//! freeze-gate drivers live in `tests/internal/`.
//!
//! Strategy lock: every primitive on the Rust track is composed
//! `from_scratch` against the format spec or BCL contract. No
//! `flate2`, no `image`, no codec-shortcut crates. std-only.
//! Modules land here as SpecDev produces them.

pub(crate) mod acroform_io;
pub(crate) mod aes_cbc;
pub(crate) mod agl;
pub(crate) mod ascii85;
pub(crate) mod asciihex;
pub(crate) mod binarize_bradley;
pub(crate) mod bmp_decoder;
pub(crate) mod bmp_encoder;
pub(crate) mod ccitt;
pub(crate) mod cff;
mod cff_data;
pub(crate) mod colorspace;
pub(crate) mod content_stream;
pub(crate) mod annotation_io;
pub(crate) mod bignum;
pub(crate) mod crypt_filter;
pub(crate) mod dct_encoder;
pub(crate) mod dctdecode;
pub(crate) mod digest;
pub(crate) mod document_encrypt;
pub(crate) mod document_sign;
pub(crate) mod embedded_file_io;
pub(crate) mod encoding;
pub(crate) mod encoding_tables;
pub(crate) mod encrypt_parser;
pub(crate) mod encrypt_writer;
pub(crate) mod flate;
pub(crate) mod font_embed;
pub(crate) mod font_subset;
pub(crate) mod glyph_rasterizer;
pub(crate) mod image_compositor;
pub(crate) mod image_placement_io;
pub(crate) mod jbig2;
pub(crate) mod lexer;
pub(crate) mod lzw;
pub(crate) mod named_dest_io;
pub(crate) mod objects;
pub(crate) mod outline_io;
pub(crate) mod outlines;
pub(crate) mod page_geometry_io;
pub(crate) mod page_label_io;
pub(crate) mod page_renderer;
pub(crate) mod page_tree_assembler;
pub(crate) mod page_tree_io;
pub(crate) mod pages_tree;
pub(crate) mod palette_quantiser;
pub(crate) mod pdf_writer_incremental;
pub(crate) mod pending;
pub(crate) mod pkcs7_keymat;
pub(crate) mod pkcs7_sign;
pub(crate) mod png_decoder;
pub(crate) mod png_encoder;
pub(crate) mod rasterizer;
pub(crate) mod rc4;
pub(crate) mod redaction_io;
pub(crate) mod rsa;
pub(crate) mod runlength;
pub(crate) mod standard14_metrics;
pub(crate) mod standard14_outlines;
pub(crate) mod svg_import;
pub(crate) mod text_extractor;
pub(crate) mod tiff_decoder;
pub(crate) mod tiff_encoder;
pub(crate) mod to_unicode;
pub(crate) mod trailer;
pub(crate) mod transform;
pub(crate) mod truetype;
pub(crate) mod viewer_prefs_io;
pub(crate) mod ximage_io;
pub(crate) mod xmp;
pub(crate) mod xref;
