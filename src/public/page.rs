//! `Aspose.Pdf.Page` — single page accessed through
//! [`Document::pages`](crate::Document::pages).
//!
//! v1 surface: 1-based [`Page::number`], plus geometry
//! ([`Page::width`], [`Page::height`], [`Page::rotation`])
//! resolved from the `/Pages` tree via the foundation
//! `pages_tree` primitive. The canonical .NET surface also
//! carries `SendTo` / `ConvertToPNGMemoryStream`; those route
//! through the device hierarchy and ship in Beat 3 (devices)
//! and a later beat (Document-level convert helpers).

use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::Arc;

use crate::public::rectangle::Rectangle;
use crate::public::rotation::Rotation;

/// Interior geometry of a page — the `/MediaBox`, `/CropBox` and
/// `/Rotate` resolved on load. Held behind an [`Rc<RefCell<…>>`] so a
/// mutation through one [`Page`] handle is visible on a fresh handle
/// obtained from the same [`Document`](crate::Document), and so the
/// document's save pipeline can read the edited state back. `dirty`
/// tracks whether any setter ran — an untouched page resaves verbatim.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct PageGeometry {
    pub media_box: Rectangle,
    pub crop_box: Rectangle,
    /// The §14.11.2 boxes. `None` is the ordinary case — the entry is
    /// absent and the box reads as the `/CropBox`.
    pub art_box: Option<Rectangle>,
    pub bleed_box: Option<Rectangle>,
    pub trim_box: Option<Rectangle>,
    pub rotate: i32,
    pub dirty: bool,
}

/// Shared, interior-mutable handle to a page's [`PageGeometry`].
pub(crate) type GeometryHandle = Rc<RefCell<PageGeometry>>;

/// A single page within a [`Document`](crate::Document).
///
/// `Page` is a borrowed view — instances are owned by the
/// document's [`PageCollection`](crate::PageCollection) and
/// accessed by reference. There is no public constructor; obtain
/// pages via [`Document::pages`](crate::Document::pages) and
/// [`PageCollection::get`](crate::PageCollection::get).
#[derive(Debug, Clone)]
pub struct Page {
    pub(crate) number: u64,
    pub(crate) width: f64,
    pub(crate) height: f64,
    pub(crate) rotation: i32,
    /// Indirect-object id of the `/Page` leaf, shared with every clone
    /// of this handle. Zero until the page exists in the file: a page
    /// created in-session gets its id from the page-tree save-through,
    /// and every edit keyed by this cell then resolves to it.
    pub(crate) leaf_id: Rc<Cell<u32>>,
    /// 0-based leaf position in the parent document's page-tree
    /// walk — the index expected by
    /// `internal::page_renderer::render`.
    pub(crate) leaf_index: u64,
    /// Shared handle to the document's source bytes, cloned by
    /// `Arc::clone` into every page when the page collection is
    /// built. Used by the device hierarchy (and the future
    /// `TextAbsorber`) to render / re-parse the page without
    /// keeping a `&Document` borrow alive.
    pub(crate) doc_bytes: Arc<Vec<u8>>,
    /// Shared content-edit buffer (cloned from the owning document).
    /// A [`TextBuilder`](crate::TextBuilder) built from this page
    /// pushes edits here; the document flushes them on save.
    pub(crate) edits: crate::public::page_edits::EditBuffer,
    /// This page's annotations — a shared handle parsed from the page's
    /// `/Annots` on load and written back on save. Exposed via
    /// [`Page::annotations`].
    pub(crate) annotations: crate::public::annotation_collection::AnnotationCollection,
    /// This page's resources — a shared handle whose
    /// [`XImageCollection`](crate::XImageCollection) is parsed from the
    /// page's `/Resources /XObject` images on load and written back on
    /// save. Exposed via [`Page::resources`].
    pub(crate) resources: crate::public::resources::Resources,
    /// This page's added paragraphs — a shared handle whose drawing
    /// [`Graph`](crate::Graph)s (and future paragraph types) are
    /// rendered into the page content on save. Exposed via
    /// [`Page::paragraphs`].
    pub(crate) paragraphs: crate::public::paragraphs::Paragraphs,
    /// This page's geometry (`/MediaBox`, `/CropBox`, `/Rotate`) — a
    /// shared, interior-mutable handle parsed on load and written back on
    /// save when edited. Backs [`Page::rect`], [`Page::media_box`],
    /// [`Page::crop_box`], [`Page::rotate`], [`Page::set_page_size`] and
    /// [`Page::get_page_rect`].
    pub(crate) geometry: GeometryHandle,
    /// This page's artifacts (watermarks / backgrounds / stamps) — a
    /// shared handle. Artifacts added through it (via interior
    /// mutability) are rendered into the page on the document's next
    /// save: a text artifact as a positioned text run, an image
    /// artifact as an embedded image XObject. Exposed via
    /// [`Page::artifacts`].
    pub(crate) artifacts: crate::public::artifact_collection::ArtifactCollection,
}

