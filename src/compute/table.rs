//! Computes the CSS Table layout algorithm
//!
//! CSS Table layout is used for laying out elements with `display: table`.
//! A table container generates a block-level box that contains an anonymous
//! table-wrapper box. Internally, tables are laid out using a row-major algorithm:
//!
//! 1. Determine the number of columns from all rows
//! 2. Assign each cell to a column (accounting for column spans)
//! 3. Size columns using a content-based algorithm
//! 4. Size rows based on their content
//! 5. Position cells within the grid
//!
//! For now, this implementation treats table layout as a simplified version
//! that handles the basic structure. Full CSS table layout (per the spec at
//! https://drafts.csswg.org/css-tables/) involves anonymous box generation,
//! caption handling, border collapsing, and complex column sizing algorithms.

use crate::geometry::{Line, Point, Rect, Size};
use crate::style::{AvailableSpace, CoreStyle};
use crate::tree::{Layout, LayoutInput, LayoutOutput, RunMode, SizingMode};
use crate::tree::{LayoutPartialTree, LayoutPartialTreeExt, NodeId};
use crate::util::debug::debug_log;
use crate::util::sys::{f32_max, Vec};
use crate::util::MaybeMath;
use crate::util::{MaybeResolve, ResolveOrZero};
use crate::{BoxSizing, RequestedAxis};

