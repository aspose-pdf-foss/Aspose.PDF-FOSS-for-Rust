//! `Aspose.Pdf.Facades.DefaultMetadataProperties` — a well-known XMP
//! metadata property key, per
//! `capabilities/default_metadata_properties.yaml`. Ordinals match
//! canonical 26.4.0.

/// A well-known XMP metadata property key.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(i32)]
pub enum DefaultMetadataProperties {
    /// `xmp:Advisory`.
    #[default]
    Advisory = 0,
    /// `xmp:BaseURL`.
    BaseURL = 1,
    /// `xmp:CreateDate`.
    CreateDate = 2,
    /// `xmp:CreatorTool`.
    CreatorTool = 3,
    /// `xmp:Identifier`.
    Identifier = 4,
    /// `xmp:MetadataDate`.
    MetadataDate = 5,
    /// `xmp:ModifyDate`.
    ModifyDate = 6,
    /// `xmp:Nickname`.
    Nickname = 7,
    /// `xmp:Thumbnails`.
    Thumbnails = 8,
}
