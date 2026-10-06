//! The size of one cell, and the arithmetic between pixels and cells.

use gpui_kit::{App, Pixels, Point, Size, font};

use super::TerminalStyle;
use crate::GridSize;
use crate::consts::FALLBACK_CELL_WIDTH;

/// One cell's size in pixels, measured from the font rather than guessed.
///
/// An overestimate reports more rows than fit, so whatever the program draws
/// at what it thinks is the bottom - an input box, a status line - lands
/// below the pane and never shows.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct CellMetrics {
    pub(crate) width:  f32,
    pub(crate) height: f32,
}

impl CellMetrics {
    pub(crate) fn measure(style: &TerminalStyle, cx: &App) -> Self {
        let text_system = cx.text_system();
        let font_id = text_system.resolve_font(&font(style.font_family.clone()));
        let width = text_system.em_advance(font_id, style.font_size)
                               .map_or(FALLBACK_CELL_WIDTH, f32::from);
        let height = text_system.ascent(font_id, style.font_size)
                     + text_system.descent(font_id, style.font_size);
        Self { width:  width.max(1.),
               height: f32::from(height).max(1.), }
    }

    /// How many whole cells fit in `bounds` - at least one each way, so a
    /// pane collapsed to nothing still leaves the program a terminal.
    pub(crate) fn grid_size(self, bounds: Size<Pixels>) -> GridSize {
        GridSize { columns: cells_in(f32::from(bounds.width), self.width).max(1),
                   rows:    cells_in(f32::from(bounds.height), self.height).max(1), }
    }

    /// The cell under `offset` (relative to the grid's top-left corner),
    /// clamped into a grid of `size`.
    pub(crate) fn cell_at(self, offset: Point<Pixels>, size: GridSize) -> (usize, usize) {
        let column = cells_in(f32::from(offset.x), self.width);
        let row = cells_in(f32::from(offset.y), self.height);
        (column.min(size.columns.saturating_sub(1)), row.min(size.rows.saturating_sub(1)))
    }

    pub(crate) fn pixels_to_lines(self, pixels: Pixels) -> f32 {
        f32::from(pixels) / self.height
    }
}

/// How many whole cells fit in `length`; a negative length (a point left of
/// or above the grid) fits none.
fn cells_in(length: f32, cell: f32) -> usize {
    (length.max(0.) / cell) as usize
}

#[cfg(test)]
mod tests;
