//! Turning a cell's colour into an RGB value.

use alacritty_terminal::vte::ansi::{Color, NamedColor, Rgb};

use super::TerminalPalette;
use crate::consts::CUBE_LEVELS;

/// A cell colour as `0xRRGGBB`, in `palette`. `is_foreground` picks the
/// default for the scheme-relative names that are not one of the sixteen;
/// everything else - the ANSI colours, the 256-colour palette, true colour -
/// is fixed whichever side of the cell it is on.
pub(crate) fn resolve(color: Color, is_foreground: bool, palette: &TerminalPalette) -> u32 {
    let fallback = if is_foreground {
        palette.foreground
    }
    else {
        palette.background
    };
    match color {
        Color::Spec(Rgb { r, g, b }) => rgb(r, g, b),
        Color::Named(NamedColor::Foreground) => palette.foreground,
        Color::Named(NamedColor::Background) => palette.background,
        Color::Named(named) => ansi_index(named).map_or(fallback, |index| palette.ansi[index]),
        Color::Indexed(index) => indexed(index, palette),
    }
}

fn rgb(r: u8, g: u8, b: u8) -> u32 {
    (u32::from(r) << 16) | (u32::from(g) << 8) | u32::from(b)
}

/// Where a named colour sits in the sixteen, if it is one of them.
fn ansi_index(color: NamedColor) -> Option<usize> {
    let index = color as usize;
    (index < 16).then_some(index)
}

/// The xterm 256-colour palette: 0-15 are `palette`'s sixteen, 16-231 a
/// 6x6x6 colour cube, 232-255 a grey ramp.
fn indexed(index: u8, palette: &TerminalPalette) -> u32 {
    match index {
        0..=15 => palette.ansi[usize::from(index)],
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
