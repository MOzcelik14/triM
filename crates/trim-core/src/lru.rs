//! Memory- and count-bounded Least Recently Used (LRU) cache implementation.
//!
//! Provides an arena-backed doubly linked list and hash map for O(1) lookups,
//! insertions, promotions, and evictions in 100% safe Rust.

use std::collections::HashMap;
use std::hash::Hash;

#[derive(Debug)]
struct Node<K, V> {
    key: K,
    value: V,
    size_bytes: usize,
    prev: Option<usize>,
    next: Option<usize>,
}

#[derive(Debug)]
pub struct LruCache<K, V> {
    entries: Vec<Option<Node<K, V>>>,
    free_indices: Vec<usize>,
    map: HashMap<K, usize>,
    head: Option<usize>,
    tail: Option<usize>,
    max_entries: usize,
    max_bytes: usize,
    current_bytes: usize,
}

impl<K: Hash + Eq + Clone, V> LruCache<K, V> {
    /// Creates a new LRU cache bounded by maximum entries and byte footprint.
    pub fn new(max_entries: usize, max_bytes: usize) -> Self {
        Self {
            entries: Vec::new(),
            free_indices: Vec::new(),
            map: HashMap::new(),
            head: None,
            tail: None,
            max_entries: max_entries.max(1),
            max_bytes: max_bytes.max(1),
            current_bytes: 0,
        }
    }

    /// Creates an LRU cache bounded solely by entry count.
    pub fn with_capacity(max_entries: usize) -> Self {
        Self::new(max_entries, usize::MAX)
    }

    /// Creates an LRU cache bounded solely by maximum memory footprint in bytes.
    pub fn with_byte_limit(max_bytes: usize) -> Self {
        Self::new(usize::MAX, max_bytes)
    }

    pub fn len(&self) -> usize {
        self.map.len()
    }

    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }

    pub fn total_bytes(&self) -> usize {
        self.current_bytes
    }

    pub fn max_bytes(&self) -> usize {
        self.max_bytes
    }

    pub fn max_entries(&self) -> usize {
        self.max_entries
    }

    pub fn contains_key(&self, key: &K) -> bool {
        self.map.contains_key(key)
    }

    /// Retrieves an immutable reference without altering LRU order.
    pub fn peek(&self, key: &K) -> Option<&V> {
        let &idx = self.map.get(key)?;
        self.entries[idx].as_ref().map(|n| &n.value)
    }

    /// Retrieves a mutable reference without altering LRU order.
    pub fn peek_mut(&mut self, key: &K) -> Option<&mut V> {
        let &idx = self.map.get(key)?;
        self.entries[idx].as_mut().map(|n| &mut n.value)
    }

    /// Retrieves an immutable reference and promotes the item to the front (most recently used).
    pub fn get(&mut self, key: &K) -> Option<&V> {
        let &idx = self.map.get(key)?;
        self.move_to_head(idx);
        self.entries[idx].as_ref().map(|n| &n.value)
    }

    /// Retrieves a mutable reference and promotes the item to the front (most recently used).
    pub fn get_mut(&mut self, key: &K) -> Option<&mut V> {
        let &idx = self.map.get(key)?;
        self.move_to_head(idx);
        self.entries[idx].as_mut().map(|n| &mut n.value)
    }

    /// Inserts a key-value pair with default unit size (1 byte/unit).
    pub fn insert(&mut self, key: K, value: V) -> Option<V> {
        self.insert_with_size(key, value, 1)
    }

    /// Inserts a key-value pair with a specific byte footprint.
    /// Evicts least recently used items if capacity or byte limit is exceeded.
    pub fn insert_with_size(&mut self, key: K, value: V, size_bytes: usize) -> Option<V> {
        if let Some(&idx) = self.map.get(&key) {
            let old_size = self.entries[idx]
                .as_ref()
                .map(|n| n.size_bytes)
                .unwrap_or(0);
            self.current_bytes = self.current_bytes.saturating_sub(old_size);
            self.current_bytes = self.current_bytes.saturating_add(size_bytes);

            self.move_to_head(idx);
            let old_node = self.entries[idx].as_mut().expect("Entry exists");
            let old_val = std::mem::replace(&mut old_node.value, value);
            old_node.size_bytes = size_bytes;

            self.evict_if_needed();
            return Some(old_val);
        }

        // Allocate slot
        let idx = if let Some(free_idx) = self.free_indices.pop() {
            self.entries[free_idx] = Some(Node {
                key: key.clone(),
                value,
                size_bytes,
                prev: None,
                next: self.head,
            });
            free_idx
        } else {
            let next_idx = self.entries.len();
            self.entries.push(Some(Node {
                key: key.clone(),
                value,
                size_bytes,
                prev: None,
                next: self.head,
            }));
            next_idx
        };

        if let Some(old_head) = self.head {
            if let Some(ref mut node) = self.entries[old_head] {
                node.prev = Some(idx);
            }
        }
        self.head = Some(idx);
        if self.tail.is_none() {
            self.tail = Some(idx);
        }

        self.map.insert(key, idx);
        self.current_bytes = self.current_bytes.saturating_add(size_bytes);

        self.evict_if_needed();
        None
    }

    /// Removes an entry by key and returns its value if present.
    pub fn remove(&mut self, key: &K) -> Option<V> {
        let idx = self.map.remove(key)?;
        self.detach(idx);
        let node = self.entries[idx].take()?;
        self.free_indices.push(idx);
        self.current_bytes = self.current_bytes.saturating_sub(node.size_bytes);
        Some(node.value)
    }

    /// Clears all entries from the cache.
    pub fn clear(&mut self) {
        self.entries.clear();
        self.free_indices.clear();
        self.map.clear();
        self.head = None;
        self.tail = None;
        self.current_bytes = 0;
    }

    fn move_to_head(&mut self, idx: usize) {
        if self.head == Some(idx) {
            return;
        }
        self.detach(idx);

        if let Some(ref mut node) = self.entries[idx] {
            node.prev = None;
            node.next = self.head;
        }

        if let Some(old_head) = self.head {
            if let Some(ref mut node) = self.entries[old_head] {
                node.prev = Some(idx);
            }
        }
        self.head = Some(idx);
        if self.tail.is_none() {
            self.tail = Some(idx);
        }
    }

    fn detach(&mut self, idx: usize) {
        let (prev, next) = if let Some(ref node) = self.entries[idx] {
            (node.prev, node.next)
        } else {
            return;
        };

        if let Some(p) = prev {
            if let Some(ref mut p_node) = self.entries[p] {
                p_node.next = next;
            }
        } else {
            self.head = next;
        }

        if let Some(n) = next {
            if let Some(ref mut n_node) = self.entries[n] {
                n_node.prev = prev;
            }
        } else {
            self.tail = prev;
        }
    }

    fn evict_if_needed(&mut self) {
        while self.map.len() > self.max_entries
            || (self.current_bytes > self.max_bytes && self.map.len() > 1)
        {
            let tail_idx = match self.tail {
                Some(idx) => idx,
                None => break,
            };

            let tail_key = match self.entries[tail_idx] {
                Some(ref node) => node.key.clone(),
                None => break,
            };

            self.remove(&tail_key);
        }
    }
}

