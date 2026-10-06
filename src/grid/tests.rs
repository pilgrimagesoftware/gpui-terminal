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

/// 30 numbered lines into a 5-row grid: the screen shows the last five, and
/// the rest went into the scrollback.
fn numbered(rows: usize) -> Grid {
    let mut grid = grid(20, rows);
    for line in 0..30 {
        grid.feed(format!("line {line}\r\n").as_bytes());
    }
    grid
}

#[test]
fn scrolling_back_shows_the_scrollback_and_scrolling_down_returns() {
    let mut grid = numbered(5);
    assert_eq!(grid.row_text(0), "line 26");
    assert_eq!(grid.display_offset(), 0);

    assert!(grid.scroll_display(10), "the view moved");
    assert_eq!(grid.display_offset(), 10);
    assert_eq!(grid.row_text(0), "line 16");
    assert_eq!(grid.row_text(4), "line 20");
    assert_eq!(grid.cursor_in_view(),
               None,
               "the cursor's row is out of sight");

    assert!(grid.scroll_display(-4));
    assert_eq!(grid.row_text(0), "line 20");

    assert!(grid.scroll_to_bottom());
    assert_eq!(grid.row_text(0), "line 26");
    assert_eq!(grid.cursor_in_view(), Some((0, 4)));
    assert!(!grid.scroll_to_bottom(), "already at the bottom");
}

#[test]
fn scrolling_stops_at_either_end() {
    let mut grid = numbered(5);
    assert!(!grid.scroll_display(-3), "nothing below the live screen");
    grid.scroll_display(1_000);
    assert_eq!(grid.row_text(0),
               "line 0",
               "the oldest line, and no further");
    assert!(!grid.scroll_display(1));
}

#[test]
fn a_selection_made_scrolled_back_covers_what_was_shown() {
    let mut grid = numbered(5);
    grid.scroll_display(10);
    grid.start_selection(0, 0);
    grid.update_selection(6, 0);
    assert!(grid.is_selected(3, 0));
    assert_eq!(grid.selection_text().as_deref(), Some("line 16"));

    grid.scroll_to_bottom();
    assert!(!grid.is_selected(3, 0), "the selection stays on its rows");
}

#[test]
fn the_alternate_screen_has_no_scrollback_to_scroll() {
    let mut grid = numbered(5);
    grid.feed(b"\x1b[?1049h\x1b[2J\x1b[Hfull screen");
    assert!(!grid.scroll_display(5));
    assert_eq!(grid.row_text(0), "full screen");
}
