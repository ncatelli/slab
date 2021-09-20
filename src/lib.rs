#![no_std]

extern crate alloc;
use alloc::alloc::{GlobalAlloc, Layout};

#[derive(Debug)]
pub struct Chunk<T> {
    free_list: usize,
    inner: [Option<T>; usize::BITS as usize],
}

pub struct SlabAllocator<T> {
    start: *mut [Chunk<T>],
    len: usize,
}

impl<T> SlabAllocator<T> {
    /// Represents the maximum number of chunks allowed in the allocator. This
    /// is equivalent to the number of bits of the pointer type.
    const CHUNK_MAX: u8 = (usize::BITS as u8 - 1);

    /// Initializes a new empty `SlabAllocator<T>`.
    pub fn new() -> Self {
        Self::default()
    }

    /// Initializes a new slab allocator
    ///
    /// # Safety
    /// Caller guarantees that this method is only called once, and that the
    /// allocator has been mapped to a region of memory **atleast** the size
    /// of the `SlabAllocator<T>` and its constituent chunks.
    pub unsafe fn init(&mut self, chunks: u8) {
        use core::mem;

        let start_of_chunks =
            (((self as *mut Self) as usize) + mem::size_of::<Self>()) as *mut Chunk<T>;
        let chunk_cnt = (chunks & Self::CHUNK_MAX) as usize;
        let chunks = core::ptr::slice_from_raw_parts_mut(start_of_chunks, chunk_cnt);

        self.start = chunks;
        self.len = chunk_cnt;
    }

    /// Returns the minimum required size of the given allocator
    pub const fn required_size(chunks: u8) -> usize {
        use core::mem;
        let chunks = (chunks & Self::CHUNK_MAX) as usize;
        let header_size = mem::size_of::<Self>();
        let chunks_size = mem::size_of::<Option<Chunk<T>>>() * chunks;

        header_size + chunks_size
    }
}

#[allow(clippy::zero_ptr)]
impl<T> Default for SlabAllocator<T> {
    fn default() -> Self {
        let null_chunk = 0 as *mut Chunk<T>;
        Self {
            start: core::ptr::slice_from_raw_parts_mut(null_chunk, 0),
            len: 0,
        }
    }
}

unsafe impl<T> GlobalAlloc for SlabAllocator<T>
where
    T: Default,
{
    unsafe fn alloc(&self, _layout: Layout) -> *mut u8 {
        core::ptr::null_mut()
    }

    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: Layout) {}
}

/// Aligns an address up to a given alignment.
///
/// Alignment must be a power of 2.
pub const fn align_up(addr: usize, align: usize) -> usize {
    let remainder = addr & align;
    if remainder == 0 {
        addr
    } else {
        addr - remainder + align
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[ignore = "need to resize tests to align on chunk slice"]
    #[test]
    fn should_align_test_to_atleast_header_size() {
        use core::mem;
        let expected_allocator_overhead = (usize::BITS as usize / 8) * 2;
        let expected_chunk_overhead = ((usize::BITS as usize) / 8) + (mem::size_of::<u8>() * 64);

        (0..=0).into_iter().for_each(|chunks| {
            assert_eq!(
                expected_allocator_overhead + (expected_chunk_overhead * (chunks as usize)),
                SlabAllocator::<u8>::required_size(chunks)
            )
        });
    }
}
