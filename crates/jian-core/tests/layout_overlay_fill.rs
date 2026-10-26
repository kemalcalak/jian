//! `fill_container` on a child that stays absolute (`role: "overlay"`).
//!
//! In an auto-layout parent a fill height becomes `flex_grow` (or a row's
//! stretch) so it can take the line's remaining space. An overlay is out of
//! the flow line, where neither applies, so it keeps `percent(1.0)` and
//! covers its parent once the parent's size is known — even when that size
//! itself comes from flex growth. Importers rely on this to pin content to
//! the bottom / right edge of a box only layout can size (an HTML
//! `position:absolute; bottom:0` row in a `flex:1` slide body).

use jian_core::document::{loader, RuntimeDocument};
use jian_core::geometry::Rect;
use jian_core::layout::LayoutEngine;
use jian_core::signal::scheduler::Scheduler;
use jian_core::state::StateGraph;
use jian_ops_schema::load_str;
use std::rc::Rc;

fn build(src: &str) -> RuntimeDocument {
    let schema = load_str(src).unwrap().value;
    let sched = Rc::new(Scheduler::new());
    let state = StateGraph::new(sched);
    loader::build(schema, &state).unwrap()
}

fn laid_out(src: &str) -> impl Fn(&str) -> Rect {
    let doc = build(src);
    let mut eng = LayoutEngine::new();
    let roots = eng.build(&doc.tree).unwrap();
    eng.compute(roots[0], (400.0, 300.0)).unwrap();
    move |id: &str| {
        eng.node_rect(doc.tree.get(id).expect("unknown id"))
            .expect("no rect")
    }
}

#[test]
fn an_overlay_fills_a_parent_sized_by_flex_growth() {
    for layout in ["vertical", "horizontal"] {
        let at = laid_out(&format!(
            r##"{{
          "version":"0.8.0",
          "children":[{{
            "type":"frame","id":"slide","width":400,"height":300,"layout":"{layout}",
            "padding":[20,20],
            "children":[
              {{"type":"rectangle","id":"title","width":100,"height":60}},
              {{"type":"frame","id":"body","width":"fill_container","height":"fill_container",
                "layout":"vertical",
                "children":[{{
                  "type":"frame","id":"anchor","role":"overlay","x":0,"y":0,
                  "width":"fill_container","height":"fill_container",
                  "layout":"vertical","justifyContent":"end",
                  "children":[{{"type":"rectangle","id":"row","width":"fill_container","height":40}}]
                }}]
              }}
            ]
          }}]
        }}"##
        ));
        let body = at("body");
        let anchor = at("anchor");
        let row = at("row");
        assert_eq!(
            anchor.size, body.size,
            "{layout}: the overlay covers its parent"
        );
        assert!(
            body.size.height > 40.0,
            "{layout}: the parent grew: {body:?}"
        );
        assert_eq!(
            row.origin.y + row.size.height,
            anchor.origin.y + anchor.size.height,
            "{layout}: justify end pins the row to the overlay's bottom"
        );
    }
}
