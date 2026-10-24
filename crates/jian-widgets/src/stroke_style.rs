//! Stroke cap / join / dash for [`Painter`](crate::Painter) strokes.
//!
//! A host sets the style with [`Painter::set_stroke_style`](crate::Painter::set_stroke_style)
//! before the stroke calls it applies to and resets it with
//! `StrokeStyle::default()` afterwards. Unset fields keep the backend's own
//! per-primitive default, so a plain stroke paints exactly as before.
//! Every type here is `Copy`, so it rides inside `Copy` scene strokes.

/// Stroke end-cap shape.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StrokeCap {
    Butt,
    Round,
    Square,
}

/// Stroke corner shape.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StrokeJoin {
    Miter,
    Round,
    Bevel,
}

/// A dash pattern plus its phase, in the coordinate space of the stroke
/// calls it applies to.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StrokeDash {
    lengths: [f32; StrokeDash::MAX_LENGTHS],
    len: u8,
    offset: f32,
}

impl StrokeDash {
    /// Longest pattern kept; longer patterns are cut at an even count.
    pub const MAX_LENGTHS: usize = 16;

    /// A drawable dash from authored on / off lengths, or `None` when
    /// nothing could be drawn: an empty pattern, a negative or
    /// non-finite length, or lengths summing to zero. An odd-length
    /// pattern repeats once, as SVG specifies. A non-finite offset
    /// reads as `0`.
    pub fn new(pattern: &[f32], offset: f32) -> Option<Self> {
        if pattern.is_empty() || pattern.iter().any(|len| !len.is_finite() || *len < 0.0) {
            return None;
        }
        let repeats = if pattern.len() % 2 == 1 { 2 } else { 1 };
        let total = (pattern.len() * repeats).min(Self::MAX_LENGTHS) & !1;
        let mut lengths = [0.0; Self::MAX_LENGTHS];
        for (slot, len) in lengths.iter_mut().zip(pattern.iter().cycle()).take(total) {
            *slot = *len;
        }
        if lengths[..total].iter().sum::<f32>() <= 0.0 {
            return None;
        }
        Some(Self {
            lengths,
            len: total as u8,
            offset: if offset.is_finite() { offset } else { 0.0 },
        })
    }

    /// The on / off lengths, always an even count of at least two.
    pub fn lengths(&self) -> &[f32] {
        &self.lengths[..self.len as usize]
    }

    /// Phase into the pattern.
    pub fn offset(&self) -> f32 {
        self.offset
    }

    /// This dash with every length and the offset multiplied by `scale`
    /// (a canvas zoom, when geometry is handed over in screen space).
    pub fn scaled(&self, scale: f32) -> Self {
        let mut out = *self;
        for len in &mut out.lengths[..self.len as usize] {
            *len *= scale;
        }
        out.offset *= scale;
        out
    }
}

/// Cap, join and dash for subsequent stroke calls. `Default` is the
/// plain style: every field unset.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct StrokeStyle {
    pub cap: Option<StrokeCap>,
    pub join: Option<StrokeJoin>,
    pub dash: Option<StrokeDash>,
}

impl StrokeStyle {
    /// No field set: backends paint their defaults.
    pub fn is_plain(&self) -> bool {
        self.cap.is_none() && self.join.is_none() && self.dash.is_none()
    }

    /// This style with the dash scaled by `scale`.
    pub fn scaled(&self, scale: f32) -> Self {
        Self {
            dash: self.dash.map(|dash| dash.scaled(scale)),
            ..*self
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dash_normalizes_and_rejects_undrawable_patterns() {
        let dash = StrokeDash::new(&[4.0, 2.0], 1.0).expect("dash");
        assert_eq!(dash.lengths(), &[4.0, 2.0]);
        assert_eq!(dash.offset(), 1.0);
        assert_eq!(
            StrokeDash::new(&[3.0], f32::NAN).unwrap().lengths(),
            &[3.0, 3.0]
        );
        assert_eq!(StrokeDash::new(&[3.0], f32::NAN).unwrap().offset(), 0.0);
        for bad in [&[][..], &[0.0, 0.0], &[-1.0, 2.0], &[f32::INFINITY, 1.0]] {
            assert_eq!(StrokeDash::new(bad, 0.0), None, "{bad:?}");
        }
    }

    #[test]
    fn long_patterns_stop_at_an_even_count() {
        let long: Vec<f32> = (1..=17).map(|n| n as f32).collect();
        let dash = StrokeDash::new(&long, 0.0).expect("dash");
        assert_eq!(dash.lengths().len(), StrokeDash::MAX_LENGTHS);
        assert_eq!(dash.lengths()[0], 1.0);
    }

    #[test]
    fn scaling_touches_only_the_dash() {
        let style = StrokeStyle {
            cap: Some(StrokeCap::Square),
            join: None,
            dash: StrokeDash::new(&[2.0, 1.0], 0.5),
        };
        let scaled = style.scaled(2.0);
        assert_eq!(scaled.cap, Some(StrokeCap::Square));
        let dash = scaled.dash.expect("dash");
        assert_eq!(dash.lengths(), &[4.0, 2.0]);
        assert_eq!(dash.offset(), 1.0);
        assert!(StrokeStyle::default().is_plain());
        assert!(!style.is_plain());
    }
}
