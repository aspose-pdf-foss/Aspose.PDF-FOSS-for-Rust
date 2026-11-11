//! `Aspose.Pdf.Drawing.Shape` — the base of the drawing shapes, per
//! `capabilities/shape.yaml`.
//!
//! Rust has no inheritance, so `Shape` is a trait the concrete
//! shapes ([`Line`](crate::Line), [`DrawingRectangle`](crate::DrawingRectangle),
//! [`Circle`](crate::Circle), [`Ellipse`](crate::Ellipse)) implement
//! via a shared `ShapeBase`. It carries the stroke / fill
//! [`GraphInfo`] and an optional text label, plus the geometry-
//! specific `check_bounds`.

use crate::public::graph_info::GraphInfo;
use crate::public::text_fragment::TextFragment;

/// Shared styling for a [`Shape`] (crate-internal).
#[derive(Debug, Default)]
pub(crate) struct ShapeBase {
    pub graph_info: GraphInfo,
    pub text: Option<TextFragment>,
}

pub(crate) mod sealed {
    use super::ShapeBase;
    pub(crate) trait HasShapeBase {
        fn shape_base(&self) -> &ShapeBase;
        fn shape_base_mut(&mut self) -> &mut ShapeBase;
    }

    /// Render a shape to PDF path-construction operators (no paint
    /// operator — the owning graph applies stroke / fill). Coordinates
    /// are the shape's own, in the graph's local space (y up).
    pub(crate) trait ShapeRender {
        fn render_path(&self, ops: &mut Vec<u8>);
    }
}

/// A drawable shape with stroke / fill styling.
#[allow(private_bounds)]
pub trait Shape: sealed::HasShapeBase + sealed::ShapeRender + std::fmt::Debug {
    /// Whether the shape fits within a `container_width` ×
    /// `container_height` box (origin at the lower-left).
    fn check_bounds(&self, container_width: f64, container_height: f64) -> bool;

    /// The stroke / fill styling.
    fn graph_info(&self) -> &GraphInfo {
        &self.shape_base().graph_info
    }
    /// Set the stroke / fill styling.
    fn set_graph_info(&mut self, value: GraphInfo) {
        self.shape_base_mut().graph_info = value;
    }

    /// The optional text label drawn with the shape.
    fn text(&self) -> Option<&TextFragment> {
        self.shape_base().text.as_ref()
    }
    /// Set the text label drawn with the shape.
    fn set_text(&mut self, value: TextFragment) {
        self.shape_base_mut().text = Some(value);
    }
}

/// Format a coordinate as a PDF real (up to 3 fractional digits,
/// trailing zeros trimmed) for a content stream.
pub(crate) fn fmt_num(v: f64) -> String {
    let mut s = format!("{v:.3}");
    if s.contains('.') {
        while s.ends_with('0') {
            s.pop();
        }
        if s.ends_with('.') {
            s.pop();
        }
    }
    s
}

/// Circle / ellipse Bézier constant: control-point offset as a
/// fraction of the radius for a quarter-arc approximation.
pub(crate) const KAPPA: f64 = 0.552_284_749_830_793_4;

/// Append the four-cubic-Bézier path of an ellipse centred at
/// `(cx, cy)` with radii `(rx, ry)`, starting and closing at the right
/// vertex.
pub(crate) fn ellipse_path(ops: &mut Vec<u8>, cx: f64, cy: f64, rx: f64, ry: f64) {
    let kx = rx * KAPPA;
    let ky = ry * KAPPA;
    let m = |x: f64, y: f64| format!("{} {} m\n", fmt_num(x), fmt_num(y));
    let c = |x1: f64, y1: f64, x2: f64, y2: f64, x3: f64, y3: f64| {
        format!(
            "{} {} {} {} {} {} c\n",
            fmt_num(x1),
            fmt_num(y1),
            fmt_num(x2),
            fmt_num(y2),
            fmt_num(x3),
            fmt_num(y3)
        )
    };
    ops.extend_from_slice(m(cx + rx, cy).as_bytes());
    ops.extend_from_slice(c(cx + rx, cy + ky, cx + kx, cy + ry, cx, cy + ry).as_bytes());
    ops.extend_from_slice(c(cx - kx, cy + ry, cx - rx, cy + ky, cx - rx, cy).as_bytes());
    ops.extend_from_slice(c(cx - rx, cy - ky, cx - kx, cy - ry, cx, cy - ry).as_bytes());
    ops.extend_from_slice(c(cx + kx, cy - ry, cx + rx, cy - ky, cx + rx, cy).as_bytes());
}

/// Implement [`sealed::HasShapeBase`] for a shape with a `base:
/// ShapeBase` field.
macro_rules! impl_has_shape_base {
    ($ty:ty) => {
        impl $crate::public::shape::sealed::HasShapeBase for $ty {
            fn shape_base(&self) -> &$crate::public::shape::ShapeBase {
                &self.base
            }
            fn shape_base_mut(&mut self) -> &mut $crate::public::shape::ShapeBase {
                &mut self.base
            }
        }
    };
}
pub(crate) use impl_has_shape_base;
