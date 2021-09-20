#![no_std]

extern crate alloc;
use alloc::alloc::{GlobalAlloc, Layout};

#[derive(Debug)]
pub struct Chunk<T> {
    free_list: usize,
    inner: [Option<T>; usize::BITS as usize],
}

pub struct SlabAllocator<T> {
    start: *mut Chunk<T>,
    len: usize,
}

impl<T> SlabAllocator<T> {
    /// Represents the maximum number of chunks allowed in the allocator. This
    /// is equivalent to the number of bits of the pointer type.
    const CHUNK_MAX: u8 = (usize::BITS as u8 - 1);

    /// Initializes a new empty `SlabAllocator<T>`.
    pub const fn new() -> Self {
        Self {
            start: core::ptr::null_mut(),
            len: 0,
        }
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
        self.start = start_of_chunks;
        self.len = (chunks & Self::CHUNK_MAX) as usize;
    }

    /// Returns the minimum required size of the given allocator
    pub const fn required_size(chunks: u8) -> usize {
        use core::mem;
        let chunks = (chunks & Self::CHUNK_MAX) as usize;
        let header_size = mem::size_of::<Self>();
        let chunks_size = mem::size_of::<Option<Chunk<T>>>() * chunks;

        header_size + chunks_size
    }

    const fn chunk_mask(&self) -> usize {
        if self.len == 0 {
            0
        } else {
            self.len - 1
        }
    }

    fn get_chunk(&self, idx: usize) {}
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

    #[test]
    fn should_align_test_to_atleast_header_size() {
        use core::mem;
        let expected_allocator_overhead = (usize::BITS as usize / 8) * 2;
        let expected_chunk_overhead = ((usize::BITS as usize) / 8) + (mem::size_of::<u8>() * 64);

        (0..=32).into_iter().for_each(|chunks| {
            assert_eq!(
                expected_allocator_overhead + (expected_chunk_overhead * (chunks as usize)),
                SlabAllocator::<u8>::required_size(chunks)
            )
        });
    }
}
