//! Bounded ring-buffer pool for reusing decoded RGBA video frame allocations.

use image::RgbaImage;
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

#[derive(Debug)]
struct PoolState {
    max_buffers: usize,
    buffers: VecDeque<Vec<u8>>,
    hits: u64,
    misses: u64,
}

/// Thread-safe bounded ring-buffer pool that recycles `Vec<u8>` RGBA frame buffers
/// across decode and UI playback threads to eliminate per-frame heap allocation churn.
#[derive(Debug, Clone)]
pub struct FrameBufferPool {
    inner: Arc<Mutex<PoolState>>,
}

impl Default for FrameBufferPool {
    fn default() -> Self {
        Self::new(16)
    }
}

impl FrameBufferPool {
    /// Creates a new frame buffer pool holding at most `max_buffers` recycled allocations.
    pub fn new(max_buffers: usize) -> Self {
        Self {
            inner: Arc::new(Mutex::new(PoolState {
                max_buffers: max_buffers.max(1),
                buffers: VecDeque::with_capacity(max_buffers.max(1)),
                hits: 0,
                misses: 0,
            })),
        }
    }

    /// Checks out a buffer of exact length `byte_len`, reusing a pooled allocation if available.
    pub fn checkout(&self, byte_len: usize) -> Vec<u8> {
        if byte_len == 0 {
            return Vec::new();
        }
        if let Ok(mut state) = self.inner.lock() {
            if let Some(pos) = state.buffers.iter().position(|b| b.len() == byte_len) {
                if let Some(buf) = state.buffers.remove(pos) {
                    state.hits += 1;
                    return buf;
                }
            }
            state.misses += 1;
        }
        vec![0u8; byte_len]
    }

    /// Returns a buffer to the pool, evicting the oldest entry if at capacity.
    pub fn checkin(&self, buf: Vec<u8>) {
        if buf.is_empty() {
            return;
        }
        if let Ok(mut state) = self.inner.lock() {
            if state.buffers.len() >= state.max_buffers {
                state.buffers.pop_front();
            }
            state.buffers.push_back(buf);
        }
    }

    /// Attempts to reclaim the backing buffer from an `Arc<RgbaImage>` if this is the last owner.
    pub fn recycle_arc(&self, frame: Arc<RgbaImage>) {
        if let Ok(img) = Arc::try_unwrap(frame) {
            self.checkin(img.into_raw());
        }
    }

    /// Returns `(pooled_count, hits, misses)` for diagnostics and testing.
    pub fn stats(&self) -> (usize, u64, u64) {
        self.inner
            .lock()
            .map(|s| (s.buffers.len(), s.hits, s.misses))
            .unwrap_or((0, 0, 0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::ImageBuffer;

    #[test]
    fn test_frame_buffer_pool_checkout_recycle_and_capacity() {
        let pool = FrameBufferPool::new(2);
        let b1 = pool.checkout(1024);
        assert_eq!(b1.len(), 1024);
        assert_eq!(pool.stats(), (0, 0, 1));

        pool.checkin(b1);
        assert_eq!(pool.stats(), (1, 0, 1));

        let b2 = pool.checkout(1024);
        assert_eq!(b2.len(), 1024);
        assert_eq!(pool.stats(), (0, 1, 1));

        // Recycle via Arc<RgbaImage>
        let img = Arc::new(ImageBuffer::from_raw(16, 16, b2).expect("16x16 rgba"));
        pool.recycle_arc(img);
        assert_eq!(pool.stats(), (1, 1, 1));

        // Capacity cap = 2
        pool.checkin(vec![0u8; 1024]);
        pool.checkin(vec![0u8; 1024]);
        assert_eq!(pool.stats().0, 2);
    }
}
