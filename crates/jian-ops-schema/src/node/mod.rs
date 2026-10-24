pub mod base;
pub mod checkbox;
pub mod container;
pub mod ellipse;
pub mod frame;
pub mod icon_font;
pub mod image;
pub mod image_src;
pub mod line;
pub mod note;
pub mod number_input;
pub mod path;
pub mod polygon;
pub mod progress;
pub mod radio_group;
pub mod ref_node;
pub mod select;
pub mod slider;
pub mod switch;
pub mod tabs;
pub mod text;
pub mod text_area;
pub mod text_input;
pub mod video;

pub use base::{BoolOrExpression, MaskType, NumberOrExpression, PenNodeBase};
pub use checkbox::CheckboxNode;
pub use container::{
    AlignItems, ContainerProps, CornerRadius, JustifyContent, LayoutMode, Padding,
};

pub use ellipse::EllipseNode;
pub use frame::{FrameNode, GroupNode, RectangleNode};
pub use icon_font::IconFontNode;
pub use image::{ImageFitMode, ImageNode};
pub use image_src::ImageSrc;
pub use line::LineNode;
pub use note::{AnnotationKind, NoteNode};
pub use number_input::NumberInputNode;
pub use path::{PathFillRule, PathNode, PenPathAnchor, PenPathHandle, PenPathPointType};
pub use polygon::PolygonNode;
pub use progress::ProgressNode;
pub use radio_group::RadioGroupNode;
pub use ref_node::{DescendantOverrides, RefNode};
pub use select::{SelectNode, SelectOption};
pub use slider::SliderNode;
pub use switch::SwitchNode;
pub use tabs::TabsNode;
pub use text::{
    FontStyleKind as TextFontStyle, FontWeight, TextAlign, TextAlignVertical, TextContent,
    TextGrowth, TextNode,
};
pub use text_area::TextAreaNode;
pub use text_input::TextInputNode;
pub use video::VideoMeta;

use serde::{Deserialize, Serialize};

/// Union of all concrete node types.
/// Tag is the JSON `"type"` field.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[cfg_attr(feature = "export-ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "export-ts", ts(export, export_to = "ops.ts"))]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum PenNode {
    Frame(FrameNode),
    Group(GroupNode),
    Rectangle(RectangleNode),
    Ellipse(EllipseNode),
    Line(LineNode),
    Polygon(PolygonNode),
    Path(PathNode),
    Text(TextNode),
    TextInput(TextInputNode),
    Image(ImageNode),
    IconFont(IconFontNode),
    TextArea(TextAreaNode),
    Select(SelectNode),
    Switch(SwitchNode),
    Checkbox(CheckboxNode),
    Slider(SliderNode),
    RadioGroup(RadioGroupNode),
    NumberInput(NumberInputNode),
    Progress(ProgressNode),
    Tabs(TabsNode),
    Ref(RefNode),
    /// A sticky note on the canvas (pen.dev `note`).
    Note(NoteNode),
    /// Background the AI reads (pen.dev `context`).
    Context(NoteNode),
    /// An instruction for a model (pen.dev `prompt`).
    Prompt(NoteNode),
}

