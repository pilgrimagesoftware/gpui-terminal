//! Cell colours to RGB.

use super::*;

#[test]
fn true_colour_passes_through() {
    assert_eq!(resolve(Color::Spec(Rgb { r: 0x12,
                                         g: 0x34,
                                         b: 0x56, }),
                       true),
               0x123456);
}

#[test]
fn the_sixteen_named_colours_are_xterms() {
    assert_eq!(resolve(Color::Named(NamedColor::Red), true), 0xCD0000);
    assert_eq!(resolve(Color::Named(NamedColor::BrightWhite), false),
               0xFFFFFF);
}

#[test]
fn scheme_defaults_depend_on_the_side_only_when_unnamed() {
    assert_eq!(resolve(Color::Named(NamedColor::Foreground), false),
               DEFAULT_FOREGROUND);
    assert_eq!(resolve(Color::Named(NamedColor::Background), true),
               DEFAULT_BACKGROUND);
    assert_eq!(resolve(Color::Named(NamedColor::DimRed), true),
               DEFAULT_FOREGROUND);
    assert_eq!(resolve(Color::Named(NamedColor::DimRed), false),
               DEFAULT_BACKGROUND);
}

#[test]
fn the_256_colour_palette_has_its_cube_and_grey_ramp() {
    assert_eq!(resolve(Color::Indexed(1), true), 0xCD0000);
    assert_eq!(resolve(Color::Indexed(16), true), 0x000000);
    assert_eq!(resolve(Color::Indexed(196), true), 0xFF0000);
    assert_eq!(resolve(Color::Indexed(231), true), 0xFFFFFF);
    assert_eq!(resolve(Color::Indexed(232), true), 0x080808);
    assert_eq!(resolve(Color::Indexed(255), true), 0xEEEEEE);
}
