//! `Aspose.Pdf.Facades.Bookmark` — a flat bookmark descriptor, per
//! `capabilities/bookmark.yaml`.
//!
//! A value type carrying a bookmark's title, target, display, style
//! flags, and child bookmarks. The `CustomAcorbatViewerMenuActionName`
//! (`System.Enum[]`) and `TitleColor` (`System.Drawing.Color`)
//! properties are dropped.

use crate::public::bookmarks::Bookmarks;

/// A bookmark (outline entry) descriptor.
#[derive(Debug, Clone)]
pub struct Bookmark {
    action: String,
    bold_flag: bool,
    child_items: Bookmarks,
    destination: String,
    italic_flag: bool,
    level: i32,
    page_display: String,
    page_display_bottom: i32,
    page_display_left: i32,
    page_display_right: i32,
    page_display_top: i32,
    page_display_zoom: i32,
    page_number: i32,
    remote_file: String,
    title: String,
    open: bool,
}

impl Default for Bookmark {
    fn default() -> Self {
        Self {
            action: String::new(),
            bold_flag: false,
            child_items: Bookmarks::new(),
            destination: String::new(),
            italic_flag: false,
            level: 1,
            page_display: String::new(),
            page_display_bottom: 0,
            page_display_left: 0,
            page_display_right: 0,
            page_display_top: 0,
            page_display_zoom: 0,
            page_number: 1,
            remote_file: String::new(),
            title: String::new(),
            open: false,
        }
    }
}

macro_rules! str_prop {
    ($g:ident, $s:ident, $f:ident, $doc:literal) => {
        #[doc = $doc]
        pub fn $g(&self) -> &str { &self.$f }
        #[doc = concat!("Set ", $doc)]
        pub fn $s(&mut self, value: impl Into<String>) { self.$f = value.into(); }
    };
}
macro_rules! int_prop {
    ($g:ident, $s:ident, $f:ident, $doc:literal) => {
        #[doc = $doc]
        pub fn $g(&self) -> i32 { self.$f }
        #[doc = concat!("Set ", $doc)]
        pub fn $s(&mut self, value: i32) { self.$f = value; }
    };
}
macro_rules! bool_prop {
    ($g:ident, $s:ident, $f:ident, $doc:literal) => {
        #[doc = $doc]
        pub fn $g(&self) -> bool { self.$f }
        #[doc = concat!("Set ", $doc)]
        pub fn $s(&mut self, value: bool) { self.$f = value; }
    };
}

impl Bookmark {
    /// Construct a default bookmark (page 1, level 1).
    pub fn new() -> Self {
        Self::default()
    }

    str_prop!(action, set_action, action, "the action verb (e.g. `GoTo`).");
    str_prop!(destination, set_destination, destination, "the named destination.");
    str_prop!(page_display, set_page_display, page_display, "the page-display mode.");
    str_prop!(remote_file, set_remote_file, remote_file, "the remote-file path.");
    str_prop!(title, set_title, title, "the bookmark title.");

    bool_prop!(bold_flag, set_bold_flag, bold_flag, "the bold style flag.");
    bool_prop!(italic_flag, set_italic_flag, italic_flag, "the italic style flag.");
    bool_prop!(open, set_open, open, "whether the bookmark is expanded.");

    int_prop!(level, set_level, level, "the nesting level (1-based).");
    int_prop!(page_number, set_page_number, page_number, "the target page number.");
    int_prop!(page_display_bottom, set_page_display_bottom, page_display_bottom, "the display bottom coordinate.");
    int_prop!(page_display_left, set_page_display_left, page_display_left, "the display left coordinate.");
    int_prop!(page_display_right, set_page_display_right, page_display_right, "the display right coordinate.");
    int_prop!(page_display_top, set_page_display_top, page_display_top, "the display top coordinate.");
    int_prop!(page_display_zoom, set_page_display_zoom, page_display_zoom, "the display zoom percentage.");

    /// The child bookmarks.
    pub fn child_items(&self) -> &Bookmarks {
        &self.child_items
    }
    /// The child bookmarks, mutably.
    pub fn child_items_mut(&mut self) -> &mut Bookmarks {
        &mut self.child_items
    }
    /// Replace the child bookmarks.
    pub fn set_child_items(&mut self, value: Bookmarks) {
        self.child_items = value;
    }
    /// The child bookmarks (alias of [`child_items`](Self::child_items)).
    pub fn child_item(&self) -> &Bookmarks {
        &self.child_items
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_and_roundtrip() {
        let mut bm = Bookmark::new();
        assert!(bm.title().is_empty());
        assert_eq!(bm.page_number(), 1);
        assert_eq!(bm.level(), 1);
        assert!(!bm.bold_flag() && !bm.italic_flag() && !bm.open());

        bm.set_title("Chapter 1");
        bm.set_page_number(5);
        bm.set_level(2);
        bm.set_bold_flag(true);
        bm.set_open(true);
        bm.set_action("GoTo");
        bm.set_page_display_left(10);
        assert_eq!(bm.title(), "Chapter 1");
        assert_eq!(bm.page_number(), 5);
        assert_eq!(bm.level(), 2);
        assert!(bm.bold_flag() && bm.open());
        assert_eq!(bm.action(), "GoTo");
        assert_eq!(bm.page_display_left(), 10);
    }

    #[test]
    fn child_items_roundtrip() {
        let mut parent = Bookmark::new();
        parent.set_title("Parent");
        let mut child = Bookmark::new();
        child.set_title("Child");
        parent.child_items_mut().add(child);
        assert_eq!(parent.child_items().count(), 1);
        assert_eq!(parent.child_item().get(0).unwrap().title(), "Child");
    }
}
