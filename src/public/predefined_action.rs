//! `Aspose.Pdf.Annotations.PredefinedAction` — a built-in viewer
//! menu / navigation action, per `capabilities/predefined_action.yaml`.
//!
//! Variant ordinals match the canonical Aspose.PDF 26.4.0
//! `Aspose.Pdf.Annotations.PredefinedAction` enum. The
//! [`std::fmt::Display`] rendering is the variant identifier — the
//! name a [`NamedAction`](crate::NamedAction) carries.

use std::fmt;

/// A built-in viewer action invoked by name (a `/Named` action).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(i32)]
#[allow(non_camel_case_types)]
pub enum PredefinedAction {
    #[default]
    /// `FirstPage` predefined viewer action.
    FirstPage = 0,
    /// `LastPage` predefined viewer action.
    LastPage = 1,
    /// `NextPage` predefined viewer action.
    NextPage = 2,
    /// `PrevPage` predefined viewer action.
    PrevPage = 3,
    /// `PrintDialog` predefined viewer action.
    PrintDialog = 4,
    /// `Print` predefined viewer action.
    Print = 5,
    /// `Bookmarks_ExpanCurrentBookmark` predefined viewer action.
    Bookmarks_ExpanCurrentBookmark = 6,
    /// `Bookmarks_HightlightCurrentBookmark` predefined viewer action.
    Bookmarks_HightlightCurrentBookmark = 7,
    /// `Document_AttachFile` predefined viewer action.
    Document_AttachFile = 8,
    /// `Document_CropPages` predefined viewer action.
    Document_CropPages = 9,
    /// `Document_DeletePages` predefined viewer action.
    Document_DeletePages = 10,
    /// `Document_ExtractPages` predefined viewer action.
    Document_ExtractPages = 11,
    /// `Document_InsertPages` predefined viewer action.
    Document_InsertPages = 12,
    /// `Document_ReplacePages` predefined viewer action.
    Document_ReplacePages = 13,
    /// `Document_RotatePages` predefined viewer action.
    Document_RotatePages = 14,
    /// `Edit_CheckSpelling_InComFieldEdit` predefined viewer action.
    Edit_CheckSpelling_InComFieldEdit = 15,
    /// `Edit_Find` predefined viewer action.
    Edit_Find = 16,
    /// `Edit_Preferences` predefined viewer action.
    Edit_Preferences = 17,
    /// `Edit_Search` predefined viewer action.
    Edit_Search = 18,
    /// `File_AttachToEmail` predefined viewer action.
    File_AttachToEmail = 19,
    /// `File_Close` predefined viewer action.
    File_Close = 20,
    /// `File_CreatePDF_FromScanner` predefined viewer action.
    File_CreatePDF_FromScanner = 21,
    /// `File_CreatePDF_FromWebPage` predefined viewer action.
    File_CreatePDF_FromWebPage = 22,
    /// `File_Exit` predefined viewer action.
    File_Exit = 23,
    /// `File_Organizer_OpenOrganizer` predefined viewer action.
    File_Organizer_OpenOrganizer = 24,
    /// `File_Print` predefined viewer action.
    File_Print = 25,
    /// `File_Properties` predefined viewer action.
    File_Properties = 26,
    /// `File_SaveAs` predefined viewer action.
    File_SaveAs = 27,
    /// `Miscellaneous_ZoomIn` predefined viewer action.
    Miscellaneous_ZoomIn = 28,
    /// `Miscellaneous_ZoomOut` predefined viewer action.
    Miscellaneous_ZoomOut = 29,
    /// `PageImages_PrintPages` predefined viewer action.
    PageImages_PrintPages = 30,
    /// `View_GoTo_NextView` predefined viewer action.
    View_GoTo_NextView = 31,
    /// `View_GoTo_Page` predefined viewer action.
    View_GoTo_Page = 32,
    /// `View_GoTo_PreDocument` predefined viewer action.
    View_GoTo_PreDocument = 33,
    /// `View_GoTo_PreView` predefined viewer action.
    View_GoTo_PreView = 34,
    /// `View_NavigationPanels_Articles` predefined viewer action.
    View_NavigationPanels_Articles = 35,
    /// `View_NavigationPanels_Attachments` predefined viewer action.
    View_NavigationPanels_Attachments = 36,
    /// `View_NavigationPanels_Boomarks` predefined viewer action.
    View_NavigationPanels_Boomarks = 37,
    /// `View_NavigationPanels_Comments` predefined viewer action.
    View_NavigationPanels_Comments = 38,
    /// `View_NavigationPanels_Fields` predefined viewer action.
    View_NavigationPanels_Fields = 39,
    /// `View_NavigationPanels_Layers` predefined viewer action.
    View_NavigationPanels_Layers = 40,
    /// `View_NavigationPanels_ModelTree` predefined viewer action.
    View_NavigationPanels_ModelTree = 41,
    /// `View_NavigationPanels_Pages` predefined viewer action.
    View_NavigationPanels_Pages = 42,
    /// `View_NavigationPanels_Signatures` predefined viewer action.
    View_NavigationPanels_Signatures = 43,
    /// `View_PageDisplay_SinglePage` predefined viewer action.
    View_PageDisplay_SinglePage = 44,
    /// `View_PageDisplay_SinglePageContinuous` predefined viewer action.
    View_PageDisplay_SinglePageContinuous = 45,
    /// `View_PageDisplay_TwoUp` predefined viewer action.
    View_PageDisplay_TwoUp = 46,
    /// `View_PageDisplay_TwoUpContinuous` predefined viewer action.
    View_PageDisplay_TwoUpContinuous = 47,
    /// `View_Toolbars_AdvanceEditing` predefined viewer action.
    View_Toolbars_AdvanceEditing = 48,
    /// `View_Toolbars_CommentMarkup` predefined viewer action.
    View_Toolbars_CommentMarkup = 49,
    /// `View_Toolbars_Edit` predefined viewer action.
    View_Toolbars_Edit = 50,
    /// `View_Toolbars_File` predefined viewer action.
    View_Toolbars_File = 51,
    /// `View_Toolbars_Find` predefined viewer action.
    View_Toolbars_Find = 52,
    /// `View_Toolbars_Forms` predefined viewer action.
    View_Toolbars_Forms = 53,
    /// `View_Toolbars_Measuring` predefined viewer action.
    View_Toolbars_Measuring = 54,
    /// `View_Toolbars_ObjectData` predefined viewer action.
    View_Toolbars_ObjectData = 55,
    /// `View_Toolbars_PageDisplay` predefined viewer action.
    View_Toolbars_PageDisplay = 56,
    /// `View_Toolbars_PageNavigation` predefined viewer action.
    View_Toolbars_PageNavigation = 57,
    /// `View_Toolbars_PrintProduction` predefined viewer action.
    View_Toolbars_PrintProduction = 58,
    /// `View_Toolbars_PropertiesBar` predefined viewer action.
    View_Toolbars_PropertiesBar = 59,
    /// `View_Toolbars_Redaction` predefined viewer action.
    View_Toolbars_Redaction = 60,
    /// `View_Toolbars_SelectZoom` predefined viewer action.
    View_Toolbars_SelectZoom = 61,
    /// `View_Toolbars_Tasks` predefined viewer action.
    View_Toolbars_Tasks = 62,
    /// `View_Toolbars_Typewriter` predefined viewer action.
    View_Toolbars_Typewriter = 63,
    /// `View_Zoom_ActualSize` predefined viewer action.
    View_Zoom_ActualSize = 64,
    /// `View_Zoom_FitHeight` predefined viewer action.
    View_Zoom_FitHeight = 65,
    /// `View_Zoom_FitPage` predefined viewer action.
    View_Zoom_FitPage = 66,
    /// `View_Zoom_FitVisible` predefined viewer action.
    View_Zoom_FitVisible = 67,
    /// `View_Zoom_FitWidth` predefined viewer action.
    View_Zoom_FitWidth = 68,
    /// `View_Zoom_ZoomTo` predefined viewer action.
    View_Zoom_ZoomTo = 69,
    /// `Window_FullScreenMode` predefined viewer action.
    Window_FullScreenMode = 70,
}

