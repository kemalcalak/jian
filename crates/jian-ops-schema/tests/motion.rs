use jian_ops_schema::{load_str, LoadWarning, NodeAnimation};

fn motion_document(extra: &str) -> String {
    format!(
        r##"{{
            "version":"1.1",
            "formatVersion":"1.1",
            "children":[{{
                "type":"rectangle","id":"hero","width":100,"height":100,
                {extra}
            }}]
        }}"##
    )
}

#[test]
fn motion_fields_round_trip_for_both_triggers_and_document_preference() {
    let source = motion_document(
        r##""transition":{"durationMs":240,"easing":"emphasized","properties":["opacity","fill"],"futureToken":42},
                "animations":[
                    {"trigger":"mount","keyframes":[
                        {"offset":0,"values":{"opacity":0,"translateY":24}},
                        {"offset":1,"values":{"opacity":1,"translateY":0}}
                    ],"durationMs":400,"easing":"standard","fillMode":"forwards"},
                    {"trigger":"inView","keyframes":[
                        {"offset":0,"values":{"opacity":0}},
                        {"offset":1,"values":{"opacity":1}}
                    ],"durationMs":300,"delayMs":80,"once":false,"fillMode":"none"}
                ]"##,
    );
    let source = source.replace("\"children\":[{", "\"motion\":\"reduced\",\"children\":[{");
    let loaded = load_str(&source).expect("motion document");
    assert_eq!(
        loaded.value.motion,
        Some(jian_ops_schema::MotionPreference::Reduced)
    );
    let node = &loaded.value.children[0];
    let (transition, animations) = node.motion_declarations();
    assert_eq!(transition.unwrap().duration_ms, 240);
    assert_eq!(animations.unwrap().len(), 2);
    assert_eq!(
        serde_json::to_value(&loaded.value).unwrap()["motion"],
        "reduced"
    );
    assert_eq!(
        serde_json::to_value(&loaded.value).unwrap()["children"][0]["transition"]["easing"],
        "emphasized"
    );
    assert_eq!(
        serde_json::to_value(&loaded.value).unwrap()["children"][0]["transition"]["futureToken"],
        42
    );
}

#[test]
fn old_documents_without_motion_load_unchanged() {
    let source = r#"{"version":"0.8.0","children":[{"type":"rectangle","id":"r"}]}"#;
    let loaded = load_str(source).expect("legacy document");
    assert_eq!(loaded.value.motion, None);
    assert_eq!(serde_json::to_string(&loaded.value).unwrap(), source);
}

#[test]
fn invalid_keyframes_are_dropped_with_a_warning() {
    for (name, keyframes) in [
        (
            "unsorted",
            r#"[{"offset":0,"values":{"opacity":0}},{"offset":0.8,"values":{"opacity":1}},{"offset":0.5,"values":{"opacity":1}}]"#,
        ),
        (
            "missing_zero",
            r#"[{"offset":0.2,"values":{"opacity":0}},{"offset":1,"values":{"opacity":1}}]"#,
        ),
        (
            "missing_one",
            r#"[{"offset":0,"values":{"opacity":0}},{"offset":0.8,"values":{"opacity":1}}]"#,
        ),
    ] {
        let source = motion_document(&format!(
            r##""animations":[{{"trigger":"mount","keyframes":{keyframes},"durationMs":200}}]"##
        ));
        let loaded = load_str(&source).unwrap_or_else(|e| panic!("{name}: {e}"));
        assert!(
            loaded.value.children[0].motion_declarations().1.is_none(),
            "{name}: the broken animation is dropped"
        );
        assert!(
            loaded.warnings.iter().any(|w| matches!(
                w,
                LoadWarning::InvalidAnimationDropped { path, node, .. }
                    if path.starts_with("$.children[0].animations[0]") && node == "hero"
            )),
            "{name}: {:?}",
            loaded.warnings
        );
    }
}

#[test]
fn unknown_animation_property_is_dropped_and_the_rest_kept() {
    let source = motion_document(
        r##""animations":[{"trigger":"mount","durationMs":200,"keyframes":[
            {"offset":0,"values":{"opacity":0,"notAProperty":1}},
            {"offset":1,"values":{"opacity":1}}
        ]}]"##,
    );
    let loaded = load_str(&source).expect("document opens");
    let animation = hero_animation_values(&source);
    assert_eq!(animation["keyframes"][0]["values"]["opacity"], 0.0);
    assert!(animation["keyframes"][0]["values"]
        .get("notAProperty")
        .is_none());
    assert!(loaded.warnings.iter().any(|w| matches!(
        w,
        LoadWarning::AnimationPropertyDropped { property, node, .. }
            if property == "notAProperty" && node == "hero"
    )));
    assert!(loaded.warnings.iter().all(LoadWarning::is_motion_repair));
    assert!(loaded.warnings[0].to_string().contains("hero"));
}

