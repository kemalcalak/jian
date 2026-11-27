//! Per-document thumbnail tables and the table painters read.
//!
//! A host that keeps several documents in flight (an editor serving MCP calls
//! on other files, a CLI running tasks in parallel, a decode worker finishing
//! after the user opened another file) gives each document its own
//! [`ImageThumbs`]: its load seeds that table, its saves snapshot it, and its
//! workers store into it, so no document ever sees another one's entries.
//!
//! Painters still look thumbnails up by paint id alone. A painter that knows
//! its document scopes its paint with [`with_thread_table`], a thread-local
//! override that concurrent renders on other threads never see. Outside such
//! a scope the free functions in the parent module (`thumb_for`,
//! `store_thumb`, `capture_snapshot`, …) use one process-wide table
//! ([`set_active`]), which keeps single-document runtimes working unchanged.

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

    /// A table of its own holding this table's entries overlaid by
    /// `newer`'s (bytes shared). For a document replaced in a way that can be
    /// undone: the old document's thumbnails must survive into the table its
    /// next save reads, whichever document that save writes.
    pub fn merged_with(&self, newer: &ImageThumbs) -> ImageThumbs {
        let mut merged = self.lock().clone();
        if !self.same_table(newer) {
            merged.extend(
                newer
                    .lock()
                    .iter()
                    .map(|(id, bytes)| (*id, Arc::clone(bytes))),
            );
        }
        Self(Arc::new(Mutex::new(merged)))
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
}

static ACTIVE: OnceLock<Mutex<ImageThumbs>> = OnceLock::new();

thread_local! {
    /// The table a [`with_thread_table`] scope set on this thread.
    static THREAD_TABLE: std::cell::RefCell<Option<ImageThumbs>> =
        const { std::cell::RefCell::new(None) };
}

fn lock_active() -> MutexGuard<'static, ImageThumbs> {
    ACTIVE
        .get_or_init(|| Mutex::new(ImageThumbs::new()))
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// Make `thumbs` the process-wide table the free functions use outside a
/// [`with_thread_table`] scope.
///
/// For single-document runtimes. A host with several documents in flight
/// scopes each paint with [`with_thread_table`] instead and never calls this,
/// because the process-wide table is shared by every thread.
pub fn set_active(thumbs: &ImageThumbs) {
    let mut active = lock_active();
    if !active.same_table(thumbs) {
        *active = thumbs.clone();
    }
}

/// The table the free functions use on this thread: the innermost
/// [`with_thread_table`] scope's, else the process-wide one.
pub fn active() -> ImageThumbs {
    THREAD_TABLE
        .with(|table| table.borrow().clone())
        .unwrap_or_else(|| lock_active().clone())
}

/// Run `f` with `thumbs` as this thread's table: `thumb_for`, `store_thumb`
/// and `capture_snapshot` inside it read and write `thumbs`, whatever any
/// other thread does meanwhile. Scopes nest; the previous table is restored
/// when `f` returns or unwinds.
pub fn with_thread_table<R>(thumbs: &ImageThumbs, f: impl FnOnce() -> R) -> R {
    struct Restore(Option<ImageThumbs>);
    impl Drop for Restore {
        fn drop(&mut self) {
            let previous = self.0.take();
            THREAD_TABLE.with(|table| *table.borrow_mut() = previous);
        }
    }
    let previous = THREAD_TABLE.with(|table| table.borrow_mut().replace(thumbs.clone()));
    let _restore = Restore(previous);
    f()
}

/// Run `f` on this thread's table without cloning its handle (paint hot
/// path: one thread-local read, then the process-wide lock only outside a
/// scope).
pub(super) fn with_active<R>(f: impl FnOnce(&ImageThumbs) -> R) -> R {
    let mut f = Some(f);
    let scoped = THREAD_TABLE.with(|table| {
        table
            .borrow()
            .as_ref()
            .map(|thumbs| (f.take().expect("called once"))(thumbs))
    });
    match scoped {
        Some(result) => result,
        None => (f.take().expect("called once"))(&lock_active()),
    }
}

/// Install a fresh process-wide table holding `loaded`. The previous table
/// is left as it was: it may belong to a document that still owns it.
pub(super) fn publish_fresh(loaded: ThumbMap) {
    *lock_active() = ImageThumbs(Arc::new(Mutex::new(loaded)));
}
