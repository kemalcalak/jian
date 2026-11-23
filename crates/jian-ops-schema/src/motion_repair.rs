//! Load-time repair of node motion declarations.
//!
//! One bad animation must not make a whole document unopenable, so the
//! loader repairs instead of refusing: the `scale` alias expands to
//! `scaleX` + `scaleY`, an unknown property is dropped, and an animation
//! whose keyframes are still invalid (offsets) is dropped. Each drop is
//! reported as a [`LoadWarning`] naming the node, so a host can tell the
//! user before a save makes the loss permanent; the lossless alias
//! expansion is not reported.

use crate::error::LoadWarning;
use crate::motion::{
    expand_keyframe_aliases, node_path, validate_keyframes, Keyframe, ANIMATION_REGISTRY_PROPERTIES,
};
use crate::node::PenNode;
use crate::PenDocument;

/// Repair every node animation in `document`, top-level children and
/// pages alike.
pub fn repair_document(document: &mut PenDocument) -> Vec<LoadWarning> {
    let mut warnings = Vec::new();
    repair_nodes(&mut document.children, "$.children", &mut warnings);
    if let Some(pages) = &mut document.pages {
        for (index, page) in pages.iter_mut().enumerate() {
            repair_nodes(
                &mut page.children,
                &format!("$.pages[{index}].children"),
                &mut warnings,
            );
        }
    }
    warnings
}

/// Repair the animations of `nodes` and their descendants. `path` is the
/// JSON path of the slice, used in the warnings.
pub fn repair_nodes(nodes: &mut [PenNode], path: &str, warnings: &mut Vec<LoadWarning>) {
    repair_nodes_at(nodes, path, &mut Vec::new(), warnings);
}

fn repair_nodes_at(
    nodes: &mut [PenNode],
    root: &str,
    trail: &mut Vec<usize>,
    warnings: &mut Vec<LoadWarning>,
) {
    for (index, node) in nodes.iter_mut().enumerate() {
        trail.push(index);
        if node.animations_mut().is_some() {
            let node_path = node_path(root, trail);
            repair_node_animations(node, &node_path, warnings);
        }
        if let Some(children) = crate::promote::children_mut(node) {
            repair_nodes_at(children, root, trail, warnings);
        }
        trail.pop();
    }
}

fn repair_node_animations(node: &mut PenNode, node_path: &str, warnings: &mut Vec<LoadWarning>) {
    let node_id = node.shared_base_mut().id.clone();
    let slot = node.animations_mut();
    let Some(animations) = slot.as_mut() else {
        return;
    };
    let before = animations.len();
    let mut index = 0;
    animations.retain_mut(|animation| {
        let keyframes_path = format!("{node_path}.animations[{index}].keyframes");
        index += 1;
        expand_keyframe_aliases(&mut animation.keyframes);
        drop_unknown_properties(
            &mut animation.keyframes,
            &keyframes_path,
            &node_id,
            warnings,
        );
        match validate_keyframes(&animation.keyframes, &keyframes_path) {
            Ok(()) => true,
            Err((path, reason)) => {
                warnings.push(LoadWarning::InvalidAnimationDropped {
                    path,
                    node: node_id.clone(),
                    reason,
                });
                false
            }
        }
    });
    // Only clear a list this repair emptied; an authored `[]` round-trips.
    if animations.is_empty() && before > 0 {
        *slot = None;
    }
}

fn drop_unknown_properties(
    keyframes: &mut [Keyframe],
    keyframes_path: &str,
    node_id: &str,
    warnings: &mut Vec<LoadWarning>,
) {
    for (index, keyframe) in keyframes.iter_mut().enumerate() {
        keyframe.values.retain(|property, _| {
            let known = ANIMATION_REGISTRY_PROPERTIES.contains(&property.as_str());
            if !known {
                warnings.push(LoadWarning::AnimationPropertyDropped {
                    path: format!("{keyframes_path}[{index}].values.{property}"),
                    node: node_id.to_owned(),
                    property: property.clone(),
                });
            }
            known
        });
    }
}
