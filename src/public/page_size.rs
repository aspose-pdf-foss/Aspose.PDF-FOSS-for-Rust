//! `Aspose.Pdf.PageSize` — value-type carrier for page dimensions
//! in PDF user-space units (1 unit = 1/72 inch).
//!
//! Twelve named-page-size constants (`A0`..`A6`, `B5`,
//! [`PageSize::page_letter`], [`PageSize::page_legal`],
//! [`PageSize::page_ledger`], [`PageSize::p11x17`]) carry the
//! canonical Aspose.PDF 26.4.0 dimensions extracted via apidev's
//! `--probe-static-values` runtime probe — same source as the C# /
//! TypeScript peer tracks. Surface mirrors
//! `capabilities/page_size.yaml`.

/// Page dimensions in PDF user-space units (1 unit = 1/72 inch).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PageSize {
    width: f32,
    height: f32,
}

impl PageSize {
    /// Construct a [`PageSize`] from explicit `width` (`x`) and
    /// `height` (`y`) in user-space units.
    pub fn new(x: f32, y: f32) -> Self {
        Self { width: x, height: y }
    }

    /// Width in user-space units.
    pub fn width(&self) -> f32 {
        self.width
    }

    /// Height in user-space units.
    pub fn height(&self) -> f32 {
        self.height
    }

    /// Replace the width. See [`PageSize::width`].
    pub fn set_width(&mut self, value: f32) {
        self.width = value;
    }

    /// Replace the height. See [`PageSize::height`].
    pub fn set_height(&mut self, value: f32) {
        self.height = value;
    }

    /// `true` when [`PageSize::width`] > [`PageSize::height`].
    pub fn is_landscape(&self) -> bool {
        self.width > self.height
    }

    /// Toggle orientation. When `value` differs from
    /// [`PageSize::is_landscape`], swaps width and height; otherwise
    /// a no-op. Mirrors the canonical `IsLandscape` setter.
    pub fn set_is_landscape(&mut self, value: bool) {
        if value != self.is_landscape() {
            std::mem::swap(&mut self.width, &mut self.height);
        }
    }

    /// A0 — 2384 × 3370 user-space units.
    pub const fn a0() -> Self {
        Self { width: 2384.0, height: 3370.0 }
    }

    /// A1 — 1684 × 2384 user-space units.
    pub const fn a1() -> Self {
        Self { width: 1684.0, height: 2384.0 }
    }

    /// A2 — 1190 × 1684 user-space units.
    pub const fn a2() -> Self {
        Self { width: 1190.0, height: 1684.0 }
    }

    /// A3 — 842 × 1190 user-space units.
    pub const fn a3() -> Self {
        Self { width: 842.0, height: 1190.0 }
    }

    /// A4 — 595 × 842 user-space units.
    pub const fn a4() -> Self {
        Self { width: 595.0, height: 842.0 }
    }

    /// A5 — 421 × 595 user-space units.
    pub const fn a5() -> Self {
        Self { width: 421.0, height: 595.0 }
    }

    /// A6 — 297 × 421 user-space units.
    pub const fn a6() -> Self {
        Self { width: 297.0, height: 421.0 }
    }

    /// B5 — 501 × 709 user-space units.
    pub const fn b5() -> Self {
        Self { width: 501.0, height: 709.0 }
    }

    /// US Letter — 612 × 792 user-space units.
    pub const fn page_letter() -> Self {
        Self { width: 612.0, height: 792.0 }
    }

    /// US Legal — 612 × 1008 user-space units.
    pub const fn page_legal() -> Self {
        Self { width: 612.0, height: 1008.0 }
    }

    /// US Ledger — 1224 × 792 user-space units (landscape).
    pub const fn page_ledger() -> Self {
        Self { width: 1224.0, height: 792.0 }
    }

    /// 11x17 — 792 × 1224 user-space units (portrait Ledger).
    pub const fn p11x17() -> Self {
        Self { width: 792.0, height: 1224.0 }
    }
}
