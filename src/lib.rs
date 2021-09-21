#![no_std]

extern crate alloc;
use alloc::alloc::{GlobalAlloc, Layout};
use core::ptr;

#[derive(Debug, Clone)]
pub struct Box<T> {
    mask: usize,
    chunk: *mut Chunk<T>,
    inner: *mut T,
}

#[derive(Debug)]
pub struct Chunk<T> {
    free_list: usize,
    inner: [Option<T>; usize::BITS as usize],
}

impl<T> Chunk<T> {
    /// The maximum number of elements in the chunk.
    const ELEMS: usize = usize::BITS as usize;

    pub fn new() -> Self {
        Self::default()
    }

    /// Finds the first 1 bit, representing a free cell in the allocator. If
    /// the chunk is full, None is returned. Otherwise the index into the cell
    /// is returned.
    fn first_free(&self) -> Option<usize> {
        let leading_zeros = self.free_list.leading_zeros() as usize;

        // if all bits are allocated return None
        if leading_zeros == Self::ELEMS {
            None
        } else {
            Some(leading_zeros)
        }
    }
}

impl<T> Default for Chunk<T> {
    #[allow(clippy::uninit_assumed_init)]
    fn default() -> Self {
        use core::mem::MaybeUninit;

        const ELEMS: usize = usize::BITS as usize;

        let inner = {
            let mut data: [Option<T>; ELEMS] = unsafe { MaybeUninit::uninit().assume_init() };

            for elem in &mut data[..] {
                *elem = None;
            }
            data
        };

        Self {
            free_list: usize::MAX,
            inner,
        }
    }
}

pub struct SlabAllocator<T> {
    len: usize,
    start: *mut Chunk<T>,
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
        let chunk_cnt = (chunks & Self::CHUNK_MAX) as usize;
        let start = (self as *const Self).add(1) as *mut Chunk<T>;

        self.len = chunk_cnt;
        self.start = start;
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
        Self {
            len: 0,
            start: ptr::null_mut(),
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

    #[test]
    fn should_default_chunk_inner_to_none() {
        let chunk = Chunk::<u8>::default();

        for chunk in (chunk.inner).iter() {
            assert_eq!(&None, chunk)
        }
    }
}
