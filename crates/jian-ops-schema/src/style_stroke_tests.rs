use super::{PenStroke, StrokeEndCap, StrokeWidthPoint, StrokeWidthPreset, StrokeWidthProfile};

#[test]
fn stroke_ends_miter_limit_and_profile_round_trip() {
    let json = r#"{"thickness":2.0,"strokeStart":"reversed_triangle","strokeEnd":"line_arrow","miterLimit":8.0,"widthProfile":{"preset":"custom","points":[{"t":0.0,"width":0.5},{"t":1.0,"width":2.0}]}}"#;
    let stroke: PenStroke = serde_json::from_str(json).expect("parse stroke");
    assert_eq!(stroke.stroke_start, Some(StrokeEndCap::ReversedTriangle));
    assert_eq!(stroke.stroke_end, Some(StrokeEndCap::LineArrow));
    assert_eq!(stroke.miter_limit, Some(8.0));
    let profile = stroke.width_profile.as_ref().expect("profile");
    assert_eq!(profile.preset, StrokeWidthPreset::Custom);
    assert_eq!(serde_json::to_string(&stroke).unwrap(), json);
}

#[test]
fn new_stroke_fields_stay_absent_by_default() {
    let json = r#"{"thickness":1.0,"cap":"round"}"#;
    let stroke: PenStroke = serde_json::from_str(json).expect("parse stroke");
    assert_eq!(stroke.stroke_start, None);
    assert_eq!(stroke.stroke_end, None);
    assert_eq!(stroke.miter_limit, None);
    assert_eq!(stroke.width_profile, None);
    assert_eq!(serde_json::to_string(&stroke).unwrap(), json);
}

#[test]
fn every_end_cap_round_trips_by_its_snake_case_name() {
    for (cap, name) in [
        (StrokeEndCap::None, "none"),
        (StrokeEndCap::Round, "round"),
        (StrokeEndCap::Square, "square"),
        (StrokeEndCap::LineArrow, "line_arrow"),
        (StrokeEndCap::TriangleArrow, "triangle_arrow"),
        (StrokeEndCap::ReversedTriangle, "reversed_triangle"),
        (StrokeEndCap::Circle, "circle"),
        (StrokeEndCap::Diamond, "diamond"),
    ] {
        let wire = format!("\"{name}\"");
        assert_eq!(serde_json::to_string(&cap).unwrap(), wire);
        assert_eq!(serde_json::from_str::<StrokeEndCap>(&wire).unwrap(), cap);
    }
}

#[test]
fn profile_stops_cover_presets_and_clean_custom_points() {
    let preset = |preset| StrokeWidthProfile {
        preset,
        points: None,
    };
    assert!(preset(StrokeWidthPreset::Uniform).stops().is_empty());
    let taper = preset(StrokeWidthPreset::TaperBoth).stops();
    assert_eq!(taper.len(), 3);
    assert_eq!((taper[1].t, taper[1].width), (0.5, 1.0));

    let custom = StrokeWidthProfile {
        preset: StrokeWidthPreset::Custom,
        points: Some(vec![
            StrokeWidthPoint { t: 0.8, width: 3.0 },
            StrokeWidthPoint {
                t: f32::NAN,
                width: 1.0,
            },
            StrokeWidthPoint {
                t: -1.0,
                width: 99.0,
            },
        ]),
    };
    let stops = custom.stops();
    assert_eq!(stops.len(), 2);
    assert_eq!((stops[0].t, stops[0].width), (0.0, super::MAX_WIDTH_FACTOR));
    assert_eq!((stops[1].t, stops[1].width), (0.8, 3.0));
}

#[test]
fn long_custom_profiles_resample_to_the_cap_and_flat_ones_are_uniform() {
    let custom = |points: Vec<StrokeWidthPoint>| StrokeWidthProfile {
        preset: StrokeWidthPreset::Custom,
        points: Some(points),
    };
    let ramp: Vec<StrokeWidthPoint> = (0..=100)
        .map(|i| StrokeWidthPoint {
            t: i as f32 / 100.0,
            width: i as f32 / 100.0,
        })
        .collect();
    let stops = custom(ramp).stops();
    assert_eq!(stops.len(), super::MAX_WIDTH_STOPS);
    assert_eq!((stops[0].t, stops[0].width), (0.0, 0.0));
    let last = stops[super::MAX_WIDTH_STOPS - 1];
    assert_eq!((last.t, last.width), (1.0, 1.0));
    assert!((super::width_at(&stops, 0.5) - 0.5).abs() < 1e-4);

    let flat = vec![
        StrokeWidthPoint { t: 0.0, width: 1.0 },
        StrokeWidthPoint { t: 1.0, width: 1.0 },
    ];
    assert!(custom(flat).stops().is_empty());
    assert_eq!(super::width_at(&[], 0.3), 1.0);
}
