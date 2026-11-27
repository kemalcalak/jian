//! Per-document thumbnail tables and the paint-active table.
//!
//! A host that keeps several documents in flight (an editor serving MCP calls
//! on other files, a CLI running tasks in parallel, a decode worker finishing
//! after the user opened another file) gives each document its own
//! [`ImageThumbs`]: its load seeds that table, its saves snapshot it, and its
//! workers store into it, so no document ever sees another one's entries.
//!
//! Painters still look thumbnails up by paint id alone, so one table is
//! *active* for paint ([`set_active`]). The free functions in the parent
//! module (`thumb_for`, `store_thumb`, `capture_snapshot`, …) act on that
//! table, which keeps single-document runtimes working unchanged.

use super::{ImageThumbSnapshot, ThumbMap, MAX_THUMB_BYTES};
use std::sync::{Arc, Mutex, MutexGuard, OnceLock};

/// One document's blur-up thumbnails, keyed by stable paint id.
///
/// Clones share one table: a request snapshot of a document, or a job a
/// worker runs for it, stores into the same table the document saves from.
/// A different document gets a table of its own through [`Self::new`],
/// [`Self::from_snapshot`] or [`Self::take_for_document`].
#[derive(Clone, Default)]
pub struct ImageThumbs(Arc<Mutex<ThumbMap>>);

impl std::fmt::Debug for ImageThumbs {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let (entries, bytes) = self.usage();
        f.debug_struct("ImageThumbs")
            .field("entries", &entries)
            .field("bytes", &bytes)
            .finish()
    }
}

impl ImageThumbs {
    /// An empty table of its own.
    pub fn new() -> Self {
        Self::default()
    }

    /// A table of its own holding `snapshot`'s entries (bytes stay shared).
    pub fn from_snapshot(snapshot: ImageThumbSnapshot) -> Self {
        Self(Arc::new(Mutex::new(snapshot.0)))
    }

    /// Take the `imageThumbs` table parsed with `doc` into a table of its
    /// own, without touching the active one.
    ///
    /// `None` when `doc` was parsed without a table (or its seed was already
    /// taken): the caller decides what absence means, typically an empty
    /// table for a new document and "keep what you have" for a document
    /// replacing another inside the same session.
    pub fn take_for_document(doc: &crate::PenDocument) -> Option<Self> {
        super::take_seed_for_document(doc).map(|seed| Self(Arc::new(Mutex::new(seed))))
    }

    fn lock(&self) -> MutexGuard<'_, ThumbMap> {
        self.0
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    /// The encoded JPEG thumbnail for `paint_id` (an `Arc` clone, no copy).
    pub fn get(&self, paint_id: u64) -> Option<Arc<[u8]>> {
        self.lock().get(&paint_id).cloned()
    }

    /// Whether a thumbnail is stored for `paint_id`.
    pub fn contains(&self, paint_id: u64) -> bool {
        self.lock().contains_key(&paint_id)
    }

    /// Store `jpeg_bytes` for `paint_id`, replacing any entry. Payloads over
    /// [`MAX_THUMB_BYTES`] are ignored; returns whether it was stored.
    pub fn store(&self, paint_id: u64, jpeg_bytes: impl Into<Arc<[u8]>>) -> bool {
        let jpeg_bytes = jpeg_bytes.into();
        if jpeg_bytes.len() > MAX_THUMB_BYTES {
            return false;
        }
        self.lock().insert(paint_id, jpeg_bytes);
        true
    }

    /// Store `jpeg_bytes` only when `paint_id` has no thumbnail yet (one
    /// lock, so two writers never replace each other's entry).
    pub fn store_if_absent(&self, paint_id: u64, jpeg_bytes: impl Into<Arc<[u8]>>) -> bool {
        let jpeg_bytes = jpeg_bytes.into();
        if jpeg_bytes.len() > MAX_THUMB_BYTES {
            return false;
        }
        let mut table = self.lock();
        if table.contains_key(&paint_id) {
            return false;
        }
        table.insert(paint_id, jpeg_bytes);
        true
    }

    /// Immutable copy of the table for a save (bytes shared through `Arc`).
    pub fn snapshot(&self) -> ImageThumbSnapshot {
        ImageThumbSnapshot(self.lock().clone())
    }

    /// Replace every entry with `snapshot`'s, in place: every clone of this
    /// table sees the change.
    pub fn replace(&self, snapshot: ImageThumbSnapshot) {
        *self.lock() = snapshot.0;
    }

    /// Entry count and encoded bytes held, for memory reports.
    pub fn usage(&self) -> (usize, usize) {
        let table = self.lock();
        (table.len(), table.values().map(|bytes| bytes.len()).sum())
    }

    /// Whether `self` and `other` are the same table (clones of one another).
    pub fn same_table(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }

    pub(super) fn replace_map(&self, loaded: ThumbMap) {
        *self.lock() = loaded;
    }

    pub(super) fn clear(&self) {
        self.lock().clear();
    }
}

static ACTIVE: OnceLock<Mutex<ImageThumbs>> = OnceLock::new();

fn lock_active() -> MutexGuard<'static, ImageThumbs> {
    ACTIVE
        .get_or_init(|| Mutex::new(ImageThumbs::new()))
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// Make `thumbs` the table painters and the free functions read and write.
///
/// A host calls this when a document becomes the one it paints (cheap when
/// it already is: a pointer compare). Loads and saves of other documents
/// never call it, so they cannot disturb the painted document's table.
pub fn set_active(thumbs: &ImageThumbs) {
    let mut active = lock_active();
    if !active.same_table(thumbs) {
        *active = thumbs.clone();
    }
}

/// The table painters currently read (a clone sharing it).
pub fn active() -> ImageThumbs {
    lock_active().clone()
}

/// Run `f` on the active table without cloning its handle (paint hot path).
pub(super) fn with_active<R>(f: impl FnOnce(&ImageThumbs) -> R) -> R {
    f(&lock_active())
}