#[test]
fn scale_alias_expands_to_both_axes_on_load() {
    let source = motion_document(
        r##""animations":[{"trigger":"mount","durationMs":200,"keyframes":[
            {"offset":0,"values":{"scale":0.5,"scaleY":0.8}},
            {"offset":1,"values":{"scale":1}}
        ]}]"##,
    );
    let loaded = load_str(&source).expect("document opens");
    assert!(loaded.warnings.is_empty(), "{:?}", loaded.warnings);
    let animation = hero_animation_values(&source);
    let first = &animation["keyframes"][0]["values"];
    assert_eq!(first["scaleX"], 0.5);
    assert_eq!(first["scaleY"], 0.8, "an explicit axis wins over the alias");
    assert!(first.get("scale").is_none());
    assert_eq!(animation["keyframes"][1]["values"]["scaleX"], 1.0);
    assert_eq!(animation["keyframes"][1]["values"]["scaleY"], 1.0);
}

#[test]
fn unknown_property_reason_names_the_valid_spelling() {
    let reason = jian_ops_schema::motion::unknown_property_reason("scale");
    assert!(
        reason.contains("scaleX") && reason.contains("scaleY"),
        "{reason}"
    );
    let reason = jian_ops_schema::motion::unknown_property_reason("blur");
    assert!(reason.contains("opacity"), "{reason}");
}

fn hero_animation_values(source: &str) -> serde_json::Value {
    let loaded = load_str(source).expect("motion document");
    serde_json::to_value(&loaded.value).unwrap()["children"][0]["animations"][0].clone()
}

#[test]
fn canonical_keyframes_keep_the_values_wrapper() {
    let source = motion_document(
        r##""animations":[{"trigger":"mount","durationMs":400,"keyframes":[
            {"offset":0,"values":{"opacity":0,"translateY":16}},
            {"offset":1,"values":{"opacity":1,"translateY":0}}
        ]}]"##,
    );
    let animation = hero_animation_values(&source);
    assert_eq!(animation["keyframes"][0]["offset"], 0.0);
    assert_eq!(animation["keyframes"][0]["values"]["opacity"], 0.0);
    assert_eq!(animation["keyframes"][0]["values"]["translateY"], 16.0);
    assert!(animation["keyframes"][0].get("opacity").is_none());
}

#[test]
fn flat_keyframes_load_as_property_maps() {
    let source = motion_document(
        r##""animations":[{"trigger":"mount","durationMs":400,"keyframes":[
            {"offset":0,"opacity":0,"translateY":16},
            {"offset":1,"opacity":1,"translateY":0}
        ]}]"##,
    );
    let loaded = load_str(&source).expect("flat keyframes");
    let (_, animations) = loaded.value.children[0].motion_declarations();
    let keyframes = &animations.expect("animations")[0].keyframes;
    assert_eq!(keyframes[0].offset, 0.0);
    assert_eq!(keyframes[0].values["opacity"], serde_json::json!(0));
    assert_eq!(keyframes[0].values["translateY"], serde_json::json!(16));
    assert_eq!(keyframes[1].offset, 1.0);
    assert_eq!(keyframes[1].values["opacity"], serde_json::json!(1));
}

#[test]
fn mixed_keyframe_list_loads_canonical_and_flat_together() {
    let source = motion_document(
        r##""animations":[{"trigger":"mount","durationMs":400,"keyframes":[
            {"offset":0,"opacity":0,"translateY":16},
            {"offset":1,"values":{"opacity":1,"translateY":0}}
        ]}]"##,
    );
    let animation = hero_animation_values(&source);
    assert_eq!(animation["keyframes"][0]["values"]["opacity"], 0.0);
    assert_eq!(animation["keyframes"][0]["values"]["translateY"], 16.0);
    assert_eq!(animation["keyframes"][1]["values"]["opacity"], 1.0);
    assert_eq!(animation["keyframes"][1]["values"]["translateY"], 0.0);
}

#[test]
fn from_to_shorthand_deserializes_to_offsets_zero_and_one() {
    let animation: NodeAnimation = serde_json::from_value(serde_json::json!({
        "trigger": "mount",
        "durationMs": 400,
        "keyframes": {
            "from": {"opacity": 0, "translateY": 16},
            "to": {"opacity": 1, "translateY": 0}
        }
    }))
    .expect("from/to shorthand");
    assert_eq!(animation.keyframes.len(), 2);
    assert_eq!(animation.keyframes[0].offset, 0.0);
    assert_eq!(animation.keyframes[1].offset, 1.0);
    assert_eq!(
        animation.keyframes[0].values["translateY"],
        serde_json::json!(16)
    );
    assert_eq!(
        animation.keyframes[1].values["opacity"],
        serde_json::json!(1)
    );
}

