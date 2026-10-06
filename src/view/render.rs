//! Drawing the grid: a plain cell grid of text runs, not a native surface.
//!
//! Runs on every frame, so it reads the grid and nothing else - no I/O, no
//! font enumeration. Cell measurement is memoised on the view.

use alacritty_terminal::term::cell::Flags;
use gpui_kit::{
    Context, Div, FontWeight, InteractiveElement, IntoElement, MouseButton, ParentElement, Render,
    Styled, Window, canvas, div, px, rgb,
};

use super::{TerminalPalette, TerminalView, palette};
use crate::{Grid, Transport};

impl<T: Transport> Render for TerminalView<T> {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let metrics = self.cell_metrics(cx);
        let colors = self.style.palette;
        let rows = self.terminal.with_grid(|grid| rows(grid, &colors));
        let view = cx.weak_entity();
        // Covers the pane, to learn the bounds the grid is laid out in.
        let fit = canvas(move |bounds, window, cx| {
                             // Gone only if the view was dropped mid-frame, in
                             // which case there is nothing left to size.
                             let _ = view.update(cx, |view, cx| view.fit(bounds, window, cx));
                         },
                         |_, _, _, _| {}).absolute()
                                         .size_full();

        div().relative()
             .flex()
             .flex_col()
             .size_full()
             .overflow_hidden()
             .track_focus(&self.focus)
             .font_family(self.style.font_family.clone())
             .text_size(self.style.font_size)
             // Rows exactly one measured cell tall, whatever line height the
             // host's theme sets - otherwise the rows drawn and the rows the
             // program was told it has drift apart.
             .line_height(px(metrics.height))
             .bg(rgb(colors.background))
             .on_mouse_down(MouseButton::Left, cx.listener(Self::on_left_down))
             .on_mouse_up(MouseButton::Left, cx.listener(Self::on_left_up))
             .on_mouse_move(cx.listener(Self::on_mouse_move))
             .on_scroll_wheel(cx.listener(Self::on_scroll))
             .on_key_down(cx.listener(Self::on_key_down))
             .child(fit)
             .children(rows)
    }
}

/// One run of cells that share colours and attributes.
struct Span {
    text:       String,
    foreground: u32,
    background: u32,
    flags:      Flags,
}

/// Every visible row, in `colors`.
fn rows(grid: &Grid, colors: &TerminalPalette) -> Vec<Div> {
    let cursor = grid.cursor();
    (0..grid.size().rows).map(|row| render_row(grid, row, cursor, colors))
                         .collect()
}

/// A cell's foreground and background: its own, inverted when it asks to
/// be; then the cursor's and the selection's - the palette's colour behind
/// the cell when it sets one, else the cell's colours swapped.
pub(super) fn cell_colors(foreground: u32, background: u32, inverse: bool, cursor: bool,
                          selected: bool, colors: &TerminalPalette)
                          -> (u32, u32) {
    let (mut foreground, mut background) = if inverse {
        (background, foreground)
    }
    else {
        (foreground, background)
    };
    let mut mark = |active: bool, color: Option<u32>| {
        if !active {
            return;
        }
        match color {
            Some(color) => background = color,
            None => std::mem::swap(&mut foreground, &mut background),
        }
    };
    mark(selected, colors.selection);
    mark(cursor, colors.cursor);
    (foreground, background)
}

fn render_row(grid: &Grid, row: usize, cursor: (usize, usize), colors: &TerminalPalette) -> Div {
    let mut spans: Vec<Span> = Vec::new();
    for (column, cell) in grid.row_cells(row).enumerate() {
        // The second column of a wide character is a placeholder; the glyph
        // before it already covers it.
        if cell.flags.contains(Flags::WIDE_CHAR_SPACER) {
            continue;
        }
        let (foreground, background) = cell_colors(palette::resolve(cell.fg, true, colors),
                                                   palette::resolve(cell.bg, false, colors),
                                                   cell.flags.contains(Flags::INVERSE),
                                                   (column, row) == cursor,
                                                   grid.is_selected(column, row),
                                                   colors);
        match spans.last_mut() {
            Some(span)
                if span.foreground == foreground
                   && span.background == background
                   && span.flags == cell.flags =>
            {
                span.text.push(cell.c);
            }
            _ => spans.push(Span { text: cell.c.to_string(),
                                   foreground,
                                   background,
                                   flags: cell.flags }),
        }
    }
    div().flex()
         .flex_row()
         .w_full()
         .children(spans.into_iter().map(render_span))
}

fn render_span(span: Span) -> Div {
    let mut element = div().text_color(rgb(span.foreground))
                           .bg(rgb(span.background))
                           .child(span.text);
    if span.flags.contains(Flags::BOLD) {
        element = element.font_weight(FontWeight::BOLD);
    }
    if span.flags.contains(Flags::ITALIC) {
        element = element.italic();
    }
    if span.flags
           .intersects(Flags::UNDERLINE | Flags::DOUBLE_UNDERLINE)
    {
        element = element.underline();
    }
    if span.flags.contains(Flags::STRIKEOUT) {
        element = element.line_through();
    }
    element
}

#[cfg(test)]
mod tests;
