use super::frame::GroupNode;
use serde::{Deserialize, Serialize};

/// A non-destructive boolean group (Figma's boolean operation). Its
/// children stay ordinary, editable nodes; what paints, hit-tests and
/// exports is the result of `op` applied to their outlines, filled and
/// stroked with the group's own `fill` / `stroke`.
///
/// The group body is embedded as is, so everything that walks a group's
/// children walks these too.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[cfg_attr(feature = "export-ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "export-ts", ts(export, export_to = "ops.ts"))]
#[serde(rename_all = "camelCase")]
pub struct BooleanNode {
    #[serde(flatten)]
    pub group: GroupNode,
    /// How the children's outlines combine, first child first.
    #[serde(default)]
    pub op: BooleanOperation,
}

/// The four path booleans. `subtract` removes every later child from the
/// first; `exclude` keeps the regions covered an odd number of times.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize, schemars::JsonSchema,
)]
#[cfg_attr(feature = "export-ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "export-ts", ts(export, export_to = "ops.ts"))]
#[serde(rename_all = "snake_case")]
pub enum BooleanOperation {
    #[default]
    Union,
    Subtract,
    Intersect,
    Exclude,
}

impl BooleanOperation {
    /// Every operation, in menu order.
    pub const ALL: [BooleanOperation; 4] = [
        BooleanOperation::Union,
        BooleanOperation::Subtract,
        BooleanOperation::Intersect,
        BooleanOperation::Exclude,
    ];

    /// The JSON spelling (`"union"`, `"subtract"`, …).
    pub fn as_str(self) -> &'static str {
        match self {
            BooleanOperation::Union => "union",
            BooleanOperation::Subtract => "subtract",
            BooleanOperation::Intersect => "intersect",
            BooleanOperation::Exclude => "exclude",
        }
    }

    /// Parse the JSON spelling (case-insensitive).
    pub fn parse(s: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|op| op.as_str().eq_ignore_ascii_case(s.trim()))
    }
}

#[cfg(test)]
mod tests {
    use super::super::PenNode;
    use super::*;

    #[test]
    fn boolean_roundtrips_with_children_and_op() {
        let json = r#"{"type":"boolean","id":"b","op":"subtract","children":[{"type":"rectangle","id":"r","width":10,"height":10},{"type":"ellipse","id":"e","width":5,"height":5}]}"#;
        let n: PenNode = serde_json::from_str(json).unwrap();
        let PenNode::Boolean(b) = &n else {
            panic!("expected boolean, got {n:?}")
        };
        assert_eq!(b.op, BooleanOperation::Subtract);
        assert_eq!(n.children_ref().map(<[PenNode]>::len), Some(2));
        assert_eq!(n.type_tag(), "boolean");
        let back = serde_json::to_value(&n).unwrap();
        assert_eq!(back["type"], "boolean");
        assert_eq!(back["op"], "subtract");
        let again: PenNode = serde_json::from_value(back).unwrap();
        assert_eq!(again, n);
    }

    #[test]
    fn missing_op_defaults_to_union() {
        let n: PenNode = serde_json::from_str(r#"{"type":"boolean","id":"b"}"#).unwrap();
        let PenNode::Boolean(b) = n else { panic!() };
        assert_eq!(b.op, BooleanOperation::Union);
    }

    #[test]
    fn parse_matches_as_str() {
        for op in BooleanOperation::ALL {
            assert_eq!(BooleanOperation::parse(op.as_str()), Some(op));
        }
        assert_eq!(BooleanOperation::parse("XOR"), None);
    }
}
