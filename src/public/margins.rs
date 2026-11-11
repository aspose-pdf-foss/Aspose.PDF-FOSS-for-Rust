//! `Aspose.Pdf.Devices.Margins` — value-type carrier for
//! left / right / top / bottom margins, in pixels.
//!
//! Carried by [`TiffSettings`](crate::TiffSettings) and not applied:
//! insetting the page would mean rendering onto a canvas larger than
//! the page and placing the raster within it, and the shared renderer
//! contract answers with a raster the size of the page it was given.
//! The four sides are stored and read back exactly.
//!
//! Surface mirrors the canonical .NET API per
//! `capabilities/margins.yaml`: default ctor (zero on all four
//! sides), four-arg ctor, and a read-write getter/setter pair for
//! each side.

/// Page-margin carrier in pixels.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Margins {
    left: i32,
    right: i32,
    top: i32,
    bottom: i32,
}

impl Margins {
    /// Default-construct with zero margins on all four sides.
    pub fn new() -> Self {
        Self::default()
    }

    /// Construct with explicit left, right, top, and bottom
    /// margins (in pixels).
    pub fn with_sides(left: i32, right: i32, top: i32, bottom: i32) -> Self {
        Self { left, right, top, bottom }
    }

    /// Left margin in pixels.
    pub fn left(&self) -> i32 {
        self.left
    }

    /// Right margin in pixels.
    pub fn right(&self) -> i32 {
        self.right
    }

    /// Top margin in pixels.
    pub fn top(&self) -> i32 {
        self.top
    }

    /// Bottom margin in pixels.
    pub fn bottom(&self) -> i32 {
        self.bottom
    }

    /// Replace the left margin. See [`Margins::left`].
    pub fn set_left(&mut self, value: i32) {
        self.left = value;
    }

    /// Replace the right margin. See [`Margins::right`].
    pub fn set_right(&mut self, value: i32) {
        self.right = value;
    }

    /// Replace the top margin. See [`Margins::top`].
    pub fn set_top(&mut self, value: i32) {
        self.top = value;
    }

    /// Replace the bottom margin. See [`Margins::bottom`].
    pub fn set_bottom(&mut self, value: i32) {
        self.bottom = value;
    }
}
