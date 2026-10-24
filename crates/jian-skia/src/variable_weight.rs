//! `wght` instancing for variable fonts.
//!
//! A variable font registers as one typeface at its default instance, so a
//! family lookup at 700 hands back the 400 instance and the painter falls
//! back to synthetic bold (a stroked outline plus a width allowance). The
//! font carries the real bold design on its `wght` axis; instancing the
//! typeface there gives the true glyph shapes and advances.

use skia_safe::font_arguments::variation_position::Coordinate;
use skia_safe::font_arguments::VariationPosition;
use skia_safe::{FontArguments, FourByteTag, Typeface};

const WGHT: FourByteTag = FourByteTag::from_chars('w', 'g', 'h', 't');

/// `typeface` instanced at `weight` on its `wght` axis, clamped to the
/// axis range, with the weight it ends up at. `None` when the face has no
/// `wght` axis, or already sits at that weight.
pub(crate) fn instance_at_weight(typeface: &Typeface, weight: f32) -> Option<(Typeface, f32)> {
    let axes = typeface.variation_design_parameters()?;
    let axis = axes.iter().find(|axis| axis.tag == WGHT)?;
    let target = weight.clamp(axis.min, axis.max);
    let current = typeface
        .variation_design_position()
        .and_then(|position| {
            position
                .iter()
                .find(|coordinate| coordinate.axis == WGHT)
                .map(|coordinate| coordinate.value)
        })
        .unwrap_or(axis.def);
    if (current - target).abs() < 0.5 {
        return None;
    }
    let coordinates = [Coordinate {
        axis: WGHT,
        value: target,
    }];
    let arguments = FontArguments::new().set_variation_design_position(VariationPosition {
        coordinates: &coordinates,
    });
    typeface
        .clone_with_arguments(&arguments)
        .map(|instance| (instance, target))
}
