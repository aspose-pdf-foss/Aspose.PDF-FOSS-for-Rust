//! `Aspose.Pdf.Drawing.Graph` — a drawing canvas holding shapes, per
//! `capabilities/graph.yaml`.
//!
//! A [`BaseParagraph`](crate::BaseParagraph) of a fixed width /
//! height that owns a polymorphic list of [`Shape`]s. Added to a
//! page via the page's paragraph collection (page wiring +
//! render-on-save are a follow-on). The `ICloneable.Clone` member
//! and the redundant `f32` constructor are dropped.

use crate::public::base_paragraph::{impl_base_paragraph, ParagraphBase};
use crate::public::border_info::BorderInfo;
use crate::public::graph_info::GraphInfo;
use crate::public::shape::Shape;
use crate::public::text_fragment::TextFragment;

/// A drawing canvas owning a list of shapes.
#[derive(Debug)]
pub struct Graph {
    base: ParagraphBase,
    shapes: Vec<Box<dyn Shape>>,
    graph_info: GraphInfo,
    border: BorderInfo,
    is_change_position: bool,
    left: f64,
    top: f64,
    title: Option<TextFragment>,
    width: f64,
    height: f64,
}

impl Graph {
    /// Construct a graph canvas of the given width and height.
    pub fn new(width: f64, height: f64) -> Self {
        Self {
            base: ParagraphBase::default(),
            shapes: Vec::new(),
            graph_info: GraphInfo::new(),
            border: BorderInfo::new(),
            is_change_position: false,
            left: 0.0,
            top: 0.0,
            title: None,
            width,
            height,
        }
    }

    /// Append a shape to the canvas.
    pub fn add_shape(&mut self, shape: Box<dyn Shape>) {
        self.shapes.push(shape);
    }

    /// The canvas shapes.
    pub fn shapes(&self) -> &[Box<dyn Shape>] {
        &self.shapes
    }
    /// The canvas shapes, mutably.
    pub fn shapes_mut(&mut self) -> &mut Vec<Box<dyn Shape>> {
        &mut self.shapes
    }

    /// The default stroke / fill styling.
    pub fn graph_info(&self) -> &GraphInfo {
        &self.graph_info
    }
    /// Set the default stroke / fill styling.
    pub fn set_graph_info(&mut self, value: GraphInfo) {
        self.graph_info = value;
    }

    /// The canvas border.
    pub fn border(&self) -> &BorderInfo {
        &self.border
    }
    /// Set the canvas border.
    pub fn set_border(&mut self, value: BorderInfo) {
        self.border = value;
    }

    /// Whether the canvas may be repositioned during layout.
    pub fn is_change_position(&self) -> bool {
        self.is_change_position
    }
    /// Set whether the canvas may be repositioned during layout.
    pub fn set_is_change_position(&mut self, value: bool) {
        self.is_change_position = value;
    }

    /// The left position, in points.
    pub fn left(&self) -> f64 {
        self.left
    }
    /// Set the left position, in points.
    pub fn set_left(&mut self, value: f64) {
        self.left = value;
    }

    /// The top position, in points.
    pub fn top(&self) -> f64 {
        self.top
    }
    /// Set the top position, in points.
    pub fn set_top(&mut self, value: f64) {
        self.top = value;
    }

    /// The canvas title.
    pub fn title(&self) -> Option<&TextFragment> {
        self.title.as_ref()
    }
    /// Set the canvas title.
    pub fn set_title(&mut self, value: TextFragment) {
        self.title = Some(value);
    }

    /// The canvas width, in points.
    pub fn width(&self) -> f64 {
        self.width
    }
    /// Set the canvas width, in points.
    pub fn set_width(&mut self, value: f64) {
        self.width = value;
    }

    /// The canvas height, in points.
    pub fn height(&self) -> f64 {
        self.height
    }
    /// Set the canvas height, in points.
    pub fn set_height(&mut self, value: f64) {
        self.height = value;
    }
}

