//! `Aspose.Pdf.Facades.PdfFileEditor.PageBreak` — a page-break marker
//! for the file editor's page-break insertion, per
//! `capabilities/page_break.yaml`.
//!
//! A small value type: the 1-based page number a break falls on and the
//! vertical position of the break on that page.

/// A page-break marker: where a break is inserted on a page.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PageBreak {
    page_number: i32,
    position: f64,
}

impl PageBreak {
    /// Construct a page break on `page_number` at vertical `position`.
    pub fn new(page_number: i32, position: f64) -> Self {
        Self { page_number, position }
    }

    /// The 1-based page number the break falls on.
    pub fn page_number(&self) -> i32 {
        self.page_number
    }
    /// Set the page number the break falls on.
    pub fn set_page_number(&mut self, value: i32) {
        self.page_number = value;
    }

    /// The vertical position of the break on the page.
    pub fn position(&self) -> f64 {
        self.position
    }
    /// Set the vertical position of the break on the page.
    pub fn set_position(&mut self, value: f64) {
        self.position = value;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constructs_and_reads() {
        let b = PageBreak::new(3, 120.5);
        assert_eq!(b.page_number(), 3);
        assert_eq!(b.position(), 120.5);
    }

    #[test]
    fn setters_round_trip() {
        let mut b = PageBreak::new(1, 0.0);
        b.set_page_number(7);
        b.set_position(42.0);
        assert_eq!(b.page_number(), 7);
        assert_eq!(b.position(), 42.0);
    }
}
