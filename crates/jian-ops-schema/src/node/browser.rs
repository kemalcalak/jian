use super::base::PenNodeBase;
use super::container::CornerRadius;
use crate::sizing::SizingBehavior;
use crate::style::{PenEffect, PenStroke};
use serde::{Deserialize, Serialize};

/// A live web page on the canvas (pen.dev `browser`): the page at `url`,
/// laid out for the `deviceId` preset (or the node's own size), scrolled
/// to `scrollX` / `scrollY` and scaled by `zoom`. The editor paints a
/// snapshot of the page; the node is editor-side, so exports and code
/// generation leave it out.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize, schemars::JsonSchema)]
#[cfg_attr(feature = "export-ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "export-ts", ts(export, export_to = "ops.ts"))]
#[serde(rename_all = "camelCase")]
pub struct BrowserNode {
    #[serde(flatten)]
    pub base: PenNodeBase,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub width: Option<SizingBehavior>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub height: Option<SizingBehavior>,
    #[serde(flatten)]
    pub limits: crate::sizing::SizeLimits,
    /// The page address (http, https or file).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    /// The device preset the page is laid out for; unknown ids are kept.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub device_id: Option<String>,
    /// Page zoom, 1 = 100 %.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub zoom: Option<f64>,
    /// Horizontal page scroll in CSS px.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scroll_x: Option<f64>,
    /// Vertical page scroll in CSS px.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scroll_y: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub corner_radius: Option<CornerRadius>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stroke: Option<PenStroke>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub effects: Option<Vec<PenEffect>>,
}
