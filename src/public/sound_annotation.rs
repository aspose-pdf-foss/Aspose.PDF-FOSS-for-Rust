//! `Aspose.Pdf.Annotations.SoundAnnotation` — a sound annotation,
//! per `capabilities/sound_annotation.yaml`.
//!
//! Extends [`MarkupAnnotation`]. Construct with a sound-file path, optionally
//! supplying [`SoundSampleData`] to set the audio parameters; the resulting
//! [`SoundData`] is then exposed via [`sound_data`](SoundAnnotation::sound_data).
//!
//! `/Name`, the icon, is written on save.
//!
//! **Carried, not written.** §12.5.6.16's required `/Sound` is a
//! *stream* — the samples themselves, with the rate and channel count
//! in its dictionary — and the annotation writer emits one dictionary
//! per annotation, with no second object to put samples in. The clip's
//! parameters round-trip in memory through [`SoundData`].

use crate::public::annotation::sealed::HasBase;
use crate::public::annotation::{Annotation, AnnotationBase};
use crate::public::annotation_type::AnnotationType;
use crate::public::markup_annotation::sealed::HasMarkupBase;
use crate::public::markup_annotation::{MarkupAnnotation, MarkupBase};
use crate::public::page::Page;
use crate::public::rectangle::Rectangle;
use crate::public::sound_data::SoundData;
use crate::public::sound_icon::SoundIcon;
use crate::public::sound_sample_data::SoundSampleData;

/// A sound annotation that plays an audio clip.
pub struct SoundAnnotation {
    base: AnnotationBase,
    markup: MarkupBase,
    icon: SoundIcon,
    sound_data: SoundData,
}

impl SoundAnnotation {
    /// Construct on a [`Page`] at the given rectangle for the sound
    /// file at `_sound_file`.
    pub fn on_page(page: &Page, rect: Rectangle, _sound_file: String) -> Self {
        let mut base = AnnotationBase::new(rect);
        base.page_index = page.number() as i32;
        Self {
            base,
            markup: MarkupBase::new(),
            icon: SoundIcon::Speaker,
            sound_data: SoundData::default(),
        }
    }

    /// Construct on a [`Page`] at the given rectangle for the sound file at
    /// `sound_file`, taking the audio parameters from `sound_sample_data`.
    pub fn on_page_with_sample_data(
        page: &Page,
        rect: Rectangle,
        sound_file: String,
        sound_sample_data: SoundSampleData,
    ) -> Self {
        let mut this = Self::on_page(page, rect, sound_file);
        this.sound_data = SoundData::from_sample_data(&sound_sample_data);
        this
    }

    /// The sound icon.
    pub fn icon(&self) -> SoundIcon {
        self.icon
    }
    /// Set the sound icon.
    pub fn set_icon(&mut self, value: SoundIcon) {
        self.icon = value;
    }

    /// The audio parameters of this annotation's sound.
    pub fn sound_data(&self) -> &SoundData {
        &self.sound_data
    }
}

impl HasBase for SoundAnnotation {
    fn base(&self) -> &AnnotationBase {
        &self.base
    }
    fn base_mut(&mut self) -> &mut AnnotationBase {
        &mut self.base
    }
    fn pdf_entries(&self) -> Vec<(&'static str, crate::internal::annotation_io::Entry)> {
        use crate::internal::annotation_io::Entry;
        // §12.5.6.16 Table 185: /Name is the icon a viewer draws,
        // /Speaker being the default an absent entry already means.
        if self.icon == SoundIcon::Mic {
            vec![("Name", Entry::Name("Mic".to_string()))]
        } else {
            Vec::new()
        }
    }
    fn markup_base(&self) -> Option<&crate::public::markup_annotation::MarkupBase> {
        Some(&self.markup)
    }
}
impl HasMarkupBase for SoundAnnotation {
    fn markup(&self) -> &MarkupBase {
        &self.markup
    }
    fn markup_mut(&mut self) -> &mut MarkupBase {
        &mut self.markup
    }
}
impl Annotation for SoundAnnotation {
    fn annotation_type(&self) -> AnnotationType {
        AnnotationType::Sound
    }
}
impl MarkupAnnotation for SoundAnnotation {}
