//! Style types for Table layout

/// The set of styles required for a Table layout container
pub trait TableContainerStyle: crate::CoreStyle {
    /// Whether borders are collapsed or separated
    #[inline(always)]
    fn border_collapse(&self) -> BorderCollapse {
        BorderCollapse::Separate
    }

    /// The spacing between table borders (only applies when border-collapse is separate)
    #[inline(always)]
    fn border_spacing(&self) -> Option<f32> {
        None
    }

    /// The caption side (top or bottom)
    #[inline(always)]
    fn caption_side(&self) -> CaptionSide {
        CaptionSide::Top
    }
}

/// The set of styles required for a Table item (child of a Table container)
pub trait TableItemStyle: crate::CoreStyle {
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