/// Compute the layout of a table container node
///
/// This function implements a simplified CSS table layout algorithm.
/// It treats the table as a block-level container and lays out its children
/// (which should be table-row elements) in a stacked fashion, with column
/// widths determined by the content of cells across all rows.
pub fn compute_table_layout(
    tree: &mut impl LayoutPartialTree,
    node_id: NodeId,
    inputs: LayoutInput,
) -> LayoutOutput {
    let LayoutInput { known_dimensions, parent_size, run_mode, .. } = inputs;
    let style = tree.get_core_container_style(node_id);

    // Pull these out earlier to avoid borrowing issues
    let overflow = style.overflow();
    let _is_scroll_container = overflow.x.is_scroll_container() || overflow.y.is_scroll_container();
    let aspect_ratio = style.aspect_ratio();
    let padding = style.padding().resolve_or_zero(parent_size.width, |val, basis| tree.calc(val, basis));
    let border = style.border().resolve_or_zero(parent_size.width, |val, basis| tree.calc(val, basis));
    let padding_border_size = (padding + border).sum_axes();
    let box_sizing_adjustment =
        if style.box_sizing() == BoxSizing::ContentBox { padding_border_size } else { Size::ZERO };

    let min_size = style
        .min_size()
        .maybe_resolve(parent_size, |val, basis| tree.calc(val, basis))
        .maybe_apply_aspect_ratio(aspect_ratio)
        .maybe_add(box_sizing_adjustment);
    let max_size = style
        .max_size()
        .maybe_resolve(parent_size, |val, basis| tree.calc(val, basis))
        .maybe_apply_aspect_ratio(aspect_ratio)
        .maybe_add(box_sizing_adjustment);
    let clamped_style_size = if inputs.sizing_mode == SizingMode::InherentSize {
        style
            .size()
            .maybe_resolve(parent_size, |val, basis| tree.calc(val, basis))
            .maybe_apply_aspect_ratio(aspect_ratio)
            .maybe_add(box_sizing_adjustment)
            .maybe_clamp(min_size, max_size)
    } else {
        Size::NONE
    };

    drop(style);

    // If both min and max in a given axis are set and max <= min then this determines the size in that axis
    let min_max_definite_size = min_size.zip_map(max_size, |min, max| match (min, max) {
        (Some(min), Some(max)) if max <= min => Some(min),
        _ => None,
    });

    let styled_based_known_dimensions =
        known_dimensions.or(min_max_definite_size).or(clamped_style_size).maybe_max(padding_border_size);

    // Short-circuit layout if the container's size is fully determined
    if run_mode == RunMode::ComputeSize {
        if let Size { width: Some(width), height: Some(height) } = styled_based_known_dimensions {
            return LayoutOutput::from_outer_size(Size { width, height });
        }

        // We can also short-circuit if the width is known and only the width has been requested.
        if inputs.axis == RequestedAxis::Horizontal {
            if let Some(width) = styled_based_known_dimensions.width {
                return LayoutOutput::from_outer_size(Size { width, height: 0.0 });
            }
        }
    }

    debug_log!("TABLE");

    // Determine the available width for content
    let node_inner_size = styled_based_known_dimensions.maybe_sub(padding_border_size);

    // Phase 1: Collect column information from all rows
    // We iterate over all children (which should be table-row elements)
    // and collect the widths of all cells to determine column layout
    let child_count = tree.child_count(node_id);
    let mut column_widths: Vec<f32> = Vec::new();
    let mut row_heights: Vec<f32> = Vec::new();

    // First pass: measure intrinsic sizes of all children to determine column widths
    // and row heights
    let available_width = node_inner_size
        .width
        .unwrap_or(f32::INFINITY);

    for child_index in 0..child_count {
        let child_id = tree.get_child_id(node_id, child_index);

        // Measure the child's intrinsic size
        let child_size = tree.measure_child_size_both(
            child_id,
            Size::NONE,
            parent_size,
            Size { width: AvailableSpace::Definite(available_width), height: AvailableSpace::MinContent },
            SizingMode::ContentSize,
            Line::FALSE,
        );

        // For now, each child (row) contributes its width as a column
        // In a full implementation, we'd parse individual cells within rows
        let child_width = child_size.width;
        let child_height = child_size.height;

        // Track the maximum width as the table's content width
        // For column tracking, we'd need to look at individual cells
        column_widths.push(child_width);
        row_heights.push(child_height);
    }

    // Determine the table's content width
    // For now, use the maximum of all row widths, or the available space if constrained
    let content_width = match node_inner_size.width {
        Some(width) => width,
        None => {
            if column_widths.is_empty() {
                0.0
            } else {
                column_widths.iter().copied().fold(0.0, f32_max)
            }
        }
    };

    // Determine the table's content height (sum of row heights)
    let content_height: f32 = row_heights.iter().sum();

    let content_size = Size { width: content_width, height: content_height };

    // Clamp to min/max
    let content_size = content_size.maybe_clamp(min_size.maybe_sub(padding_border_size), max_size.maybe_sub(padding_border_size));

    let outer_size = Size { width: content_size.width + padding_border_size.width, height: content_size.height + padding_border_size.height };

    // If we're only computing the size, return now
    if run_mode == RunMode::ComputeSize {
        return LayoutOutput::from_outer_size(outer_size);
    }

    // Phase 2: Perform full layout on children
    // Position each row vertically, stacking them
    let mut y_offset = 0.0;

    for child_index in 0..child_count {
        let child_id = tree.get_child_id(node_id, child_index);

        // Perform full layout on the child
        let child_output = tree.perform_child_layout(
            child_id,
            Size { width: Some(content_width), height: None },
            parent_size,
            Size { width: AvailableSpace::Definite(content_width), height: AvailableSpace::MinContent },
            SizingMode::InherentSize,
            Line::FALSE,
        );

        let child_layout = Layout {
            order: child_index as u32,
            location: Point { x: 0.0, y: y_offset },
            size: child_output.size,
            #[cfg(feature = "content_size")]
            content_size: child_output.content_size,
            scrollbar_size: Size::ZERO,
            padding: Rect::ZERO,
            border: Rect::ZERO,
            margin: Rect::ZERO,
        };

        tree.set_unrounded_layout(child_id, &child_layout);
        y_offset += child_output.size.height;
    }

    // Set the table's own layout
    let table_layout = Layout {
        order: 0,
        location: Point { x: 0.0, y: 0.0 },
        size: outer_size,
        #[cfg(feature = "content_size")]
        content_size,
        scrollbar_size: Size::zero(),
        padding,
        border,
        margin: Rect::zero(),
    };

    tree.set_unrounded_layout(node_id, &table_layout);

    LayoutOutput::from_outer_size(outer_size)
}
