//! Tests for CSS Table layout
//!
//! These tests verify the table layout algorithm's handling of:
//! - Basic table structure (rows and cells)
//! - Column width computation
//! - Row height computation
//! - Cell positioning
//! - Padding and border handling
//! - Min/max size constraints
//! - Fixed and auto table layout modes
//! - Column and row spans
//! - Integration with block layout (table as block child)

use taffy::prelude::*;
use taffy::style_helpers::*;
use taffy::{AvailableSpace, Display, Layout, LengthPercentage, Rect, Size, Style, TaffyTree};
use taffy_test_helpers::{new_test_tree, TestNodeContext};

/// Helper: create a table cell (leaf node) with a fixed size
fn cell(width: f32, height: f32) -> Style {
    Style {
        size: Size::from_lengths(width, height),
        ..Default::default()
    }
}

/// Helper: create a LengthPercentage rect from f32 values
fn lp_rect(left: f32, right: f32, top: f32, bottom: f32) -> Rect<LengthPercentage> {
    Rect {
        left: LengthPercentage::from_length(left),
        right: LengthPercentage::from_length(right),
        top: LengthPercentage::from_length(top),
        bottom: LengthPercentage::from_length(bottom),
    }
}

/// Helper: create a LengthPercentage size from f32 values
fn lp_size(width: f32, height: f32) -> Size<LengthPercentage> {
    Size {
        width: LengthPercentage::from_length(width),
        height: LengthPercentage::from_length(height),
    }
}

/// Helper: get the layout of a node
fn get_layout(taffy: &TaffyTree<TestNodeContext>, node: taffy::NodeId) -> &Layout {
    taffy.layout(node).unwrap()
}

/// Helper: assert a node's size
fn assert_size(taffy: &TaffyTree<TestNodeContext>, node: taffy::NodeId, width: f32, height: f32) {
    let layout = get_layout(taffy, node);
    assert!(
        (layout.size.width - width).abs() < 0.1,
        "expected width {} but got {} for node {:?}",
        width,
        layout.size.width,
        node
    );
    assert!(
        (layout.size.height - height).abs() < 0.1,
        "expected height {} but got {} for node {:?}",
        height,
        layout.size.height,
        node
    );
}

/// Helper: assert a node's position
fn assert_position(taffy: &TaffyTree<TestNodeContext>, node: taffy::NodeId, x: f32, y: f32) {
    let layout = get_layout(taffy, node);
    assert!(
        (layout.location.x - x).abs() < 0.1,
        "expected x {} but got {} for node {:?}",
        x,
        layout.location.x,
        node
    );
    assert!(
        (layout.location.y - y).abs() < 0.1,
        "expected y {} but got {} for node {:?}",
        y,
        layout.location.y,
        node
    );
}

#[test]
fn table_empty() {
    let mut taffy = new_test_tree();
    let table = taffy.new_with_children(Style { display: Display::Table, ..Default::default() }, &[]).unwrap();
    taffy.compute_layout_with_measure(table, Size::MAX_CONTENT, taffy_test_helpers::test_measure_function).unwrap();

    assert_size(&taffy, table, 0.0, 0.0);
}

#[test]
fn table_single_row_single_cell() {
    let mut taffy = new_test_tree();
    let cell = taffy.new_leaf(cell(100.0, 50.0)).unwrap();
    let row = taffy.new_with_children(Style { display: Display::Block, ..Default::default() }, &[cell]).unwrap();
    let table = taffy.new_with_children(Style { display: Display::Table, ..Default::default() }, &[row]).unwrap();
    taffy.compute_layout_with_measure(table, Size::MAX_CONTENT, taffy_test_helpers::test_measure_function).unwrap();

    // Table should be the size of its single cell
    assert_size(&taffy, table, 100.0, 50.0);
    assert_size(&taffy, row, 100.0, 50.0);
    assert_size(&taffy, cell, 100.0, 50.0);
    assert_position(&taffy, cell, 0.0, 0.0);
}

