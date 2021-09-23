#![no_std]

extern crate alloc;

/// A custom, and minimal, `Box`-like implementation for the time being. This
/// is acting as a placeholder until the allocator api stabilizes.
///
/// # Warnings
/// This internal type makes no guarantees of compatibility or even api
/// similarity. With the `alloc::boxed::Box` implementation.
#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub struct Box<T> {
    free_mask: usize,
    chunk: *mut Chunk<T>,
    inner: *mut T,
}

impl<T> core::fmt::Display for Box<T>
where
    T: core::fmt::Display,
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}", self.as_ref())
    }
}

impl<T> AsRef<T> for Box<T> {
    fn as_ref(&self) -> &T {
        unsafe { self.inner.as_ref().unwrap() }
    }
}

impl<T> AsMut<T> for Box<T> {
    fn as_mut(&mut self) -> &mut T {
        unsafe { self.inner.as_mut().unwrap() }
    }
}

impl<T> PartialEq<T> for Box<T>
where
    T: PartialEq,
{
    fn eq(&self, other: &T) -> bool {
        unsafe { self.inner.as_ref() == Some(other) }
    }
}

impl<T> Eq for Box<T> where T: Eq {}

impl<T> core::ops::Deref for Box<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.as_ref()
    }
}

impl<T> core::ops::DerefMut for Box<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.as_mut()
    }
}

impl<T> Drop for Box<T> {
    fn drop(&mut self) {
        let chunk = unsafe { self.chunk.as_mut() }.expect("chunk couldn't be borrowed");

        chunk.free_list |= self.free_mask;
    }
}

/// Chunk is a typed segment of memory consisting of a fixed number of cells
/// represented by the bit-width of the architectures pointer type. The Chunk
/// handles tracking allocation of cells.
#[derive(Debug)]
pub struct Chunk<T> {
    free_list: usize,
    inner: [T; usize::BITS as usize],
}

impl<T> Chunk<T> {
    /// The maximum number of elements in the chunk.
    const ELEMS: usize = usize::BITS as usize;

    /// Initializes a new empty chunk.
    #[allow(clippy::uninit_assumed_init)]
    pub fn new() -> Self {
        use core::mem::MaybeUninit;

        let inner = { unsafe { MaybeUninit::uninit().assume_init() } };

        Self {
            free_list: usize::MAX,
            inner,
        }
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

    /// Returns true if no cells have been allocated.
    pub fn empty(&self) -> bool {
        self.free_list == usize::MAX
    }

    /// Returns true if all cells have been allocated.
    pub fn full(&self) -> bool {
        self.free_list == usize::MIN
    }
}

impl<T> Default for Chunk<T> {
    fn default() -> Self {
        Self::new()
    }
}

/// Provides a SlabAllocator implementation containing a constantly defined
/// array of sequential `Chunks` for a type.
///
/// # Example
///
/// ```
///  extern crate std;
///  use slab::*;
///
///  let mut slab = SlabAllocator::<u8, 1>::new();
///  let optional_boxed_five = slab.boxed(5);
///
///  assert!(optional_boxed_five.unwrap().as_ref() == &5u8);
///
/// ```
pub struct SlabAllocator<T, const N: usize> {
    chunks: [Chunk<T>; N],
}

impl<T, const N: usize> SlabAllocator<T, N> {
    /// Represents the maximum number of chunks allowed in the allocator. This
    /// is equivalent to the number of bits of the pointer type.
    const CHUNK_MAX: u8 = (usize::BITS as u8 - 1);

    /// Initializes a new empty `SlabAllocator<T>`.
    pub fn new() -> Self {
        Self::default()
    }

    /// Allocates a value, returning a box to it.
    pub fn boxed(&mut self, value: T) -> Option<Box<T>> {
        let optional_chunk = self.find_chunk_with_space();
        optional_chunk.map(|offset| {
            let chunk = self.borrow_chunk_mut(offset).unwrap();
            // safe to unwrap due to above free space guarantee.
            let free_cell_offset = chunk.first_free().unwrap();

            let cell_ptr = {
                let cell = &mut chunk.inner[free_cell_offset];
                *cell = value;
                cell as *mut T
            };

            (*chunk).free_list &= alloc_mask(free_cell_offset as u8);

            Box {
                free_mask: free_mask(free_cell_offset as u8),
                chunk: chunk as *mut Chunk<T>,
                inner: cell_ptr as *mut T,
            }
        })
    }

