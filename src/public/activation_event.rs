//! `Aspose.Pdf.Annotations.RichMediaAnnotation.ActivationEvent` — when a
//! rich-media annotation activates, per
//! `capabilities/activation_event.yaml`.
//!
//! Variant ordinals match the canonical Aspose.PDF 26.4.0 enum.

/// When a [`RichMediaAnnotation`](crate::RichMediaAnnotation) activates.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(i32)]
pub enum ActivationEvent {
    /// Activate when clicked.
    #[default]
    Click = 0,
    /// Activate when the page opens.
    PageOpen = 1,
    /// Activate when the page becomes visible.
    PageVisible = 2,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ordinals_match_canonical() {
        assert_eq!(ActivationEvent::Click as i32, 0);
        assert_eq!(ActivationEvent::PageOpen as i32, 1);
        assert_eq!(ActivationEvent::PageVisible as i32, 2);
    }

    #[test]
    fn default_is_click() {
        assert_eq!(ActivationEvent::default(), ActivationEvent::Click);
    }
}
