//! `Aspose.Pdf.BaseParagraph` — the abstract base of the paragraph
//! types, per `capabilities/base_paragraph.yaml`.
//!
//! Canonically an abstract class (no public constructor) carrying
//! the layout-engine geometry shared by paragraphs. Rust has no
//! inheritance, so it is modelled as a trait that the concrete
//! paragraph types (e.g. [`FloatingBox`](crate::FloatingBox))
//! implement. The `ICloneable.Clone` member (returns
//! `System.Object`) is dropped in favour of the implementors'
//! derived `Clone`.

use crate::public::horizontal_alignment::HorizontalAlignment;
use crate::public::hyperlink::Hyperlink;
use crate::public::margin_info::MarginInfo;
use crate::public::vertical_alignment::VerticalAlignment;

pub(crate) mod sealed {
    use crate::public::page_edits::EditFont;

    /// A whole new page emitted by a paragraph's render (a broken
    /// [`Table`](crate::Table)'s overflow continuation page): the
    /// baked-in content-stream operators, the fonts they reference, and
    /// the page's media box `[llx, lly, urx, ury]`.
    pub(crate) struct RenderedPage {
        pub content: Vec<u8>,
        pub fonts: Vec<EditFont>,
        pub media_box: [f64; 4],
    }

    /// A paragraph's rendered page content: the content-stream
    /// operators for the host page, the font resources they reference,
    /// and any whole continuation pages the paragraph spills onto (empty
    /// for everything but a broken table that overflows its page).
    pub(crate) struct RenderedParagraph {
        pub content: Vec<u8>,
        pub fonts: Vec<EditFont>,
        pub continuations: Vec<RenderedPage>,
    }

    // ★ What a paragraph can and cannot declare, which is the reason
    // behind several "carried, not applied" notes in this layer.
    //
    // `render_content` is handed a page HEIGHT and nothing else: it does
    // not know its page, and it answers with content bytes plus font
    // resources. Fonts work only because `RenderedParagraph` has a slot
    // for them that the save path fills in. Anything else a paragraph
    // might want to add to its page — an image XObject for a cell
    // background, a link annotation over a hyperlink's box — has no such
    // slot and no page to be added to, so it cannot be declared from
    // here at all.
    //
    // Unblocking any of them is the same piece of work: a slot for
    // page-level additions alongside `fonts`, or a render that is given
    // its page. Each property names this note rather than guessing at a
    // reason of its own.

    /// Render a paragraph to a page's content on save, given the page
    /// height (for the top-origin → bottom-origin flip). `None` (the
    /// default) means the paragraph renders nothing — drawing
    /// [`Graph`](crate::Graph)s and [`TextFragment`](crate::TextFragment)s
    /// override it.
    pub(crate) trait Renderable {
        fn render_content(&self, _page_height: f64) -> Option<RenderedParagraph> {
            None
        }

        /// This paragraph as a text fragment, when it is one. A
        /// [`Table`](crate::Table) cell uses it to apply its
        /// `DefaultCellTextState` to the text it holds; everything else
        /// answers `None` and is rendered as it stands.
        fn as_text_fragment(&self) -> Option<&crate::public::text_fragment::TextFragment> {
            None
        }

        /// The width this paragraph's content occupies once rendered,
        /// when that is measurable. A [`Table`](crate::Table) cell uses
        /// it to align its content inside the cell; `None` — the
        /// default — means the content cannot be measured, and the
        /// caller places it at the left edge.
        fn rendered_width(&self) -> Option<f64> {
            None
        }
    }
}

/// The layout geometry common to paragraph types.
#[allow(private_bounds)]
pub trait BaseParagraph: sealed::Renderable {
    /// The vertical alignment within the layout column.
    fn vertical_alignment(&self) -> VerticalAlignment;
    /// Set the vertical alignment within the layout column.
    fn set_vertical_alignment(&mut self, value: VerticalAlignment);

    /// The horizontal alignment within the layout column.
    fn horizontal_alignment(&self) -> HorizontalAlignment;
    /// Set the horizontal alignment within the layout column.
    fn set_horizontal_alignment(&mut self, value: HorizontalAlignment);

    /// The margins applied around the paragraph.
    fn margin(&self) -> MarginInfo;
    /// Set the margins applied around the paragraph.
    fn set_margin(&mut self, value: MarginInfo);