impl PartialEq for Page {
    fn eq(&self, other: &Self) -> bool {
        // `doc_bytes` is identity-shared, not value-compared —
        // two pages from independently-opened copies of the same
        // PDF have distinct Arcs but otherwise-equal payloads.
        self.number == other.number
            && self.width == other.width
            && self.height == other.height
            && self.rotation == other.rotation
            && self.leaf_id.get() == other.leaf_id.get()
            && self.leaf_index == other.leaf_index
    }
}

impl Page {
    /// Synthesize a blank 612×792 (US Letter) page that shares the owning
    /// document's source bytes and content-edit buffer. Carries
    /// `leaf_id == 0` — the save path assigns it a fresh object id and
    /// writes a `/Type /Page` leaf with a `/MediaBox [0 0 612 792]`, an
    /// empty `/Contents`, and an empty `/Resources`. Used by
    /// [`PageCollection::add`](crate::PageCollection::add) and
    /// [`PageCollection::insert`](crate::PageCollection::insert).
    pub(crate) fn new_blank(
        doc_bytes: Arc<Vec<u8>>,
        edits: crate::public::page_edits::EditBuffer,
    ) -> Page {
        let rect = Rectangle::new(0.0, 0.0, 612.0, 792.0, false);
        let geometry = Rc::new(RefCell::new(PageGeometry {
            media_box: rect,
            crop_box: rect,
            art_box: None,
            bleed_box: None,
            trim_box: None,
            rotate: 0,
            // A blank page has no source object — it is always emitted by
            // the page-tree save-through, so its geometry need not raise a
            // separate geometry-update.
            dirty: false,
        }));
        Page {
            number: 0,
            width: 612.0,
            height: 792.0,
            rotation: 0,
            // 0 marks a not-yet-persisted leaf; the page-tree save-through
            // assigns a fresh indirect object id into this cell.
            leaf_id: Rc::new(Cell::new(0)),
            // Out of range for the source bytes' page list, so an
            // in-session `TextAbsorber::visit_page` reads no text (a blank
            // page carries none).
            leaf_index: u64::MAX,
            doc_bytes,
            edits,
            annotations: crate::public::annotation_collection::AnnotationCollection::new(),
            resources: crate::public::resources::Resources::new(),
            paragraphs: crate::public::paragraphs::Paragraphs::new(),
            geometry,
            artifacts: crate::public::artifact_collection::ArtifactCollection::new(),
        }
    }

    /// Synthesize a copy of `source` that reuses its `/Contents` and
    /// `/Resources` (the copy renders the same text / graphics). The copy
    /// shares `source`'s `leaf_id` so the save path can reuse its content
    /// stream by reference, and its `leaf_index` so an in-session
    /// [`TextAbsorber`](crate::TextAbsorber) reads `source`'s page text.
    /// Sub-handles (annotations / images / paragraphs / artifacts) are
    /// fresh, so mutating the copy doesn't bleed into the source. Used by
    /// [`PageCollection::add_page`](crate::PageCollection::add_page).
    pub(crate) fn new_copy(source: &Page) -> Page {
        let g = *source.geometry.borrow();
        let geometry = Rc::new(RefCell::new(PageGeometry { dirty: false, ..g }));
        Page {
            number: source.number,
            width: source.width,
            height: source.height,
            rotation: source.rotation,
            leaf_id: Rc::new(Cell::new(source.leaf_id.get())),
            leaf_index: source.leaf_index,
            doc_bytes: Arc::clone(&source.doc_bytes),
            edits: Rc::clone(&source.edits),
            annotations: crate::public::annotation_collection::AnnotationCollection::new(),
            resources: crate::public::resources::Resources::new(),
            paragraphs: crate::public::paragraphs::Paragraphs::new(),
            geometry,
            artifacts: crate::public::artifact_collection::ArtifactCollection::new(),
        }
    }