impl PenNode {
    /// Borrow the node-level motion declarations without serializing the
    /// recursive node tree.
    pub fn motion_declarations(
        &self,
    ) -> (
        Option<&crate::motion::Transition>,
        Option<&Vec<crate::motion::NodeAnimation>>,
    ) {
        match self {
            PenNode::Frame(node) => (node.base.transition.as_ref(), node.base.animations.as_ref()),
            PenNode::Group(node) => (node.base.transition.as_ref(), node.base.animations.as_ref()),
            PenNode::Rectangle(node) => {
                (node.base.transition.as_ref(), node.base.animations.as_ref())
            }
            PenNode::Ellipse(node) => {
                (node.base.transition.as_ref(), node.base.animations.as_ref())
            }
            PenNode::Line(node) => (node.base.transition.as_ref(), node.base.animations.as_ref()),
            PenNode::Polygon(node) => {
                (node.base.transition.as_ref(), node.base.animations.as_ref())
            }
            PenNode::Path(node) => (node.base.transition.as_ref(), node.base.animations.as_ref()),
            PenNode::Text(node) => (node.base.transition.as_ref(), node.base.animations.as_ref()),
            PenNode::TextInput(node) => {
                (node.base.transition.as_ref(), node.base.animations.as_ref())
            }
            PenNode::Image(node) => (node.base.transition.as_ref(), node.base.animations.as_ref()),
            PenNode::IconFont(node) => {
                (node.base.transition.as_ref(), node.base.animations.as_ref())
            }
            PenNode::TextArea(node) => {
                (node.base.transition.as_ref(), node.base.animations.as_ref())
            }
            PenNode::Select(node) => (node.base.transition.as_ref(), node.base.animations.as_ref()),
            PenNode::Switch(node) => (node.base.transition.as_ref(), node.base.animations.as_ref()),
            PenNode::Checkbox(node) => {
                (node.base.transition.as_ref(), node.base.animations.as_ref())
            }
            PenNode::Slider(node) => (node.base.transition.as_ref(), node.base.animations.as_ref()),
            PenNode::RadioGroup(node) => {
                (node.base.transition.as_ref(), node.base.animations.as_ref())
            }
            PenNode::NumberInput(node) => {
                (node.base.transition.as_ref(), node.base.animations.as_ref())
            }
            PenNode::Progress(node) => {
                (node.base.transition.as_ref(), node.base.animations.as_ref())
            }
            PenNode::Tabs(node) => (node.base.transition.as_ref(), node.base.animations.as_ref()),
            PenNode::Ref(node) => (node.base.transition.as_ref(), node.base.animations.as_ref()),
            PenNode::Note(node) | PenNode::Context(node) | PenNode::Prompt(node) => {
                (node.base.transition.as_ref(), node.base.animations.as_ref())
            }
        }
    }

    /// Children of a container node, if it has a children field.
    pub fn children_ref(&self) -> Option<&[PenNode]> {
        match self {
            PenNode::Frame(node) => node.children.as_deref(),
            PenNode::Group(node) => node.children.as_deref(),
            PenNode::Rectangle(node) => node.children.as_deref(),
            PenNode::Tabs(node) => node.children.as_deref(),
            PenNode::Ref(node) => node.children.as_deref(),
            _ => None,
        }
    }

    /// Borrow the optional `gestures` and `semantics` blocks from any
    /// variant. Used by the focus-chain extractor in `jian-core` to
    /// avoid round-tripping the entire schema through
    /// `serde_json::to_value` for each lookup — which would
    /// re-serialise every container's recursive `children` subtree
    /// O(n) times during a chain rebuild on a nested document. The
    /// `rawPointer` opt-in still goes through `serde_json::to_value`
    /// today (see `gesture::raw::find_raw_root` in `jian-core`);
    /// migrating it onto this accessor is a follow-up.
    pub fn gestures_and_semantics(
        &self,
    ) -> (
        Option<&crate::gestures::GestureOverrides>,
        Option<&crate::semantics::SemanticsMeta>,
    ) {
        match self {
            PenNode::Frame(n) => (n.gestures.as_ref(), n.semantics.as_ref()),
            PenNode::Group(n) => (n.gestures.as_ref(), n.semantics.as_ref()),
            PenNode::Rectangle(n) => (n.gestures.as_ref(), n.semantics.as_ref()),
            PenNode::Ellipse(n) => (n.gestures.as_ref(), n.semantics.as_ref()),
            PenNode::Line(n) => (n.gestures.as_ref(), n.semantics.as_ref()),
            PenNode::Polygon(n) => (n.gestures.as_ref(), n.semantics.as_ref()),
            PenNode::Path(n) => (n.gestures.as_ref(), n.semantics.as_ref()),
            PenNode::Text(n) => (n.gestures.as_ref(), n.semantics.as_ref()),
            PenNode::TextInput(n) => (n.gestures.as_ref(), n.semantics.as_ref()),
            PenNode::Image(n) => (n.gestures.as_ref(), n.semantics.as_ref()),
            PenNode::IconFont(n) => (n.gestures.as_ref(), n.semantics.as_ref()),
            PenNode::TextArea(n) => (n.gestures.as_ref(), n.semantics.as_ref()),
            PenNode::Select(n) => (n.gestures.as_ref(), n.semantics.as_ref()),
            PenNode::Switch(n) => (n.gestures.as_ref(), n.semantics.as_ref()),
            PenNode::Checkbox(n) => (n.gestures.as_ref(), n.semantics.as_ref()),
            PenNode::Slider(n) => (n.gestures.as_ref(), n.semantics.as_ref()),
            PenNode::RadioGroup(n) => (n.gestures.as_ref(), n.semantics.as_ref()),
            PenNode::NumberInput(n) => (n.gestures.as_ref(), n.semantics.as_ref()),
            PenNode::Progress(n) => (n.gestures.as_ref(), n.semantics.as_ref()),
            PenNode::Tabs(n) => (n.gestures.as_ref(), n.semantics.as_ref()),
            PenNode::Ref(n) => (n.gestures.as_ref(), n.semantics.as_ref()),
            PenNode::Note(_) | PenNode::Context(_) | PenNode::Prompt(_) => (None, None),
        }
    }

