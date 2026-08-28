// SPDX-License-Identifier: AGPL-3.0

//! Byte-budgeted LRU caches of page bitmaps.
//!
//! `ImageCache<CacheKey>` stores original (unrecolorized) bitmaps keyed by
//! `(page, scale_q)` where `scale_q` is the render scale in hundredths of a
//! pixel per point; quantized zoom keeps the key space small.
//! `ImageCache<RecolorKey>` stores recolorized bitmaps keyed additionally by
//! the recolor colors, so theme switches reuse them instead of re-mapping
//! every page. Both budgets count RGBA bytes (`width * height * 4`).

use std::collections::{HashMap, VecDeque};
use std::hash::Hash;

/// Default budget in bytes: 192 MB.
pub const DEFAULT_BUDGET_BYTES: usize = 192 * 1024 * 1024;

/// Budget of the recolored-bitmap cache: 96 MB, enough for the visible
/// window and prefetch at typical zooms. Recolored entries are pure
/// accelerators — a miss just re-maps from the original cache.
pub const RECOLORED_BUDGET_BYTES: usize = 96 * 1024 * 1024;

/// Bitmap cache key: page index plus quantized render scale.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct CacheKey {
    pub page: usize,
    pub scale_q: u16,
}

/// Recolorized-bitmap cache key: [`CacheKey`] plus the page colors the
/// recolor pass mapped onto, so a theme switch (or theme edit) addresses a
/// different entry and unrelated themes share nothing.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct RecolorKey {
    pub page: usize,
    pub scale_q: u16,
    pub theme: u64,
}

/// Borrowed view of a cached bitmap.
#[derive(Clone, Copy)]
pub struct CachedImage<'a> {
    pub width: u32,
    pub height: u32,
    pub rgba: &'a [u8],
}

struct Entry {
    width: u32,
    height: u32,
    rgba: Vec<u8>,
    bytes: usize,
}

/// LRU cache with byte accounting. Recency updates and evictions are O(n) in
/// entry count (hundreds at most), which beats carrying a linked list.
pub struct ImageCache<K = CacheKey> {
    entries: HashMap<K, Entry>,
    order: VecDeque<K>,
    bytes: usize,
    budget: usize,
}

impl<K: Copy + Eq + Hash> ImageCache<K> {
    pub fn new(budget: usize) -> Self {
        Self {
            entries: HashMap::new(),
            order: VecDeque::new(),
            bytes: 0,
            budget,
        }
    }

    pub fn contains(&self, key: K) -> bool {
        self.entries.contains_key(&key)
    }

