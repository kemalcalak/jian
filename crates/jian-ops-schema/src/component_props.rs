//! Component properties: named inputs a component master declares and its
//! instances set.
//!
//! A master frame (`reusable: true`) lists its properties in
//! `componentProperties`; each one binds to fields of nodes inside the
//! master. An instance (`type: "ref"`) sets values by name in `props`.
//! Editors turn the values into `descendants` patches before they expand the
//! instance, so a property never needs a second override path.
//!
//! ```json
//! "componentProperties": {
//!   "label":    {"type": "text", "default": "Button",
//!                "bind": [{"node": "btn-label", "field": "content"}]},
//!   "showIcon": {"type": "boolean", "default": true,
//!                "bind": [{"node": "btn-icon", "field": "visible"}]},
//!   "icon":     {"type": "instance", "default": "icon-arrow",
//!                "preferred": ["icon-arrow", "icon-plus"],
//!                "bind": [{"node": "btn-icon", "field": "ref"}]},
//!   "tone":     {"type": "color", "default": "$primary",
//!                "bind": [{"node": "btn-bg", "field": "fill"}]}
//! }
//! ```
//!
//! Unknown property types and binding fields load as `Other` and are written
//! back unchanged, so a file from a newer editor keeps them.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

/// A master's properties by name.
pub type ComponentProperties = BTreeMap<String, ComponentProperty>;

/// An instance's property values by name (`RefNode::props`).
pub type InstanceProps = BTreeMap<String, Value>;

/// One property a component master declares.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[cfg_attr(feature = "export-ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "export-ts", ts(export, export_to = "ops.ts"))]
#[serde(rename_all = "camelCase")]
pub struct ComponentProperty {
    /// `text`, `boolean`, `instance` or `color`.
    #[serde(rename = "type")]
    #[schemars(with = "String")]
    #[cfg_attr(feature = "export-ts", ts(type = "string"))]
    pub kind: ComponentPropertyType,
    /// The value an instance shows when it sets none: a string for text,
    /// a bool for boolean, a component id for instance, a colour (`#hex`
    /// or `$variable`) for color.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default: Option<Value>,
    /// The node fields the property drives. One property may drive several.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub bind: Vec<PropertyBinding>,
    /// Instance properties: the components a picker offers first.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub preferred: Option<Vec<String>>,
}

/// The type of a component property.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(from = "String", into = "String")]
pub enum ComponentPropertyType {
    /// A string, bound to a text node's `content`.
    Text,
    /// A bool, bound to a node's `visible`.
    Boolean,
    /// A component id, bound to a nested instance's `ref`.
    Instance,
    /// A colour (`#hex` or `$variable`), bound to a node's solid `fill`.
    Color,
    /// A type this version does not know, kept verbatim.
    Other(String),
}

impl ComponentPropertyType {
    /// The spelling used in files.
    pub fn as_str(&self) -> &str {
        match self {
            Self::Text => "text",
            Self::Boolean => "boolean",
            Self::Instance => "instance",
            Self::Color => "color",
            Self::Other(other) => other,
        }
    }
}

impl From<String> for ComponentPropertyType {
    fn from(raw: String) -> Self {
        match raw.as_str() {
            "text" => Self::Text,
            "boolean" => Self::Boolean,
            "instance" => Self::Instance,
            "color" => Self::Color,
            _ => Self::Other(raw),
        }
    }
}

impl From<ComponentPropertyType> for String {
    fn from(kind: ComponentPropertyType) -> Self {
        kind.as_str().to_string()
    }
}

/// One node field a property drives.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[cfg_attr(feature = "export-ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "export-ts", ts(export, export_to = "ops.ts"))]
pub struct PropertyBinding {
    /// Id of a node inside the master (the master root included).
    pub node: String,
    /// `content`, `visible`, `ref`, `fill`, or `props.<name>` to pass the
    /// value on to a property of the nested instance `node`.
    #[schemars(with = "String")]
    #[cfg_attr(feature = "export-ts", ts(type = "string"))]
    pub field: BindingField,
}

