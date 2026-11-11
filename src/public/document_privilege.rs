//! `Aspose.Pdf.Facades.DocumentPrivilege` — a document-permission
//! set used by the security facades, per
//! `capabilities/document_privilege.yaml`.
//!
//! Carries the eight `Allow*` permission flags plus the three
//! coarse print / change / copy levels, the canonical preset
//! factories (`forbid_all` / `allow_all` / per-operation presets),
//! and `compare_to` for ordering two privilege sets by how much
//! they grant. The canonical `CompareTo(System.Object)` is
//! narrowed to take `&DocumentPrivilege` (the Rust analogue of the
//! `IComparable` contract).

/// A document-permission set.
///
/// A default-constructed value grants nothing (equivalent to
/// [`DocumentPrivilege::forbid_all`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct DocumentPrivilege {
    allow_print: bool,
    allow_degraded_printing: bool,
    allow_modify_contents: bool,
    allow_copy: bool,
    allow_modify_annotations: bool,
    allow_fill_in: bool,
    allow_screen_readers: bool,
    allow_assembly: bool,
    print_allow_level: i32,
    change_allow_level: i32,
    copy_allow_level: i32,
}

impl DocumentPrivilege {
    /// Construct an empty privilege set (grants nothing).
    pub fn new() -> Self {
        Self::default()
    }

    /// Whether high-resolution printing is allowed.
    pub fn allow_print(&self) -> bool {
        self.allow_print
    }
    /// Set whether high-resolution printing is allowed.
    pub fn set_allow_print(&mut self, value: bool) {
        self.allow_print = value;
    }

    /// Whether low-resolution (degraded) printing is allowed.
    pub fn allow_degraded_printing(&self) -> bool {
        self.allow_degraded_printing
    }
    /// Set whether low-resolution (degraded) printing is allowed.
    pub fn set_allow_degraded_printing(&mut self, value: bool) {
        self.allow_degraded_printing = value;
    }

    /// Whether modifying document contents is allowed.
    pub fn allow_modify_contents(&self) -> bool {
        self.allow_modify_contents
    }
    /// Set whether modifying document contents is allowed.
    pub fn set_allow_modify_contents(&mut self, value: bool) {
        self.allow_modify_contents = value;
    }

    /// Whether copying / extracting content is allowed.
    pub fn allow_copy(&self) -> bool {
        self.allow_copy
    }
    /// Set whether copying / extracting content is allowed.
    pub fn set_allow_copy(&mut self, value: bool) {
        self.allow_copy = value;
    }

    /// Whether modifying annotations is allowed.
    pub fn allow_modify_annotations(&self) -> bool {
        self.allow_modify_annotations
    }
    /// Set whether modifying annotations is allowed.
    pub fn set_allow_modify_annotations(&mut self, value: bool) {
        self.allow_modify_annotations = value;
    }

    /// Whether filling in form fields is allowed.
    pub fn allow_fill_in(&self) -> bool {
        self.allow_fill_in
    }
    /// Set whether filling in form fields is allowed.
    pub fn set_allow_fill_in(&mut self, value: bool) {
        self.allow_fill_in = value;
    }

    /// Whether screen-reader (accessibility) extraction is allowed.
    pub fn allow_screen_readers(&self) -> bool {
        self.allow_screen_readers
    }
    /// Set whether screen-reader extraction is allowed.
    pub fn set_allow_screen_readers(&mut self, value: bool) {
        self.allow_screen_readers = value;
    }

    /// Whether document assembly is allowed.
    pub fn allow_assembly(&self) -> bool {
        self.allow_assembly
    }
    /// Set whether document assembly is allowed.
    pub fn set_allow_assembly(&mut self, value: bool) {
        self.allow_assembly = value;
    }

    /// The coarse print-permission level (`0` none, `1` degraded,
    /// `2` full).
    pub fn print_allow_level(&self) -> i32 {
        self.print_allow_level
    }
    /// Set the coarse print-permission level.
    pub fn set_print_allow_level(&mut self, value: i32) {
        self.print_allow_level = value;
    }