#[test]
fn table_single_row_multiple_cells() {
    let mut taffy = new_test_tree();
    let cell0 = taffy.new_leaf(cell(100.0, 50.0)).unwrap();
    let cell1 = taffy.new_leaf(cell(80.0, 50.0)).unwrap();
    let cell2 = taffy.new_leaf(cell(120.0, 50.0)).unwrap();
    let row = taffy
        .new_with_children(Style { display: Display::Block, ..Default::default() }, &[cell0, cell1, cell2])
        .unwrap();
    let table = taffy.new_with_children(Style { display: Display::Table, ..Default::default() }, &[row]).unwrap();
    taffy.compute_layout_with_measure(table, Size::MAX_CONTENT, taffy_test_helpers::test_measure_function).unwrap();

    // Table width = sum of cell widths = 100 + 80 + 120 = 300
    // Table height = max cell height = 50
    assert_size(&taffy, table, 300.0, 50.0);
    assert_size(&taffy, row, 300.0, 50.0);

    // Cells should be positioned side by side
    assert_position(&taffy, cell0, 0.0, 0.0);
    assert_position(&taffy, cell1, 100.0, 0.0);
    assert_position(&taffy, cell2, 180.0, 0.0);
}

#[test]
fn table_multiple_rows() {
    let mut taffy = new_test_tree();
    // Row 0: two cells of width 100 and 80, height 50
    let r0c0 = taffy.new_leaf(cell(100.0, 50.0)).unwrap();
    let r0c1 = taffy.new_leaf(cell(80.0, 50.0)).unwrap();
    let row0 = taffy.new_with_children(Style { display: Display::Block, ..Default::default() }, &[r0c0, r0c1]).unwrap();

    // Row 1: two cells of width 60 and 120, height 40
    let r1c0 = taffy.new_leaf(cell(60.0, 40.0)).unwrap();
    let r1c1 = taffy.new_leaf(cell(120.0, 40.0)).unwrap();
    let row1 = taffy.new_with_children(Style { display: Display::Block, ..Default::default() }, &[r1c0, r1c1]).unwrap();

    let table = taffy.new_with_children(Style { display: Display::Table, ..Default::default() }, &[row0, row1]).unwrap();
    taffy.compute_layout_with_measure(table, Size::MAX_CONTENT, taffy_test_helpers::test_measure_function).unwrap();

    // Column widths: max(100, 60) = 100, max(80, 120) = 120
    // Table width = 100 + 120 = 220
    // Table height = 50 + 40 = 90
    assert_size(&taffy, table, 220.0, 90.0);

    // Row 0 should be at y=0, height 50
    assert_position(&taffy, row0, 0.0, 0.0);
    assert_size(&taffy, row0, 220.0, 50.0);

    // Row 1 should be at y=50, height 40
    assert_position(&taffy, row1, 0.0, 50.0);
    assert_size(&taffy, row1, 220.0, 40.0);

    // Row 0 cells: positioned at column widths
    assert_position(&taffy, r0c0, 0.0, 0.0);
    assert_size(&taffy, r0c0, 100.0, 50.0);
    assert_position(&taffy, r0c1, 100.0, 0.0);
    assert_size(&taffy, r0c1, 120.0, 50.0);

    // Row 1 cells: positioned at column widths
    assert_position(&taffy, r1c0, 0.0, 50.0);
    assert_size(&taffy, r1c0, 100.0, 40.0);
    assert_position(&taffy, r1c1, 100.0, 50.0);
    assert_size(&taffy, r1c1, 120.0, 40.0);
}

#[test]
fn table_with_padding_and_border() {
    let mut taffy = new_test_tree();
    let cell_node = taffy.new_leaf(cell(100.0, 50.0)).unwrap();
    let row = taffy.new_with_children(Style { display: Display::Block, ..Default::default() }, &[cell_node]).unwrap();
    let table = taffy
        .new_with_children(
            Style {
                display: Display::Table,
                padding: lp_rect(10.0, 10.0, 10.0, 10.0),
                border: lp_rect(5.0, 5.0, 5.0, 5.0),
                ..Default::default()
            },
            &[row],
        )
        .unwrap();
    taffy.compute_layout_with_measure(table, Size::MAX_CONTENT, taffy_test_helpers::test_measure_function).unwrap();

    // Table outer size = cell size + padding (10*2) + border (5*2) = 100+30 = 130, 50+30 = 80
    assert_size(&taffy, table, 130.0, 80.0);

    // The cell should be offset by padding + border = 15 on each side
    assert_position(&taffy, cell_node, 15.0, 15.0);
}

#[test]
fn table_with_explicit_width() {
    let mut taffy = new_test_tree();
    let cell0 = taffy.new_leaf(cell(100.0, 50.0)).unwrap();
    let cell1 = taffy.new_leaf(cell(100.0, 50.0)).unwrap();
    let row = taffy.new_with_children(Style { display: Display::Block, ..Default::default() }, &[cell0, cell1]).unwrap();
    let table = taffy
        .new_with_children(
            Style { display: Display::Table, size: Size { width: taffy::style::Dimension::from_length(400.0), height: taffy::style::Dimension::AUTO }, ..Default::default() },
            &[row],
        )
        .unwrap();
    taffy.compute_layout_with_measure(table, Size::MAX_CONTENT, taffy_test_helpers::test_measure_function).unwrap();

    // Table width should be 400 (explicitly set)
    assert_size(&taffy, table, 400.0, 50.0);

    // Each cell should get 200px (400 / 2 columns)
    assert_size(&taffy, cell0, 200.0, 50.0);
    assert_size(&taffy, cell1, 200.0, 50.0);
}