    /// Re-seat this page at 0-based position `index` in the owning
    /// collection: updates the 1-based [`Page::number`] only.
    ///
    /// [`Page::internal_leaf_index`] is deliberately **not** touched — it
    /// is the page's position in the (unchanged-in-session) source byte
    /// buffer, which is what [`TextAbsorber`](crate::TextAbsorber) and the
    /// renderer index into. A loaded page's content lives at its load-time
    /// tree position regardless of how the collection is reordered in
    /// memory, so its `leaf_index` must stay pinned there until a save
    /// re-parses fresh bytes.
    pub(crate) fn set_position(&mut self, index: u64) {
        self.number = index + 1;
    }

    /// 1-based index of this page within the owning document.
    /// The first page is `1`, not `0`.
    pub fn number(&self) -> u64 {
        self.number
    }

    /// Page width in PDF user-space units (1/72 inch). Resolved
    /// from `/MediaBox` per PDF 32000-1:2008 §14.11.2 with the
    /// `/Pages`-tree inheritance chain applied.
    pub fn width(&self) -> f64 {
        self.geometry.borrow().crop_box.width()
    }

    /// Page height in PDF user-space units (1/72 inch). Same
    /// resolution rules as [`Page::width`].
    pub fn height(&self) -> f64 {
        self.geometry.borrow().crop_box.height()
    }

    /// Page rotation in degrees, clockwise — one of
    /// `0`, `90`, `180`, `270`. Resolved from `/Rotate` with
    /// inheritance and normalised into `[0, 360)`.
    pub fn rotation(&self) -> i32 {
        self.rotation
    }

    /// This page's annotations. The returned collection is a live
    /// handle — annotations added through it (via interior mutability)
    /// belong to the page and are written to the page's `/Annots` on
    /// the document's next save. Mirrors the canonical
    /// `Page.Annotations` property.
    pub fn annotations(&self) -> &crate::public::annotation_collection::AnnotationCollection {
        &self.annotations
    }

    /// This page's resources. The returned [`Resources`](crate::Resources) is a live
    /// handle — images added through its
    /// [`XImageCollection`](crate::XImageCollection) (via interior
    /// mutability) are embedded into the page's `/Resources /XObject`
    /// on the document's next save. Mirrors the canonical
    /// `Page.Resources` property.
    pub fn resources(&self) -> &crate::public::resources::Resources {
        &self.resources
    }

    /// This page's paragraph collection. The returned [`Paragraphs`](crate::Paragraphs) is
    /// a live handle — drawing [`Graph`](crate::Graph)s added through it
    /// (via interior mutability) are rendered into the page's content
    /// stream on the document's next save. Mirrors the canonical
    /// `Page.Paragraphs` property.
    pub fn paragraphs(&self) -> &crate::public::paragraphs::Paragraphs {
        &self.paragraphs
    }

    /// This page's artifact collection. The returned
    /// [`ArtifactCollection`](crate::ArtifactCollection) is a live handle
    /// — artifacts added through it (via interior mutability) are
    /// rendered into the page on the document's next save (a text
    /// artifact becomes a positioned text run; an image artifact is
    /// embedded as a `/Resources /XObject /Image`). Mirrors the canonical
    /// `Page.Artifacts` property.
    pub fn artifacts(&self) -> &crate::public::artifact_collection::ArtifactCollection {
        &self.artifacts
    }

