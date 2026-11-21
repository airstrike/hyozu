//! Text placed in a diagram's normalized square, and the regions that decide
//! whether a data label fits inside its own geometry.

/// One label in normalized diagram space.
#[derive(Clone, Debug)]
pub(crate) struct Label {
    pub(crate) text: String,
    pub(crate) anchor: [f32; 2],
    pub(crate) place: Place,
    /// The entry this label describes. Data labels wait for the entrance
    /// animation to settle, while guide labels (axes, ticks) never move.
    pub(crate) entry: Option<usize>,
}

/// How a label's box relates to its anchor.
#[derive(Clone, Debug)]
pub(crate) enum Place {
    /// Aligned by the anchor's horizontal position, or centered when the
    /// diagram asks for centered labels.
    Free,
    /// On the side of the anchor that a screen-space direction points to.
    Toward([f32; 2]),
    /// Centered on the anchor when the box fits inside `region`.
    Inside { region: Region, otherwise: Overflow },
}

/// What a label does when it cannot fit inside its region.
#[derive(Clone, Debug)]
pub(crate) enum Overflow {
    Hide,
    /// Moves next to another anchor, as [`Place::Toward`].
    Beside {
        anchor: [f32; 2],
        toward: [f32; 2],
    },
}

/// Geometry a label must stay within, in normalized diagram space.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Region {
    Rect {
        min: [f32; 2],
        max: [f32; 2],
    },
    /// A convex polygon, such as a funnel stage.
    Convex(Vec<[f32; 2]>),
    /// An annular sector about the square's center. Radii are in units of
    /// the square side. Angles are screen radians (y down), and the sector
    /// sweeps `span` radians clockwise on screen from `start`.
    Sector {
        inner: f32,
        outer: f32,
        start: f32,
        span: f32,
    },
}

impl Label {
    pub(crate) fn free(anchor: [f32; 2], text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            anchor,
            place: Place::Free,
            entry: None,
        }
    }

    pub(crate) fn toward(anchor: [f32; 2], toward: [f32; 2], text: impl Into<String>) -> Self {
        Self {
            place: Place::Toward(toward),
            ..Self::free(anchor, text)
        }
    }

    pub(crate) fn inside(entry: usize, anchor: [f32; 2], region: Region, text: impl Into<String>) -> Self {
        Self {
            place: Place::Inside {
                region,
                otherwise: Overflow::Hide,
            },
            entry: Some(entry),
            ..Self::free(anchor, text)
        }
    }

    /// Describes `entry`, so the label appears once its geometry settles.
    pub(crate) fn of(mut self, entry: usize) -> Self {
        self.entry = Some(entry);
        self
    }

    /// Falls back to a position beside `anchor` instead of hiding.
    pub(crate) fn or_beside(mut self, anchor: [f32; 2], toward: [f32; 2]) -> Self {
        if let Place::Inside { otherwise, .. } = &mut self.place {
            *otherwise = Overflow::Beside { anchor, toward };
        }
        self
    }
}
