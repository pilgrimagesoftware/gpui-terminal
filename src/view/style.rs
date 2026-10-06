//! How a terminal view draws its text.

use gpui_kit::{Pixels, SharedString};

/// The font a [`crate::TerminalView`] draws and measures its cells in.
///
/// `font_family` must be one GPUI's text system can resolve - registered or
/// installed. An unresolvable name does not fail: GPUI falls back to the
/// app's UI font, which is usually proportional, and a terminal grid in a
/// proportional face is unreadable. Resolve the user's choice against
/// `cx.text_system().all_font_names()` before handing it over, and do that
/// once per change rather than per frame - enumerating installed fonts is
/// slow.
#[derive(Debug, Clone, PartialEq)]
pub struct TerminalStyle {
    pub font_family: SharedString,
    pub font_size:   Pixels,
}

impl TerminalStyle {
    pub fn new(font_family: impl Into<SharedString>, font_size: Pixels) -> Self {
        Self { font_family: font_family.into(),
               font_size }
    }
}
