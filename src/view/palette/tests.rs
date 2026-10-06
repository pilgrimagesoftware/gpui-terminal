//! Cell colours to RGB.

use super::*;
use crate::consts::{DEFAULT_BACKGROUND, DEFAULT_FOREGROUND};

const PALETTE: TerminalPalette = TerminalPalette { foreground: DEFAULT_FOREGROUND,
                                                   background: DEFAULT_BACKGROUND,
                                                   ansi:       crate::consts::ANSI_16,
                                                   cursor:     None,
                                                   selection:  None, };

#[test]
fn true_colour_passes_through() {
    assert_eq!(resolve(Color::Spec(Rgb { r: 0x12,
                                         g: 0x34,
                                         b: 0x56, }),
                       true,
                       &PALETTE),
               0x123456);
}

#[test]
fn the_sixteen_named_colours_are_xterms() {
    assert_eq!(resolve(Color::Named(NamedColor::Red), true, &PALETTE),
               0xCD0000);
    assert_eq!(resolve(Color::Named(NamedColor::BrightWhite), false, &PALETTE),
               0xFFFFFF);
}

#[test]
fn scheme_defaults_depend_on_the_side_only_when_unnamed() {
    assert_eq!(resolve(Color::Named(NamedColor::Foreground), false, &PALETTE),
               DEFAULT_FOREGROUND);
    assert_eq!(resolve(Color::Named(NamedColor::Background), true, &PALETTE),
               DEFAULT_BACKGROUND);
    assert_eq!(resolve(Color::Named(NamedColor::DimRed), true, &PALETTE),
               DEFAULT_FOREGROUND);
    assert_eq!(resolve(Color::Named(NamedColor::DimRed), false, &PALETTE),
               DEFAULT_BACKGROUND);
}

#[test]
fn the_256_colour_palette_has_its_cube_and_grey_ramp() {
    assert_eq!(resolve(Color::Indexed(1), true, &PALETTE), 0xCD0000);
    assert_eq!(resolve(Color::Indexed(16), true, &PALETTE), 0x000000);
    assert_eq!(resolve(Color::Indexed(196), true, &PALETTE), 0xFF0000);
    assert_eq!(resolve(Color::Indexed(231), true, &PALETTE), 0xFFFFFF);
    assert_eq!(resolve(Color::Indexed(232), true, &PALETTE), 0x080808);
    assert_eq!(resolve(Color::Indexed(255), true, &PALETTE), 0xEEEEEE);
}

#[test]
fn a_host_palette_replaces_the_sixteen_and_the_defaults() {
    let mut ansi = crate::consts::ANSI_16;
    ansi[1] = 0xAA0000;
    ansi[9] = 0xFF5555;
    let host = TerminalPalette { foreground: 0x111111,
                                 background: 0xFAFAFA,
                                 ansi,
                                 ..PALETTE };
    assert_eq!(resolve(Color::Named(NamedColor::Red), true, &host),
               0xAA0000);
    assert_eq!(resolve(Color::Indexed(9), false, &host), 0xFF5555);
    assert_eq!(resolve(Color::Named(NamedColor::Foreground), false, &host),
               0x111111);
    assert_eq!(resolve(Color::Named(NamedColor::Background), true, &host),
               0xFAFAFA);
    assert_eq!(resolve(Color::Named(NamedColor::DimRed), false, &host),
               0xFAFAFA);
    // Beyond the sixteen, the 256-colour palette is fixed.
    assert_eq!(resolve(Color::Indexed(196), true, &host), 0xFF0000);
}
