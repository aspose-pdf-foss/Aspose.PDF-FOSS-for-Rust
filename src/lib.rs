//! Aspose.PDF FOSS for Rust — a strict subset of the canonical
//! Aspose.PDF for .NET public API, with no dependencies: every
//! primitive is implemented from its specification.
//!
//! Public types live under [`public`] and are re-exported at the
//! crate root for the canonical
//! `use aspose_pdf_foss::{Document, Page, ...}` shape. Foundation
//! primitives live under `internal` (crate-private) and back the
//! public bodies — they are not part of the lib's public API.
//!
//! See the repository README for the feature matrix and quick-start
//! examples.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod public;

pub(crate) mod internal;

pub use public::annotation::Annotation;
pub use public::annotation_collection::AnnotationCollection;
pub use public::artifact::Artifact;
pub use public::artifact_collection::ArtifactCollection;
pub use public::artifact_subtype::ArtifactSubtype;
pub use public::artifact_type::ArtifactType;
pub use public::base_paragraph::BaseParagraph;
pub use public::bookmark::Bookmark;
pub use public::bookmarks::Bookmarks;
pub use public::border_info::BorderInfo;
pub use public::border_side::BorderSide;
pub use public::cell::Cell;
pub use public::cells::Cells;
pub use public::circle::Circle;
pub use public::drawing_rectangle::DrawingRectangle;
pub use public::ellipse::Ellipse;
pub use public::graph::Graph;
pub use public::facade::Facade;
pub use public::graph_info::GraphInfo;
pub use public::i_facade::IFacade;
pub use public::i_saveable_facade::ISaveableFacade;
pub use public::line::Line;
pub use public::load_options::LoadOptions;
pub use public::password_type::PasswordType;
pub use public::pdf_extractor::PdfExtractor;
pub use public::pdf_annotation_editor::PdfAnnotationEditor;
pub use public::pdf_bookmark_editor::PdfBookmarkEditor;
pub use public::pdf_content_editor::PdfContentEditor;
pub use public::encoding_type::EncodingType;
pub use public::field_type::FieldType;
pub use public::font_style::FontStyle;
pub use public::alignment_type::AlignmentType;
pub use public::auto_rotate_mode::AutoRotateMode;
pub use public::blending_color_space::BlendingColorSpace;
pub use public::data_type::DataType;
pub use public::default_metadata_properties::DefaultMetadataProperties;
pub use public::image_merge_mode::ImageMergeMode;
pub use public::positioning_mode::PositioningMode;
pub use public::stamp_type::StampType;
pub use public::vertical_alignment_type::VerticalAlignmentType;
pub use public::word_wrap_mode::WordWrapMode;
pub use public::algorithm::Algorithm;
pub use public::key_size::KeySize;
pub use public::permissions::Permissions;
pub use public::signature_name::SignatureName;
pub use public::form_editor::FormEditor;
pub use public::form_field_facade::FormFieldFacade;
pub use public::property_flag::PropertyFlag;
pub use public::submit_form_flag::SubmitFormFlag;
pub use public::pdf_converter::PdfConverter;
pub use public::pdf_page_editor::PdfPageEditor;
pub use public::pdf_file_editor::PdfFileEditor;
pub use public::pdf_file_info::PdfFileInfo;
pub use public::pdf_file_security::PdfFileSecurity;
pub use public::pdf_file_signature::PdfFileSignature;
pub use public::pdf_file_stamp::PdfFileStamp;
pub use public::pdf_format::PdfFormat;
pub use public::pdf_xmp_metadata::PdfXmpMetadata;
pub use public::saveable_facade::SaveableFacade;
pub use public::row::Row;
pub use public::rows::Rows;
pub use public::resources::Resources;
pub use public::shape::Shape;
pub use public::svg_load_options::SvgLoadOptions;
pub use public::tab_alignment_type::TabAlignmentType;
pub use public::tab_leader_type::TabLeaderType;
pub use public::tab_stop::TabStop;
pub use public::tab_stops::TabStops;
pub use public::table::Table;
pub use public::watermark_artifact::WatermarkArtifact;
pub use public::x_image::XImage;
pub use public::x_image_collection::XImageCollection;
pub use public::image_placement::ImagePlacement;
pub use public::image_placement_absorber::ImagePlacementAbsorber;
pub use public::image_placement_collection::ImagePlacementCollection;
pub use public::bitmap_info::BitmapInfo;
pub use public::pixel_format::PixelFormat;
pub use public::extract_image_mode::ExtractImageMode;
pub use public::image_delete_action::ImageDeleteAction;
pub use public::image_filter_type::ImageFilterType;
pub use public::blend_mode::BlendMode;
pub use public::compositing_parameters::CompositingParameters;
pub use public::border::Border;
pub use public::bmp_device::BmpDevice;
pub use public::activation_event::ActivationEvent;
pub use public::af_relationship::AFRelationship;
pub use public::characteristics::Characteristics;
pub use public::circle_annotation::CircleAnnotation;
pub use public::color::Color;
pub use public::common_figure_annotation::CommonFigureAnnotation;
pub use public::color_depth::ColorDepth;
pub use public::color_space::ColorSpace;
pub use public::color_type::ColorType;
pub use public::content_type::ContentType;
pub use public::contents_resize_parameters::ContentsResizeParameters;
pub use public::contents_resize_value::ContentsResizeValue;
pub use public::compression_type::CompressionType;
pub use public::concatenate_corrupted_file_action::ConcatenateCorruptedFileAction;
pub use public::crypto_algorithm::CryptoAlgorithm;
pub use public::dash::Dash;
pub use public::device::{Device, DocumentDevice, PageDevice};
pub use public::default_appearance::DefaultAppearance;
pub use public::destination_collection::DestinationCollection;
pub use public::direction::Direction;
pub use public::document::Document;
pub use public::document_info::DocumentInfo;
pub use public::document_privilege::DocumentPrivilege;
pub use public::embedded_file_collection::EmbeddedFileCollection;
pub use public::error::Error;
pub use public::explicit_destination::ExplicitDestination;
pub use public::explicit_destination_type::ExplicitDestinationType;
pub use public::fit_b_explicit_destination::FitBExplicitDestination;
pub use public::fit_bh_explicit_destination::FitBHExplicitDestination;
pub use public::fit_bv_explicit_destination::FitBVExplicitDestination;
pub use public::fit_explicit_destination::FitExplicitDestination;
pub use public::fit_h_explicit_destination::FitHExplicitDestination;
pub use public::fit_r_explicit_destination::FitRExplicitDestination;
pub use public::fit_v_explicit_destination::FitVExplicitDestination;
pub use public::field::Field;
pub use public::file_encoding::FileEncoding;
pub use public::file_params::FileParams;
pub use public::file_specification::FileSpecification;
pub use public::encrypted_payload::EncryptedPayload;
pub use public::fixed_print::FixedPrint;
pub use public::form::Form;
pub use public::fraction_style::FractionStyle;
pub use public::form_presentation_mode::FormPresentationMode;
pub use public::horizontal_alignment::HorizontalAlignment;
pub use public::hyperlink::Hyperlink;
pub use public::i_appointment::IAppointment;
pub use public::i_index_bitmap_converter::IIndexBitmapConverter;
pub use public::image_device::ImageDevice;
pub use public::jpeg_device::JpegDevice;
pub use public::line_annotation::LineAnnotation;
pub use public::link_annotation::LinkAnnotation;
pub use public::margins::Margins;
pub use public::markup_annotation::MarkupAnnotation;
pub use public::matrix::Matrix;
pub use public::measure::Measure;
pub use public::metadata::Metadata;
pub use public::named_destination::NamedDestination;
pub use public::named_destination_collection::NamedDestinationCollection;
pub use public::numbering_style::NumberingStyle;
pub use public::go_to_action::GoToAction;
pub use public::go_to_uri_action::GoToURIAction;
pub use public::javascript_action::JavascriptAction;
pub use public::named_action::NamedAction;
pub use public::pdf_action::PdfAction;
pub use public::predefined_action::PredefinedAction;
pub use public::submit_form_action::SubmitFormAction;
pub use public::outline_collection::OutlineCollection;
pub use public::outline_item_collection::OutlineItemCollection;
pub use public::outlines::Outlines;
pub use public::page::Page;
pub use public::page_break::PageBreak;
pub use public::page_info::PageInfo;
pub use public::page_layout::PageLayout;
pub use public::page_mode::PageMode;
pub use public::page_collection::PageCollection;
pub use public::page_label::PageLabel;
pub use public::page_label_collection::PageLabelCollection;
pub use public::page_coordinate_type::PageCoordinateType;
pub use public::page_size::PageSize;
pub use public::point::Point;
pub use public::png_device::PngDevice;
pub use public::rectangle::Rectangle;
pub use public::rendering_options::RenderingOptions;
pub use public::resolution::Resolution;
pub use public::rotation::Rotation;
pub use public::shape_type::ShapeType;
pub use public::square_annotation::SquareAnnotation;
pub use public::xyz_explicit_destination::XYZExplicitDestination;
pub use public::annotation_flags::AnnotationFlags;
pub use public::annotation_state::AnnotationState;
pub use public::annotation_state_model::AnnotationStateModel;
pub use public::annotation_type::AnnotationType;
pub use public::border_effect::BorderEffect;
pub use public::border_style::BorderStyle;
pub use public::cap_style::CapStyle;
pub use public::caption_position::CaptionPosition;
pub use public::caret_symbol::CaretSymbol;
pub use public::colors_of_cmyk::ColorsOfCMYK;
pub use public::file_icon::FileIcon;
pub use public::free_text_intent::FreeTextIntent;
pub use public::highlighting_mode::HighlightingMode;
pub use public::justification::Justification;
pub use public::line_ending::LineEnding;
pub use public::line_intent::LineIntent;
pub use public::poly_intent::PolyIntent;
pub use public::print_duplex::PrintDuplex;
pub use public::print_scaling::PrintScaling;
pub use public::printer_mark_corner_position::PrinterMarkCornerPosition;
pub use public::printer_mark_side_position::PrinterMarkSidePosition;
pub use public::printer_marks_kind::PrinterMarksKind;
pub use public::reply_type::ReplyType;
pub use public::rich_media_annotation::RichMediaAnnotation;
pub use public::rich_text_font_styles::RichTextFontStyles;
pub use public::sound_encoding::SoundEncoding;
pub use public::sound_icon::SoundIcon;
pub use public::sound_sample_data_encoding_format::SoundSampleDataEncodingFormat;
pub use public::stamp_icon::StampIcon;
pub use public::text_alignment::TextAlignment;
pub use public::text_icon::TextIcon;
pub use public::vertical_alignment::VerticalAlignment;
pub use public::text_absorber::TextAbsorber;
pub use public::text_annotation::TextAnnotation;
pub use public::text_device::TextDevice;
pub use public::tiff_device::TiffDevice;
pub use public::tiff_settings::TiffSettings;
pub use public::xmp_value::XmpValue;

