/// Horizontal alignment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Horizontal {
    Left,
    Center,
    Right,
}

impl From<Horizontal> for crate::core::alignment::Horizontal {
    fn from(align: Horizontal) -> Self {
        match align {
            Horizontal::Left => crate::core::alignment::Horizontal::Left,
            Horizontal::Center => crate::core::alignment::Horizontal::Center,
            Horizontal::Right => crate::core::alignment::Horizontal::Right,
        }
    }
}

/// Vertical alignment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Vertical {
    Top,
    Center,
    Bottom,
}

impl From<Vertical> for crate::core::alignment::Vertical {
    fn from(align: Vertical) -> Self {
        match align {
            Vertical::Top => crate::core::alignment::Vertical::Top,
            Vertical::Center => crate::core::alignment::Vertical::Center,
            Vertical::Bottom => crate::core::alignment::Vertical::Bottom,
        }
    }
}

impl From<Horizontal> for crate::core::text::Alignment {
    fn from(align: Horizontal) -> Self {
        match align {
            Horizontal::Left => crate::core::text::Alignment::Left,
            Horizontal::Center => crate::core::text::Alignment::Center,
            Horizontal::Right => crate::core::text::Alignment::Right,
        }
    }
}