    /// Place an in-memory image (PNG / JPEG bytes) on the page. The image
    /// is embedded as a `/Resources /XObject /Image` — it surfaces in
    /// [`Page::resources`]`().images()` — and painted into `image_rect`
    /// by operators appended to the page content stream.
    ///
    /// A viewer draws the result, and so does this crate's own
    /// rasteriser — [`PngDevice`](crate::PngDevice) and its siblings
    /// paint the image back.
    ///
    /// `image_rect` is the placement rectangle in user space. With
    /// `auto_adjust_rectangle` the image is scaled to the largest size
    /// that fits inside it without distortion and centred there;
    /// otherwise it is stretched to fill the rectangle exactly. A
    /// rectangle with no area paints nothing. `bbox`, when given, clips
    /// the drawing to that rectangle.
    ///
    /// Returns an error for image bytes whose format can't be recognised
    /// (the canonical void overload signals failure by throwing).
    pub fn add_image(
        &self,
        image_stream: &[u8],
        image_rect: Rectangle,
        bbox: Option<Rectangle>,
        auto_adjust_rectangle: bool,
    ) -> Result<(), crate::public::error::Error> {
        let probe = crate::public::x_image::XImage::from_bytes(image_stream.to_vec(), "probe");
        if probe.width() <= 0 || probe.height() <= 0 {
            return Err(crate::public::error::Error::InvalidImage(
                "unsupported or malformed image bytes".to_string(),
            ));
        }
        let name = self.resources.images().add_stream(image_stream);

        let mut rect = [
            image_rect.llx(),
            image_rect.lly(),
            image_rect.urx(),
            image_rect.ury(),
        ];
        if auto_adjust_rectangle {
            rect = fit_preserving_aspect(rect, probe.width() as f64, probe.height() as f64);
        }
        let clip = bbox.map(|b| [b.llx(), b.lly(), b.urx(), b.ury()]);

        self.edits.borrow_mut().content.push(
            crate::public::page_edits::PageContentEdit {
                leaf_id: self.leaf_ref(),
                content: crate::public::page_edits::image_draw_ops(&name, rect, clip),
                fonts: Vec::new(),
            },
        );
        Ok(())
    }

    /// The page rectangle — the effective `/CropBox` (which defaults to
    /// the `/MediaBox`). Reads the live geometry handle, so a prior
    /// [`Page::set_rect`] / [`Page::set_page_size`] is reflected even
    /// across a fresh `doc.pages().get(n)`. Mirrors the canonical
    /// `Page.Rect` getter.
    pub fn rect(&self) -> Rectangle {
        self.geometry.borrow().crop_box
    }

    /// Set the page rectangle. Replaces both the `/CropBox` and the
    /// `/MediaBox` (the canonical `Page.Rect` setter resizes the page),
    /// and marks the geometry dirty so the change persists on save.
    pub fn set_rect(&self, value: Rectangle) {
        let mut g = self.geometry.borrow_mut();
        g.media_box = value;
        g.crop_box = value;
        g.dirty = true;
    }

    /// The `/MediaBox` — the boundaries of the physical medium.
    /// Mirrors the canonical `Page.MediaBox` getter.
    pub fn media_box(&self) -> Rectangle {
        self.geometry.borrow().media_box
    }

    /// The `/CropBox` — the visible page region (defaults to the
    /// `/MediaBox`). Mirrors the canonical `Page.CropBox` getter.
    pub fn crop_box(&self) -> Rectangle {
        self.geometry.borrow().crop_box
    }

    /// Set the `/CropBox`. Marks the geometry dirty so the change
    /// persists on save. Mirrors the canonical `Page.CropBox` setter.
    pub fn set_crop_box(&self, value: Rectangle) {
        let mut g = self.geometry.borrow_mut();
        g.crop_box = value;
        g.dirty = true;
    }

