use super::frame::FrameNode;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// A live script node (pen.dev `script`). It is a frame whose children a
/// JavaScript program generates from typed inputs and the node's size.
///
/// The frame body is embedded as is, so a script lays out, paints, clips
/// and exports like a frame holding its generated children. Those
/// children are saved with the document: readers without a script engine
/// show the last output, and `script_hash` tells an engine whether that
/// output is still current.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[cfg_attr(feature = "export-ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "export-ts", ts(export, export_to = "ops.ts"))]
#[serde(rename_all = "camelCase")]
pub struct ScriptNode {
    #[serde(flatten)]
    pub frame: FrameNode,
    /// The program's source. It declares its inputs with `@input`
    /// annotations and returns an array of nodes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub script: Option<String>,
    /// Where the source came from (pen.dev's `scriptUri`). Kept for
    /// provenance only; `script` is what runs.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub script_uri: Option<String>,
    /// Input values by name. A string starting with `$` binds the input
    /// to a design variable.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub inputs: Option<BTreeMap<String, ScriptInputValue>>,
    /// Fingerprint of the source, resolved inputs and size the saved
    /// children were generated from.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub script_hash: Option<String>,
}

/// One script input value. Colors, node refs and enum choices are strings.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[cfg_attr(feature = "export-ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "export-ts", ts(export, export_to = "ops.ts"))]
#[serde(untagged)]
pub enum ScriptInputValue {
    Bool(bool),
    Number(f64),
    Text(String),
}

impl ScriptInputValue {
    /// The variable name when this value is a `$variable` binding.
    pub fn variable_ref(&self) -> Option<&str> {
        match self {
            ScriptInputValue::Text(text) => text.strip_prefix('$'),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::PenNode;
    use super::*;

    #[test]
    fn script_roundtrips_with_frame_fields_and_inputs() {
        let json = r##"{"type":"script","id":"s","width":200,"height":100,"clipContent":true,
            "script":"/** @input count: number = 3 */ return [];",
            "scriptUri":"charts/bar.js",
            "inputs":{"count":4,"label":"Sales","accent":"$brand","on":true},
            "scriptHash":"abc",
            "children":[{"type":"rectangle","id":"s-g0","width":10,"height":10}]}"##;
        let node: PenNode = serde_json::from_str(json).unwrap();
        let PenNode::Script(script) = &node else {
            panic!("expected a script node, got {node:?}");
        };
        assert_eq!(script.frame.base.id, "s");
        assert_eq!(script.frame.container.clip_content, Some(true));
        assert_eq!(script.frame.children.as_ref().map(Vec::len), Some(1));
        assert_eq!(script.script_uri.as_deref(), Some("charts/bar.js"));
        let inputs = script.inputs.as_ref().unwrap();
        assert_eq!(inputs["count"], ScriptInputValue::Number(4.0));
        assert_eq!(inputs["on"], ScriptInputValue::Bool(true));
        assert_eq!(inputs["accent"].variable_ref(), Some("brand"));
        assert_eq!(inputs["label"].variable_ref(), None);
        let back = serde_json::to_value(&node).unwrap();
        assert_eq!(back["type"], "script");
        assert_eq!(back["scriptHash"], "abc");
        let again: PenNode = serde_json::from_value(back).unwrap();
        assert_eq!(again, node);
    }
}
