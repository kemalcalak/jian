//! The blur-up thumbnail table a scene page was built with.

use jian_ops_schema::image_thumbs::{with_thread_table, ImageThumbs};

/// The thumbnail table of the document a [`ScenePage`](crate::layout_scene::ScenePage)
/// was built from, when the builder knew it.
///
/// A renderer that is not the editor canvas (an export, a screenshot, a
/// preview capture) paints the page inside [`Self::scope`], so an image that
/// has no bytes yet draws its own document's placeholder whatever other
/// document is open or rendering on another thread. A page built without a
/// table (`None`, the default) leaves whatever table the caller scoped.
///
/// It is a paint-side lookup table, not scene content: two pages compare
/// equal whatever tables they carry, so scene comparisons keep deciding
/// re-layout and repaint by geometry alone.
#[derive(Clone, Debug, Default)]
pub struct SceneImageThumbs(pub Option<ImageThumbs>);

impl SceneImageThumbs {
    /// Run `f` with this table as the thread's thumbnail table (see
    /// [`with_thread_table`]); without a table, just run `f`.
    pub fn scope<R>(&self, f: impl FnOnce() -> R) -> R {
        match &self.0 {
            Some(thumbs) => with_thread_table(thumbs, f),
            None => f(),
        }
    }
}

impl PartialEq for SceneImageThumbs {
    fn eq(&self, _other: &Self) -> bool {
        true
    }
}

impl From<ImageThumbs> for SceneImageThumbs {
    fn from(thumbs: ImageThumbs) -> Self {
        Self(Some(thumbs))
    }
}
