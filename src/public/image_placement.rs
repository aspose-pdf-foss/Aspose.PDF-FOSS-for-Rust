//! `Aspose.Pdf.ImagePlacement` — one drawn instance of an image on a
//! page, per `capabilities/image_placement.yaml`.
//!
//! Describes *where* an image XObject is painted: the transformation
//! [`matrix`](ImagePlacement::matrix) in effect at the draw, the
//! resulting page-space [`rectangle`](ImagePlacement::rectangle), the
//! [`rotation`](ImagePlacement::rotation) it carries, and the effective
//! [`resolution`](ImagePlacement::resolution) (the image's pixel count
//! over its placed size). The placed [`image`](ImagePlacement::image)
//! and its containing [`page`](ImagePlacement::page) are exposed too.
//!
//! Produced by [`ImagePlacementAbsorber`](crate::ImagePlacementAbsorber).
//! There is no public constructor. The `Operator` accessor and the `Hide` /
//! `Replace` / `Save` methods are dropped (see `targets/rust/drops.yaml`):
//! the accessor needs an uncataloged type, and the mutators need the
//! content-editing / rendering path, which the read-only placement walk
//! does not provide.

use crate::public::compositing_parameters::CompositingParameters;
use crate::public::matrix::Matrix;
use crate::public::page::Page;
use crate::public::rectangle::Rectangle;
use crate::public::resolution::Resolution;
use crate::public::x_image::XImage;

/// A single drawn placement of an image on a page.
#[derive(Debug, Clone)]
pub struct ImagePlacement {
    matrix: Matrix,
    rectangle: Rectangle,
    rotation: f32,
    resolution: Resolution,
    image: XImage,
    page: Page,
    compositing_parameters: CompositingParameters,
}

impl ImagePlacement {
    /// Assemble a placement from its computed geometry, image, page, and
    /// compositing state. Crate-internal — built by the absorber.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        matrix: Matrix,
        rectangle: Rectangle,
        rotation: f32,
        resolution: Resolution,
        image: XImage,
        page: Page,
        compositing_parameters: CompositingParameters,
    ) -> Self {
        Self { matrix, rectangle, rotation, resolution, image, page, compositing_parameters }
    }

    /// The transformation matrix in effect where the image is drawn.
    pub fn matrix(&self) -> Matrix {
        self.matrix
    }

    /// The image's page-space bounding rectangle.
    pub fn rectangle(&self) -> Rectangle {
        self.rectangle
    }

    /// The image's rotation, in degrees, derived from the matrix.
    pub fn rotation(&self) -> f32 {
        self.rotation
    }

    /// The effective resolution — the image's pixel dimensions over
    /// its placed size, in DPI.
    pub fn resolution(&self) -> Resolution {
        self.resolution
    }

    /// The placed image.
    pub fn image(&self) -> &XImage {
        &self.image
    }

    /// The page the image is placed on.
    pub fn page(&self) -> &Page {
        &self.page
    }

    /// The compositing state — blend mode, compression filter and mask
    /// flag — under which the image is drawn.
    pub fn compositing_parameters(&self) -> &CompositingParameters {
        &self.compositing_parameters
    }
}
