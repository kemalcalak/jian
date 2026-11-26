//! Stroke geometry the host precomputes (#389) and which way a line runs.
//! Re-exported from [`crate::layout_scene`].

use jian_widgets::{Point2D, Rect};

/// Precomputed stroke geometry of a line or open path whose stroke draws
/// per-end caps or arrowheads, as SVG path data in node-local doc px
/// (relative to the node's normalised `bounds.origin`). Any part may be
/// empty. Built once per scene, never per frame.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct SceneStrokeGeometry {
    /// Centre line, stroked at the stroke width with the stroke's join /
    /// dash / miter limit. A solid line is stroked with butt caps (its
    /// ends are drawn by `fill_d`); a dashed one keeps the authored cap on
    /// every dash and is already shortened so no dash cap reaches past an
    /// end point or into a head.
    pub stroke_d: String,
    /// Shapes filled with the stroke paint: arrowheads, end dots, the
    /// per-end round / square caps, and the authored cap on the inner ends
    /// of a multi-subpath stroke. Filled with the nonzero rule, so shapes
    /// that overlap never cancel out.
    pub fill_d: String,
    /// Open arms stroked at the stroke width with miter joins and butt
    /// caps, never dashed (line arrows).
    pub arms_d: String,
    /// One disc per end shape that reaches past the stroke band, covering
    /// it, for hit-testing (node-local, like the paths).
    pub head_hits: Vec<SceneStrokeHead>,
}

/// A disc covering one end shape: a point within `radius` of `center`
/// hits the head.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SceneStrokeHead {
    pub center: Point2D,
    pub radius: f32,
}

impl SceneStrokeGeometry {
    /// Whether `local` (node-local doc px) falls on one of the heads.
    pub fn hits_head(&self, local: Point2D, slack: f32) -> bool {
        self.head_hits.iter().any(|head| {
            let (dx, dy) = (local.x - head.center.x, local.y - head.center.y);
            dx.hypot(dy) <= head.radius + slack
        })
    }
}

/// Which way a line runs across its box. The scene keeps a line's
/// `bounds` normalised (positive size, so aggregate bounds and layout see
/// a box), and this keeps the authored start and end apart: unset, the
/// line runs from the box's top-left to its bottom-right corner; each
/// flag starts it on the other side of that axis. Node flips still apply
/// on top, as for any node.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SceneLineDirection {
    /// The line starts on the right edge and runs left.
    pub right_to_left: bool,
    /// The line starts on the bottom edge and runs up.
    pub bottom_to_top: bool,
}

impl SceneLineDirection {
    /// The direction of a line drawn from `(0, 0)` to `(dx, dy)`.
    pub fn of_delta(dx: f32, dy: f32) -> Self {
        Self {
            right_to_left: dx < 0.0,
            bottom_to_top: dy < 0.0,
        }
    }

    /// Start and end of the line across `rect`. A rect with a negative
    /// extent (hand-built scenes) runs from its origin along that extent,
    /// as before, and the flags reverse that axis.
    pub fn endpoints(self, rect: Rect) -> (Point2D, Point2D) {
        let (x0, x1) = (rect.origin.x, rect.origin.x + rect.size.x);
        let (y0, y1) = (rect.origin.y, rect.origin.y + rect.size.y);
        let (sx, ex) = if self.right_to_left {
            (x1, x0)
        } else {
            (x0, x1)
        };
        let (sy, ey) = if self.bottom_to_top {
            (y1, y0)
        } else {
            (y0, y1)
        };
        (Point2D::new(sx, sy), Point2D::new(ex, ey))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn endpoints_follow_the_authored_direction_in_all_four_quadrants() {
        let rect = Rect::xywh(10.0, 20.0, 30.0, 40.0);
        let cases = [
            ((5.0, 5.0), ((10.0, 20.0), (40.0, 60.0))),
            ((-5.0, 5.0), ((40.0, 20.0), (10.0, 60.0))),
            ((5.0, -5.0), ((10.0, 60.0), (40.0, 20.0))),
            ((-5.0, -5.0), ((40.0, 60.0), (10.0, 20.0))),
        ];
        for ((dx, dy), (start, end)) in cases {
            let (s, e) = SceneLineDirection::of_delta(dx, dy).endpoints(rect);
            assert_eq!((s.x, s.y), start, "({dx}, {dy})");
            assert_eq!((e.x, e.y), end, "({dx}, {dy})");
        }
    }

    #[test]
    fn heads_hit_inside_their_disc_only() {
        let geometry = SceneStrokeGeometry {
            head_hits: vec![SceneStrokeHead {
                center: Point2D::new(10.0, 0.0),
                radius: 4.0,
            }],
            ..SceneStrokeGeometry::default()
        };
        assert!(geometry.hits_head(Point2D::new(13.0, 0.0), 0.0));
        assert!(!geometry.hits_head(Point2D::new(15.0, 0.0), 0.0));
        assert!(geometry.hits_head(Point2D::new(15.0, 0.0), 1.5));
    }
}