    /// Look up a bitmap and mark it most-recently-used.
    pub fn get(&mut self, key: K) -> Option<CachedImage<'_>> {
        let entry = self.entries.get(&key)?;
        if self.order.back() != Some(&key)
            && let Some(pos) = self.order.iter().position(|k| *k == key)
        {
            self.order.remove(pos);
            self.order.push_back(key);
        }
        let view = CachedImage {
            width: entry.width,
            height: entry.height,
            rgba: &entry.rgba,
        };
        Some(view)
    }

    /// Insert a bitmap, evicting least-recently-used entries until it fits.
    /// Returns `false` when the bitmap alone exceeds the budget; nothing is
    /// stored then. Replacing an existing entry never evicts anything else.
    pub fn insert(&mut self, key: K, width: u32, height: u32, rgba: Vec<u8>) -> bool {
        let bytes = rgba.len();
        if bytes > self.budget {
            return false;
        }
        if let Some(old) = self.entries.remove(&key) {
            self.bytes -= old.bytes;
            if let Some(pos) = self.order.iter().position(|k| *k == key) {
                self.order.remove(pos);
            }
        }
        while self.bytes + bytes > self.budget {
            let Some(evicted) = self.order.pop_front() else {
                break;
            };
            if let Some(entry) = self.entries.remove(&evicted) {
                self.bytes -= entry.bytes;
            }
        }
        self.order.push_back(key);
        self.entries.insert(
            key,
            Entry {
                width,
                height,
                rgba,
                bytes,
            },
        );
        self.bytes += bytes;
        true
    }

    /// Total cached RGBA bytes.
    #[cfg(test)]
    pub fn byte_len(&self) -> usize {
        self.bytes
    }

    #[cfg(test)]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    #[cfg(test)]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(page: usize) -> CacheKey {
        CacheKey { page, scale_q: 100 }
    }

    fn bitmap(bytes: usize) -> Vec<u8> {
        vec![7u8; bytes]
    }

    #[test]
    fn miss_then_hit() {
        let mut cache = ImageCache::new(1000);
        assert!(!cache.contains(key(0)));
        assert!(cache.get(key(0)).is_none());
        assert!(cache.insert(key(0), 5, 5, bitmap(100)));
        let img = cache.get(key(0)).unwrap();
        assert_eq!((img.width, img.height), (5, 5));
        assert_eq!(img.rgba, &[7; 100][..]);
        assert_eq!(cache.byte_len(), 100);
    }

    #[test]
    fn eviction_keeps_bytes_under_budget() {
        // Budget fits two 100-byte pages.
        let mut cache = ImageCache::new(200);
        assert!(cache.insert(key(0), 1, 1, bitmap(100)));
        assert!(cache.insert(key(1), 1, 1, bitmap(100)));
        assert_eq!(cache.len(), 2);
        assert!(cache.insert(key(2), 1, 1, bitmap(100)));
        assert_eq!(cache.len(), 2);
        assert_eq!(cache.byte_len(), 200);
        assert!(!cache.contains(key(0)), "oldest entry evicted first");
        assert!(cache.contains(key(1)));
        assert!(cache.contains(key(2)));
    }

    #[test]
    fn get_refreshes_recency() {
        let mut cache = ImageCache::new(200);
        cache.insert(key(0), 1, 1, bitmap(100));
        cache.insert(key(1), 1, 1, bitmap(100));
        // Touch page 0 so page 1 becomes the least recently used.
        cache.get(key(0)).unwrap();
        cache.insert(key(2), 1, 1, bitmap(100));
        assert!(cache.contains(key(0)));
        assert!(
            !cache.contains(key(1)),
            "untouched entry evicted despite age"
        );
    }

    #[test]
    fn oversized_bitmaps_are_refused_without_disturbing_the_cache() {
        let mut cache = ImageCache::new(150);
        cache.insert(key(0), 1, 1, bitmap(100));
        assert!(!cache.insert(key(1), 1, 1, bitmap(151)));
        assert_eq!(cache.byte_len(), 100);
        assert_eq!(cache.len(), 1);
        assert!(cache.contains(key(0)));
    }

    #[test]
    fn replacing_an_entry_does_not_double_count_or_evict() {
        let mut cache = ImageCache::new(250);
        cache.insert(key(0), 1, 1, bitmap(100));
        cache.insert(key(1), 1, 1, bitmap(100));
        assert!(cache.insert(key(0), 2, 2, bitmap(100)));
        assert_eq!(cache.byte_len(), 200);
        assert_eq!(cache.len(), 2);
        assert!(cache.contains(key(1)), "replacement must not evict others");
        // Replacement counts as a fresh insertion (most recent).
        cache.insert(key(2), 1, 1, bitmap(100));
        assert!(!cache.contains(key(1)));
        assert!(cache.contains(key(0)));
    }

    #[test]
    fn zero_budget_caches_nothing() {
        let mut cache = ImageCache::new(0);
        assert!(!cache.insert(key(0), 1, 1, bitmap(4)));
        assert!(cache.is_empty());
    }

    #[test]
    fn recolor_keys_stay_distinct_and_evict_by_recency() {
        let base = RecolorKey {
            page: 0,
            scale_q: 100,
            theme: 1,
        };
        let other_theme = RecolorKey { theme: 2, ..base };
        let mut cache = ImageCache::<RecolorKey>::new(200);
        assert!(cache.insert(base, 1, 1, bitmap(100)));
        assert!(cache.insert(other_theme, 1, 1, bitmap(100)));
        cache.insert(
            RecolorKey {
                page: 1,
                theme: 1,
                ..base
            },
            1,
            1,
            bitmap(100),
        );
        assert!(!cache.contains(base), "oldest theme entry evicted first");
        assert!(cache.contains(other_theme));
    }
}