/// The node field a binding writes.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(from = "String", into = "String")]
pub enum BindingField {
    /// A text node's `content`.
    Content,
    /// A node's `visible`.
    Visible,
    /// A nested instance's `ref` (its component).
    Ref,
    /// A node's `fill`, as one solid paint.
    Fill,
    /// A property of a nested instance (`props.<name>`): the outer
    /// component exposes the nested instance's property.
    Prop(String),
    /// A field this version does not know, kept verbatim.
    Other(String),
}

/// Prefix of a [`BindingField::Prop`] spelling.
pub const PROP_FIELD_PREFIX: &str = "props.";

impl BindingField {
    /// The spelling used in files.
    pub fn to_wire(&self) -> String {
        match self {
            Self::Content => "content".into(),
            Self::Visible => "visible".into(),
            Self::Ref => "ref".into(),
            Self::Fill => "fill".into(),
            Self::Prop(name) => format!("{PROP_FIELD_PREFIX}{name}"),
            Self::Other(other) => other.clone(),
        }
    }
}

impl From<String> for BindingField {
    fn from(raw: String) -> Self {
        match raw.as_str() {
            "content" => Self::Content,
            "visible" => Self::Visible,
            "ref" => Self::Ref,
            "fill" => Self::Fill,
            _ => match raw.strip_prefix(PROP_FIELD_PREFIX) {
                Some(name) if !name.is_empty() => Self::Prop(name.to_string()),
                _ => Self::Other(raw),
            },
        }
    }
}

impl From<BindingField> for String {
    fn from(field: BindingField) -> Self {
        field.to_wire()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::node::PenNode;

    #[test]
    fn master_properties_and_instance_props_round_trip() {
        let master = r##"{"type":"frame","id":"btn","reusable":true,
            "componentProperties":{
              "label":{"type":"text","default":"Button",
                       "bind":[{"node":"btn-label","field":"content"}]},
              "icon":{"type":"instance","default":"icon-a","preferred":["icon-a","icon-b"],
                      "bind":[{"node":"btn-icon","field":"ref"}]},
              "size":{"type":"text","bind":[{"node":"btn-icon","field":"props.size"}]}
            }}"##;
        let node: PenNode = serde_json::from_str(master).unwrap();
        let PenNode::Frame(frame) = &node else {
            panic!("frame");
        };
        let props = frame.component_properties.as_ref().unwrap();
        assert_eq!(props["label"].kind, ComponentPropertyType::Text);
        assert_eq!(props["icon"].bind[0].field, BindingField::Ref);
        assert_eq!(
            props["size"].bind[0].field,
            BindingField::Prop("size".into())
        );
        let back: PenNode = serde_json::from_value(serde_json::to_value(&node).unwrap()).unwrap();
        assert_eq!(back, node);

        let instance = r#"{"type":"ref","id":"i1","ref":"btn","props":{"label":"Save"}}"#;
        let node: PenNode = serde_json::from_str(instance).unwrap();
        let PenNode::Ref(reference) = &node else {
            panic!("ref");
        };
        assert_eq!(reference.props.as_ref().unwrap()["label"], "Save");
        let json = serde_json::to_value(&node).unwrap();
        assert_eq!(json["props"]["label"], "Save");
    }

    #[test]
    fn unknown_types_and_fields_survive_a_round_trip() {
        let raw = r#"{"type":"slot","bind":[{"node":"a","field":"children"}]}"#;
        let property: ComponentProperty = serde_json::from_str(raw).unwrap();
        assert_eq!(property.kind, ComponentPropertyType::Other("slot".into()));
        assert_eq!(
            property.bind[0].field,
            BindingField::Other("children".into())
        );
        assert_eq!(
            serde_json::to_value(&property).unwrap(),
            serde_json::from_str::<Value>(raw).unwrap()
        );
    }
}
