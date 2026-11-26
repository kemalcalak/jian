use crate::document::NodeTree;
use crate::layout::LayoutEngine;
use jian_ops_schema::node::PenNode;
use serde_json::{json, Value};

fn layout(root: Value) -> impl Fn(&str) -> [f32; 4] {
    let node: PenNode = serde_json::from_value(root).unwrap();
    let mut tree = NodeTree::new();
    tree.insert_subtree(node, None);
    let mut engine = LayoutEngine::new();
    let roots = engine.build(&tree).expect("build");
    engine.compute(roots[0], (2000.0, 2000.0)).expect("compute");
    move |id: &str| {
        let r = engine.node_rect(tree.get(id).expect(id)).expect("rect");
        [r.origin.x, r.origin.y, r.size.width, r.size.height]
    }
}

fn card(id: &str) -> Value {
    json!({"type":"rectangle","id":id,"width":"fill_container","height":80})
}

#[test]
fn three_column_card_grid() {
    let rect = layout(json!({
        "type":"frame","id":"grid","width":340,"height":"fit_content",
        "layout":"grid","columns":3,"gap":20,"padding":10,
        "children":[card("a"),card("b"),card("c"),card("d")]
    }));
    // 340 - 2 * 10 padding - 2 * 20 gaps = 280 → 3 columns of 93.33.
    let col = 280.0 / 3.0;
    let [ax, ay, aw, ah] = rect("a");
    assert_eq!((ax, ay), (10.0, 10.0));
    assert!((aw - col).abs() < 1.0, "{aw}");
    assert_eq!(ah, 80.0);
    let [bx, _, _, _] = rect("b");
    assert!((bx - (10.0 + col + 20.0)).abs() < 1.0, "{bx}");
    // The fourth card wraps to row two, column one.
    let [dx, dy, _, _] = rect("d");
    assert_eq!(dx, 10.0);
    assert_eq!(dy, 10.0 + 80.0 + 20.0);
    // The frame hugs its two rows.
    assert_eq!(rect("grid")[3], 10.0 + 80.0 + 20.0 + 80.0 + 10.0);
}

#[test]
fn placement_spans_and_row_gap() {
    let rect = layout(json!({
        "type":"frame","id":"grid","width":300,"height":"fit_content",
        "layout":"grid","columns":[100,"1fr"],"rows":[40],"columnGap":0,"rowGap":10,
        "children":[
            {"type":"rectangle","id":"wide","width":"fill_container","height":"fill_container","gridColumn":"span 2"},
            {"type":"rectangle","id":"right","width":20,"height":20,"gridColumn":2,"gridRow":2,
             "justifySelf":"end","alignSelf":"center"}
        ]
    }));
    assert_eq!(rect("wide"), [0.0, 0.0, 300.0, 40.0]);
    let [x, y, w, _] = rect("right");
    assert_eq!(w, 20.0);
    assert_eq!(x, 300.0 - 20.0);
    // Row two is auto: as tall as its 20 px child, 10 px below row one.
    assert_eq!(y, 50.0);
}

#[test]
fn horizontal_wrap_moves_overflow_to_a_new_line() {
    let tag = |id: &str| json!({"type":"rectangle","id":id,"width":60,"height":20});
    let rect = layout(json!({
        "type":"frame","id":"tags","width":150,"height":"fit_content",
        "layout":"horizontal","wrap":true,"gap":10,"rowGap":4,
        "children":[tag("t1"),tag("t2"),tag("t3")]
    }));
    assert_eq!(rect("t1")[..2], [0.0, 0.0]);
    assert_eq!(rect("t2")[..2], [70.0, 0.0]);
    assert_eq!(rect("t3")[..2], [0.0, 24.0]);
    assert_eq!(rect("tags")[3], 44.0);
}

#[test]
fn without_wrap_the_row_keeps_one_line() {
    let tag = |id: &str| json!({"type":"rectangle","id":id,"width":60,"height":20});
    let rect = layout(json!({
        "type":"frame","id":"tags","width":150,"height":"fit_content",
        "layout":"horizontal","gap":10,
        "children":[tag("t1"),tag("t2"),tag("t3")]
    }));
    assert_eq!(rect("t3")[1], 0.0);
}

#[test]
fn stale_child_position_is_ignored_in_a_grid() {
    let rect = layout(json!({
        "type":"frame","id":"grid","width":200,"height":100,
        "layout":"grid","columns":2,
        "children":[{"type":"rectangle","id":"a","x":150,"y":70,"width":10,"height":10}]
    }));
    assert_eq!(rect("a")[..2], [0.0, 0.0]);
}