    /// Render this page to a PNG byte buffer through a default
    /// [`PngDevice`](crate::PngDevice). Mirrors the canonical
    /// `Page.ConvertToPNGMemoryStream`, whose `MemoryStream` return
    /// becomes the owned buffer a Rust caller would expect.
    ///
    /// The page carries its own document back-channel, so this needs
    /// nothing from the document; it is the page-level twin of
    /// [`Document::convert_page_to_png`](crate::Document::convert_page_to_png).
    pub fn convert_to_png(&self) -> Result<Vec<u8>, crate::public::error::Error> {
        use crate::public::device::PageDevice;
        let mut buf: Vec<u8> = Vec::new();
        crate::public::png_device::PngDevice::new().process(self, &mut buf)?;
        Ok(buf)
    }

    /// The `/ArtBox` — the extent of the page's meaningful content
    /// (PDF 32000-1:2008 §14.11.2). Absent from the file it reads as
    /// the `/CropBox`, which is what the specification defines as its
    /// default. Mirrors the canonical `Page.ArtBox` getter.
    pub fn art_box(&self) -> Rectangle {
        let g = self.geometry.borrow();
        g.art_box.unwrap_or(g.crop_box)
    }

    /// Set the `/ArtBox`. Mirrors the canonical `Page.ArtBox` setter.
    pub fn set_art_box(&self, value: Rectangle) {
        let mut g = self.geometry.borrow_mut();
        g.art_box = Some(value);
        g.dirty = true;
    }

    /// The `/BleedBox` — the region the page is clipped to in a
    /// production environment (§14.11.2), defaulting to the `/CropBox`.
    /// Mirrors the canonical `Page.BleedBox` getter.
    pub fn bleed_box(&self) -> Rectangle {
        let g = self.geometry.borrow();
        g.bleed_box.unwrap_or(g.crop_box)
    }

    /// Set the `/BleedBox`. Mirrors the canonical `Page.BleedBox` setter.
    pub fn set_bleed_box(&self, value: Rectangle) {
        let mut g = self.geometry.borrow_mut();
        g.bleed_box = Some(value);
        g.dirty = true;
    }

    /// The `/TrimBox` — the intended finished dimensions after trimming
    /// (§14.11.2), defaulting to the `/CropBox`. Mirrors the canonical
    /// `Page.TrimBox` getter.
    pub fn trim_box(&self) -> Rectangle {
        let g = self.geometry.borrow();
        g.trim_box.unwrap_or(g.crop_box)
    }

    /// Set the `/TrimBox`. Mirrors the canonical `Page.TrimBox` setter.
    pub fn set_trim_box(&self, value: Rectangle) {
        let mut g = self.geometry.borrow_mut();
        g.trim_box = Some(value);
        g.dirty = true;
    }

    /// Page rotation as a [`Rotation`] quadrant. Reads the live
    /// geometry handle. Mirrors the canonical `Page.Rotate` getter.
    pub fn rotate(&self) -> Rotation {
        match self.geometry.borrow().rotate.rem_euclid(360) {
            90 => Rotation::On90,
            180 => Rotation::On180,
            270 => Rotation::On270,
            _ => Rotation::None,
        }
    }

    /// Set the page rotation. Marks the geometry dirty so the change
    /// persists on save. Mirrors the canonical `Page.Rotate` setter.
    pub fn set_rotate(&self, value: Rotation) {
        let mut g = self.geometry.borrow_mut();
        g.rotate = (value as i32) * 90;
        g.dirty = true;
    }

    /// Resize the page to `width` × `height` user-space units, anchored
    /// at the origin. Sets the `/MediaBox` to `0 0 width height` and the
    /// `/CropBox` to match. Mirrors the canonical
    /// `Page.SetPageSize(double, double)`.
    pub fn set_page_size(&self, width: f64, height: f64) {
        let r = Rectangle::new(0.0, 0.0, width, height, false);
        let mut g = self.geometry.borrow_mut();
        g.media_box = r;
        g.crop_box = r;
        g.dirty = true;
    }

