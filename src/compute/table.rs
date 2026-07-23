//! Computes the CSS Table layout algorithm
//!
//! CSS Table layout is used for laying out elements with `display: table`.
//! A table container generates a block-level box that contains rows, each of
//! which contains cells. Internally, tables are laid out using a row-major
//! algorithm:
//!
//! 1. Measure each row's intrinsic (min-content and max-content) width
//! 2. Determine the table's content width from the row measurements
//! 3. Size rows based on their content
//! 4. Position rows vertically within the table
//!
//! This implementation treats each row as an opaque container that is
//! measured and laid out via the standard `compute_child_layout` mechanism.
//! Column width alignment across rows is approximate in this implementation
//! — each row receives the same definite width and lays out its cells
//! independently. A future enhancement could extend the tree traits to
//! provide direct access to cells (grandchildren) for precise column
//! width computation.
//!
//! See: <https://drafts.csswg.org/css-tables-3/>

use crate::geometry::{Line, Point, Rect, Size};
use crate::style::{AvailableSpace, CoreStyle};
use crate::tree::{Layout, LayoutInput, LayoutOutput, RunMode, SizingMode};
use crate::tree::{LayoutPartialTreeExt, NodeId};
use crate::util::debug::debug_log;
use crate::util::sys::{f32_max, Vec};
use crate::util::MaybeMath;
use crate::util::{MaybeResolve, ResolveOrZero};
use crate::{BoxSizing, LayoutTableContainer, RequestedAxis, TableContainerStyle};