impl<K: Hash + Eq + Clone, V> Default for LruCache<K, V> {
    fn default() -> Self {
        Self::new(100, usize::MAX)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lru_capacity_eviction() {
        let mut cache = LruCache::with_capacity(2);
        cache.insert("a", 1);
        cache.insert("b", 2);
        assert_eq!(cache.len(), 2);

        // Access "a" to make it most recent
        assert_eq!(cache.get(&"a"), Some(&1));

        // Insert "c", should evict "b"
        cache.insert("c", 3);
        assert_eq!(cache.len(), 2);
        assert!(cache.contains_key(&"a"));
        assert!(cache.contains_key(&"c"));
        assert!(!cache.contains_key(&"b"));
    }

    #[test]
    fn test_lru_byte_limit_eviction() {
        let mut cache = LruCache::with_byte_limit(100);
        cache.insert_with_size("item1", vec![0u8; 40], 40);
        cache.insert_with_size("item2", vec![0u8; 40], 40);
        assert_eq!(cache.total_bytes(), 80);

        // Insert item3 with 50 bytes; 80 + 50 = 130 > 100.
        // item1 was LRU, so it should be evicted.
        cache.insert_with_size("item3", vec![0u8; 50], 50);
        assert!(!cache.contains_key(&"item1"));
        assert!(cache.contains_key(&"item2"));
        assert!(cache.contains_key(&"item3"));
        assert_eq!(cache.total_bytes(), 90);
    }

    #[test]
    fn test_lru_update_and_remove() {
        let mut cache = LruCache::new(5, 500);
        cache.insert_with_size("k1", 10, 50);
        assert_eq!(cache.total_bytes(), 50);

        // Update value with larger size
        cache.insert_with_size("k1", 20, 80);
        assert_eq!(cache.len(), 1);
        assert_eq!(cache.total_bytes(), 80);
        assert_eq!(cache.get(&"k1"), Some(&20));

        // Remove
        let removed = cache.remove(&"k1");
        assert_eq!(removed, Some(20));
        assert_eq!(cache.len(), 0);
        assert_eq!(cache.total_bytes(), 0);
    }
}
