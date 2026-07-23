//! Style types for Table layout
use crate::geometry::Size;
use crate::style::LengthPercentage;
use crate::CoreStyle;

/// The set of styles required for a Table layout container
pub trait TableContainerStyle: CoreStyle {
    /// Whether borders are collapsed or separated
    #[inline(always)]
    fn border_collapse(&self) -> BorderCollapse {
        BorderCollapse::Separate
    }

    /// The spacing between table cells (only applies when border-collapse is separate).
    /// This is the CSS `border-spacing` property, expressed as a `Size<LengthPercentage>`
    /// where `width` is the horizontal spacing and `height` is the vertical spacing.
    #[inline(always)]
    fn border_spacing(&self) -> Size<LengthPercentage> {
        Size::zero()
    }

    /// The caption side (top or bottom)
    #[inline(always)]
    fn caption_side(&self) -> CaptionSide {
        CaptionSide::Top
    }

    /// Whether the table uses fixed or auto table layout
    #[inline(always)]
    fn table_layout(&self) -> TableLayout {
        TableLayout::Auto
    }
}

/// The set of styles required for a Table item (child of a Table container)
pub trait TableItemStyle: CoreStyle {
    /// Whether this item is a table cell
    #[inline(always)]
    fn is_table_cell(&self) -> bool {
        false
    }

    /// Whether this item is a table row
    #[inline(always)]
    fn is_table_row(&self) -> bool {
        false
    }

    /// The number of columns this cell spans (CSS `colspan`). Default is 1.
    #[inline(always)]
    fn column_span(&self) -> u32 {
        1
    }

    /// The number of rows this cell spans (CSS `rowspan`). Default is 1.
    #[inline(always)]
    fn row_span(&self) -> u32 {
        1
    }
}

/// Whether table borders are collapsed or separated
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum BorderCollapse {
    /// Borders are separated (the default)
    #[default]
    Separate,
    /// Borders are collapsed into a single border
    Collapse,
}

/// Whether the table caption is placed at the top or bottom of the table
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum CaptionSide {
    /// Caption is placed at the top of the table
    #[default]
    Top,
    /// Caption is placed at the bottom of the table
    Bottom,
}

/// Whether the table uses the fixed or auto layout algorithm
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum TableLayout {
    /// The auto table layout algorithm (the default). Column widths are computed
    /// based on the content of all cells in each column.
    #[default]
    Auto,
    /// The fixed table layout algorithm. Column widths are determined by the first
    /// row and explicit column widths only.
    Fixed,
}

#[cfg(feature = "parse")]
crate::util::parse::impl_parse_for_keyword_enum!(TableLayout,
    "auto" => Auto,
    "fixed" => Fixed,
);

#[cfg(feature = "parse")]
crate::util::parse::impl_parse_for_keyword_enum!(BorderCollapse,
    "separate" => Separate,
    "collapse" => Collapse,
);

#[cfg(feature = "parse")]
crate::util::parse::impl_parse_for_keyword_enum!(CaptionSide,
    "top" => Top,
    "bottom" => Bottom,
);