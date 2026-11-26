//! Per-end stroke caps / arrowheads and variable-width profiles carried on
//! [`StrokeStyle`](crate::StrokeStyle). Backends paint neither: hosts build
//! the geometry (arrowhead shapes, the outline of a variable-width stroke)
//! and hand it over as ordinary fills. Both types are `Copy` so they ride
//! inside `Copy` scene strokes.

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

/// Width stops of a variable-width stroke: `(t, factor)` pairs sorted by
/// `t` (`0..=1` along the stroke), the factor multiplying the stroke
/// width. Longer authored profiles are resampled to [`Self::MAX_STOPS`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StrokeWidthProfile {
    stops: [[f32; 2]; StrokeWidthProfile::MAX_STOPS],
    len: u8,
}

impl StrokeWidthProfile {
    /// Most stops kept.
    pub const MAX_STOPS: usize = 32;

    /// A profile from `(t, factor)` stops in any order, or `None` when
    /// nothing varies: no finite stop, or every stop at factor `1`.
    /// `t` is clamped into `0..=1` and factors below `0` read as `0`.
    pub fn new(stops: &[[f32; 2]]) -> Option<Self> {
        let mut clean: Vec<[f32; 2]> = stops
            .iter()
            .filter(|[t, w]| t.is_finite() && w.is_finite())
            .map(|[t, w]| [t.clamp(0.0, 1.0), w.max(0.0)])
            .collect();
        if clean.iter().all(|[_, w]| (*w - 1.0).abs() < 1e-6) {
            return None;
        }
        clean.sort_by(|a, b| a[0].total_cmp(&b[0]));
        let mut out = Self {
            stops: [[0.0; 2]; Self::MAX_STOPS],
            len: 0,
        };
        if clean.len() <= Self::MAX_STOPS {
            out.stops[..clean.len()].copy_from_slice(&clean);
            out.len = clean.len() as u8;
        } else {
            // Even resample over the authored span, keeping both ends.
            let (first, last) = (clean[0][0], clean[clean.len() - 1][0]);
            for (i, slot) in out.stops.iter_mut().enumerate() {
                let t = first + (last - first) * i as f32 / (Self::MAX_STOPS - 1) as f32;
                *slot = [t, factor_at(&clean, t)];
            }
            out.len = Self::MAX_STOPS as u8;
        }
        Some(out)
    }

    /// The `(t, factor)` stops, sorted by `t`, at least one.
    pub fn stops(&self) -> &[[f32; 2]] {
        &self.stops[..self.len as usize]
    }

    /// The width factor at `t` (`0..=1`): linear between stops, held
    /// before the first and after the last.
    pub fn factor_at(&self, t: f32) -> f32 {
        factor_at(self.stops(), t)
    }

    /// The largest factor anywhere along the stroke.
    pub fn max_factor(&self) -> f32 {
        self.stops().iter().fold(0.0_f32, |acc, [_, w]| acc.max(*w))
    }
}

fn factor_at(stops: &[[f32; 2]], t: f32) -> f32 {
    let Some(first) = stops.first() else {
        return 1.0;
    };
    if t <= first[0] {
        return first[1];
    }
    for pair in stops.windows(2) {
        let ([t0, w0], [t1, w1]) = (pair[0], pair[1]);
        if t <= t1 {
            let span = t1 - t0;
            return if span <= f32::EPSILON {
                w1
            } else {
                w0 + (w1 - w0) * (t - t0) / span
            };
        }
    }
    stops[stops.len() - 1][1]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uniform_stops_are_no_profile() {
        assert_eq!(StrokeWidthProfile::new(&[]), None);
        assert_eq!(StrokeWidthProfile::new(&[[0.0, 1.0], [1.0, 1.0]]), None);
        assert_eq!(StrokeWidthProfile::new(&[[f32::NAN, 2.0]]), None);
    }

    #[test]
    fn factors_interpolate_between_sorted_stops_and_hold_past_the_ends() {
        let profile = StrokeWidthProfile::new(&[[1.0, 0.0], [0.25, 2.0]]).expect("profile");
        assert_eq!(profile.stops(), &[[0.25, 2.0], [1.0, 0.0]]);
        assert_eq!(profile.factor_at(0.0), 2.0);
        assert!((profile.factor_at(0.625) - 1.0).abs() < 1e-6);
        assert_eq!(profile.factor_at(1.0), 0.0);
        assert_eq!(profile.max_factor(), 2.0);
    }

    #[test]
    fn long_profiles_resample_to_the_cap_keeping_both_ends() {
        let stops: Vec<[f32; 2]> = (0..=100)
            .map(|i| [i as f32 / 100.0, i as f32 / 100.0])
            .collect();
        let profile = StrokeWidthProfile::new(&stops).expect("profile");
        assert_eq!(profile.stops().len(), StrokeWidthProfile::MAX_STOPS);
        assert_eq!(profile.stops()[0], [0.0, 0.0]);
        assert_eq!(
            profile.stops()[StrokeWidthProfile::MAX_STOPS - 1],
            [1.0, 1.0]
        );
        assert!((profile.factor_at(0.5) - 0.5).abs() < 1e-4);
    }

    #[test]
    fn only_shapes_count_as_markers() {
        assert!(!StrokeEnd::Round.is_marker());
        assert!(StrokeEnd::Diamond.is_marker());
    }
}
