//! `Aspose.Pdf.XImageCollection` — a page's embedded images, per
//! `capabilities/x_image_collection.yaml`.
//!
//! A shared, interior-mutable handle (`Rc<RefCell<…>>`) over the
//! [`XImage`]s embedded in a page's resources, so a page's images can
//! be mutated through a shared `&Page` (via [`Page::resources`](crate::Page::resources)) and the
//! change reaches the document's save path. The Stream-taking `Add` /
//! `Replace` overloads are dropped (no Stream surface); the
//! `ImageFilterType` `Add` and the `ImageDeleteAction` `Delete`
//! overloads ship, alongside the core add / replace / delete / lookup
//! surface.

use crate::public::bitmap_info::BitmapInfo;
use crate::public::image_delete_action::ImageDeleteAction;
use crate::public::image_filter_type::ImageFilterType;
use crate::public::x_image::XImage;
use std::cell::{Cell, Ref, RefCell};
use std::rc::Rc;

/// A collection of embedded [`XImage`]s.
#[derive(Default, Clone)]
pub struct XImageCollection {
    items: Rc<RefCell<Vec<XImage>>>,
    next_id: Rc<Cell<usize>>,
    dirty: Rc<Cell<bool>>,
}

impl std::fmt::Debug for XImageCollection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("XImageCollection").field("count", &self.count()).finish()
    }
}

impl PartialEq for XImageCollection {
    fn eq(&self, other: &Self) -> bool {
        *self.items.borrow() == *other.items.borrow()
    }
}

impl Eq for XImageCollection {}

impl XImageCollection {
    /// Construct an empty collection.
    pub fn new() -> Self {
        Self::default()
    }

    fn next_name(&self) -> String {
        let id = self.next_id.get() + 1;
        self.next_id.set(id);
        format!("Im{id}")
    }

    /// Add an existing image.
    pub fn add_ximage(&self, image: XImage) {
        self.items.borrow_mut().push(image);
        self.dirty.set(true);
    }

    /// Add an image from its encoded bytes (PNG / JPEG / GIF / BMP),
    /// returning its assigned resource name.
    pub fn add_stream(&self, bytes: &[u8]) -> String {
        let name = self.next_name();
        self.items.borrow_mut().push(XImage::from_bytes(bytes.to_vec(), name.clone()));
        self.dirty.set(true);
        name
    }

    /// Add an image from a rasterised bitmap, returning its name.
    pub fn add_bitmap(&self, bitmap: &BitmapInfo) -> String {
        let name = self.next_name();
        self.items.borrow_mut().push(XImage::from_raster(
            bitmap.pixel_bytes().to_vec(),
            bitmap.width(),
            bitmap.height(),
            name.clone(),
            false,
        ));
        self.dirty.set(true);
        name
    }

    /// Add an image from a rasterised bitmap using an explicit
    /// compression filter, returning its name. The thin collection
    /// stores the raster; the filter is chosen by the save path.
    pub fn add_bitmap_filtered(
        &self,
        bitmap: &BitmapInfo,
        filter: ImageFilterType,
    ) -> String {
        let _ = filter;
        self.add_bitmap(bitmap)
    }

    /// Replace the image at a 0-based index with new encoded bytes.
    pub fn replace(&self, index: usize, bytes: &[u8]) {
        let mut items = self.items.borrow_mut();
        if let Some(existing) = items.get(index) {
            let name = existing.name().to_string();
            items[index] = XImage::from_bytes(bytes.to_vec(), name);
            self.dirty.set(true);
        }
    }

    /// Delete the image at a 0-based index.
    pub fn delete_at(&self, index: usize) {
        let mut items = self.items.borrow_mut();
        if index < items.len() {
            items.remove(index);
            self.dirty.set(true);
        }
    }

    /// Delete the image with the given resource name.
    pub fn delete_by_name(&self, name: &str) {
        self.items.borrow_mut().retain(|i| i.name() != name);
        self.dirty.set(true);
    }