    /// finds the first free chunk.
    fn find_chunk_with_space(&self) -> Option<usize> {
        for chunk_offset in 0..(Self::CHUNK_MAX as usize) {
            let chunk = self.borrow_chunk(chunk_offset)?;
            if !chunk.full() {
                return Some(chunk_offset);
            }
        }

        None
    }

    /// Borrows a chunk determined by a given offset. This value must be less
    /// than the Slab's max chunk count.
    fn borrow_chunk(&self, offset: usize) -> Option<&Chunk<T>> {
        self.chunks.get(offset)
    }

    /// Borrows a chunk determined by a given offset. This value must be less
    /// than the Slab's max chunk count.
    fn borrow_chunk_mut(&mut self, offset: usize) -> Option<&mut Chunk<T>> {
        self.chunks.get_mut(offset)
    }
}

#[allow(clippy::zero_ptr)]
impl<T, const N: usize> Default for SlabAllocator<T, N> {
    #[allow(clippy::uninit_assumed_init)]
    fn default() -> Self {
        use core::mem::MaybeUninit;

        let mut chunks: [Chunk<T>; N] = { unsafe { MaybeUninit::uninit().assume_init() } };
        for chunk in chunks.iter_mut() {
            *chunk = Chunk::<T>::default();
        }

        Self { chunks }
    }
}

/// Generates a mask for a given position used to assign an allocation to a chunk.
const fn alloc_mask(pos: u8) -> usize {
    let shift = ((usize::BITS - 1) as usize) - pos as usize;
    usize::MAX ^ (1 << shift)
}

/// Generates the mask for a given postion to free an allocation on a chunk.
const fn free_mask(pos: u8) -> usize {
    let shift = ((usize::BITS - 1) as usize) - pos as usize;
    !usize::MAX ^ (1 << shift)
}

#[cfg(test)]
mod tests {
    extern crate std;
    use super::*;

    #[test]
    fn should_mask_off_allocation() {
        let mut slab = SlabAllocator::<u8, 1>::new();
        let optional_boxed_five = slab.boxed(5);

        assert_eq!(
            Some(usize::MAX >> 1),
            slab.borrow_chunk(0).map(|chunk| chunk.free_list)
        );
        assert_eq!(Some(5), optional_boxed_five.map(|boxed| *boxed));

        // check freed after drop
        assert_eq!(
            Some(usize::MAX),
            slab.borrow_chunk(0).map(|chunk| chunk.free_list)
        );
    }

    #[test]
    fn should_safely_drop_multiple_allocations() {
        let mut slab = SlabAllocator::<u8, 1>::new();
        let boxed_values: alloc::vec::Vec<_> = (0..usize::BITS as u8)
            .into_iter()
            .map(|x| slab.boxed(x))
            .collect();

        assert_eq!(Some(0), slab.borrow_chunk(0).map(|chunk| chunk.free_list));

        core::mem::drop(boxed_values);
        assert_eq!(
            Some(usize::MAX),
            slab.borrow_chunk(0).map(|chunk| chunk.free_list)
        );
    }

    #[test]
    fn should_allow_allocations_over_multiple_chunks() {
        let mut slab = SlabAllocator::<u8, 2>::new();
        let boxed_values: alloc::vec::Vec<_> = (0..(usize::BITS * 2) as u8)
            .into_iter()
            .map(|x| slab.boxed(x))
            .collect();

        assert_eq!(Some(0), slab.borrow_chunk(0).map(|chunk| chunk.free_list));
        assert_eq!(Some(0), slab.borrow_chunk(1).map(|chunk| chunk.free_list));

        core::mem::drop(boxed_values);
        assert_eq!(
            Some(usize::MAX),
            slab.borrow_chunk(0).map(|chunk| chunk.free_list)
        );
        assert_eq!(
            Some(usize::MAX),
            slab.borrow_chunk(1).map(|chunk| chunk.free_list)
        );
    }
}
