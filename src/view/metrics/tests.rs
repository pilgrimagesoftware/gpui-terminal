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
