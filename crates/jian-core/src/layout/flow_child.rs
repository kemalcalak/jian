//! Flow rules for children of an auto-layout (`vertical` / `horizontal`)
//! container.
//!
//! `node_to_style` turns any authored `x` / `y` into a `Position::Absolute`
//! inset. That is right for free-form and `layout: "none"` parents, but
//! inside an auto-layout parent the layout owns both axes: generated nodes
//! routinely carry stale coordinates, and honouring them pulled the child
//! out of flow — its authored `y` beat `alignItems`, `fill_container`
//! resolved against the padding box, and siblings closed the gap it left.
//! Only an explicit absolute contract keeps the inset: `role: "overlay"`
//! (badges, notification dots) or responsive `constraints`.

use jian_ops_schema::node::container::LayoutMode;
use jian_ops_schema::node::{PenNode, ScriptNode};
use taffy::prelude::*;

/// Put a child of an auto-layout `parent` back into flow unless it opted
/// into absolute positioning. Children of `layout: "none"` stacks and of
/// legacy frames with no `layout` keep their authored position.
pub(crate) fn apply_flow_child(style: &mut Style, child: &PenNode, parent: &PenNode) {
    if style.position != Position::Absolute || !is_auto_layout(parent) || keeps_absolute(child) {
        return;
    }
    style.position = Position::Relative;
    style.inset = Rect {
        left: LengthPercentageAuto::Auto,
        right: LengthPercentageAuto::Auto,
        top: LengthPercentageAuto::Auto,
        bottom: LengthPercentageAuto::Auto,
    };
}

fn is_auto_layout(n: &PenNode) -> bool {
    let layout = match n {
        PenNode::Frame(f) | PenNode::Script(ScriptNode { frame: f, .. }) => {
            f.container.layout.as_ref()
        }
        PenNode::Group(g) => g.container.layout.as_ref(),
        PenNode::Rectangle(r) => r.container.layout.as_ref(),
        _ => None,
    };
    matches!(layout, Some(LayoutMode::Vertical | LayoutMode::Horizontal))
}

/// Keep in sync with de-pen-loader's `layout_repair::layout_child`, which
/// leaves the same nodes out of its in-flow reflow.
fn keeps_absolute(n: &PenNode) -> bool {
    super::resolve::node_base(n)
        .is_some_and(|base| base.role.as_deref() == Some("overlay") || base.constraints.is_some())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::resolve::node_to_style;

    fn node(json: &str) -> PenNode {
        serde_json::from_str(json).unwrap()
    }

    fn child_style(child: &str, parent_layout: &str) -> Style {
        let parent = node(&format!(
            r#"{{"type":"frame","id":"p","width":100,"height":100{parent_layout}}}"#
        ));
        let child = node(child);
        let mut style = node_to_style(&child);
        apply_flow_child(&mut style, &child, &parent);
        style
    }

    const POSITIONED: &str = r#"{"type":"text","id":"t","content":"a","x":30,"y":10}"#;

    #[test]
    fn auto_layout_child_ignores_authored_position() {
        for layout in [r#","layout":"horizontal""#, r#","layout":"vertical""#] {
            let style = child_style(POSITIONED, layout);
            assert_eq!(style.position, Position::Relative, "{layout}");
            assert_eq!(style.inset.top, LengthPercentageAuto::Auto, "{layout}");
            assert_eq!(style.inset.left, LengthPercentageAuto::Auto, "{layout}");
        }
    }

    #[test]
    fn overlay_and_constrained_children_stay_absolute() {
        for child in [
            r#"{"type":"text","id":"t","content":"a","role":"overlay","x":30,"y":10}"#,
            r#"{"type":"text","id":"t","content":"a","x":30,"y":10,"constraints":{"h":"right","v":"top"}}"#,
        ] {
            let style = child_style(child, r#","layout":"horizontal""#);
            assert_eq!(style.position, Position::Absolute, "{child}");
        }
    }

    #[test]
    fn stack_and_legacy_parents_keep_authored_position() {
        for layout in [r#","layout":"none""#, ""] {
            let style = child_style(POSITIONED, layout);
            assert_eq!(style.position, Position::Absolute, "{layout:?}");
        }
    }
}
