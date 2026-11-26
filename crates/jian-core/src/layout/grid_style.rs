//! Wrap and grid auto layout as taffy styles.
//!
//! - `horizontal` / `vertical` + `wrap: true` is a wrapping flex line;
//!   `rowGap` / `columnGap` override `gap` per axis (CSS `row-gap` /
//!   `column-gap`).
//! - `layout: "grid"` is a CSS grid over the container's `columns` /
//!   `rows` tracks; rows past the explicit ones are `auto`. Alignment
//!   reads like the `layout: "none"` stack: `justifyContent` places each
//!   child horizontally in its cell, `alignItems` vertically, and a
//!   child's own `justifySelf` / `alignSelf` win.
//! - A grid child sits where its `gridColumn` / `gridRow` say, else in
//!   the next free cell. `fill_container` stretches it across its cell.

use jian_ops_schema::node::base::NumberOrExpression;
use jian_ops_schema::node::container::{
    AlignItems as OpsAlign, ContainerProps, JustifyContent as OpsJustify, LayoutMode,
};
use jian_ops_schema::node::grid::{GridPlacement, GridTrack, SelfAlign, TrackSize};
use jian_ops_schema::node::{PenNode, PenNodeBase, ScriptNode};
use jian_ops_schema::sizing::{SizingBehavior, SizingKeyword};
use taffy::prelude::*;

fn gap_px(v: Option<&NumberOrExpression>) -> LengthPercentage {
    match v {
        Some(NumberOrExpression::Number(n)) => length(*n as f32),
        _ => zero(),
    }
}

/// Per-axis gaps and wrapping on a flex container's style.
pub(crate) fn apply_flex_extras(c: &ContainerProps, style: &mut Style) {
    style.gap = Size {
        width: gap_px(c.column_gap_value()),
        height: gap_px(c.row_gap_value()),
    };
    if c.wraps() {
        style.flex_wrap = FlexWrap::Wrap;
        // Lines pack at the start, as in a browser; the default
        // `stretch` would spread them over a fixed-height frame.
        style.align_content = Some(AlignContent::FlexStart);
    }
}

fn min_track(s: TrackSize) -> MinTrackSizingFunction {
    match s {
        TrackSize::Px(v) => length(v as f32),
        // A `min` is never `fr`; a stray one reads as `auto`.
        TrackSize::Fr(_) | TrackSize::Auto => auto(),
    }
}

fn max_track(s: TrackSize) -> MaxTrackSizingFunction {
    match s {
        TrackSize::Px(v) => length(v as f32),
        TrackSize::Fr(v) => fr(v as f32),
        TrackSize::Auto => auto(),
    }
}

/// One schema track as a taffy track.
pub fn grid_track(t: &GridTrack) -> NonRepeatedTrackSizingFunction {
    match *t {
        GridTrack::Size(TrackSize::Px(v)) => length(v as f32),
        GridTrack::Size(TrackSize::Fr(v)) => fr(v as f32),
        GridTrack::Size(TrackSize::Auto) => auto(),
        GridTrack::MinMax { min, max } => minmax(min_track(min), max_track(max)),
    }
}

fn self_align(a: SelfAlign) -> AlignItems {
    match a {
        SelfAlign::Start => AlignItems::Start,
        SelfAlign::Center => AlignItems::Center,
        SelfAlign::End => AlignItems::End,
        SelfAlign::Stretch => AlignItems::Stretch,
    }
}

/// Style for a `layout: "grid"` container, built on the flex style
/// `container_to_style` made (size, padding, limits stay).
pub(crate) fn grid_container_style(c: &ContainerProps, mut style: Style) -> Style {
    style.display = Display::Grid;
    style.grid_template_columns = c
        .columns
        .as_ref()
        .map(|t| t.tracks().iter().map(|t| grid_track(t).into()).collect())
        .unwrap_or_default();
    style.grid_template_rows = c
        .rows
        .as_ref()
        .map(|t| t.tracks().iter().map(|t| grid_track(t).into()).collect())
        .unwrap_or_default();
    style.grid_auto_rows = vec![auto()];
    style.grid_auto_columns = vec![auto()];
    style.justify_content = None;
    style.align_content = None;
    style.justify_items = Some(match c.justify_content {
        Some(OpsJustify::Center) => AlignItems::Center,
        Some(OpsJustify::End) => AlignItems::End,
        _ => AlignItems::Start,
    });
    style.align_items = Some(match c.align_items {
        Some(OpsAlign::Center) => AlignItems::Center,
        Some(OpsAlign::End) => AlignItems::End,
        Some(OpsAlign::Stretch) => AlignItems::Stretch,
        _ => AlignItems::Start,
    });
    style.gap = Size {
        width: gap_px(c.column_gap_value()),
        height: gap_px(c.row_gap_value()),
    };
    style
}

fn placement(p: Option<&GridPlacement>) -> Line<taffy::style::GridPlacement> {
    let p = p.copied().unwrap_or_default();
    match p.start {
        Some(start) => Line {
            start: line(start.min(i16::MAX as u16) as i16),
            end: span(p.span),
        },
        None => Line {
            start: span(p.span),
            end: taffy::style::GridPlacement::Auto,
        },
    }
}

/// A child of a grid `parent`: back into flow unless it is an overlay or
/// constrained (like any auto-layout child), then placed in its cell.
pub(crate) fn apply_grid_parent_child(style: &mut Style, child: &PenNode, parent: &PenNode) {
    super::flow_child::apply_flow_child(style, child, parent);
    if style.position != Position::Absolute {
        apply_grid_child(style, child);
    }
}

/// Place a child of a grid container and size it within its cell.
pub(crate) fn apply_grid_child(style: &mut Style, child: &PenNode) {
    let Some(base) = super::resolve::node_base(child) else {
        return;
    };
    apply_grid_child_base(style, base, child);
}

fn apply_grid_child_base(style: &mut Style, base: &PenNodeBase, child: &PenNode) {
    style.grid_column = placement(base.grid_column.as_ref());
    style.grid_row = placement(base.grid_row.as_ref());
    // Flex hints mean nothing in a cell.
    style.flex_grow = 0.0;
    style.flex_shrink = 0.0;
    let fill = |s: Option<&SizingBehavior>| {
        matches!(
            s,
            Some(SizingBehavior::Keyword(SizingKeyword::FillContainer))
        )
    };
    let (w, h) = super::resolve::node_size_refs(child);
    if fill(w) {
        style.size.width = auto();
        style.justify_self = Some(AlignItems::Stretch);
    }
    if fill(h) {
        style.size.height = auto();
        style.align_self = Some(AlignItems::Stretch);
    }
    if let Some(a) = base.justify_self {
        style.justify_self = Some(self_align(a));
    }
    if let Some(a) = base.align_self {
        style.align_self = Some(self_align(a));
    }
}

/// True when the node is a `layout: "grid"` container.
pub fn node_is_grid_container(n: &PenNode) -> bool {
    let layout = match n {
        PenNode::Frame(f) | PenNode::Script(ScriptNode { frame: f, .. }) => {
            f.container.layout.as_ref()
        }
        PenNode::Group(g) => g.container.layout.as_ref(),
        PenNode::Rectangle(r) => r.container.layout.as_ref(),
        _ => None,
    };
    matches!(layout, Some(LayoutMode::Grid))
}

#[cfg(test)]
#[path = "grid_style_tests.rs"]
mod tests;
