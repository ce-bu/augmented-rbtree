use std::cell::RefCell;
use std::collections::LinkedList;
use std::ptr::NonNull;
use std::rc::Rc;

use augmented_rbtree::{AllocError, Allocator, Layout};

#[allow(clippy::linkedlist)]
#[derive(Clone)]
pub(crate) struct LimitedAllocator {
    slot_size: usize,
    free_slots: Rc<RefCell<LinkedList<*mut u8>>>,
    allocated_slots: Rc<RefCell<LinkedList<*mut u8>>>,
}

impl LimitedAllocator {
    pub(crate) fn new(num_slots: usize, slot_size: usize) -> Self {
        let mut free_slots = LinkedList::new();
        let slot_layout = Layout::from_size_align(slot_size, 8).unwrap();

        for _ in 0..num_slots {
            unsafe {
                // Allocate raw system memory directly for the mock blocks
                let raw_ptr = std::alloc::alloc(slot_layout);
                println!("Allocated raw slot at: {raw_ptr:?}");
                assert!(
                    !raw_ptr.is_null(),
                    "OOM during LimitedAllocator initialization"
                );
                free_slots.push_back(raw_ptr);
            }
        }

        Self {
            free_slots: Rc::new(RefCell::new(free_slots)),
            allocated_slots: Rc::new(RefCell::new(LinkedList::new())),
            slot_size,
        }
    }

    pub(crate) fn num_free_slots(&self) -> usize {
        self.free_slots.borrow().len()
    }

    pub(crate) fn num_allocated_slots(&self) -> usize {
        self.allocated_slots.borrow().len()
    }
}

// Ensure we clean up the raw allocated system slots when the allocator environment drops
impl Drop for LimitedAllocator {
    fn drop(&mut self) {
        // Only clean up if this is the last cloned instance of the allocator handle
        if Rc::strong_count(&self.free_slots) == 1 {
            let slot_layout = Layout::from_size_align(self.slot_size, 8).unwrap();
            unsafe {
                while let Some(ptr) = self.free_slots.borrow_mut().pop_front() {
                    println!("Deallocating raw slot at: {ptr:?}");
                    std::alloc::dealloc(ptr, slot_layout);
                }
                while let Some(ptr) = self.allocated_slots.borrow_mut().pop_front() {
                    println!("Deallocating raw allocated slot at: {ptr:?}");
                    std::alloc::dealloc(ptr, slot_layout);
                }
            }
        }
    }
}

unsafe impl Allocator for LimitedAllocator {
    fn allocate(&self, layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        if layout.size() > self.slot_size || layout.align() > 8 {
            return Err(AllocError);
        }

        let mut free_slots = self.free_slots.borrow_mut();
        let ptr = free_slots.pop_front().ok_or(AllocError)?;

        println!("LimitedAllocator: Allocated slot at: {ptr:?}");
        // Track the raw pointer value cleanly
        self.allocated_slots.borrow_mut().push_back(ptr);

        let slice = core::ptr::slice_from_raw_parts_mut(ptr, layout.size());
        Ok(NonNull::new(slice).unwrap())
    }

    unsafe fn deallocate(&self, ptr: core::ptr::NonNull<u8>, _layout: Layout) {
        let mut free_slots = self.free_slots.borrow_mut();
        let mut allocated_slots = self.allocated_slots.borrow_mut();

        let raw_target_ptr = ptr.as_ptr();

        // Remove the ampersand from the pattern to match the &mut *mut u8 type
        if let Some(slot) = allocated_slots
            .extract_if(|slot| *slot == raw_target_ptr)
            .next()
        {
            println!("LimitedAllocator: Deallocated slot at: {raw_target_ptr:?}");
            free_slots.push_back(slot);
        } else {
            panic!("Attempted to deallocate a pointer that was not allocated by this allocator");
        }
    }
}
