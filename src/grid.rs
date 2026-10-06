//! Output-to-cells parsing, backed by `alacritty_terminal`: feed raw bytes
//! into a VT/ANSI parser, read back the cell grid and the events the running
//! program raised (title changes, clipboard requests, bell, replies it
//! expects written back).

use std::sync::mpsc;

use alacritty_terminal::event::{Event, EventListener};
use alacritty_terminal::grid::{Dimensions, Scroll};
use alacritty_terminal::index::{Column, Line, Point, Side};
use alacritty_terminal::selection::{Selection, SelectionType};
use alacritty_terminal::term::cell::Cell;
use alacritty_terminal::term::{Config as TermConfig, Term, TermMode};
use alacritty_terminal::vte::ansi::Processor;

/// A terminal's visible size, in columns and rows. Scrollback beyond `rows`
/// is tracked by the grid; [`Grid::scroll_display`] moves the visible rows
/// into it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GridSize {
    pub columns: usize,
    pub rows:    usize,
}

impl Dimensions for GridSize {
    fn total_lines(&self) -> usize {
        self.rows
    }

    fn screen_lines(&self) -> usize {
        self.rows
    }

    fn columns(&self) -> usize {
        self.columns
    }
}

/// Forwards `alacritty_terminal` events to a channel the grid drains after
/// each feed. `EventListener::send_event` takes `&self`, so a channel - not a
/// `Vec` - is the simplest way to record them without a lock.
#[derive(Clone)]
pub(crate) struct EventForwarder(mpsc::Sender<Event>);

impl EventListener for EventForwarder {
    fn send_event(&self, event: Event) {
        // The receiver lives in the same `Grid`, so it outlives every send.
        let _ = self.0.send(event);
    }
}

/// One terminal's parsed state.
pub struct Grid {
    term:   Term<EventForwarder>,
    parser: Processor,
    events: mpsc::Receiver<Event>,
    /// Set whenever what is drawn changes (feed, resize, selection), and
    /// cleared by the view's pump when it asks for a repaint.
    dirty:  bool,
}

impl Grid {
    pub fn new(size: GridSize) -> Self {
        let (tx, rx) = mpsc::channel();
        let term = Term::new(TermConfig::default(), &size, EventForwarder(tx));
        Self { term,
               parser: Processor::new(),
               events: rx,
               dirty: false }
    }

    /// Parses `bytes` - raw program output - into the grid.
    pub fn feed(&mut self, bytes: &[u8]) {
        self.parser.advance(&mut self.term, bytes);
        self.dirty = true;
    }

    /// Resizes the grid. The transport's own size is the caller's to change;
    /// [`crate::Terminal::resize`] does both.
    pub fn resize(&mut self, size: GridSize) {
        self.term.resize(size);
        self.dirty = true;
    }

    pub fn size(&self) -> GridSize {
        GridSize { columns: self.term.columns(),
                   rows:    self.term.screen_lines(), }
    }

    /// The cursor's column and row within the screen, 0-indexed - where the
    /// program put it, wherever the view is scrolled.
    pub fn cursor(&self) -> (usize, usize) {
        let point = self.term.grid().cursor.point;
        (point.column.0, point.line.0.max(0) as usize)
    }

    /// The cursor's column and visible row, or `None` while the view is
    /// scrolled far enough back that the cursor's row is out of sight.
    pub(crate) fn cursor_in_view(&self) -> Option<(usize, usize)> {
        let (column, row) = self.cursor();
        let row = row + self.display_offset();
        (row < self.size().rows).then_some((column, row))
    }

    /// Moves the visible rows `lines` back into the scrollback (negative:
    /// forward, toward the live screen), stopping at either end. Whether the
    /// view moved. Nothing to scroll on the alternate screen, which keeps no
    /// scrollback.
    pub fn scroll_display(&mut self, lines: i32) -> bool {
        let before = self.display_offset();
        self.term.scroll_display(Scroll::Delta(lines));
        let moved = self.display_offset() != before;
        self.dirty |= moved;
        moved
    }

