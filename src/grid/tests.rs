//! Parsing output into the grid, selection, and the dirty flag.

use super::*;

fn grid(columns: usize, rows: usize) -> Grid {
    Grid::new(GridSize { columns, rows })
}

#[test]
fn feeds_plain_text_into_the_grid() {
    let mut grid = grid(20, 5);
    grid.feed(b"hello");
    assert_eq!(grid.row_text(0), "hello");
}

#[test]
fn alternate_screen_content_and_cursor_are_reflected() {
    let mut grid = grid(20, 5);
    grid.feed(b"primary screen text");
    // Enter the alternate screen buffer (what full-screen TUIs like
    // Claude Code's use), clear it, and draw new content plus move the
    // cursor - both should read from the now-active alt screen, not the
    // primary one still holding "primary screen text".
    grid.feed(b"\x1b[?1049h\x1b[2J\x1b[H");
    grid.feed(b"alt screen line");
    grid.feed(b"\x1b[3;1Hprompt row");
    assert_eq!(grid.row_text(0), "alt screen line");
    assert_eq!(grid.row_text(2), "prompt row");
    assert_eq!(grid.cursor(), (10, 2));

    // Leaving the alt screen restores the primary content untouched.
    grid.feed(b"\x1b[?1049l");
    assert_eq!(grid.row_text(0), "primary screen text");
}

#[test]
fn tracks_cursor_position_after_writes() {
    let mut grid = grid(20, 5);
    grid.feed(b"hi");
    assert_eq!(grid.cursor(), (2, 0));
}

#[test]
fn newline_and_carriage_return_move_to_the_next_line() {
    let mut grid = grid(20, 5);
    grid.feed(b"one\r\ntwo");
    assert_eq!(grid.row_text(0), "one");
    assert_eq!(grid.row_text(1), "two");
}

#[test]
fn resize_updates_reported_size() {
    let mut grid = grid(20, 5);
    grid.resize(GridSize { columns: 40,
                           rows:    10, });
    assert_eq!(grid.size(),
               GridSize { columns: 40,
                          rows:    10, });
}

#[test]
fn title_escape_sequence_produces_a_title_event() {
    let mut grid = grid(20, 5);
    grid.feed(b"\x1b]0;my title\x07");
    let events = grid.drain_events();
    assert!(events.iter().any(|event| matches!(
                             event,
                             Event::Title(title) if title == "my title"
                         )));
}

#[test]
fn sgr_color_codes_set_cell_foreground() {
    use alacritty_terminal::term::cell::Flags;

    let mut grid = grid(20, 5);
    grid.feed(b"\x1b[1mbold");
    let first = grid.row_cells(0).next().expect("a row has cells");
    assert!(first.flags.contains(Flags::BOLD));
}

#[test]
fn no_selection_text_before_selecting() {
    let mut grid = grid(20, 5);
    grid.feed(b"hello world");
    assert_eq!(grid.selection_text(), None);
}

#[test]
fn dragging_a_selection_captures_the_spanned_text() {
    let mut grid = grid(20, 5);
    grid.feed(b"hello world");
    grid.start_selection(0, 0);
    grid.update_selection(4, 0);
    assert_eq!(grid.selection_text(), Some("hello".to_string()));
}

#[test]
fn is_selected_reports_cells_within_the_selection() {
    let mut grid = grid(20, 5);
    grid.feed(b"hello world");
    grid.start_selection(0, 0);
    grid.update_selection(4, 0);
    assert!(grid.is_selected(0, 0));
    assert!(grid.is_selected(4, 0));
    assert!(!grid.is_selected(6, 0));
}

#[test]
fn clear_selection_removes_it() {
    let mut grid = grid(20, 5);
    grid.feed(b"hello world");
    grid.start_selection(0, 0);
    grid.update_selection(4, 0);
    grid.clear_selection();
    assert_eq!(grid.selection_text(), None);
    assert!(!grid.is_selected(0, 0));
}

#[test]
fn take_dirty_reports_and_clears_changes() {
    let mut grid = grid(20, 5);
    assert!(!grid.take_dirty(), "a fresh grid has nothing to repaint");

    grid.feed(b"hi");
    assert!(grid.take_dirty());
    assert!(!grid.take_dirty(), "dirty flag clears after being read");

    grid.resize(GridSize { columns: 30,
                           rows:    10, });
    assert!(grid.take_dirty());
}

#[test]
fn clearing_a_selection_marks_the_grid_dirty_only_when_there_was_one() {
    let mut grid = grid(20, 5);
    grid.take_dirty();

    grid.clear_selection();
    assert!(!grid.take_dirty(),
            "nothing was selected, so nothing to redraw");

    grid.start_selection(0, 0);
    grid.take_dirty();
    grid.clear_selection();
    assert!(grid.take_dirty());
}
