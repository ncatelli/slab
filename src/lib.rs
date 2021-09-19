#![no_std]

extern crate alloc;
use alloc::alloc::{GlobalAlloc, Layout};

pub struct Chunk<T> {
    free_list: usize,
    r#type: core::marker::PhantomData<T>,
}

pub struct SlabAllocator<T> {
    r#type: core::marker::PhantomData<T>,
    start: *mut Chunk<T>,
    chunks: usize,
}

impl<T> SlabAllocator<T> {
    /// Represents the maximum number of chunks allowed in the allocator. This
    /// is equivalent to the number of bits of the pointer type.
    const CHUNK_MAX: u8 = (usize::BITS as u8 - 1);

    pub const fn new() -> Self {
        Self {
            r#type: core::marker::PhantomData,
            start: core::ptr::null_mut(),
            chunks: 0,
        }
    }

    pub unsafe fn init(&mut self, chunks: u8) {
        let chunks = chunks & Self::CHUNK_MAX;
    }

    /// Returns the minimum required size of the given allocator
    pub const fn required_size(chunks: u8) -> usize {
        use core::mem;
        let chunks = (chunks & Self::CHUNK_MAX) as usize;
        let header_size = mem::size_of::<Self>();
        let chunks_size = mem::size_of::<Chunk<T>>() * chunks;

        header_size + chunks_size
    }

    const fn chunk_mask(&self) -> usize {
        if self.chunks == 0 {
            0
        } else {
            self.chunks - 1
        }
    }
}

unsafe impl<T> GlobalAlloc for SlabAllocator<T> {
    unsafe fn alloc(&self, _layout: Layout) -> *mut u8 {
        core::ptr::null_mut()
    }

    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: Layout) {}
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_align_test_to_atleast_header_size() {
        let expected_allocator_overhead = (usize::BITS as usize / 8) * 2;
        let expected_chunk_overhead = (usize::BITS as usize) / 8;

        (0..=32).into_iter().for_each(|chunks| {
            assert_eq!(
                expected_allocator_overhead + (expected_chunk_overhead * (chunks as usize)),
                SlabAllocator::<u8>::required_size(chunks)
            )
        });
    }
}