    /// Returns the view to the live screen. Whether it moved.
    pub fn scroll_to_bottom(&mut self) -> bool {
        let moved = self.display_offset() != 0;
        if moved {
            self.term.scroll_display(Scroll::Bottom);
            self.dirty = true;
        }
        moved
    }

    /// How many rows back into the scrollback the view is: 0 on the live
    /// screen.
    pub fn display_offset(&self) -> usize {
        self.term.grid().display_offset()
    }

    /// Whether the program has turned on SGR mouse reporting. When it has
    /// not, mouse input drives the grid's own selection instead of being sent
    /// to the program.
    pub fn sgr_mouse_mode(&self) -> bool {
        let mode = self.term.mode();
        mode.contains(TermMode::SGR_MOUSE) && mode.intersects(TermMode::MOUSE_MODE)
    }

    /// Whether the program has turned on bracketed paste, and so wants pasted
    /// text wrapped in markers it can recognise.
    ///
    /// A shell that has it on uses it to refuse to *run* a pasted multi-line
    /// command until the user presses Enter, which is the difference between
    /// pasting a script and executing one by accident.
    pub fn bracketed_paste_mode(&self) -> bool {
        self.term.mode().contains(TermMode::BRACKETED_PASTE)
    }

    /// Starts a plain text selection at this visible cell, replacing any
    /// other. Cells are where they show, so a selection made scrolled back
    /// covers the scrollback rows it was made over.
    pub fn start_selection(&mut self, column: usize, row: usize) {
        let point = self.point_at(column, row);
        self.term.selection = Some(Selection::new(SelectionType::Simple, point, Side::Left));
        self.dirty = true;
    }

    /// Extends the selection in progress, if any, to this visible cell.
    pub fn update_selection(&mut self, column: usize, row: usize) {
        let point = self.point_at(column, row);
        if let Some(selection) = &mut self.term.selection {
            selection.update(point, Side::Right);
            self.dirty = true;
        }
    }

    pub fn clear_selection(&mut self) {
        if self.term.selection.take().is_some() {
            self.dirty = true;
        }
    }

    /// The selected text, or `None` for no selection or an empty one.
    pub fn selection_text(&self) -> Option<String> {
        self.term.selection_to_string()
    }

    /// Whether this cell is inside the selection, for drawing a highlight.
    pub fn is_selected(&self, column: usize, row: usize) -> bool {
        self.term
            .selection
            .as_ref()
            .and_then(|selection| selection.to_range(&self.term))
            .is_some_and(|range| range.contains(self.point_at(column, row)))
    }

    /// A visible row's text, trailing blanks trimmed - scrolled back, a row of
    /// the scrollback.
    ///
    /// # Panics
    ///
    /// If `row` is not below [`Self::size`]'s `rows`.
    pub fn row_text(&self, row: usize) -> String {
        let mut text: String = self.row_cells(row).map(|cell| cell.c).collect();
        text.truncate(text.trim_end_matches(' ').len());
        text
    }

    /// A visible row's cells, left to right. Panics like [`Self::row_text`].
    pub(crate) fn row_cells(&self, row: usize) -> impl Iterator<Item = &Cell> {
        self.term.grid()[self.point_at(0, row).line].into_iter()
    }

    /// Whether the grid changed since the last call, clearing the flag.
    pub(crate) fn take_dirty(&mut self) -> bool {
        std::mem::take(&mut self.dirty)
    }

    /// Every event raised since the last call.
    pub(crate) fn drain_events(&mut self) -> Vec<Event> {
        self.events.try_iter().collect()
    }
}

impl Grid {
    /// The grid point a visible cell shows: `row` rows down from the top of
    /// the view, which is `display_offset` rows back into the scrollback.
    fn point_at(&self, column: usize, row: usize) -> Point {
        Point::new(Line(row as i32 - self.display_offset() as i32),
                   Column(column))
    }
}

#[cfg(test)]
mod tests;
