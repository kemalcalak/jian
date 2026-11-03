//! CSS font matching inside one family.
//!
//! `FontMgr::match_family_style` on macOS asks CoreText for the nearest
//! descriptor, which hands a 500 request the Bold face of a family that has
//! only Regular and Bold (Arial); browsers render that text with Regular.
//! These helpers pick the face the way CSS Fonts 4 §5.2 does instead: the
//! closest width, then slant, then weight, where weight prefers
//! - 400–500: the desired weight, heavier ones up to 500, lighter ones, then heavier ones;
//! - below 400: lighter ones first, then heavier ones;
//! - above 500: heavier ones first, then lighter ones.

use skia_safe::{font_style::Slant, FontMgr, FontStyle, Typeface};

/// Index of the face in `available` that CSS font matching picks for
/// `desired`, or `None` when `available` is empty.
pub fn css_match_style(desired: FontStyle, available: &[FontStyle]) -> Option<usize> {
    available
        .iter()
        .enumerate()
        .min_by_key(|(_, style)| {
            (
                width_rank(*desired.width(), *style.width()),
                slant_rank(desired.slant(), style.slant()),
                weight_rank(*desired.weight(), *style.weight()),
            )
        })
        .map(|(index, _)| index)
}

/// The face of `family` in `mgr` that CSS font matching picks for `style`.
/// Falls back to the manager's own `match_family_style` when the family
/// exposes no style set (a name only a descriptor lookup resolves).
pub fn match_family_style_css(mgr: &FontMgr, family: &str, style: FontStyle) -> Option<Typeface> {
    let mut set = mgr.match_family(family);
    let count = set.count();
    if count == 0 {
        return mgr.match_family_style(family, style);
    }
    let styles = (0..count)
        .map(|index| set.style(index).0)
        .collect::<Vec<_>>();
    css_match_style(style, &styles)
        .and_then(|index| set.new_typeface(index))
        .or_else(|| mgr.match_family_style(family, style))
}

/// Normal width (CSS `font-stretch: 100%`) and narrower prefer narrower
/// faces first; wider requests prefer wider faces first.
fn width_rank(desired: i32, width: i32) -> (u8, i32) {
    if width == desired {
        return (0, 0);
    }
    let distance = (width - desired).abs();
    let narrower = width < desired;
    let narrower_first = desired <= *skia_safe::font_style::Width::NORMAL;
    (if narrower == narrower_first { 1 } else { 2 }, distance)
}

fn slant_rank(desired: Slant, slant: Slant) -> u8 {
    let order = match desired {
        Slant::Upright => [Slant::Upright, Slant::Oblique, Slant::Italic],
        Slant::Italic => [Slant::Italic, Slant::Oblique, Slant::Upright],
        Slant::Oblique => [Slant::Oblique, Slant::Italic, Slant::Upright],
    };
    order
        .iter()
        .position(|candidate| *candidate == slant)
        .unwrap_or(order.len()) as u8
}

fn weight_rank(desired: i32, weight: i32) -> (u8, i32) {
    if weight == desired {
        return (0, 0);
    }
    let distance = (weight - desired).abs();
    let tier = if (400..=500).contains(&desired) {
        if weight > desired && weight <= 500 {
            1
        } else if weight < desired {
            2
        } else {
            3
        }
    } else if desired < 400 {
        if weight < desired {
            1
        } else {
            2
        }
    } else if weight > desired {
        1
    } else {
        2
    };
    (tier, distance)
}

#[cfg(test)]
mod tests {
    use super::*;
    use skia_safe::font_style::{Weight, Width};

    fn style(weight: i32, slant: Slant) -> FontStyle {
        FontStyle::new(Weight::from(weight), Width::NORMAL, slant)
    }

    fn picked_weight(desired: i32, available: &[i32]) -> i32 {
        let faces = available
            .iter()
            .map(|weight| style(*weight, Slant::Upright))
            .collect::<Vec<_>>();
        let index = css_match_style(style(desired, Slant::Upright), &faces).expect("a face");
        available[index]
    }

    #[test]
    fn medium_on_a_regular_bold_family_picks_regular() {
        assert_eq!(picked_weight(500, &[400, 700]), 400);
        assert_eq!(picked_weight(500, &[700, 400]), 400);
    }

    #[test]
    fn semibold_on_a_regular_bold_family_picks_bold() {
        assert_eq!(picked_weight(600, &[400, 700]), 700);
    }

    #[test]
    fn every_css_weight_on_a_regular_bold_family_matches_browsers() {
        for (desired, expected) in [
            (100, 400),
            (200, 400),
            (300, 400),
            (400, 400),
            (500, 400),
            (600, 700),
            (700, 700),
            (800, 700),
            (900, 700),
        ] {
            assert_eq!(picked_weight(desired, &[400, 700]), expected, "{desired}");
        }
    }

    #[test]
    fn weight_search_order_follows_css() {
        // 400 tries 500 before the lighter faces.
        assert_eq!(picked_weight(400, &[300, 500, 600]), 500);
        // 450 tries up to 500, then lighter, then heavier.
        assert_eq!(picked_weight(450, &[300, 600]), 300);
        // Below 400 goes lighter first.
        assert_eq!(picked_weight(300, &[200, 400]), 200);
        assert_eq!(picked_weight(300, &[400, 500]), 400);
        // Above 500 goes heavier first.
        assert_eq!(picked_weight(600, &[500, 800]), 800);
        assert_eq!(picked_weight(600, &[400, 500]), 500);
    }

    #[test]
    fn slant_wins_over_weight() {
        let faces = [
            style(400, Slant::Upright),
            style(400, Slant::Italic),
            style(700, Slant::Upright),
            style(700, Slant::Italic),
        ];
        assert_eq!(css_match_style(style(500, Slant::Italic), &faces), Some(1));
        assert_eq!(css_match_style(style(600, Slant::Italic), &faces), Some(3));
        // An italic request with no italic face keeps the upright one.
        assert_eq!(
            css_match_style(style(700, Slant::Italic), &faces[..1]),
            Some(0)
        );
    }

    #[test]
    fn normal_width_prefers_narrower_faces() {
        let condensed = FontStyle::new(Weight::NORMAL, Width::CONDENSED, Slant::Upright);
        let expanded = FontStyle::new(Weight::NORMAL, Width::EXPANDED, Slant::Upright);
        let normal_bold = FontStyle::new(Weight::BOLD, Width::NORMAL, Slant::Upright);
        assert_eq!(
            css_match_style(style(400, Slant::Upright), &[expanded, condensed]),
            Some(1)
        );
        assert_eq!(
            css_match_style(style(400, Slant::Upright), &[condensed, normal_bold]),
            Some(1)
        );
    }

    #[test]
    fn empty_family_matches_nothing() {
        assert_eq!(css_match_style(style(400, Slant::Upright), &[]), None);
    }
}
