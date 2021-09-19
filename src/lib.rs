#![no_std]

extern crate alloc;
use alloc::alloc::{GlobalAlloc, Layout};

pub struct SlabAllocator<T> {
    r#type: core::marker::PhantomData<T>,
}

impl<T> SlabAllocator<T> {
    pub const fn new() -> Self {
        Self {
            r#type: core::marker::PhantomData,
        }
    }

    pub unsafe fn init(&mut self, _heap_start: usize, _heap_size: usize) {}
}

unsafe impl<T> GlobalAlloc for SlabAllocator<T> {
    unsafe fn alloc(&self, _layout: Layout) -> *mut u8 {
        core::ptr::null_mut()
    }

    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: Layout) {}
}
