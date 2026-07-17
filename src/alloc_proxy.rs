#[cfg(any(feature = "nightly", feature = "allocator-api"))]
pub mod proxy {
    #[cfg(feature = "nightly")]
    #[allow(unused_imports)]
    pub use core::alloc::{AllocError, Allocator, Layout};

    #[cfg(feature = "nightly")]
    #[allow(unused_imports)]
    pub use alloc::alloc::{Global, handle_alloc_error}; // Global lives in the alloc crate!

    #[cfg(all(not(feature = "nightly"), feature = "allocator-api"))]
    #[allow(unused_imports)]
    pub use allocator_api2::alloc::{AllocError, Allocator, Global, Layout, handle_alloc_error};
}

#[cfg(all(not(feature = "nightly"), not(feature = "allocator-api")))]
pub mod proxy {
    pub use core::alloc::Layout;

    /// A trait for custom memory allocators.
    ///
    /// # Safety
    ///
    /// Implementors must ensure that the `allocate` and `deallocate` methods are safe to call and that they correctly manage memory.
    /// The `allocate` method must return a valid pointer to a block of memory of the requested size and alignment, or an `AllocError` if allocation fails.
    /// The `deallocate` method must free the memory pointed to by the given pointer, which must have been previously allocated by the same allocator.
    /// Implementors must also ensure that the allocator is thread-safe if it will be used in a multi-threaded context.
    ///
    pub unsafe trait Allocator {
        /// Allocates a block of memory with the given layout.
        /// # Result
        ///
        /// The `allocate` method returns a `Result` containing a non-null pointer to the allocated memory block on success, or an `AllocError` on failure. The `deallocate`
        ///
        /// # Errors
        ///
        /// The `allocate` method may return an `AllocError` if the allocation fails, for example, due to insufficient memory. The caller must handle this error appropriately.
        ///
        fn allocate(&self, layout: Layout) -> Result<core::ptr::NonNull<[u8]>, AllocError>;

        /// Deallocates a previously allocated block of memory.
        ///
        /// # Safety
        /// The caller must ensure that the pointer was allocated by this allocator and that the layout matches the original allocation.
        unsafe fn deallocate(&self, ptr: core::ptr::NonNull<u8>, layout: Layout);
    }

    /// A simple global allocator that uses the built-in global allocator.
    #[derive(Copy, Clone, Debug, PartialEq, Eq)]
    pub struct AllocError;

    /// This is the default global allocator that uses the built-in global allocator.
    /// It is used when no custom allocator is provided.
    #[derive(Copy, Clone, Default, Debug)]
    pub struct Global;

    unsafe impl Allocator for Global {
        #[inline]
        #[allow(unused_variables)]
        fn allocate(&self, layout: Layout) -> Result<core::ptr::NonNull<[u8]>, AllocError> {
            #[cfg(feature = "alloc")]
            unsafe {
                // 1. Call the built-in global allocator function directly
                let raw_ptr = alloc::alloc::alloc(layout);
                if raw_ptr.is_null() {
                    return Err(AllocError);
                }

                // 2. Turn the raw pointer into a slice pointer tracking the size
                let slice_ptr = core::ptr::slice_from_raw_parts_mut(raw_ptr, layout.size());
                Ok(core::ptr::NonNull::new_unchecked(slice_ptr))
            }

            // Fallback for environments compiling with absolutely zero heap features
            #[cfg(not(feature = "alloc"))]
            {
                Err(AllocError)
            }
        }

        #[inline]
        #[allow(unused_variables)]
        unsafe fn deallocate(&self, ptr: core::ptr::NonNull<u8>, layout: Layout) {
            #[cfg(feature = "alloc")]
            unsafe {
                alloc::alloc::dealloc(ptr.as_ptr(), layout);
            }
        }
    }

    #[inline]
    pub fn handle_alloc_error(layout: Layout) -> ! {
        // If the 'alloc' feature is active, forward to the official alloc error handler
        #[cfg(feature = "alloc")]
        {
            alloc::alloc::handle_alloc_error(layout);
        }

        // If 'alloc' is completely compiled out, panic directly
        #[cfg(not(feature = "alloc"))]
        {
            panic!("OOM: Allocation of {} bytes failed", layout.size());
        }
    }
}
