//! Turning a cell's colour into an RGB value.

use alacritty_terminal::vte::ansi::{Color, NamedColor, Rgb};

use crate::consts::{ANSI_16, CUBE_LEVELS, DEFAULT_BACKGROUND, DEFAULT_FOREGROUND};

/// A cell colour as `0xRRGGBB`. `is_foreground` picks the default for the
/// scheme-relative names that are not one of the sixteen; everything else -
/// the ANSI colours, the 256-colour palette, true colour - is fixed whichever
/// side of the cell it is on.
pub(crate) fn resolve(color: Color, is_foreground: bool) -> u32 {
    let fallback = if is_foreground {
        DEFAULT_FOREGROUND
    }
    else {
        DEFAULT_BACKGROUND
    };
    match color {
        Color::Spec(Rgb { r, g, b }) => rgb(r, g, b),
        Color::Named(NamedColor::Foreground) => DEFAULT_FOREGROUND,
        Color::Named(NamedColor::Background) => DEFAULT_BACKGROUND,
        Color::Named(named) => ansi_index(named).map_or(fallback, |index| ANSI_16[index]),
        Color::Indexed(index) => indexed(index),
    }
}

fn rgb(r: u8, g: u8, b: u8) -> u32 {
    (u32::from(r) << 16) | (u32::from(g) << 8) | u32::from(b)
}

/// Where a named colour sits in the sixteen, if it is one of them.
fn ansi_index(color: NamedColor) -> Option<usize> {
    let index = color as usize;
    (index < ANSI_16.len()).then_some(index)
}

/// The xterm 256-colour palette: 0-15 are the named colours, 16-231 a 6x6x6
/// colour cube, 232-255 a grey ramp.
fn indexed(index: u8) -> u32 {
    match index {
        0..=15 => ANSI_16[usize::from(index)],
        16..=231 => {
            let cube = index - 16;
            rgb(CUBE_LEVELS[usize::from(cube / 36)],
                CUBE_LEVELS[usize::from((cube / 6) % 6)],
                CUBE_LEVELS[usize::from(cube % 6)])
        }
        232..=255 => {
            let level = 8 + (index - 232) * 10;
            rgb(level, level, level)
        }
    }
}

#[cfg(test)]
mod tests;
