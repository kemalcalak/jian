//! Stroke shape beyond cap / join / dash: per-end caps and arrowheads,
//! and variable-width profiles. Re-exported from [`crate::style`].

use serde::{Deserialize, Serialize};

/// What an open stroke draws at one of its two ends: a cap, or an
/// arrowhead / marker whose size scales with the stroke width (Figma's
/// per-end stroke caps). Closed shapes have no ends and ignore it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[cfg_attr(feature = "export-ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "export-ts", ts(export, export_to = "ops.ts"))]
#[serde(rename_all = "snake_case")]
pub enum StrokeEndCap {
    None,
    Round,
    Square,
    /// Two open strokes meeting at the end point (`>`).
    LineArrow,
    /// A filled triangle pointing away from the line.
    TriangleArrow,
    /// A filled triangle pointing back along the line.
    ReversedTriangle,
    /// A filled circle centred on the end point.
    Circle,
    /// A filled diamond centred on the end point.
    Diamond,
}

/// Named stroke width profiles. Every preset but `custom` is fully
/// described by its name; `custom` reads [`StrokeWidthProfile::points`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[cfg_attr(feature = "export-ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "export-ts", ts(export, export_to = "ops.ts"))]
#[serde(rename_all = "snake_case")]
pub enum StrokeWidthPreset {
    /// One width along the whole stroke (the same as no profile).
    Uniform,
    /// Zero width at the start, full width at the end.
    TaperStart,
    /// Full width at the start, zero width at the end.
    TaperEnd,
    /// Zero width at both ends, full width in the middle.
    TaperBoth,
    /// The width follows `points`.
    Custom,
}

/// One width stop of a variable-width stroke.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[cfg_attr(feature = "export-ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "export-ts", ts(export, export_to = "ops.ts"))]
pub struct StrokeWidthPoint {
    /// Position along the stroke as a fraction of its length, `0..=1`.
    pub t: f32,
    /// Width at `t` as a multiple of the stroke thickness (`1` = the
    /// thickness, `0` = a point). Widths between stops interpolate
    /// linearly; before the first and after the last they hold.
    pub width: f32,
}

/// A variable-width stroke (Figma's width profiles, Illustrator's width
/// tool). Editors paint it as a filled outline around the centre line,
/// so `align` does not apply.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[cfg_attr(feature = "export-ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "export-ts", ts(export, export_to = "ops.ts"))]
#[serde(rename_all = "camelCase")]
pub struct StrokeWidthProfile {
    pub preset: StrokeWidthPreset,
    /// `custom` only: the width stops, in any order.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "export-ts", ts(optional = nullable))]
    pub points: Option<Vec<StrokeWidthPoint>>,
}

impl StrokeWidthProfile {
    /// The width stops this profile describes, sorted by `t`: the
    /// preset's own stops, or the `custom` points with non-finite stops
    /// dropped and `t` / `width` clamped into range. Empty means uniform.
    pub fn stops(&self) -> Vec<StrokeWidthPoint> {
        let point = |t: f32, width: f32| StrokeWidthPoint { t, width };
        match self.preset {
            StrokeWidthPreset::Uniform => Vec::new(),
            StrokeWidthPreset::TaperStart => vec![point(0.0, 0.0), point(1.0, 1.0)],
            StrokeWidthPreset::TaperEnd => vec![point(0.0, 1.0), point(1.0, 0.0)],
            StrokeWidthPreset::TaperBoth => {
                vec![point(0.0, 0.0), point(0.5, 1.0), point(1.0, 0.0)]
            }
            StrokeWidthPreset::Custom => {
                let mut stops: Vec<StrokeWidthPoint> = self
                    .points
                    .iter()
                    .flatten()
                    .filter(|p| p.t.is_finite() && p.width.is_finite())
                    .map(|p| point(p.t.clamp(0.0, 1.0), p.width.clamp(0.0, MAX_WIDTH_FACTOR)))
                    .collect();
                stops.sort_by(|a, b| a.t.total_cmp(&b.t));
                stops
            }
        }
    }
}

/// Largest width a profile stop can ask for, as a multiple of the
/// stroke thickness.
pub const MAX_WIDTH_FACTOR: f32 = 16.0;