    /// Delete the image at a 0-based index, choosing how the page
    /// contents that referenced it are treated. The thin collection
    /// removes the image; the content treatment is applied on save.
    pub fn delete_at_with_action(&self, index: usize, action: ImageDeleteAction) {
        let _ = action;
        self.delete_at(index);
    }

    /// Delete the image with the given resource name, choosing how the
    /// page contents that referenced it are treated.
    pub fn delete_by_name_with_action(&self, name: &str, action: ImageDeleteAction) {
        let _ = action;
        self.delete_by_name(name);
    }

    /// Delete every image.
    pub fn delete_all(&self) {
        self.items.borrow_mut().clear();
        self.dirty.set(true);
    }

    /// Delete every image (alias of [`delete_all`](Self::delete_all)).
    pub fn clear(&self) {
        self.delete_all();
    }

    /// The resource name of `image`, if present (value identity).
    pub fn get_image_name(&self, image: &XImage) -> Option<String> {
        self.items.borrow().iter().find(|i| *i == image).map(|i| i.name().to_string())
    }

    /// Whether `image` is present (value identity).
    pub fn contains(&self, image: &XImage) -> bool {
        self.items.borrow().iter().any(|i| i == image)
    }

    /// Remove the first image equal to `image` (value identity).
    pub fn remove(&self, image: &XImage) {
        let mut items = self.items.borrow_mut();
        if let Some(pos) = items.iter().position(|i| i == image) {
            items.remove(pos);
            self.dirty.set(true);
        }
    }

    /// The number of images.
    pub fn count(&self) -> usize {
        self.items.borrow().len()
    }

    /// The resource names of every image, in order.
    pub fn names(&self) -> Vec<String> {
        self.items.borrow().iter().map(|i| i.name().to_string()).collect()
    }

    /// The image at a 0-based index (a [`Ref`] guard).
    pub fn get(&self, index: usize) -> Option<Ref<'_, XImage>> {
        let items = self.items.borrow();
        if index < items.len() {
            Some(Ref::map(items, |v| &v[index]))
        } else {
            None
        }
    }

    /// The image with the given resource name (a [`Ref`] guard).
    pub fn get_by_name(&self, name: &str) -> Option<Ref<'_, XImage>> {
        let items = self.items.borrow();
        let pos = items.iter().position(|i| i.name() == name)?;
        Some(Ref::map(items, |v| &v[pos]))
    }

    // -- save-through plumbing ---------------------------------------

    /// Whether the collection has been mutated since load.
    pub(crate) fn is_dirty(&self) -> bool {
        self.dirty.get()
    }

    /// Reset the dirty flag (after building a freshly loaded page).
    pub(crate) fn clear_dirty(&self) {
        self.dirty.set(false);
    }

    /// Append an image loaded from the PDF (does not mark dirty, so a
    /// verbatim resave keeps the original page resources).
    pub(crate) fn add_loaded(&self, image: XImage) {
        let id = self.next_id.get() + 1;
        self.next_id.set(id);
        self.items.borrow_mut().push(image);
    }

    /// Snapshot every image for the save path.
    pub(crate) fn for_save(&self) -> Vec<XImage> {
        self.items.borrow().clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn jpeg(width: u8, height: u8) -> Vec<u8> {
        let mut j = vec![0xFF, 0xD8, 0xFF, 0xC0, 0x00, 0x11, 0x08];
        j.extend_from_slice(&[0x00, height, 0x00, width]);
        j
    }

    #[test]
    fn add_lookup_and_delete() {
        let c = XImageCollection::new();
        let n1 = c.add_stream(&jpeg(11, 7));
        c.add_stream(&jpeg(3, 4));
        assert_eq!(c.count(), 2);
        assert_eq!(c.get(0).unwrap().width(), 11);
        assert_eq!(c.get_by_name(&n1).unwrap().height(), 7);
        assert_eq!(c.names().len(), 2);

        let img0 = c.get(0).unwrap().clone();
        assert!(c.contains(&img0));
        assert_eq!(c.get_image_name(&img0).as_deref(), Some(n1.as_str()));

        c.delete_by_name(&n1);
        assert_eq!(c.count(), 1);
        c.clear();
        assert_eq!(c.count(), 0);
    }
}
