//! Component sets: one component's variants, grouped by named axes.
//!
//! A component set is a frame holding its variant masters. It names its
//! axes in order, each with its values, and a default value per axis:
//!
//! ```json
//! {"type": "frame", "id": "button", "name": "Button",
//!  "componentSet": {
//!    "axes": {"variant": ["primary", "ghost"], "size": ["sm", "md"]},
//!    "default": {"variant": "primary", "size": "md"}},
//!  "children": [
//!    {"type": "frame", "id": "btn-primary-md", "reusable": true,
//!     "name": "variant=primary, size=md",
//!     "variantValues": {"variant": "primary", "size": "md"}}]}
//! ```
//!
//! Each variant master stays a `reusable` frame and records its
//! `variantValues`; an axis it leaves out matches any value. An instance
//! names the variant it shows in `RefNode::variant`.
//!
//! The order of `axes` is meaningful (it breaks ties when a requested
//! combination has no master), so it is kept as written.

use serde::de::{MapAccess, Visitor};
use serde::ser::SerializeMap;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::collections::BTreeMap;
use std::fmt;

/// Axis name → value: a variant master's `variantValues`, an instance's
/// `variant`, a set's `default`.
pub type VariantValues = BTreeMap<String, String>;

/// What makes a frame a component set.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[cfg_attr(feature = "export-ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "export-ts", ts(export, export_to = "ops.ts"))]
#[serde(rename_all = "camelCase")]
pub struct ComponentSet {
    /// Axis name → its values, in order.
    #[serde(default)]
    #[cfg_attr(
        feature = "export-ts",
        ts(type = "{ [key in string]?: Array<string> }")
    )]
    pub axes: VariantAxes,
    /// The value an instance takes on an axis it does not name. An axis
    /// missing here defaults to its first value.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub default: VariantValues,
}

impl ComponentSet {
    /// The default value of `axis`: the set's `default`, else its first
    /// value.
    pub fn default_value(&self, axis: &str) -> Option<&str> {
        self.default
            .get(axis)
            .map(String::as_str)
            .filter(|value| self.axes.has_value(axis, value))
            .or_else(|| self.axes.values(axis)?.first().map(String::as_str))
    }
}

/// Axis names with their values, in the order they were written.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct VariantAxes(pub Vec<(String, Vec<String>)>);

impl VariantAxes {
    /// The axes in order.
    pub fn iter(&self) -> impl Iterator<Item = (&str, &[String])> {
        self.0
            .iter()
            .map(|(name, values)| (name.as_str(), values.as_slice()))
    }

    /// The axis names in order.
    pub fn names(&self) -> impl Iterator<Item = &str> {
        self.0.iter().map(|(name, _)| name.as_str())
    }

    /// The values of `axis`.
    pub fn values(&self, axis: &str) -> Option<&Vec<String>> {
        self.0
            .iter()
            .find(|(name, _)| name == axis)
            .map(|(_, values)| values)
    }

    /// The values of `axis`, to edit.
    pub fn values_mut(&mut self, axis: &str) -> Option<&mut Vec<String>> {
        self.0
            .iter_mut()
            .find(|(name, _)| name == axis)
            .map(|(_, values)| values)
    }

    /// Whether `axis` lists `value`.
    pub fn has_value(&self, axis: &str, value: &str) -> bool {
        self.values(axis)
            .is_some_and(|values| values.iter().any(|v| v == value))
    }

    /// Number of axes.
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// True when the set names no axis.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl Serialize for VariantAxes {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut map = serializer.serialize_map(Some(self.0.len()))?;
        for (name, values) in &self.0 {
            map.serialize_entry(name, values)?;
        }
        map.end()
    }
}

impl<'de> Deserialize<'de> for VariantAxes {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct AxesVisitor;
        impl<'de> Visitor<'de> for AxesVisitor {
            type Value = VariantAxes;
            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("an object of axis name to a list of values")
            }
            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
                let mut axes: Vec<(String, Vec<String>)> = Vec::new();
                while let Some((name, values)) = map.next_entry::<String, Vec<String>>()? {
                    // A repeated key keeps its first place and its last values,
                    // like a JSON object read into a map.
                    match axes.iter_mut().find(|(existing, _)| *existing == name) {
                        Some(entry) => entry.1 = values,
                        None => axes.push((name, values)),
                    }
                }
                Ok(VariantAxes(axes))
            }
        }
        deserializer.deserialize_map(AxesVisitor)
    }
}

impl schemars::JsonSchema for VariantAxes {
    fn schema_name() -> String {
        "VariantAxes".to_string()
    }

    fn json_schema(gen: &mut schemars::gen::SchemaGenerator) -> schemars::schema::Schema {
        let mut schema = gen
            .subschema_for::<BTreeMap<String, Vec<String>>>()
            .into_object();
        schema.metadata().description =
            Some("Axis name → its values, in order. The order breaks fallback ties.".to_string());
        schemars::schema::Schema::Object(schema)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::node::PenNode;

    #[test]
    fn set_master_and_instance_round_trip_with_axis_order() {
        let set = r#"{"type":"frame","id":"button","name":"Button",
            "componentSet":{"axes":{"variant":["primary","ghost"],"size":["sm","md"]},
                            "default":{"size":"md"}},
            "children":[{"type":"frame","id":"m1","reusable":true,
                         "variantValues":{"variant":"primary","size":"md"}}]}"#;
        let node: PenNode = serde_json::from_str(set).unwrap();
        let PenNode::Frame(frame) = &node else {
            panic!("frame");
        };
        let component_set = frame.component_set.as_ref().unwrap();
        let names: Vec<&str> = component_set.axes.names().collect();
        assert_eq!(names, ["variant", "size"], "axis order is kept");
        assert_eq!(component_set.default_value("size"), Some("md"));
        assert_eq!(
            component_set.default_value("variant"),
            Some("primary"),
            "an axis without a default takes its first value"
        );
        let Some(PenNode::Frame(master)) = frame.children.as_ref().map(|c| &c[0]) else {
            panic!("master");
        };
        assert_eq!(master.variant_values.as_ref().unwrap()["size"], "md");
        let json = serde_json::to_string(&node).unwrap();
        assert!(
            json.find("\"variant\":[").unwrap() < json.find("\"size\":[").unwrap(),
            "axes serialize in order"
        );
        let back: PenNode = serde_json::from_str(&json).unwrap();
        assert_eq!(back, node);

        let instance = r#"{"type":"ref","id":"i","ref":"button","variant":{"size":"sm"}}"#;
        let node: PenNode = serde_json::from_str(instance).unwrap();
        let PenNode::Ref(reference) = &node else {
            panic!("ref");
        };
        assert_eq!(reference.variant.as_ref().unwrap()["size"], "sm");
        assert_eq!(
            serde_json::to_value(&node).unwrap()["variant"]["size"],
            "sm"
        );
    }

    #[test]
    fn a_default_outside_the_axis_falls_back_to_the_first_value() {
        let set: ComponentSet =
            serde_json::from_str(r#"{"axes":{"size":["sm","md"]},"default":{"size":"xl"}}"#)
                .unwrap();
        assert_eq!(set.default_value("size"), Some("sm"));
        assert_eq!(set.default_value("missing"), None);
    }
}
