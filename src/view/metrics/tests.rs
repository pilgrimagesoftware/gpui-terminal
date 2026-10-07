//! Pixel-to-cell arithmetic.

use gpui_kit::{point, px, size};

use super::*;

const CELL: CellMetrics = CellMetrics { width:  8.,
                                        height: 16., };

#[test]
fn a_pane_holds_the_whole_cells_that_fit() {
    assert_eq!(CELL.grid_size(size(px(805.), px(330.))),
               GridSize { columns: 100,
                          rows:    20, });
}

#[test]
fn a_collapsed_pane_still_holds_one_cell() {
    assert_eq!(CELL.grid_size(size(px(0.), px(3.))),
               GridSize { columns: 1,
                          rows:    1, });
}

#[test]
fn a_point_maps_to_the_cell_it_falls_in() {
    let grid = GridSize { columns: 80,
                          rows:    24, };

    assert_eq!(CELL.cell_at(point(px(0.), px(0.)), grid), (0, 0));
    assert_eq!(CELL.cell_at(point(px(7.9), px(15.9)), grid), (0, 0));
    assert_eq!(CELL.cell_at(point(px(8.), px(16.)), grid), (1, 1));
    assert_eq!(CELL.cell_at(point(px(20.), px(40.)), grid), (2, 2));
}

#[test]
fn a_point_outside_the_grid_clamps_to_its_edge() {
    let grid = GridSize { columns: 10,
                          rows:    5, };

    assert_eq!(CELL.cell_at(point(px(-4.), px(-4.)), grid), (0, 0));
    assert_eq!(CELL.cell_at(point(px(1000.), px(1000.)), grid), (9, 4));
}

#[test]
fn pixels_scroll_by_whole_and_partial_lines() {
    assert!((CELL.pixels_to_lines(px(24.)) - 1.5).abs() < f32::EPSILON);
}

/// The descent counts by size, whichever sign the platform gives it: macOS
/// reports it negative, and adding it as given made rows shorter than their
/// glyphs.
#[test]
fn a_row_holds_the_ascent_and_descent_whatever_the_descents_sign() {
    assert_eq!(line_height(13.3, -3.6), 17.);
    assert_eq!(line_height(13.3, 3.6), 17.);
    assert_eq!(line_height(0., 0.), 1., "never collapses");
}

/// Measured from a real (headless) text system, whose font reports its
/// descent negative as macOS does: a cell is at least the font's full
/// line - ascent plus descent - at the default size and at a larger one.
#[gpui_kit::test]
fn a_cell_fits_the_fonts_full_line_at_every_size(cx: &mut gpui_kit::TestAppContext) {
    cx.update(|cx| {
          for size in [13., 26.] {
              let style = TerminalStyle::new("Menlo", px(size));
              let text_system = cx.text_system();
              let font_id = text_system.resolve_font(&gpui_kit::font(style.font_family.clone()));
              let ascent = f32::from(text_system.ascent(font_id, style.font_size));
              let descent = f32::from(text_system.descent(font_id, style.font_size));
              let line = ascent + descent.abs();

              let cell = CellMetrics::measure(&style, cx);

              assert!(cell.height >= line,
                      "{size}px: a {:.1}px row must hold the font's {line:.1}px line",
                      cell.height);
              assert!(cell.height < line + 1.,
                      "and only round up to a whole pixel");
          }
      });
}
