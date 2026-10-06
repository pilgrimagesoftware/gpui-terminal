//! Output-to-cells parsing, backed by `alacritty_terminal`: feed raw bytes
//! into a VT/ANSI parser, read back the cell grid and the events the running
//! program raised (title changes, clipboard requests, bell, replies it
//! expects written back).

use std::sync::mpsc;

use alacritty_terminal::event::{Event, EventListener};
use alacritty_terminal::grid::Dimensions;
use alacritty_terminal::index::{Column, Line, Point, Side};
use alacritty_terminal::selection::{Selection, SelectionType};
use alacritty_terminal::term::cell::Cell;
use alacritty_terminal::term::{Config as TermConfig, Term, TermMode};
use alacritty_terminal::vte::ansi::Processor;

/// A terminal's visible size, in columns and rows. Scrollback beyond `rows`
/// is tracked by the grid but not addressed through this.
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

    /// The cursor's column and row within the visible grid, 0-indexed.
    pub fn cursor(&self) -> (usize, usize) {
        let point = self.term.grid().cursor.point;
        (point.column.0, point.line.0.max(0) as usize)
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

    /// Starts a plain text selection at this cell, replacing any other.
    pub fn start_selection(&mut self, column: usize, row: usize) {
        self.term.selection =
            Some(Selection::new(SelectionType::Simple, point_at(column, row), Side::Left));
        self.dirty = true;
    }

    /// Extends the selection in progress, if any, to this cell.
    pub fn update_selection(&mut self, column: usize, row: usize) {
        if let Some(selection) = &mut self.term.selection {
            selection.update(point_at(column, row), Side::Right);
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
            .is_some_and(|range| range.contains(point_at(column, row)))
    }

    /// A visible row's text, trailing blanks trimmed.
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
        self.term.grid()[Line(row as i32)].into_iter()
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

fn point_at(column: usize, row: usize) -> Point {
    Point::new(Line(row as i32), Column(column))
}

#[cfg(test)]
mod tests;