    /// Borrow the optional `lifecycle` block from any variant, the same
    /// typed way `gestures_and_semantics` exposes gestures/semantics.
    /// Runtime lifecycle dispatch resolves `onMount` / `onUnmount`
    /// through this accessor instead of round-tripping the node schema
    /// through `serde_json::to_value`.
    pub fn lifecycle(&self) -> Option<&crate::lifecycle::NodeLifecycleHooks> {
        match self {
            PenNode::Frame(n) => n.lifecycle.as_ref(),
            PenNode::Group(n) => n.lifecycle.as_ref(),
            PenNode::Rectangle(n) => n.lifecycle.as_ref(),
            PenNode::Ellipse(n) => n.lifecycle.as_ref(),
            PenNode::Line(n) => n.lifecycle.as_ref(),
            PenNode::Polygon(n) => n.lifecycle.as_ref(),
            PenNode::Path(n) => n.lifecycle.as_ref(),
            PenNode::Text(n) => n.lifecycle.as_ref(),
            PenNode::TextInput(n) => n.lifecycle.as_ref(),
            PenNode::Image(n) => n.lifecycle.as_ref(),
            PenNode::IconFont(n) => n.lifecycle.as_ref(),
            PenNode::TextArea(n) => n.lifecycle.as_ref(),
            PenNode::Select(n) => n.lifecycle.as_ref(),
            PenNode::Switch(n) => n.lifecycle.as_ref(),
            PenNode::Checkbox(n) => n.lifecycle.as_ref(),
            PenNode::Slider(n) => n.lifecycle.as_ref(),
            PenNode::RadioGroup(n) => n.lifecycle.as_ref(),
            PenNode::NumberInput(n) => n.lifecycle.as_ref(),
            PenNode::Progress(n) => n.lifecycle.as_ref(),
            PenNode::Tabs(n) => n.lifecycle.as_ref(),
            PenNode::Ref(n) => n.lifecycle.as_ref(),
            PenNode::Note(_) | PenNode::Context(_) | PenNode::Prompt(_) => None,
        }
    }