#[test]
fn table_with_explicit_height() {
    let mut taffy = new_test_tree();
    let cell0 = taffy.new_leaf(cell(100.0, 50.0)).unwrap();
    let row = taffy.new_with_children(Style { display: Display::Block, ..Default::default() }, &[cell0]).unwrap();
    let table = taffy
        .new_with_children(
            Style { display: Display::Table, size: Size { width: taffy::style::Dimension::AUTO, height: taffy::style::Dimension::from_length(200.0) }, ..Default::default() },
            &[row],
        )
        .unwrap();
    taffy.compute_layout_with_measure(table, Size::MAX_CONTENT, taffy_test_helpers::test_measure_function).unwrap();

    // Table height should be 200 (explicitly set)
    assert_size(&taffy, table, 100.0, 200.0);
    // Row should stretch to fill the table height
    assert_size(&taffy, row, 100.0, 200.0);
}

#[test]
fn table_with_min_max_constraints() {
    let mut taffy = new_test_tree();
    let cell = taffy.new_leaf(cell(100.0, 50.0)).unwrap();
    let row = taffy.new_with_children(Style { display: Display::Block, ..Default::default() }, &[cell]).unwrap();
    let table = taffy
        .new_with_children(
            Style {
                display: Display::Table,
                min_size: Size::from_lengths(150.0, 80.0),
                ..Default::default()
            },
            &[row],
        )
        .unwrap();
    taffy.compute_layout_with_measure(table, Size::MAX_CONTENT, taffy_test_helpers::test_measure_function).unwrap();

    // Min size should override content size
    assert_size(&taffy, table, 150.0, 80.0);
}

#[test]
fn table_with_colspan() {
    let mut taffy = new_test_tree();
    // 3 columns, but the first cell spans 2 columns
    let cell0 = taffy
        .new_leaf(Style { size: Size::from_lengths(200.0, 50.0), column_span: 2, ..Default::default() })
        .unwrap();
    let cell1 = taffy.new_leaf(cell(100.0, 50.0)).unwrap();
    let row = taffy.new_with_children(Style { display: Display::Block, ..Default::default() }, &[cell0, cell1]).unwrap();

    // Second row: 3 cells, each 80px wide
    let r1c0 = taffy.new_leaf(cell(80.0, 40.0)).unwrap();
    let r1c1 = taffy.new_leaf(cell(80.0, 40.0)).unwrap();
    let r1c2 = taffy.new_leaf(cell(80.0, 40.0)).unwrap();
    let row1 = taffy.new_with_children(Style { display: Display::Block, ..Default::default() }, &[r1c0, r1c1, r1c2]).unwrap();

    let table = taffy.new_with_children(Style { display: Display::Table, ..Default::default() }, &[row, row1]).unwrap();
    taffy.compute_layout_with_measure(table, Size::MAX_CONTENT, taffy_test_helpers::test_measure_function).unwrap();

    // Column widths: col0 = max(200/2 from span, 80) = 100, col1 = max(200/2 from span, 80) = 100, col2 = max(100, 80) = 100
    // Table width = 100 + 100 + 100 = 300
    assert_size(&taffy, table, 300.0, 90.0);

    // First row: cell0 spans columns 0 and 1 (width 200), cell1 is column 2 (width 100)
    assert_size(&taffy, cell0, 200.0, 50.0);
    assert_size(&taffy, cell1, 100.0, 50.0);
}

#[test]
fn table_stretch_to_available_width() {
    let mut taffy = new_test_tree();
    let cell = taffy.new_leaf(cell(100.0, 50.0)).unwrap();
    let row = taffy.new_with_children(Style { display: Display::Block, ..Default::default() }, &[cell]).unwrap();
    let table = taffy.new_with_children(Style { display: Display::Table, ..Default::default() }, &[row]).unwrap();

    // Give the table a definite available width of 500px
    taffy.compute_layout_with_measure(
        table,
        Size { width: AvailableSpace::Definite(500.0), height: AvailableSpace::MaxContent },
        taffy_test_helpers::test_measure_function,
    )
    .unwrap();

    // Table should stretch to fill the available width (like a block)
    assert_size(&taffy, table, 500.0, 50.0);
    // The cell should stretch to fill the table width
    assert_size(&taffy, cell, 500.0, 50.0);
}

