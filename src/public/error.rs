//! Crate-level public `Error` type returned by every fallible
//! method on [`Document`](crate::Document),
//! [`PageCollection`](crate::PageCollection), and
//! [`DocumentInfo`](crate::DocumentInfo).
//!
//! Foundation primitives surface their own per-primitive error
//! enums (see `crate::internal::*::Error`); those are crate-
//! private and folded into [`Error::InvalidPdf`] with the
//! primitive's `Display` rendering at the public boundary.

use std::fmt;
use std::io;

use crate::internal::{objects, pages_tree, pdf_writer_incremental, trailer, xmp, xref};

/// Errors returned by the public Aspose.PDF FOSS API.
#[derive(Debug)]
pub enum Error {
    /// I/O error reading or writing a file. Wraps the standard
    /// [`std::io::Error`].
    Io(io::Error),

    /// The PDF byte stream is structurally invalid — header,
    /// cross-reference table, trailer, page tree, or indirect-
    /// object body failed to parse. The wrapped string carries
    /// the underlying foundation primitive's diagnostic.
    InvalidPdf(String),

    /// A 1-based page index was out of range for the document.
    /// The first page is `1`; the last is `count()`.
    PageIndexOutOfRange {
        /// The index the caller passed.
        index: usize,
        /// Total number of pages in the document.
        count: usize,
    },

    /// [`DocumentInfo::add`](crate::DocumentInfo::add) was called
    /// with a key that already exists in the dictionary.
    DuplicateInfoKey(String),

    /// [`Document::save`](crate::Document::save) was called on a
    /// document constructed via
    /// [`Document::from_bytes`](crate::Document::from_bytes), so
    /// no source filename is tracked. Use
    /// [`Document::save_to`](crate::Document::save_to) instead.
    NoSourceFilename,

    /// The document has pending `/Info` mutations but the source
    /// PDF carries no `/Info` indirect — adding a brand-new
    /// `/Info` object is out of v1 scope. Stage the changes
    /// through a tool that adds `/Info` first, or rewrite the
    /// document.
    NoInfoForUpdate,

    /// A rendering device was asked for a page that the document's
    /// bytes do not contain yet.
    ///
    /// The devices rasterise the document as it stands on disk, and a
    /// page added in this session only becomes part of those bytes when
    /// the document is saved — so "add a page, render it" has nothing
    /// to render. Save the document (or
    /// [`Document::save_to_writer`](crate::Document::save_to_writer)
    /// into a buffer and reopen it) first.
    PageNotPersisted,

    /// [`Metadata::add`](crate::Metadata::add) was called with a
    /// key that already exists in the map.
    DuplicateMetadataKey(String),

    /// An [`XmpValue`](crate::XmpValue) conversion (`to_integer` /
    /// `to_double` / `to_array`) was called on a variant that
    /// does not support it, or a string variant failed to parse
    /// as the requested numeric type.
    XmpValueConversion(String),

    /// A [`Resolution`](crate::Resolution) constructor was passed
    /// a non-positive DPI value. The carried integer is the
    /// offending axis value (widened to `i64` so the renderer
    /// diagnostic surface can carry either ctor argument).
    InvalidResolution(i64),

    /// A collection lookup (by index or by name) matched no entry —
    /// the Rust analogue of the canonical indexer's bounds /
    /// missing-key throw. The wrapped string carries which lookup
    /// failed.
    CollectionItemNotFound(String),

    /// A font file could not be parsed as a supported font — the
    /// bytes are not a TrueType program, or the program is
    /// malformed. Returned by
    /// [`FontRepository::open_font`](crate::FontRepository::open_font).
    /// The wrapped string carries the underlying diagnostic.
    InvalidFont(String),

    /// Image bytes passed to [`Page::add_image`](crate::Page::add_image)
    /// (or an image [`Artifact`](crate::Artifact)) could not be
    /// recognised as a supported image format. The wrapped string
    /// carries the underlying diagnostic.
    InvalidImage(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Io(e) => write!(f, "i/o error: {e}"),
            Error::InvalidPdf(msg) => write!(f, "invalid PDF: {msg}"),
            Error::PageIndexOutOfRange { index, count } => write!(
                f,
                "page index {index} out of range; document has {count} page(s) (valid range: 1..={count})"
            ),
            Error::DuplicateInfoKey(k) => {
                write!(f, "DocumentInfo: key '{k}' already present")
            }
            Error::NoSourceFilename => write!(
                f,
                "Document::save: no source filename to save to. \
                 This Document was constructed via from_bytes; \
                 use save_to(path) instead."
            ),
            Error::NoInfoForUpdate => write!(
                f,
                "Document::save: source carries no /Info indirect; \
                 adding a new /Info object is out of v1 scope."
            ),
            Error::PageNotPersisted => write!(
                f,
                "page rendering: this page was added in this session and is \
                 not in the document's bytes yet; save the document first."
            ),
            Error::DuplicateMetadataKey(k) => {
                write!(f, "Metadata: key '{k}' already present")
            }
            Error::XmpValueConversion(msg) => {
                write!(f, "XmpValue conversion: {msg}")
            }
            Error::InvalidResolution(v) => {
                write!(f, "Resolution: DPI must be positive (got {v})")
            }
            Error::CollectionItemNotFound(msg) => {
                write!(f, "collection item not found: {msg}")
            }
            Error::InvalidFont(msg) => write!(f, "invalid font: {msg}"),
            Error::InvalidImage(msg) => write!(f, "invalid image: {msg}"),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Io(e) => Some(e),
            _ => None,
        }
    }
}

impl From<io::Error> for Error {
    fn from(e: io::Error) -> Self {
        Error::Io(e)
    }
}

impl From<trailer::Error> for Error {
    fn from(e: trailer::Error) -> Self {
        Error::InvalidPdf(e.to_string())
    }
}

impl From<xref::Error> for Error {
    fn from(e: xref::Error) -> Self {
        Error::InvalidPdf(e.to_string())
    }
}

impl From<objects::Error> for Error {
    fn from(e: objects::Error) -> Self {
        Error::InvalidPdf(e.to_string())
    }
}

impl From<pages_tree::Error> for Error {
    fn from(e: pages_tree::Error) -> Self {
        Error::InvalidPdf(e.to_string())
    }
}

impl From<pdf_writer_incremental::Error> for Error {
    fn from(e: pdf_writer_incremental::Error) -> Self {
        Error::InvalidPdf(e.to_string())
    }
}

impl From<xmp::Error> for Error {
    fn from(e: xmp::Error) -> Self {
        Error::InvalidPdf(e.to_string())
    }
}
