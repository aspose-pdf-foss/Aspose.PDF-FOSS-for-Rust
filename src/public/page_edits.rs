//! Shared content-edit buffer for page mutation.
//!
//! [`Document`](crate::Document) owns an [`EditBuffer`] and stamps a
//! clone of the handle into every [`Page`](crate::Page) (alongside
//! the shared source bytes). A [`TextBuilder`](crate::TextBuilder)
//! built from a page pushes [`PageContentEdit`]s into that buffer;
//! [`Document::save`](crate::Document::save) flushes them as an
//! incremental update (new content-stream + font objects, page
//! `/Contents` + `/Resources` rewrite). Crate-internal.

use std::cell::RefCell;
use std::rc::Rc;

/// A handle shared by a document and its pages.
pub(crate) type EditBuffer = Rc<RefCell<EditData>>;

/// A page leaf's object id, shared with the page handle it came from. An
/// edit against a page created in-session is recorded before the id
/// exists; the flush reads the cell, by which time the page-tree
/// save-through has filled it in.
pub(crate) type LeafRef = Rc<std::cell::Cell<u32>>;

/// The mutable payload behind an [`EditBuffer`]: content-stream
/// appends (from `TextBuilder` / drawing render-on-save) plus
/// pending redaction requests (from `RedactionAnnotation::redact`).
#[derive(Debug, Default, Clone)]
pub(crate) struct EditData {
    /// Content-stream chunks to append, one per edit.
    pub content: Vec<PageContentEdit>,
    /// Redaction regions to burn out of the existing page content.
    pub redactions: Vec<RedactionRequest>,
    /// Whole new pages to append (a broken table's overflow continuation
    /// pages), each with its content + fonts baked in.
    pub new_pages: Vec<NewPage>,
}

/// A whole new page to append on save, with its content stream + fonts
/// baked in (a broken [`Table`](crate::Table)'s continuation page).
#[derive(Debug, Clone)]
pub(crate) struct NewPage {
    /// Content-stream operators for the page.
    pub content: Vec<u8>,
    /// Font resources the content references.
    pub fonts: Vec<EditFont>,
    /// The page media box `[llx, lly, urx, ury]`.
    pub media_box: [f64; 4],
}

/// A request to remove every text-show whose origin falls inside
/// `rect` (PDF user space) from page `leaf_id`'s content stream(s).
#[derive(Debug, Clone)]
pub(crate) struct RedactionRequest {
    /// Object id of the target page leaf.
    pub leaf_id: u32,
    /// The redaction rectangle `[llx, lly, urx, ury]` in PDF user space.
    pub rect: [f64; 4],
}

/// Content-stream operators that paint the image XObject named
/// `resource_name` into `rect` (`[llx, lly, urx, ury]`, user space),
/// optionally clipped to `clip`.
///
/// An image XObject is always drawn through the unit square, so the
/// placement rectangle *is* the transform: scale by the rectangle's size,
/// translate to its lower-left corner (PDF 32000-1:2008 §8.9.5.2). A
/// rectangle with no area paints nothing, which is what the operators
/// say; the corners are ordered first, so a rectangle given upper-left
/// first does not mirror the image.
pub(crate) fn image_draw_ops(
    resource_name: &str,
    rect: [f64; 4],
    clip: Option<[f64; 4]>,
) -> Vec<u8> {
    use crate::public::shape::fmt_num;

    let (x0, x1) = (rect[0].min(rect[2]), rect[0].max(rect[2]));
    let (y0, y1) = (rect[1].min(rect[3]), rect[1].max(rect[3]));

    let mut ops = b"q\n".to_vec();
    if let Some(c) = clip {
        let (cx0, cx1) = (c[0].min(c[2]), c[0].max(c[2]));
        let (cy0, cy1) = (c[1].min(c[3]), c[1].max(c[3]));
        ops.extend_from_slice(
            format!(
                "{} {} {} {} re\nW\nn\n",
                fmt_num(cx0),
                fmt_num(cy0),
                fmt_num(cx1 - cx0),
                fmt_num(cy1 - cy0)
            )
            .as_bytes(),
        );
    }
    ops.extend_from_slice(
        format!(
            "{} 0 0 {} {} {} cm\n",
            fmt_num(x1 - x0),
            fmt_num(y1 - y0),
            fmt_num(x0),
            fmt_num(y0)
        )
        .as_bytes(),
    );
    ops.extend_from_slice(format!("/{resource_name} Do\nQ\n").as_bytes());
    ops
}

/// Wrap `content` so it is drawn rotated by `degrees` about `(x, y)`,
/// counter-clockwise (PDF 32000-1:2008 §8.3.3). Returns the content
/// untouched for a zero rotation, so an unrotated artifact emits exactly
/// what it did before.
pub(crate) fn rotated_about(content: Vec<u8>, degrees: f64, x: f64, y: f64) -> Vec<u8> {
    use crate::public::shape::fmt_num;
    if degrees == 0.0 || !degrees.is_finite() {
        return content;
    }
    let (sin, cos) = degrees.to_radians().sin_cos();
    // Translate the pivot to the origin, rotate, translate back — the
    // three multiply into one matrix.
    let e = x - x * cos + y * sin;
    let f = y - x * sin - y * cos;
    let mut out = format!(
        "q\n{} {} {} {} {} {} cm\n",
        fmt_num(cos),
        fmt_num(sin),
        fmt_num(-sin),
        fmt_num(cos),
        fmt_num(e),
        fmt_num(f)
    )
    .into_bytes();
    out.extend_from_slice(&content);
    out.extend_from_slice(b"Q\n");
    out
}

/// Construct an empty edit buffer.
pub(crate) fn new_edit_buffer() -> EditBuffer {
    Rc::new(RefCell::new(EditData::default()))
}

/// A chunk of content-stream operators to append to one page, plus
/// the font resources the chunk references.
#[derive(Debug, Clone)]
pub(crate) struct PageContentEdit {
    /// The target page leaf's id cell — read at flush time.
    pub leaf_id: LeafRef,
    /// Content-stream operators to append.
    pub content: Vec<u8>,
    /// Font resources the content references (added to the page's
    /// `/Resources` `/Font`).
    pub fonts: Vec<EditFont>,
}

/// A font resource referenced by appended content. When `program` is
/// present (an `open_font` TrueType program) the save pipeline embeds a
/// `/TrueType` font with a `/FontDescriptor` + `/FontFile2`; otherwise
/// it emits a base (non-embedded) `/Type1` font dictionary.
#[derive(Debug, Clone)]
pub(crate) struct EditFont {
    /// The resource name used in the content (`/<name> … Tf`),
    /// without the leading slash.
    pub resource_name: Vec<u8>,
    /// The PDF `/BaseFont` name.
    pub base_font: String,
    /// The embeddable TrueType program, when the font was opened from a
    /// file and embedding is enabled.
    pub program: Option<Vec<u8>>,
    /// Whether to embed only the used glyphs (subset) rather than the
    /// whole program. Set from [`Font::is_subset`](crate::Font::is_subset).
    pub is_subset: bool,
    /// The WinAnsi byte char-codes drawn with this font, accumulated
    /// across every run sharing the resource name. Drives glyph
    /// selection when `is_subset` is set.
    pub used_codes: Vec<u8>,
}
