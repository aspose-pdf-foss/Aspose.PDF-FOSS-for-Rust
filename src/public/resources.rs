//! `Aspose.Pdf.Resources` — a page's resource dictionary, per
//! `capabilities/resources.yaml`.
//!
//! The shipped subset exposes the embedded [`XImageCollection`].
//! Reached canonically via [`Page::resources`](crate::Page::resources);
//! `Resources` is a `Clone`-shared handle (its `XImageCollection` is
//! interior-mutable), so a clone observes the same images.

use crate::public::x_image_collection::XImageCollection;

/// A page's resources.
#[derive(Debug, Default, Clone)]
pub struct Resources {
    images: XImageCollection,
}

impl Resources {
    /// Construct empty resources.
    pub fn new() -> Self {
        Self::default()
    }

    /// The embedded images.
    pub fn images(&self) -> &XImageCollection {
        &self.images
    }
    /// The embedded images, mutably.
    pub fn images_mut(&mut self) -> &mut XImageCollection {
        &mut self.images
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn holds_images() {
        let mut r = Resources::new();
        assert_eq!(r.images().count(), 0);
        let png = {
            let mut p = vec![0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
            p.extend_from_slice(&[0, 0, 0, 13]);
            p.extend_from_slice(b"IHDR");
            p.extend_from_slice(&[0, 0, 0, 2, 0, 0, 0, 2, 8, 6, 0, 0, 0]);
            p
        };
        r.images_mut().add_stream(&png);
        assert_eq!(r.images().count(), 1);
        assert_eq!(r.images().get(0).unwrap().width(), 2);
    }
}
