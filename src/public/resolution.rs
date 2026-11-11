//! `Aspose.Pdf.Devices.Resolution` — output resolution in DPI for
//! every Aspose.Pdf.Devices rendering device.
//!
//! Two construction shapes mirror the canonical .NET API:
//! [`Resolution::new`] (square — both axes share one DPI) and
//! [`Resolution::with_axes`] (independent horizontal / vertical
//! DPI). Both reject non-positive values with
//! [`Error::InvalidResolution`].

use super::error::Error;

/// Output resolution in dots-per-inch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Resolution {
    x: i32,
    y: i32,
}

impl Resolution {
    /// Construct a square resolution where both axes share
    /// `value` DPI. Rejects `value <= 0` with
    /// [`Error::InvalidResolution`].
    pub fn new(value: i32) -> Result<Self, Error> {
        if value <= 0 {
            return Err(Error::InvalidResolution(value as i64));
        }
        Ok(Self { x: value, y: value })
    }

    /// Construct a resolution with distinct horizontal and
    /// vertical DPI. Rejects any non-positive axis with
    /// [`Error::InvalidResolution`].
    pub fn with_axes(value_x: i32, value_y: i32) -> Result<Self, Error> {
        if value_x <= 0 {
            return Err(Error::InvalidResolution(value_x as i64));
        }
        if value_y <= 0 {
            return Err(Error::InvalidResolution(value_y as i64));
        }
        Ok(Self { x: value_x, y: value_y })
    }

    /// Horizontal DPI as set at construction.
    pub fn x(&self) -> i32 {
        self.x
    }

    /// Vertical DPI as set at construction.
    pub fn y(&self) -> i32 {
        self.y
    }
}
