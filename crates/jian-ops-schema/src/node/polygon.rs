use super::base::PenNodeBase;
use crate::sizing::SizingBehavior;
use crate::style::{PenEffect, PenFill, PenStroke};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[cfg_attr(feature = "export-ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "export-ts", ts(export, export_to = "ops.ts"))]
#[serde(rename_all = "camelCase")]
pub struct PolygonNode {
    #[serde(flatten)]
    pub base: PenNodeBase,
    pub polygon_count: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub width: Option<SizingBehavior>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub height: Option<SizingBehavior>,
    #[serde(flatten)]
    pub limits: crate::sizing::SizeLimits,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub corner_radius: Option<f64>,
    /// Star inner radius as a fraction of the outer radius (`0.0..=1.0`).
    /// Absent = a regular polygon; present = a star with `polygon_count`
    /// points whose valleys sit at this fraction (Figma `STAR` /
    /// `starInnerScale`). Mirrors `EllipseNode::inner_radius`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub inner_radius: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fill: Option<Vec<PenFill>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stroke: Option<PenStroke>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub effects: Option<Vec<PenEffect>>,
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
    use super::PolygonNode;

    #[test]
    fn inner_radius_round_trips_and_stays_absent_by_default() {
        let star = r#"{"id":"s","polygonCount":5,"innerRadius":0.382}"#;
        let node: PolygonNode = serde_json::from_str(star).expect("parse star");
        assert_eq!(node.inner_radius, Some(0.382));
        assert_eq!(serde_json::to_string(&node).unwrap(), star);

        let plain = r#"{"id":"p","polygonCount":6}"#;
        let node: PolygonNode = serde_json::from_str(plain).expect("parse polygon");
        assert_eq!(node.inner_radius, None);
        assert_eq!(serde_json::to_string(&node).unwrap(), plain);
    }
}