/// Charset codec promoted from the foundation `encoding` primitive
/// — substitutes BCL `System.Text.Encoding` on the
/// [`TextDevice`] public surface (the
/// encoding-taking ctor and the `Encoding` property).
pub use public::free_text_annotation::FreeTextAnnotation;
pub use public::highlight_annotation::HighlightAnnotation;
pub use public::polygon_annotation::PolygonAnnotation;
pub use public::polyline_annotation::PolylineAnnotation;
pub use public::poly_annotation::PolyAnnotation;
pub use public::squiggly_annotation::SquigglyAnnotation;
pub use public::strike_out_annotation::StrikeOutAnnotation;
pub use public::text_markup_annotation::TextMarkupAnnotation;
pub use public::underline_annotation::UnderlineAnnotation;

pub use public::ink_annotation::InkAnnotation;
pub use public::stamp_annotation::StampAnnotation;
pub use public::popup_annotation::PopupAnnotation;
pub use public::caret_annotation::CaretAnnotation;
pub use public::file_attachment_annotation::FileAttachmentAnnotation;

pub use public::sound_annotation::SoundAnnotation;
pub use public::sound_data::SoundData;
pub use public::sound_sample_data::SoundSampleData;
pub use public::movie_annotation::MovieAnnotation;
pub use public::screen_annotation::ScreenAnnotation;
pub use public::redaction_annotation::RedactionAnnotation;
pub use public::watermark_annotation::WatermarkAnnotation;

