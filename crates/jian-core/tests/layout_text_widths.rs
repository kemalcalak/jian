//! `LayoutEngine::compute_with_text_widths`: the width a text leaf was
//! measured at, before taffy rounds its box to whole pixels.

use jian_core::document::loader;
use jian_core::layout::LayoutEngine;
use jian_core::signal::scheduler::Scheduler;
use jian_core::state::StateGraph;
use jian_ops_schema::load_str;
use std::rc::Rc;

#[test]
fn text_widths_keep_the_fractional_fill_share() {
    // Two `fill_container` texts split a 301 px row: 150.5 px each before
    // rounding, one 150 and one 151 px box after.
    let schema = load_str(
        r##"{
      "version":"0.8.0",
      "children":[{
        "type":"frame","id":"row","width":301,"height":100,"layout":"horizontal",
        "children":[
          {"type":"text","id":"a","width":"fill_container","content":"Left column text",
           "textGrowth":"fixed-width"},
          {"type":"text","id":"b","width":"fill_container","content":"Right column text",
           "textGrowth":"fixed-width"}
        ]
      }]
    }"##,
    )
    .unwrap()
    .value;
    let state = StateGraph::new(Rc::new(Scheduler::new()));
    let doc = loader::build(schema, &state).unwrap();
    let mut eng = LayoutEngine::new();
    let roots = eng.build(&doc.tree).unwrap();
    let widths = eng
        .compute_with_text_widths(roots[0], (301.0, 100.0))
        .unwrap();

    let a = doc.tree.get("a").unwrap();
    let b = doc.tree.get("b").unwrap();
    assert_eq!(widths.get(&a), Some(&150.5));
    assert_eq!(widths.get(&b), Some(&150.5));
    assert_eq!(widths.len(), 2, "only text leaves are reported");

    // The rounded layout is what `compute` alone produces.
    let rounded: f32 = [a, b]
        .iter()
        .map(|key| eng.node_rect(*key).unwrap().size.width)
        .sum();
    assert_eq!(rounded, 301.0);
    for key in [a, b] {
        let w = eng.node_rect(key).unwrap().size.width;
        assert_eq!(w, w.round(), "node_rect stays on whole pixels");
    }
}
