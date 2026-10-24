use super::base::PenNodeBase;
use super::text::{FontStyleKind, FontWeight, TextAlign, TextAlignVertical, TextContent};
use crate::sizing::SizingBehavior;
use serde::{Deserialize, Serialize};

/// Body of the canvas annotation kinds: `note` (a sticky note),
/// `context` (background the AI reads) and `prompt` (an instruction for
/// a model). They share one shape and differ only in the `type` tag, so
/// `PenNode::Note`, `PenNode::Context` and
/// `PenNode::Prompt` all carry this struct. Annotations are
/// editor-side: exports and code generation leave them out.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize, schemars::JsonSchema)]
#[cfg_attr(feature = "export-ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "export-ts", ts(export, export_to = "ops.ts"))]
#[serde(rename_all = "camelCase")]
pub struct NoteNode {
    #[serde(flatten)]
    pub base: PenNodeBase,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub width: Option<SizingBehavior>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub height: Option<SizingBehavior>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content: Option<TextContent>,
    /// The model a `prompt` is meant for. Kept on the other kinds too,
    /// so switching the kind back and forth loses nothing.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub font_family: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub font_size: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub font_weight: Option<FontWeight>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub font_style: Option<FontStyleKind>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub letter_spacing: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub line_height: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text_align: Option<TextAlign>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text_align_vertical: Option<TextAlignVertical>,
}

impl NoteNode {
    /// The content as plain text: styled segments are joined.
    pub fn plain_content(&self) -> String {
        match &self.content {
            None => String::new(),
            Some(TextContent::Plain(text)) => text.clone(),
            Some(TextContent::Styled(segments)) => {
                segments.iter().map(|s| s.text.as_str()).collect()
            }
        }
    }
}

/// Which annotation a [`NoteNode`] is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AnnotationKind {
    Note,
    Context,
    Prompt,
}

impl AnnotationKind {
    /// Every kind, in menu order.
    pub const ALL: [AnnotationKind; 3] = [
        AnnotationKind::Note,
        AnnotationKind::Context,
        AnnotationKind::Prompt,
    ];

    /// The node's JSON `"type"` tag.
    pub fn tag(self) -> &'static str {
        match self {
            AnnotationKind::Note => "note",
            AnnotationKind::Context => "context",
            AnnotationKind::Prompt => "prompt",
        }
    }

    /// The kind a `"type"` tag names.
    pub fn from_tag(tag: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|kind| kind.tag() == tag)
    }

    /// Wrap a body into the matching [`super::PenNode`] variant.
    pub fn into_node(self, body: NoteNode) -> super::PenNode {
        match self {
            AnnotationKind::Note => super::PenNode::Note(body),
            AnnotationKind::Context => super::PenNode::Context(body),
            AnnotationKind::Prompt => super::PenNode::Prompt(body),
        }
    }
}