    /// The coarse change-permission level.
    pub fn change_allow_level(&self) -> i32 {
        self.change_allow_level
    }
    /// Set the coarse change-permission level.
    pub fn set_change_allow_level(&mut self, value: i32) {
        self.change_allow_level = value;
    }

    /// The coarse copy-permission level.
    pub fn copy_allow_level(&self) -> i32 {
        self.copy_allow_level
    }
    /// Set the coarse copy-permission level.
    pub fn set_copy_allow_level(&mut self, value: i32) {
        self.copy_allow_level = value;
    }

    /// Preset: allow low-resolution (degraded) printing only.
    pub fn degraded_printing() -> DocumentPrivilege {
        let mut p = DocumentPrivilege::new();
        p.allow_degraded_printing = true;
        p.print_allow_level = 1;
        p
    }

    /// Preset: allow high-resolution printing.
    pub fn print() -> DocumentPrivilege {
        let mut p = DocumentPrivilege::new();
        p.allow_print = true;
        p.print_allow_level = 2;
        p
    }

    /// Preset: allow modifying document contents.
    pub fn modify_contents() -> DocumentPrivilege {
        let mut p = DocumentPrivilege::new();
        p.allow_modify_contents = true;
        p.change_allow_level = 2;
        p
    }

    /// Preset: allow copying / extracting content.
    pub fn copy() -> DocumentPrivilege {
        let mut p = DocumentPrivilege::new();
        p.allow_copy = true;
        p.copy_allow_level = 2;
        p
    }

    /// Preset: allow modifying annotations.
    pub fn modify_annotations() -> DocumentPrivilege {
        let mut p = DocumentPrivilege::new();
        p.allow_modify_annotations = true;
        p
    }

    /// Preset: allow filling in form fields.
    pub fn fill_in() -> DocumentPrivilege {
        let mut p = DocumentPrivilege::new();
        p.allow_fill_in = true;
        p
    }

    /// Preset: allow screen-reader extraction.
    pub fn screen_readers() -> DocumentPrivilege {
        let mut p = DocumentPrivilege::new();
        p.allow_screen_readers = true;
        p
    }

    /// Preset: allow document assembly.
    pub fn assembly() -> DocumentPrivilege {
        let mut p = DocumentPrivilege::new();
        p.allow_assembly = true;
        p
    }

    /// Preset: grant every permission at the maximum level.
    pub fn allow_all() -> DocumentPrivilege {
        DocumentPrivilege {
            allow_print: true,
            allow_degraded_printing: true,
            allow_modify_contents: true,
            allow_copy: true,
            allow_modify_annotations: true,
            allow_fill_in: true,
            allow_screen_readers: true,
            allow_assembly: true,
            print_allow_level: 2,
            change_allow_level: 2,
            copy_allow_level: 2,
        }
    }

    /// Preset: deny every permission (equivalent to [`new`]).
    ///
    /// [`new`]: DocumentPrivilege::new
    pub fn forbid_all() -> DocumentPrivilege {
        DocumentPrivilege::new()
    }

    /// Order this privilege set against `other` by how much it
    /// grants: returns a negative value when `self` grants less,
    /// zero when the two grant the same, and a positive value when
    /// `self` grants more.
    pub fn compare_to(&self, other: &DocumentPrivilege) -> i32 {
        self.magnitude().cmp(&other.magnitude()) as i32
    }

    /// A monotone privilege-magnitude key: more granted flags and
    /// higher levels rank higher. Used by [`compare_to`].
    ///
    /// [`compare_to`]: DocumentPrivilege::compare_to
    fn magnitude(&self) -> i32 {
        let flags = [
            self.allow_print,
            self.allow_degraded_printing,
            self.allow_modify_contents,
            self.allow_copy,
            self.allow_modify_annotations,
            self.allow_fill_in,
            self.allow_screen_readers,
            self.allow_assembly,
        ];
        let flag_count = flags.iter().filter(|f| **f).count() as i32;
        flag_count + self.print_allow_level + self.change_allow_level + self.copy_allow_level
    }
}