    /// Whether this is the first paragraph in its column.
    /// **Carried, not applied.** These three are column- and
    /// page-flow hints, and paragraphs are placed at the coordinates
    /// they carry rather than flowed through columns, so there is no
    /// layout pass for a hint to influence.
    fn is_first_paragraph_in_column(&self) -> bool;
    /// Set whether this is the first paragraph in its column.
    fn set_is_first_paragraph_in_column(&mut self, value: bool);

    /// Whether this paragraph is kept together with the next.
    /// **Carried, not applied** — see
    /// [`is_first_paragraph_in_column`](Self::is_first_paragraph_in_column).
    fn is_kept_with_next(&self) -> bool;
    /// Set whether this paragraph is kept together with the next.
    fn set_is_kept_with_next(&mut self, value: bool);

    /// Whether this paragraph starts on a new page.
    ///
    /// Carried, not applied: starting a new page is the host's
    /// decision, and a paragraph render answers with content for the
    /// page it was given. A [`Table`](crate::Table) overflows onto
    /// continuation pages because it measures its own rows against the
    /// page height and declares them; a paragraph has nothing to
    /// measure a break against.
    fn is_in_new_page(&self) -> bool;
    /// Set whether this paragraph starts on a new page.
    fn set_is_in_new_page(&mut self, value: bool);

    /// Whether this paragraph is laid out inline.
    /// **Carried, not applied** — see
    /// [`is_first_paragraph_in_column`](Self::is_first_paragraph_in_column).
    fn is_in_line_paragraph(&self) -> bool;
    /// Set whether this paragraph is laid out inline.
    fn set_is_in_line_paragraph(&mut self, value: bool);

    /// The paragraph's hyperlink, if any.
    ///
    /// **Carried, not applied.** A clickable region is a link
    /// annotation, which is a page-level object, and a paragraph's
    /// render can declare only content bytes and font resources — it is
    /// not given its page (see the note on `RenderedParagraph`). Even
    /// with that slot it would need the box the paragraph ended up
    /// occupying, which a paragraph reports no more than its width.
    fn hyperlink(&self) -> Option<&dyn Hyperlink>;
    /// Set the paragraph's hyperlink.
    fn set_hyperlink(&mut self, value: Box<dyn Hyperlink>);

    /// The stacking (z) order of the paragraph.
    fn z_index(&self) -> i32;
    /// Set the stacking (z) order of the paragraph.
    fn set_z_index(&mut self, value: i32);
}

/// Shared storage for the [`BaseParagraph`] geometry. A concrete
/// paragraph type embeds this and delegates the trait accessors to
/// it (crate-internal). Carries the `hyperlink`, which keeps the
/// embedding type from deriving `Clone` / `PartialEq`.
#[derive(Default)]
pub(crate) struct ParagraphBase {
    pub vertical_alignment: VerticalAlignment,
    pub horizontal_alignment: HorizontalAlignment,
    pub margin: MarginInfo,
    pub is_first_paragraph_in_column: bool,
    pub is_kept_with_next: bool,
    pub is_in_new_page: bool,
    pub is_in_line_paragraph: bool,
    pub hyperlink: Option<Box<dyn Hyperlink>>,
    pub z_index: i32,
}

impl std::fmt::Debug for ParagraphBase {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ParagraphBase")
            .field("vertical_alignment", &self.vertical_alignment)
            .field("horizontal_alignment", &self.horizontal_alignment)
            .field("margin", &self.margin)
            .field("z_index", &self.z_index)
            .finish()
    }
}