    /// The page rectangle, optionally accounting for rotation. When
    /// `consider_rotation` is set and the page is rotated 90° or 270°,
    /// width and height are swapped. Mirrors the canonical
    /// `Page.GetPageRect(bool)`.
    pub fn get_page_rect(&self, consider_rotation: bool) -> Rectangle {
        let g = self.geometry.borrow();
        let base = g.crop_box;
        if consider_rotation {
            let quadrant = g.rotate.rem_euclid(360) / 90;
            if quadrant == 1 || quadrant == 3 {
                // 90° / 270°: present the page in landscape — the
                // width / height of the displayed rectangle swap.
                return Rectangle::new(
                    base.llx(),
                    base.lly(),
                    base.llx() + base.height(),
                    base.lly() + base.width(),
                    false,
                );
            }
        }
        base
    }

    /// Indirect-object id of the underlying `/Page` dict. Used
    /// by neighbouring public-API types (`TextAbsorber`,
    /// devices) to address the page back into the source PDF;
    /// not part of the canonical .NET surface.
    #[doc(hidden)]
    pub fn leaf_id(&self) -> u32 {
        self.leaf_id.get()
    }

    /// 0-based position in the document's page-tree leaf list —
    /// the index `internal::page_renderer::render` consumes.
    /// Mirrors `Page.InternalLeafIndex` in the C# / TypeScript
    /// peer tracks; not part of the canonical .NET surface.
    #[doc(hidden)]
    pub(crate) fn internal_leaf_index(&self) -> u64 {
        self.leaf_index
    }

    /// The leaf index, or
    /// [`Error::PageNotPersisted`](crate::Error::PageNotPersisted) when
    /// this page has none because it was added in this session.
    ///
    /// Every path that rasterises the document's bytes asks through
    /// here. `u64::MAX` is the sentinel `new_blank` sets, and handing it
    /// to the renderer produced an out-of-range complaint quoting a
    /// wrapped index — true but unreadable, and it told the caller
    /// nothing about what to do.
    pub(crate) fn require_persisted_leaf(&self) -> Result<u64, crate::public::error::Error> {
        if self.leaf_index == u64::MAX {
            return Err(crate::public::error::Error::PageNotPersisted);
        }
        Ok(self.leaf_index)
    }

    /// The page leaf's PDF object id — the key the object / resource
    /// parsers index by. Not part of the canonical .NET surface.
    #[doc(hidden)]
    pub(crate) fn internal_leaf_id(&self) -> u32 {
        self.leaf_id.get()
    }

    /// The shared leaf-id cell, for edits that outlive the value: an edit
    /// recorded against a page created in-session must resolve to the id
    /// the save path assigns later, not to the zero it reads today.
    pub(crate) fn leaf_ref(&self) -> Rc<Cell<u32>> {
        Rc::clone(&self.leaf_id)
    }

    /// Record the object id the save path emitted for this leaf.
    pub(crate) fn assign_leaf_id(&self, id: u32) {
        self.leaf_id.set(id);
    }

    /// Borrowed slice over the document's source bytes — the
    /// buffer `internal::page_renderer::render` reads. Mirrors
    /// `Page.InternalDocBytes` in the C# / TypeScript peer
    /// tracks; not part of the canonical .NET surface.
    #[doc(hidden)]
    pub(crate) fn internal_doc_bytes(&self) -> &[u8] {
        self.doc_bytes.as_ref()
    }
}

/// The largest rectangle with `w`:`h` proportions that fits inside
/// `rect`, centred on it. Used by [`Page::add_image`] when the caller
/// asks for the placement rectangle to be adjusted rather than the image
/// stretched into it.
fn fit_preserving_aspect(rect: [f64; 4], w: f64, h: f64) -> [f64; 4] {
    let (x0, x1) = (rect[0].min(rect[2]), rect[0].max(rect[2]));
    let (y0, y1) = (rect[1].min(rect[3]), rect[1].max(rect[3]));
    let (box_w, box_h) = (x1 - x0, y1 - y0);
    if box_w <= 0.0 || box_h <= 0.0 || w <= 0.0 || h <= 0.0 {
        return rect;
    }
    let scale = (box_w / w).min(box_h / h);
    let (fit_w, fit_h) = (w * scale, h * scale);
    let left = x0 + (box_w - fit_w) / 2.0;
    let bottom = y0 + (box_h - fit_h) / 2.0;
    [left, bottom, left + fit_w, bottom + fit_h]
}
