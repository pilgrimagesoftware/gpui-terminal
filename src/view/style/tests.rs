//! The style's palette.

use gpui_kit::{Rgba, hsla};

use super::*;

#[test]
fn the_default_palette_is_xterms_on_dark_grey() {
    let palette = TerminalStyle::new("Menlo", gpui_kit::px(13.)).palette;
    assert_eq!(palette, TerminalPalette::default());
    assert_eq!(palette.ansi, ANSI_16);
    assert_eq!((palette.foreground, palette.background),
               (DEFAULT_FOREGROUND, DEFAULT_BACKGROUND));
    assert_eq!((palette.cursor, palette.selection), (None, None));
}

#[test]
fn a_style_takes_a_palette() {
    let palette = TerminalPalette { background: 0xFFFFFF,
                                    ..TerminalPalette::default() };
    let style = TerminalStyle::new("Menlo", gpui_kit::px(13.)).palette(palette);
    assert_eq!(style.palette.background, 0xFFFFFF);
}

#[test]
fn rgb_turns_a_gpui_colour_into_hex_dropping_alpha() {
    assert_eq!(TerminalPalette::rgb(Rgba { r: 1.,
                                           g: 0.5,
                                           b: 0.,
                                           a: 0.3, }),
               0xFF8000);
    assert_eq!(TerminalPalette::rgb(hsla(0., 0., 0., 1.)), 0x000000);
    assert_eq!(TerminalPalette::rgb(hsla(0., 0., 1., 1.)), 0xFFFFFF);
}