/// Compute the layout of a table container node
///
/// This function implements the CSS table layout algorithm. It treats the
/// table's direct children as rows, measures each row's intrinsic size,
/// determines the table's content width, and then lays out each row with
/// that width.
pub fn compute_table_layout(
    tree: &mut impl LayoutTableContainer,
    node_id: NodeId,
    inputs: LayoutInput,
) -> LayoutOutput {
    let LayoutInput { known_dimensions, parent_size, run_mode, available_space, .. } = inputs;
    let style = tree.get_table_container_style(node_id);

    // Pull these out earlier to avoid borrowing issues
    let overflow = style.overflow();
    let _is_scroll_container = overflow.x.is_scroll_container() || overflow.y.is_scroll_container();
    let aspect_ratio = style.aspect_ratio();
    let padding = style.padding().resolve_or_zero(parent_size.width, |val, basis| tree.calc(val, basis));
    let border = style.border().resolve_or_zero(parent_size.width, |val, basis| tree.calc(val, basis));
    let padding_border_size = (padding + border).sum_axes();
    let box_sizing_adjustment =
        if style.box_sizing() == BoxSizing::ContentBox { padding_border_size } else { Size::ZERO };

    // Resolve border spacing (horizontal and vertical spacing between cells)
    let border_spacing = style.border_spacing().maybe_resolve(parent_size, |val, basis| tree.calc(val, basis));
    let border_spacing_h = border_spacing.width.unwrap_or(0.0);
    let border_spacing_v = border_spacing.height.unwrap_or(0.0);

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

    let child_count = tree.child_count(node_id);

    // Phase 1: Measure each row's intrinsic size
    //
    // We measure each row with:
    //   - MaxContent available width to get the row's preferred width
    //   - MinContent available width to get the row's minimum width
    //
    // The table's max-content width is the max of all rows' max-content widths.
    // The table's min-content width is the max of all rows' min-content widths.

    let mut row_max_content_widths: Vec<f32> = Vec::with_capacity(child_count);
    let mut row_min_content_widths: Vec<f32> = Vec::with_capacity(child_count);

    for child_index in 0..child_count {
        let child_id = tree.get_child_id(node_id, child_index);

        // Measure max-content width
        let max_content_size = tree.measure_child_size_both(
            child_id,
            Size::NONE,
            parent_size,
            Size { width: AvailableSpace::MaxContent, height: AvailableSpace::MaxContent },
            SizingMode::InherentSize,
            Line::FALSE,
        );
        row_max_content_widths.push(max_content_size.width);

        // Measure min-content width
        let min_content_size = tree.measure_child_size_both(
            child_id,
            Size::NONE,
            parent_size,
            Size { width: AvailableSpace::MinContent, height: AvailableSpace::MinContent },
            SizingMode::InherentSize,
            Line::FALSE,
        );
        row_min_content_widths.push(min_content_size.width);
    }

    // Compute the table's intrinsic content width
    let table_max_content_width = row_max_content_widths.iter().copied().fold(0.0, f32_max);
    let table_min_content_width = row_min_content_widths.iter().copied().fold(0.0, f32_max);

    // Account for border spacing in the intrinsic width
    let table_max_content_width = table_max_content_width + border_spacing_h * 2.0;
    let table_min_content_width = table_min_content_width + border_spacing_h * 2.0;

    // Determine the table's content width
    let content_width = match node_inner_size.width {
        Some(width) => width,
        None => {
            // No explicit width: tables use shrink-to-fit sizing.
            // They fill available width but not beyond max-content,
            // and not below min-content.
            match available_space.width {
                AvailableSpace::Definite(avail) => {
                    avail.max(table_min_content_width).min(table_max_content_width)
                }
                AvailableSpace::MinContent => table_min_content_width,
                AvailableSpace::MaxContent => table_max_content_width,
            }
        }
    };

    // Clamp content width to min/max (accounting for padding/border)
    let content_min = min_size.maybe_sub(padding_border_size);
    let content_max = max_size.maybe_sub(padding_border_size);
    let content_width = content_width
        .maybe_clamp(content_min.width, content_max.width)
        .max(0.0);

    // The width available for rows (excluding border spacing)
    let row_content_width = (content_width - border_spacing_h * 2.0).max(0.0);

    // Phase 2: Measure each row's height with the resolved content width
    let mut row_heights: Vec<f32> = Vec::with_capacity(child_count);

    for child_index in 0..child_count {
        let child_id = tree.get_child_id(node_id, child_index);

        // Measure the row's height with the resolved content width
        let row_size = tree.measure_child_size_both(
            child_id,
            Size { width: Some(row_content_width), height: None },
            parent_size,
            Size { width: AvailableSpace::Definite(row_content_width), height: AvailableSpace::MaxContent },
            SizingMode::InherentSize,
            Line::FALSE,
        );
        row_heights.push(row_size.height);
    }

    // Determine the table's content height (sum of row heights + border spacing)
    let content_height: f32 = match node_inner_size.height {
        Some(height) => height,
        None => row_heights.iter().sum::<f32>() + border_spacing_v * 2.0,
    };

    // Clamp content height to min/max (accounting for padding/border)
    let content_height = content_height
        .maybe_clamp(content_min.height, content_max.height)
        .max(0.0);

    let content_size = Size { width: content_width, height: content_height };

    let outer_size = Size {
        width: content_size.width + padding_border_size.width,
        height: content_size.height + padding_border_size.height,
    };

    // If we're only computing the size, return now
    if run_mode == RunMode::ComputeSize {
        return LayoutOutput::from_sizes(outer_size, content_size);
    }

    // Phase 3: Perform full layout on each row
    let mut y_offset = padding.top + border.top + border_spacing_v;
    let x_offset = padding.left + border.left + border_spacing_h;

    for child_index in 0..child_count {
        let child_id = tree.get_child_id(node_id, child_index);

        // Perform full layout on the row with the resolved content width
        // and height (if known, so the row can stretch)
        let child_known_height = if child_count == 1 {
            // Single row: stretch to fill the table's content height
            Some(content_height - border_spacing_v * 2.0)
        } else {
            None
        };
        let child_output = tree.perform_child_layout(
            child_id,
            Size { width: Some(row_content_width), height: child_known_height },
            parent_size,
            Size { width: AvailableSpace::Definite(row_content_width), height: AvailableSpace::MaxContent },
            SizingMode::InherentSize,
            Line::FALSE,
        );

        let child_layout = Layout {
            order: child_index as u32,
            location: Point { x: x_offset, y: y_offset },
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

    LayoutOutput::from_sizes(outer_size, content_size)
}