impl PredefinedAction {
    /// The action's canonical identifier (e.g. `"NextPage"`).
    pub fn name(self) -> &'static str {
        match self {
            PredefinedAction::FirstPage => "FirstPage",
            PredefinedAction::LastPage => "LastPage",
            PredefinedAction::NextPage => "NextPage",
            PredefinedAction::PrevPage => "PrevPage",
            PredefinedAction::PrintDialog => "PrintDialog",
            PredefinedAction::Print => "Print",
            PredefinedAction::Bookmarks_ExpanCurrentBookmark => "Bookmarks_ExpanCurrentBookmark",
            PredefinedAction::Bookmarks_HightlightCurrentBookmark => "Bookmarks_HightlightCurrentBookmark",
            PredefinedAction::Document_AttachFile => "Document_AttachFile",
            PredefinedAction::Document_CropPages => "Document_CropPages",
            PredefinedAction::Document_DeletePages => "Document_DeletePages",
            PredefinedAction::Document_ExtractPages => "Document_ExtractPages",
            PredefinedAction::Document_InsertPages => "Document_InsertPages",
            PredefinedAction::Document_ReplacePages => "Document_ReplacePages",
            PredefinedAction::Document_RotatePages => "Document_RotatePages",
            PredefinedAction::Edit_CheckSpelling_InComFieldEdit => "Edit_CheckSpelling_InComFieldEdit",
            PredefinedAction::Edit_Find => "Edit_Find",
            PredefinedAction::Edit_Preferences => "Edit_Preferences",
            PredefinedAction::Edit_Search => "Edit_Search",
            PredefinedAction::File_AttachToEmail => "File_AttachToEmail",
            PredefinedAction::File_Close => "File_Close",
            PredefinedAction::File_CreatePDF_FromScanner => "File_CreatePDF_FromScanner",
            PredefinedAction::File_CreatePDF_FromWebPage => "File_CreatePDF_FromWebPage",
            PredefinedAction::File_Exit => "File_Exit",
            PredefinedAction::File_Organizer_OpenOrganizer => "File_Organizer_OpenOrganizer",
            PredefinedAction::File_Print => "File_Print",
            PredefinedAction::File_Properties => "File_Properties",
            PredefinedAction::File_SaveAs => "File_SaveAs",
            PredefinedAction::Miscellaneous_ZoomIn => "Miscellaneous_ZoomIn",
            PredefinedAction::Miscellaneous_ZoomOut => "Miscellaneous_ZoomOut",
            PredefinedAction::PageImages_PrintPages => "PageImages_PrintPages",
            PredefinedAction::View_GoTo_NextView => "View_GoTo_NextView",
            PredefinedAction::View_GoTo_Page => "View_GoTo_Page",
            PredefinedAction::View_GoTo_PreDocument => "View_GoTo_PreDocument",
            PredefinedAction::View_GoTo_PreView => "View_GoTo_PreView",
            PredefinedAction::View_NavigationPanels_Articles => "View_NavigationPanels_Articles",
            PredefinedAction::View_NavigationPanels_Attachments => "View_NavigationPanels_Attachments",
            PredefinedAction::View_NavigationPanels_Boomarks => "View_NavigationPanels_Boomarks",
            PredefinedAction::View_NavigationPanels_Comments => "View_NavigationPanels_Comments",
            PredefinedAction::View_NavigationPanels_Fields => "View_NavigationPanels_Fields",
            PredefinedAction::View_NavigationPanels_Layers => "View_NavigationPanels_Layers",
            PredefinedAction::View_NavigationPanels_ModelTree => "View_NavigationPanels_ModelTree",
            PredefinedAction::View_NavigationPanels_Pages => "View_NavigationPanels_Pages",
            PredefinedAction::View_NavigationPanels_Signatures => "View_NavigationPanels_Signatures",
            PredefinedAction::View_PageDisplay_SinglePage => "View_PageDisplay_SinglePage",
            PredefinedAction::View_PageDisplay_SinglePageContinuous => "View_PageDisplay_SinglePageContinuous",
            PredefinedAction::View_PageDisplay_TwoUp => "View_PageDisplay_TwoUp",
            PredefinedAction::View_PageDisplay_TwoUpContinuous => "View_PageDisplay_TwoUpContinuous",
            PredefinedAction::View_Toolbars_AdvanceEditing => "View_Toolbars_AdvanceEditing",
            PredefinedAction::View_Toolbars_CommentMarkup => "View_Toolbars_CommentMarkup",
            PredefinedAction::View_Toolbars_Edit => "View_Toolbars_Edit",
            PredefinedAction::View_Toolbars_File => "View_Toolbars_File",
            PredefinedAction::View_Toolbars_Find => "View_Toolbars_Find",
            PredefinedAction::View_Toolbars_Forms => "View_Toolbars_Forms",
            PredefinedAction::View_Toolbars_Measuring => "View_Toolbars_Measuring",
            PredefinedAction::View_Toolbars_ObjectData => "View_Toolbars_ObjectData",
            PredefinedAction::View_Toolbars_PageDisplay => "View_Toolbars_PageDisplay",
            PredefinedAction::View_Toolbars_PageNavigation => "View_Toolbars_PageNavigation",
            PredefinedAction::View_Toolbars_PrintProduction => "View_Toolbars_PrintProduction",
            PredefinedAction::View_Toolbars_PropertiesBar => "View_Toolbars_PropertiesBar",
            PredefinedAction::View_Toolbars_Redaction => "View_Toolbars_Redaction",
            PredefinedAction::View_Toolbars_SelectZoom => "View_Toolbars_SelectZoom",
            PredefinedAction::View_Toolbars_Tasks => "View_Toolbars_Tasks",
            PredefinedAction::View_Toolbars_Typewriter => "View_Toolbars_Typewriter",
            PredefinedAction::View_Zoom_ActualSize => "View_Zoom_ActualSize",
            PredefinedAction::View_Zoom_FitHeight => "View_Zoom_FitHeight",
            PredefinedAction::View_Zoom_FitPage => "View_Zoom_FitPage",
            PredefinedAction::View_Zoom_FitVisible => "View_Zoom_FitVisible",
            PredefinedAction::View_Zoom_FitWidth => "View_Zoom_FitWidth",
            PredefinedAction::View_Zoom_ZoomTo => "View_Zoom_ZoomTo",
            PredefinedAction::Window_FullScreenMode => "Window_FullScreenMode",
        }
    }
}

impl fmt::Display for PredefinedAction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}