pub use public::printer_mark_annotation::PrinterMarkAnnotation;
pub use public::corner_printer_mark_annotation::CornerPrinterMarkAnnotation;
pub use public::color_bar_annotation::ColorBarAnnotation;
pub use public::registration_mark_annotation::RegistrationMarkAnnotation;
pub use public::page_information_annotation::PageInformationAnnotation;
pub use public::trim_mark_annotation::TrimMarkAnnotation;
pub use public::bleed_mark_annotation::BleedMarkAnnotation;
pub use public::annotation_selector::AnnotationSelector;

pub use public::box_style::BoxStyle;
pub use public::doc_mdp_access_permissions::DocMDPAccessPermissions;
pub use public::form_type::FormType;
pub use public::icon_caption_position::IconCaptionPosition;
pub use public::scaling_mode::ScalingMode;
pub use public::scaling_reason::ScalingReason;
pub use public::sign_dependent_elements_rendering_modes::SignDependentElementsRenderingModes;
pub use public::subject_name_elements::SubjectNameElements;
pub use public::symbology::Symbology;

pub use public::widget_annotation::WidgetAnnotation;

pub use public::text_box_field::TextBoxField;
pub use public::button_field::ButtonField;
pub use public::checkbox_field::CheckboxField;

pub use public::option::Option;
pub use public::option_collection::OptionCollection;
pub use public::choice_field::ChoiceField;
pub use public::combo_box_field::ComboBoxField;
pub use public::list_box_field::ListBoxField;
pub use public::radio_button_field::RadioButtonField;
pub use public::radio_button_option_field::RadioButtonOptionField;

