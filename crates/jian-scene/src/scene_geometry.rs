//! Pure scene geometry helpers, split out of `layout_scene.rs` to keep that
//! file under the 800-line cap. These touch no `SceneNode` internals — only
//! `Rect` / `Point2D` / `str` — and are re-exported from `layout_scene` so the
//! existing `layout_scene::{regular_polygon_points, stable_image_source_id}`
//! call paths stay valid.

use jian_widgets::{Point2D, Rect};

pub(crate) fn rect_has_extent(rect: Rect) -> bool {
    rect.size.x > 0.0 || rect.size.y > 0.0
}

pub fn stable_image_source_id(src: &str) -> u64 {
    jian_ops_schema::node::image_src::paint_image_id(src)
}

/// Vertices for a regular polygon fitted inside `rect`.
pub fn regular_polygon_points(rect: Rect, sides: u32) -> Vec<Point2D> {
    let n = sides.clamp(3, 100) as usize;
    let cx = rect.origin.x + rect.size.x / 2.0;
    let cy = rect.origin.y + rect.size.y / 2.0;
    let rx = rect.size.x / 2.0;
    let ry = rect.size.y / 2.0;
    let start = -std::f32::consts::FRAC_PI_2;
    (0..n)
        .map(|i| {
            let angle = start + i as f32 * std::f32::consts::TAU / n as f32;
            Point2D::new(cx + rx * angle.cos(), cy + ry * angle.sin())
        })
        .collect()
}

/// Smallest / largest star inner radius the geometry honours, as a
/// fraction of the outer radius (`PolygonNode::inner_radius`).
pub const STAR_MIN_INNER_RADIUS: f32 = 0.01;
pub const STAR_MAX_INNER_RADIUS: f32 = 1.0;

/// Vertices of a polygon node fitted inside `rect`: the regular polygon,
/// or a star when `star_inner_radius` is set. Paint, export, booleans and
/// hit-testing all go through here so they agree on the outline.
pub fn polygon_points(rect: Rect, sides: u32, star_inner_radius: Option<f32>) -> Vec<Point2D> {
    match star_inner_radius.filter(|r| r.is_finite()) {
        Some(inner) => star_polygon_points(rect, sides, inner),
        None => regular_polygon_points(rect, sides),
    }
}

/// `2 × points` star vertices, outer tip first at the top. The star is
/// built on a unit circle and its bounding box is stretched onto `rect`,
/// so the tips touch every edge (Figma's star geometry).
pub fn star_polygon_points(rect: Rect, points: u32, inner_radius: f32) -> Vec<Point2D> {
    let n = points.clamp(3, 100) as usize;
    let inner = inner_radius.clamp(STAR_MIN_INNER_RADIUS, STAR_MAX_INNER_RADIUS);
    let step = std::f32::consts::PI / n as f32;
    let mut pts: Vec<Point2D> = (0..n * 2)
        .map(|i| {
            let radius = if i % 2 == 0 { 1.0 } else { inner };
            let angle = -std::f32::consts::FRAC_PI_2 + i as f32 * step;
            Point2D::new(radius * angle.cos(), radius * angle.sin())
        })
        .collect();
    let (mut min_x, mut max_x, mut min_y, mut max_y) = (f32::MAX, f32::MIN, f32::MAX, f32::MIN);
    for p in &pts {
        min_x = min_x.min(p.x);
        max_x = max_x.max(p.x);
        min_y = min_y.min(p.y);
        max_y = max_y.max(p.y);
    }
    // n >= 3 always spans both axes; the guard only rules out a divide by zero.
    let span_x = (max_x - min_x).max(f32::EPSILON);
    let span_y = (max_y - min_y).max(f32::EPSILON);
    for p in &mut pts {
        p.x = rect.origin.x + (p.x - min_x) / span_x * rect.size.x;
        p.y = rect.origin.y + (p.y - min_y) / span_y * rect.size.y;
    }
    pts
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rect(w: f32, h: f32) -> Rect {
        Rect::xywh(10.0, 20.0, w, h)
    }

    #[test]
    fn a_star_has_two_vertices_per_point_and_fills_its_box() {
        let pts = star_polygon_points(rect(100.0, 80.0), 5, 0.382);
        assert_eq!(pts.len(), 10);
        let min_x = pts.iter().map(|p| p.x).fold(f32::MAX, f32::min);
        let max_x = pts.iter().map(|p| p.x).fold(f32::MIN, f32::max);
        let min_y = pts.iter().map(|p| p.y).fold(f32::MAX, f32::min);
        let max_y = pts.iter().map(|p| p.y).fold(f32::MIN, f32::max);
        assert!((min_x - 10.0).abs() < 1e-3 && (max_x - 110.0).abs() < 1e-3);
        assert!((min_y - 20.0).abs() < 1e-3 && (max_y - 100.0).abs() < 1e-3);
        // The first vertex is the top tip, horizontally centred.
        assert!((pts[0].x - 60.0).abs() < 1e-3 && (pts[0].y - 20.0).abs() < 1e-3);
    }

    #[test]
    fn star_valleys_sit_at_the_inner_radius() {
        // A 4-point star is symmetric, so the box centre is the star centre.
        let pts = star_polygon_points(rect(200.0, 200.0), 4, 0.5);
        let dist = |p: &Point2D| ((p.x - 110.0).powi(2) + (p.y - 120.0).powi(2)).sqrt();
        assert!((dist(&pts[0]) - 100.0).abs() < 1e-2);
        assert!((dist(&pts[1]) - 50.0).abs() < 1e-2);
    }

    #[test]
    fn star_points_and_inner_radius_are_clamped() {
        assert_eq!(star_polygon_points(rect(10.0, 10.0), 1, 0.5).len(), 6);
        assert_eq!(star_polygon_points(rect(10.0, 10.0), 500, 0.5).len(), 200);
        assert!(star_polygon_points(rect(10.0, 10.0), 5, -3.0)
            .iter()
            .all(|p| p.x.is_finite() && p.y.is_finite()));
    }

    #[test]
    fn polygon_points_picks_star_or_regular_geometry() {
        assert_eq!(polygon_points(rect(10.0, 10.0), 6, None).len(), 6);
        assert_eq!(polygon_points(rect(10.0, 10.0), 6, Some(0.4)).len(), 12);
        assert_eq!(polygon_points(rect(10.0, 10.0), 6, Some(f32::NAN)).len(), 6);
    }

    #[test]
    fn scene_image_id_matches_the_persisted_schema_id() {
        assert_eq!(
            stable_image_source_id("data:image/png;base64,AA=="),
            0x641a_8b95_c7ff_c372
        );
    }
}