impl crate::public::base_paragraph::sealed::Renderable for Graph {
    /// Render the canvas's shapes as content-stream operators. The
    /// graph's local origin (bottom-left) is placed at
    /// `(left, page_height - top - height)` via a translate, and each
    /// shape is stroked / filled per its [`GraphInfo`].
    fn render_content(
        &self,
        page_height: f64,
    ) -> Option<crate::public::base_paragraph::sealed::RenderedParagraph> {
        use crate::public::shape::fmt_num;

        if self.shapes.is_empty() {
            return None;
        }
        let tx = self.left;
        let ty = page_height - self.top - self.height;

        let mut ops = Vec::new();
        ops.extend_from_slice(format!("q\n1 0 0 1 {} {} cm\n", fmt_num(tx), fmt_num(ty)).as_bytes());
        for shape in &self.shapes {
            let gi = shape.graph_info();
            ops.extend_from_slice(b"q\n");
            ops.extend_from_slice(format!("{} w\n", fmt_num(gi.line_width() as f64)).as_bytes());

            // §8.4.3.6: the dash pattern. An empty array is the default
            // solid line and needs no operator; anything else has to be
            // written, or the pattern the caller set never reaches the
            // file.
            let dash = gi.dash_array();
            if !dash.is_empty() {
                let pattern = dash
                    .iter()
                    .map(|n| fmt_num(*n as f64))
                    .collect::<Vec<_>>()
                    .join(" ");
                ops.extend_from_slice(
                    format!("[{}] {} d\n", pattern, fmt_num(gi.dash_phase() as f64)).as_bytes(),
                );
            }

            let stroke = gi.color();
            let has_stroke = !stroke.is_empty();
            if has_stroke {
                let (r, g, b) = stroke.rgb();
                ops.extend_from_slice(
                    format!("{} {} {} RG\n", fmt_num(r), fmt_num(g), fmt_num(b)).as_bytes(),
                );
            }
            let fill = gi.fill_color();
            let has_fill = !fill.is_empty();
            if has_fill {
                let (r, g, b) = fill.rgb();
                ops.extend_from_slice(
                    format!("{} {} {} rg\n", fmt_num(r), fmt_num(g), fmt_num(b)).as_bytes(),
                );
            }

            shape.render_path(&mut ops);

            // Paint: fill + stroke, fill only, or stroke (the default).
            let paint: &[u8] = match (has_fill, has_stroke) {
                (true, true) => b"B\n",
                (true, false) => b"f\n",
                _ => b"S\n",
            };
            ops.extend_from_slice(paint);
            ops.extend_from_slice(b"Q\n");
        }
        ops.extend_from_slice(b"Q\n");
        Some(crate::public::base_paragraph::sealed::RenderedParagraph {
            content: ops,
            fonts: Vec::new(),
            continuations: Vec::new(),
        })
    }
}

impl_base_paragraph!(Graph);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::public::base_paragraph::BaseParagraph;
    use crate::public::circle::Circle;
    use crate::public::drawing_rectangle::DrawingRectangle;
    use crate::public::line::Line;

    #[test]
    fn owns_shapes_polymorphically() {
        let mut graph = Graph::new(200.0, 150.0);
        assert_eq!(graph.width(), 200.0);
        graph.add_shape(Box::new(Line::new(vec![0.0, 0.0, 10.0, 10.0])));
        graph.add_shape(Box::new(DrawingRectangle::new(0.0, 0.0, 50.0, 50.0)));
        graph.add_shape(Box::new(Circle::new(25.0, 25.0, 10.0)));
        assert_eq!(graph.shapes().len(), 3);

        // Graph is a BaseParagraph.
        graph.set_z_index(5);
        assert_eq!(graph.z_index(), 5);

        // Mutate a shape's styling through the list.
        let mut gi = GraphInfo::new();
        gi.set_line_width(2.0);
        graph.shapes_mut()[0].set_graph_info(gi);
        assert_eq!(graph.shapes()[0].graph_info().line_width(), 2.0);
    }
}
