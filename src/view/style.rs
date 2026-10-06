//! How a terminal view draws its text: the font, and the colours.

use gpui_kit::{Pixels, Rgba, SharedString};

use crate::consts::{ANSI_16, DEFAULT_BACKGROUND, DEFAULT_FOREGROUND};

/// The font and colours a [`crate::TerminalView`] draws its cells in.
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
    pub palette:     TerminalPalette,
}

impl TerminalStyle {
    /// `font_family` at `font_size`, in the [default
    /// palette](TerminalPalette::default).
    pub fn new(font_family: impl Into<SharedString>, font_size: Pixels) -> Self {
        Self { font_family: font_family.into(),
               font_size,
               palette: TerminalPalette::default() }
    }

    /// This style in `palette` instead - a host's theme, say.
    pub fn palette(mut self, palette: TerminalPalette) -> Self {
        self.palette = palette;
        self
    }
}

/// The colours a terminal draws with, each `0xRRGGBB` ([`Self::rgb`] turns a
/// GPUI colour into one). The default is xterm's, on a dark grey pane.
///
/// `ansi` is the sixteen a program addresses by name (SGR 30-37, 90-97 and
/// their backgrounds), which are also the first sixteen of the 256-colour
/// palette; the rest of the 256 and true colour are fixed. `cursor` and
/// `selection` are the cursor cell's and selected cells' backgrounds; `None`
/// draws them by swapping the cell's own colours instead.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TerminalPalette {
    pub foreground: u32,
    pub background: u32,
    pub ansi:       [u32; 16],
    pub cursor:     Option<u32>,
    pub selection:  Option<u32>,
}

impl TerminalPalette {
    /// `color` as `0xRRGGBB`, its alpha dropped: a theme's `Hsla` or `Rgba`
    /// becomes a palette entry.
    pub fn rgb(color: impl Into<Rgba>) -> u32 {
        let Rgba { r, g, b, .. } = color.into();
        let channel = |value: f32| (value.clamp(0., 1.) * 255.).round() as u32;
        (channel(r) << 16) | (channel(g) << 8) | channel(b)
    }
}

impl Default for TerminalPalette {
    fn default() -> Self {
        Self { foreground: DEFAULT_FOREGROUND,
               background: DEFAULT_BACKGROUND,
               ansi:       ANSI_16,
               cursor:     None,
               selection:  None, }
    }
}

#[cfg(test)]
mod tests;
