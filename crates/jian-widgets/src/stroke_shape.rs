//! Per-end stroke caps / arrowheads carried on
//! [`StrokeStyle`](crate::StrokeStyle). Backends do not paint them: hosts
//! build the geometry (arrowhead shapes) and hand it over as ordinary
//! paths. `Copy` and one byte, so it rides inside `Copy` scene strokes.
//! Variable-width profiles are not render style: hosts read them from the
//! schema stroke (`jian_ops_schema::style::StrokeWidthProfile`) and bake
//! them into geometry.

/// What an open stroke draws at one end.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StrokeEnd {
    None,
    Round,
    Square,
    LineArrow,
    TriangleArrow,
    ReversedTriangle,
    Circle,
    Diamond,
}

impl StrokeEnd {
    /// Whether this end is a marker shape rather than a plain cap.
    pub fn is_marker(self) -> bool {
        !matches!(self, Self::None | Self::Round | Self::Square)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_shapes_count_as_markers() {
        assert!(!StrokeEnd::Round.is_marker());
        assert!(StrokeEnd::Diamond.is_marker());
    }
}