pub use public::barcode_field::BarcodeField;
pub use public::date_field::DateField;
pub use public::date_time::DateTime;
pub use public::digest_hash_algorithm::DigestHashAlgorithm;
pub use public::number_field::NumberField;
pub use public::number_format::NumberFormat;
pub use public::number_format_list::NumberFormatList;
pub use public::password_box_field::PasswordBoxField;
pub use public::rich_text_box_field::RichTextBoxField;
pub use public::file_select_box_field::FileSelectBoxField;

pub use public::signature_custom_appearance::SignatureCustomAppearance;
pub use public::external_signature::ExternalSignature;
pub use public::pkcs1::PKCS1;
pub use public::pkcs7::PKCS7;
pub use public::pkcs7_detached::PKCS7Detached;
pub use public::signature::Signature;
pub use public::doc_mdp_signature::DocMDPSignature;
pub use public::signature_field::SignatureField;

pub use public::flatten_settings::FlattenSettings;
pub use public::icon_fit::IconFit;
pub use public::id::Id;
pub use public::xfa::XFA;

pub use public::floating_box::FloatingBox;
pub use public::font::Font;
pub use public::font_repository::FontRepository;
pub use public::margin_info::MarginInfo;
pub use public::paragraphs::Paragraphs;
pub use public::position::Position;
pub use public::text_builder::TextBuilder;
pub use public::text_fragment::TextFragment;
pub use public::clipping_paths_processing_mode::ClippingPathsProcessingMode;
pub use public::font_replace::FontReplace;
pub use public::language_transformation::LanguageTransformation;
pub use public::no_character_action::NoCharacterAction;
pub use public::text_edit_options::TextEditOptions;
pub use public::text_fragment_absorber::TextFragmentAbsorber;
pub use public::text_fragment_collection::TextFragmentCollection;
pub use public::text_fragment_state::TextFragmentState;
pub use public::text_paragraph::TextParagraph;
pub use public::font_size_adjustment::FontSizeAdjustment;
pub use public::replace_adjustment::ReplaceAdjustment;
pub use public::scope::Scope;
pub use public::text_replace_options::TextReplaceOptions;
pub use public::text_search_options::TextSearchOptions;
pub use public::text_state::TextState;
pub use public::text_style::TextStyle;

pub use crate::internal::encoding::Encoding;

/// Compiles the README's Rust snippets as doctests.
///
/// The README is the first thing a caller reads, so its examples must be
/// held to the same standard as the rest of the surface. Gated on
/// `cfg(doctest)`, this item exists only while rustdoc collects doctests:
/// it never appears in the generated documentation and costs nothing at
/// build time.
#[cfg(doctest)]
#[doc = include_str!("../README.md")]
struct ReadmeExamples;
