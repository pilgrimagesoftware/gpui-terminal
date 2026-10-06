//! A cell's colours: inverse, the cursor and the selection.

use super::cell_colors;
use crate::TerminalPalette;

const FG: u32 = 0x111111;
const BG: u32 = 0xEEEEEE;

fn swapping() -> TerminalPalette {
    TerminalPalette::default()
}

fn coloured() -> TerminalPalette {
    TerminalPalette { cursor: Some(0x00FF00),
                      selection: Some(0x0000FF),
                      ..TerminalPalette::default() }
}

#[test]
fn a_plain_cell_keeps_its_colours() {
    assert_eq!(cell_colors(FG, BG, false, false, false, &coloured()),
               (FG, BG));
}

#[test]
fn without_palette_colours_the_cursor_and_selection_swap_the_cell() {
    assert_eq!(cell_colors(FG, BG, false, true, false, &swapping()),
               (BG, FG));
    assert_eq!(cell_colors(FG, BG, false, false, true, &swapping()),
               (BG, FG));
    assert_eq!(cell_colors(FG, BG, true, false, false, &swapping()),
               (BG, FG),
               "inverse");
}

#[test]
fn palette_colours_go_behind_the_cursor_and_selection() {
    assert_eq!(cell_colors(FG, BG, false, true, false, &coloured()),
               (FG, 0x00FF00));
    assert_eq!(cell_colors(FG, BG, false, false, true, &coloured()),
               (FG, 0x0000FF));
    assert_eq!(cell_colors(FG, BG, false, true, true, &coloured()),
               (FG, 0x00FF00),
               "the cursor shows over a selection");
}