    /// The node's JSON `"type"` tag (`"frame"`, `"text_input"`, `"note"`,
    /// …). One table for every consumer that names a kind, so a new
    /// variant is spelled once.
    pub fn type_tag(&self) -> &'static str {
        match self {
            PenNode::Frame(_) => "frame",
            PenNode::Group(_) => "group",
            PenNode::Rectangle(_) => "rectangle",
            PenNode::Ellipse(_) => "ellipse",
            PenNode::Line(_) => "line",
            PenNode::Polygon(_) => "polygon",
            PenNode::Path(_) => "path",
            PenNode::Text(_) => "text",
            PenNode::TextInput(_) => "text_input",
            PenNode::Image(_) => "image",
            PenNode::IconFont(_) => "icon_font",
            PenNode::TextArea(_) => "text_area",
            PenNode::Select(_) => "select",
            PenNode::Switch(_) => "switch",
            PenNode::Checkbox(_) => "checkbox",
            PenNode::Slider(_) => "slider",
            PenNode::RadioGroup(_) => "radio_group",
            PenNode::NumberInput(_) => "number_input",
            PenNode::Progress(_) => "progress",
            PenNode::Tabs(_) => "tabs",
            PenNode::Ref(_) => "ref",
            PenNode::Note(_) => "note",
            PenNode::Context(_) => "context",
            PenNode::Prompt(_) => "prompt",
        }
    }

    /// The annotation kind and body of a `note` / `context` / `prompt`
    /// node, `None` for every other kind.
    pub fn annotation(&self) -> Option<(AnnotationKind, &NoteNode)> {
        match self {
            PenNode::Note(n) => Some((AnnotationKind::Note, n)),
            PenNode::Context(n) => Some((AnnotationKind::Context, n)),
            PenNode::Prompt(n) => Some((AnnotationKind::Prompt, n)),
            _ => None,
        }
    }

    /// Mutable twin of [`PenNode::annotation`].
    pub fn annotation_mut(&mut self) -> Option<(AnnotationKind, &mut NoteNode)> {
        match self {
            PenNode::Note(n) => Some((AnnotationKind::Note, n)),
            PenNode::Context(n) => Some((AnnotationKind::Context, n)),
            PenNode::Prompt(n) => Some((AnnotationKind::Prompt, n)),
            _ => None,
        }
    }

    /// True for nodes that live on the editing canvas only: exports and
    /// code generation skip them (and their subtree) by default. Today
    /// these are the annotation kinds; other editor-only kinds join here.
    pub fn is_canvas_only(&self) -> bool {
        self.annotation().is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frame_roundtrip() {
        let json = r#"{"type":"frame","id":"f1","width":200,"height":100}"#;
        let n: PenNode = serde_json::from_str(json).unwrap();
        assert!(matches!(n, PenNode::Frame(_)));
    }

    #[test]
    fn nested_frame_with_children() {
        let json =
            r#"{"type":"frame","id":"root","children":[{"type":"text","id":"t","content":"hi"}]}"#;
        let n: PenNode = serde_json::from_str(json).unwrap();
        if let PenNode::Frame(f) = n {
            let children = f.children.unwrap();
            assert_eq!(children.len(), 1);
            assert!(matches!(children[0], PenNode::Text(_)));
        } else {
            panic!();
        }
    }

    #[test]
    fn ellipse_roundtrip() {
        let json = r#"{"type":"ellipse","id":"e","startAngle":0.0,"sweepAngle":90.0}"#;
        let n: PenNode = serde_json::from_str(json).unwrap();
        assert!(matches!(n, PenNode::Ellipse(_)));
    }

    #[test]
    fn path_with_anchors() {
        let json = r#"{"type":"path","id":"p","d":"M 0 0 L 10 10","anchors":[{"x":0.0,"y":0.0,"handleIn":null,"handleOut":null}]}"#;
        let n: PenNode = serde_json::from_str(json).unwrap();
        if let PenNode::Path(p) = n {
            assert_eq!(p.anchors.as_ref().unwrap().len(), 1);
        } else {
            panic!();
        }
    }

    #[test]
    fn image_with_filters() {
        let json = r#"{"type":"image","id":"i","src":"https://example.com/x.png","exposure":10.0}"#;
        let n: PenNode = serde_json::from_str(json).unwrap();
        assert!(matches!(n, PenNode::Image(_)));
    }

    #[test]
    fn text_input_roundtrip() {
        let json = r#"{"type":"text_input","id":"email","width":200,"height":40,"placeholder":"you@example.com","value":""}"#;
        let n: PenNode = serde_json::from_str(json).unwrap();
        match n {
            PenNode::TextInput(ti) => {
                assert_eq!(ti.base.id, "email");
                assert_eq!(ti.placeholder.as_deref(), Some("you@example.com"));
                assert_eq!(ti.value.as_deref(), Some(""));
            }
            other => panic!("expected TextInput, got {:?}", other),
        }
    }

    #[test]
    fn widget_nodes_deserialize_by_snake_case_tag() {
        let n: PenNode =
            serde_json::from_str(r#"{"type":"text_area","id":"a","maxVisibleLines":4}"#).unwrap();
        assert!(matches!(n, PenNode::TextArea(_)));
        let n: PenNode = serde_json::from_str(
            r#"{"type":"select","id":"s","options":[{"value":"a","label":"A"}]}"#,
        )
        .unwrap();
        let PenNode::Select(sel) = &n else { panic!() };
        assert_eq!(sel.options.as_ref().unwrap()[0].value, "a");
        let n: PenNode =
            serde_json::from_str(r#"{"type":"switch","id":"w","checked":true}"#).unwrap();
        assert!(matches!(n, PenNode::Switch(_)));
        let n: PenNode = serde_json::from_str(r#"{"type":"checkbox","id":"c"}"#).unwrap();
        assert!(matches!(n, PenNode::Checkbox(_)));
        let n: PenNode = serde_json::from_str(
            r#"{"type":"slider","id":"l","min":0,"max":100,"step":5,"value":40}"#,
        )
        .unwrap();
        assert!(matches!(n, PenNode::Slider(_)));
        let n: PenNode = serde_json::from_str(
            r#"{"type":"radio_group","id":"rg","value":"a","options":[{"value":"a","label":"A"}]}"#,
        )
        .unwrap();
        let PenNode::RadioGroup(rg) = &n else {
            panic!()
        };
        assert_eq!(rg.options.as_ref().unwrap()[0].label, "A");
        let n: PenNode = serde_json::from_str(
            r#"{"type":"number_input","id":"ni","min":0,"max":10,"step":1,"value":3}"#,
        )
        .unwrap();
        assert!(matches!(n, PenNode::NumberInput(_)));
        let n: PenNode =
            serde_json::from_str(r#"{"type":"progress","id":"pg","value":40,"max":100}"#).unwrap();
        assert!(matches!(n, PenNode::Progress(_)));
        let n: PenNode = serde_json::from_str(
            r#"{"type":"tabs","id":"tb","value":"one","tabs":[{"value":"one","label":"One"}],
                "children":[{"type":"frame","id":"panel1"}]}"#,
        )
        .unwrap();
        let PenNode::Tabs(tb) = &n else { panic!() };
        assert_eq!(tb.children.as_ref().unwrap().len(), 1);
    }

    #[test]
    fn text_input_accepts_states_and_corner_radius() {
        let n: PenNode = serde_json::from_str(
            r#"{"type":"text_input","id":"i","cornerRadius":8,"states":{"focused":{"opacity":1.0}}}"#,
        )
        .unwrap();
        let PenNode::TextInput(ti) = &n else { panic!() };
        assert!(ti.states.is_some());
    }

    #[test]
    fn annotation_kinds_roundtrip_by_type_tag() {
        for kind in AnnotationKind::ALL {
            let json = format!(
                r#"{{"type":"{}","id":"n","width":240,"height":160,"content":"Ship it","model":"m1","fontSize":16}}"#,
                kind.tag()
            );
            let n: PenNode = serde_json::from_str(&json).unwrap();
            let (got, body) = n.annotation().expect("annotation");
            assert_eq!(got, kind);
            assert_eq!(body.plain_content(), "Ship it");
            assert_eq!(body.model.as_deref(), Some("m1"));
            assert!(n.is_canvas_only());
            let back = serde_json::to_value(&n).unwrap();
            assert_eq!(back["type"], kind.tag());
            assert_eq!(n.type_tag(), kind.tag());
            assert_eq!(AnnotationKind::from_tag(kind.tag()), Some(kind));
        }
        let text: PenNode =
            serde_json::from_str(r#"{"type":"text","id":"t","content":"hi"}"#).unwrap();
        assert!(text.annotation().is_none());
        assert!(!text.is_canvas_only());
    }

    #[test]
    fn ref_node_with_descendants() {
        let json = r#"{"type":"ref","id":"r","ref":"button-primary","descendants":{"label":{"content":"OK"}}}"#;
        let n: PenNode = serde_json::from_str(json).unwrap();
        if let PenNode::Ref(r) = n {
            assert_eq!(r.target, "button-primary");
            let d = r.descendants.as_ref().unwrap();
            assert!(d.contains_key("label"));
        } else {
            panic!();
        }
    }
}