#[test]
fn table_compute_size_mode() {
    let mut taffy = new_test_tree();
    let cell0 = taffy.new_leaf(cell(100.0, 50.0)).unwrap();
    let cell1 = taffy.new_leaf(cell(80.0, 50.0)).unwrap();
    let row = taffy.new_with_children(Style { display: Display::Block, ..Default::default() }, &[cell0, cell1]).unwrap();
    let table = taffy.new_with_children(Style { display: Display::Table, ..Default::default() }, &[row]).unwrap();

    // Compute layout with max content available space
    taffy.compute_layout_with_measure(table, Size::MAX_CONTENT, taffy_test_helpers::test_measure_function).unwrap();

    // Table should size to content: width = 100 + 80 = 180, height = 50
    assert_size(&taffy, table, 180.0, 50.0);
}

#[test]
fn table_as_block_child() {
    let mut taffy = new_test_tree();
    let cell = taffy.new_leaf(cell(100.0, 50.0)).unwrap();
    let row = taffy.new_with_children(Style { display: Display::Block, ..Default::default() }, &[cell]).unwrap();
    let table = taffy.new_with_children(Style { display: Display::Table, ..Default::default() }, &[row]).unwrap();
    let block = taffy
        .new_with_children(
            Style { display: Display::Block, size: Size::from_lengths(500.0, 200.0), ..Default::default() },
            &[table],
        )
        .unwrap();
    taffy.compute_layout_with_measure(block, Size::MAX_CONTENT, taffy_test_helpers::test_measure_function).unwrap();

    // Table should not stretch to fill the block's width by default
    // (CSS tables are shrink-to-fit when inside a block container)
    // Table width = 100 (content), height = 50
    assert_size(&taffy, table, 100.0, 50.0);
}

#[test]
fn table_with_border_spacing() {
    let mut taffy = new_test_tree();
    let cell0 = taffy.new_leaf(cell(100.0, 50.0)).unwrap();
    let cell1 = taffy.new_leaf(cell(80.0, 50.0)).unwrap();
    let row = taffy.new_with_children(Style { display: Display::Block, ..Default::default() }, &[cell0, cell1]).unwrap();
    let table = taffy
        .new_with_children(
            Style {
                display: Display::Table,
                border_spacing: lp_size(5.0, 3.0),
                ..Default::default()
            },
            &[row],
        )
        .unwrap();
    taffy.compute_layout_with_measure(table, Size::MAX_CONTENT, taffy_test_helpers::test_measure_function).unwrap();

    // Table width = cell0(100) + spacing(5) + cell1(80) + spacing_left(5) + spacing_right(5) = 195
    // Table height = cell(50) + spacing_top(3) + spacing_bottom(3) = 56
    assert_size(&taffy, table, 195.0, 56.0);

    // Cell0 should be offset by border_spacing
    assert_position(&taffy, cell0, 5.0, 3.0);
    // Cell1 should be after cell0 + spacing
    assert_position(&taffy, cell1, 110.0, 3.0);
}

#[test]
fn table_nested() {
    let mut taffy = new_test_tree();
    // Inner table: 1 row, 1 cell of 60x30
    let inner_cell = taffy.new_leaf(cell(60.0, 30.0)).unwrap();
    let inner_row = taffy
        .new_with_children(Style { display: Display::Block, ..Default::default() }, &[inner_cell])
        .unwrap();
    let inner_table = taffy
        .new_with_children(Style { display: Display::Table, ..Default::default() }, &[inner_row])
        .unwrap();

    // Outer table: 1 row, 1 cell containing the inner table
    let outer_cell = taffy
        .new_with_children(Style { display: Display::Block, ..Default::default() }, &[inner_table])
        .unwrap();
    let outer_row = taffy
        .new_with_children(Style { display: Display::Block, ..Default::default() }, &[outer_cell])
        .unwrap();
    let outer_table = taffy
        .new_with_children(Style { display: Display::Table, ..Default::default() }, &[outer_row])
        .unwrap();

    taffy.compute_layout_with_measure(outer_table, Size::MAX_CONTENT, taffy_test_helpers::test_measure_function).unwrap();

    // Inner table should be 60x30
    assert_size(&taffy, inner_table, 60.0, 30.0);
    // Outer table should be 60x30 (containing the inner table)
    assert_size(&taffy, outer_table, 60.0, 30.0);
}