/// Implement [`BaseParagraph`] for a type with a `base:
/// ParagraphBase` field by delegating every accessor.
macro_rules! impl_base_paragraph {
    ($ty:ty) => {
        impl $crate::public::base_paragraph::BaseParagraph for $ty {
            fn vertical_alignment(
                &self,
            ) -> $crate::public::vertical_alignment::VerticalAlignment {
                self.base.vertical_alignment
            }
            fn set_vertical_alignment(
                &mut self,
                value: $crate::public::vertical_alignment::VerticalAlignment,
            ) {
                self.base.vertical_alignment = value;
            }
            fn horizontal_alignment(
                &self,
            ) -> $crate::public::horizontal_alignment::HorizontalAlignment {
                self.base.horizontal_alignment
            }
            fn set_horizontal_alignment(
                &mut self,
                value: $crate::public::horizontal_alignment::HorizontalAlignment,
            ) {
                self.base.horizontal_alignment = value;
            }
            fn margin(&self) -> $crate::public::margin_info::MarginInfo {
                self.base.margin
            }
            fn set_margin(&mut self, value: $crate::public::margin_info::MarginInfo) {
                self.base.margin = value;
            }
            fn is_first_paragraph_in_column(&self) -> bool {
                self.base.is_first_paragraph_in_column
            }
            fn set_is_first_paragraph_in_column(&mut self, value: bool) {
                self.base.is_first_paragraph_in_column = value;
            }
            fn is_kept_with_next(&self) -> bool {
                self.base.is_kept_with_next
            }
            fn set_is_kept_with_next(&mut self, value: bool) {
                self.base.is_kept_with_next = value;
            }
            fn is_in_new_page(&self) -> bool {
                self.base.is_in_new_page
            }
            fn set_is_in_new_page(&mut self, value: bool) {
                self.base.is_in_new_page = value;
            }
            fn is_in_line_paragraph(&self) -> bool {
                self.base.is_in_line_paragraph
            }
            fn set_is_in_line_paragraph(&mut self, value: bool) {
                self.base.is_in_line_paragraph = value;
            }
            fn hyperlink(
                &self,
            ) -> Option<&dyn $crate::public::hyperlink::Hyperlink> {
                self.base.hyperlink.as_deref()
            }
            fn set_hyperlink(
                &mut self,
                value: Box<dyn $crate::public::hyperlink::Hyperlink>,
            ) {
                self.base.hyperlink = Some(value);
            }
            fn z_index(&self) -> i32 {
                self.base.z_index
            }
            fn set_z_index(&mut self, value: i32) {
                self.base.z_index = value;
            }
        }
    };
}
pub(crate) use impl_base_paragraph;

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default)]
    struct TestParagraph {
        valign: VerticalAlignment,
        halign: HorizontalAlignment,
        margin: MarginInfo,
        first: bool,
        kept: bool,
        new_page: bool,
        inline: bool,
        z: i32,
    }

    impl sealed::Renderable for TestParagraph {}

    impl BaseParagraph for TestParagraph {
        fn vertical_alignment(&self) -> VerticalAlignment {
            self.valign
        }
        fn set_vertical_alignment(&mut self, value: VerticalAlignment) {
            self.valign = value;
        }
        fn horizontal_alignment(&self) -> HorizontalAlignment {
            self.halign
        }
        fn set_horizontal_alignment(&mut self, value: HorizontalAlignment) {
            self.halign = value;
        }
        fn margin(&self) -> MarginInfo {
            self.margin
        }
        fn set_margin(&mut self, value: MarginInfo) {
            self.margin = value;
        }
        fn is_first_paragraph_in_column(&self) -> bool {
            self.first
        }
        fn set_is_first_paragraph_in_column(&mut self, value: bool) {
            self.first = value;
        }
        fn is_kept_with_next(&self) -> bool {
            self.kept
        }
        fn set_is_kept_with_next(&mut self, value: bool) {
            self.kept = value;
        }
        fn is_in_new_page(&self) -> bool {
            self.new_page
        }
        fn set_is_in_new_page(&mut self, value: bool) {
            self.new_page = value;
        }
        fn is_in_line_paragraph(&self) -> bool {
            self.inline
        }
        fn set_is_in_line_paragraph(&mut self, value: bool) {
            self.inline = value;
        }
        fn hyperlink(&self) -> Option<&dyn Hyperlink> {
            None
        }
        fn set_hyperlink(&mut self, _value: Box<dyn Hyperlink>) {}
        fn z_index(&self) -> i32 {
            self.z
        }
        fn set_z_index(&mut self, value: i32) {
            self.z = value;
        }
    }

    #[test]
    fn trait_is_implementable_and_accessors_roundtrip() {
        let mut p = TestParagraph::default();
        p.set_vertical_alignment(VerticalAlignment::Top);
        p.set_horizontal_alignment(HorizontalAlignment::Center);
        p.set_margin(MarginInfo::with_margins(1.0, 2.0, 3.0, 4.0));
        p.set_is_first_paragraph_in_column(true);
        p.set_is_kept_with_next(true);
        p.set_is_in_new_page(true);
        p.set_is_in_line_paragraph(true);
        p.set_z_index(5);

        assert_eq!(p.vertical_alignment(), VerticalAlignment::Top);
        assert_eq!(p.horizontal_alignment(), HorizontalAlignment::Center);
        assert_eq!(p.margin().top(), 4.0);
        assert!(p.is_first_paragraph_in_column());
        assert!(p.is_kept_with_next());
        assert!(p.is_in_new_page());
        assert!(p.is_in_line_paragraph());
        assert!(p.hyperlink().is_none());
        assert_eq!(p.z_index(), 5);
    }
}
