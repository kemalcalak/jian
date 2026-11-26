use super::*;

fn track(json: &str) -> GridTrack {
    serde_json::from_str(json).unwrap()
}

#[test]
fn tracks_parse_css_and_round_trip() {
    assert_eq!(track("120"), GridTrack::Size(TrackSize::Px(120.0)));
    assert_eq!(track("\"120px\""), GridTrack::Size(TrackSize::Px(120.0)));
    assert_eq!(track("\"2fr\""), GridTrack::Size(TrackSize::Fr(2.0)));
    assert_eq!(track("\"auto\""), GridTrack::Size(TrackSize::Auto));
    assert_eq!(
        track("\"minmax(100px, 1fr)\""),
        GridTrack::MinMax {
            min: TrackSize::Px(100.0),
            max: TrackSize::Fr(1.0)
        }
    );
    for json in ["120", "\"1fr\"", "\"auto\"", "\"minmax(100px, 1fr)\""] {
        assert_eq!(serde_json::to_string(&track(json)).unwrap(), json);
    }
}

#[test]
fn bad_tracks_are_rejected() {
    for json in ["-4", "\"1xx\"", "\"minmax(1fr, 2fr)\"", "\"minmax(10)\""] {
        assert!(serde_json::from_str::<GridTrack>(json).is_err(), "{json}");
    }
}

#[test]
fn template_takes_count_array_or_css_string() {
    let count: GridTemplate = serde_json::from_str("3").unwrap();
    assert_eq!(count, GridTemplate::Count(3));
    assert_eq!(count.tracks(), vec![GridTrack::FR1; 3]);
    assert_eq!(serde_json::to_string(&count).unwrap(), "3");

    let list: GridTemplate = serde_json::from_str(r#"[200, "1fr", "auto"]"#).unwrap();
    assert_eq!(list.len(), 3);
    assert_eq!(
        serde_json::to_string(&list).unwrap(),
        r#"[200,"1fr","auto"]"#
    );

    let css: GridTemplate = serde_json::from_str(r#""repeat(2, 1fr) 80px""#).unwrap();
    assert_eq!(
        css.tracks(),
        vec![
            GridTrack::FR1,
            GridTrack::FR1,
            GridTrack::Size(TrackSize::Px(80.0))
        ]
    );
    let nested = GridTemplate::parse("repeat(3, minmax(0, 1fr))").unwrap();
    assert_eq!(nested.len(), 3);
    assert!(nested.tracks()[0].is_flexible());
}

#[test]
fn template_short_text_reparses() {
    let t = GridTemplate::parse("1fr 120 minmax(80px, 1fr) auto").unwrap();
    assert_eq!(t.short_text(), "1fr 120 minmax(80px, 1fr) auto");
    assert_eq!(GridTemplate::parse(&t.short_text()), Some(t));
    assert_eq!(GridTemplate::parse(""), None);
    assert_eq!(GridTemplate::parse("1fr nope"), None);
}

#[test]
fn placement_forms() {
    let p = |s: &str| GridPlacement::parse(s).unwrap();
    assert_eq!(
        p("2"),
        GridPlacement {
            start: Some(2),
            span: 1
        }
    );
    assert_eq!(
        p("span 2"),
        GridPlacement {
            start: None,
            span: 2
        }
    );
    assert_eq!(
        p("1 / span 3"),
        GridPlacement {
            start: Some(1),
            span: 3
        }
    );
    assert_eq!(
        p("2 / 4"),
        GridPlacement {
            start: Some(2),
            span: 2
        }
    );
    assert_eq!(p("auto"), GridPlacement::default());
    for bad in ["0", "3 / 2", "span 0", "span 2 / 3", "x"] {
        assert_eq!(GridPlacement::parse(bad), None, "{bad}");
    }
}

#[test]
fn placement_round_trips() {
    for json in ["2", "\"span 2\"", "\"1 / span 3\""] {
        let p: GridPlacement = serde_json::from_str(json).unwrap();
        assert_eq!(serde_json::to_string(&p).unwrap(), json);
    }
    let p: GridPlacement = serde_json::from_str("\"2 / 4\"").unwrap();
    assert_eq!(serde_json::to_string(&p).unwrap(), "\"2 / span 2\"");
    assert!(serde_json::from_str::<GridPlacement>("1.5").is_err());
}
