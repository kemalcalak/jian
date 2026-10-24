//! Text widths before taffy rounds the layout to whole pixels.
//!
//! Taffy rounds every box edge to a pixel, so a text leaf measured and
//! wrapped at a fractional width (a `fill_container` child, say 349.6 px)
//! ends up with a box one pixel narrower or wider than the width its lines
//! were broken at. A host that wraps text again at paint time can read the
//! width the measure saw from here and break the lines in the same places.

use std::collections::HashMap;

use taffy::prelude::*;

use super::LayoutEngine;
use crate::document::NodeKey;
use crate::error::CoreResult;

impl LayoutEngine {
    /// [`LayoutEngine::compute`], also returning the unrounded laid-out
    /// width of every text leaf under `root`. The rounded layout
    /// (`node_rect`) is exactly what `compute` produces: the second pass
    /// only re-rounds taffy's cached results.
    pub fn compute_with_text_widths(
        &mut self,
        root: NodeId,
        available: (f32, f32),
    ) -> CoreResult<HashMap<NodeKey, f32>> {
        self.tree.disable_rounding();
        let unrounded = self.compute(root, available);
        let mut widths = HashMap::new();
        if unrounded.is_ok() {
            for (key, id) in self.map.iter() {
                let is_text = self
                    .tree
                    .get_node_context(*id)
                    .is_some_and(|context| context.is_some());
                if is_text {
                    if let Ok(layout) = self.tree.layout(*id) {
                        widths.insert(key, layout.size.width);
                    }
                }
            }
        }
        self.tree.enable_rounding();
        unrounded?;
        self.compute(root, available)?;
        Ok(widths)
    }
}
