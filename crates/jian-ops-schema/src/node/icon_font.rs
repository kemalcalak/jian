use super::base::PenNodeBase;
use crate::sizing::SizingBehavior;
use crate::style::{PenFill, PenStroke};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[cfg_attr(feature = "export-ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "export-ts", ts(export, export_to = "ops.ts"))]
#[serde(rename_all = "camelCase")]
pub struct IconFontNode {
    #[serde(flatten)]
    pub base: PenNodeBase,
    pub icon_font_name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon_font_family: Option<String>,
    /// Icon weight on the CSS scale (`100..=700`, 400 = regular), same type
    /// as a text node's `fontWeight` so `700`, `"700"` and `"bold"` all parse.
    /// Absent = regular. Families without weights ignore it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub weight: Option<super::FontWeight>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub width: Option<SizingBehavior>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub height: Option<SizingBehavior>,
    #[serde(flatten)]
    pub limits: crate::sizing::SizeLimits,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fill: Option<Vec<PenFill>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stroke: Option<PenStroke>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub state: Option<crate::state::StateSchema>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bindings: Option<crate::events::Bindings>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub events: Option<crate::events::EventHandlers>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lifecycle: Option<crate::lifecycle::NodeLifecycleHooks>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub semantics: Option<crate::semantics::SemanticsMeta>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gestures: Option<crate::gestures::GestureOverrides>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub route: Option<crate::navigation::NavigationRoute>,
}

#[cfg(test)]
mod tests {
    use super::IconFontNode;
    use crate::node::FontWeight;

    #[test]
    fn weight_round_trips_and_stays_absent_by_default() {
        let bold = r#"{"id":"i","iconFontName":"house","iconFontFamily":"phosphor","weight":700}"#;
        let node: IconFontNode = serde_json::from_str(bold).expect("parse weighted icon");
        assert_eq!(node.weight, Some(FontWeight::Number(700)));
        assert_eq!(serde_json::to_string(&node).unwrap(), bold);

        let keyword = r#"{"id":"i","iconFontName":"house","weight":"bold"}"#;
        let node: IconFontNode = serde_json::from_str(keyword).expect("parse keyword weight");
        assert_eq!(node.weight, Some(FontWeight::Keyword("bold".into())));

        let plain = r#"{"id":"i","iconFontName":"house"}"#;
        let node: IconFontNode = serde_json::from_str(plain).expect("parse icon");
        assert_eq!(node.weight, None);
        assert_eq!(serde_json::to_string(&node).unwrap(), plain);
    }
}