#[test]
fn missing_keyframe_offset_names_the_index() {
    let err = serde_json::from_str::<NodeAnimation>(
        r#"{"trigger":"mount","durationMs":200,"keyframes":[{"opacity":0},{"offset":1,"opacity":1}]}"#,
    )
    .unwrap_err();
    let msg = err.to_string();
    assert!(
        msg.contains("missing field `offset`"),
        "expected missing offset, got {msg}"
    );
    assert!(
        msg.contains("at keyframe 0"),
        "expected keyframe index, got {msg}"
    );
}

#[test]
fn flat_keyframes_serialize_canonical_values_wrapper() {
    let source = motion_document(
        r##""animations":[{"trigger":"mount","durationMs":400,"keyframes":[
            {"offset":0,"opacity":0,"translateY":16},
            {"offset":1,"opacity":1,"translateY":0}
        ]}]"##,
    );
    let animation = hero_animation_values(&source);
    assert_eq!(
        animation["keyframes"][0],
        serde_json::json!({"offset": 0.0, "values": {"opacity": 0, "translateY": 16}})
    );
    assert_eq!(
        animation["keyframes"][1],
        serde_json::json!({"offset": 1.0, "values": {"opacity": 1, "translateY": 0}})
    );
    assert!(animation["keyframes"][0].get("opacity").is_none());
    assert!(animation["keyframes"][0].get("translateY").is_none());
}

#[test]
fn pointer_triggers_and_size_properties_load_and_round_trip() {
    let source = motion_document(
        r##""animations":[
                    {"trigger":"hover","keyframes":{"from":{"width":100},"to":{"width":120}},"durationMs":150},
                    {"trigger":"tap","keyframes":{"from":{"height":40},"to":{"height":36}},"durationMs":100}
                ]"##,
    );
    let loaded = load_str(&source).expect("pointer motion document");
    let animations = loaded.value.children[0].motion_declarations().1.unwrap();
    assert_eq!(animations[0].trigger, jian_ops_schema::MotionTrigger::Hover);
    assert_eq!(animations[1].trigger, jian_ops_schema::MotionTrigger::Tap);
    assert!(animations.iter().all(|a| a.trigger.is_pointer()));
    assert!(jian_ops_schema::motion::P1_MOTION_PROPERTIES.contains(&"width"));
    let json = serde_json::to_value(&loaded.value).unwrap();
    assert_eq!(json["children"][0]["animations"][0]["trigger"], "hover");
    assert_eq!(json["children"][0]["animations"][1]["trigger"], "tap");
}

#[test]
fn slide_transition_defaults_and_round_trips() {
    let source = motion_document(r##""slideTransition":{"kind":"push","futureToken":1}"##);
    let loaded = load_str(&source).expect("slide transition document");
    let json = serde_json::to_value(&loaded.value.children[0]).unwrap();
    assert_eq!(json["slideTransition"]["kind"], "push");
    assert_eq!(json["slideTransition"]["durationMs"], 400);
    assert_eq!(json["slideTransition"]["futureToken"], 1);
    let parsed: jian_ops_schema::SlideTransition =
        serde_json::from_value(json["slideTransition"].clone()).unwrap();
    assert_eq!(parsed.kind, jian_ops_schema::SlideTransitionKind::Push);
    assert_eq!(
        jian_ops_schema::SlideTransition::default().kind,
        jian_ops_schema::SlideTransitionKind::None
    );
}

#[test]
fn warning_paths_name_nested_nodes() {
    let source = r##"{"version":"1.1","formatVersion":"1.1","children":[
        {"type":"frame","id":"a","children":[
            {"type":"rectangle","id":"b"},
            {"type":"rectangle","id":"c","animations":[{"trigger":"mount","durationMs":1,"keyframes":[
                {"offset":0,"values":{"blur":1}},{"offset":1,"values":{"opacity":1}}]}]}
        ]}
    ]}"##;
    let loaded = load_str(source).expect("document opens");
    let LoadWarning::AnimationPropertyDropped { path, node, .. } = &loaded.warnings[0] else {
        panic!("{:?}", loaded.warnings);
    };
    assert_eq!(node, "c");
    assert_eq!(
        path,
        "$.children[0].children[1].animations[0].keyframes[0].values.blur"
    );
}
