use core::ptr::NonNull;

use crate::{
    alloc_proxy::proxy::Allocator, alloc_proxy::proxy::Layout, augmented_rbtree::OutOfMemoryError,
    node::Color, node::Node, node::internal_details::NodeRef,
};
#[derive(Debug, Clone)]
pub(crate) struct NodeAllocator<A> {
    pub(crate) alloc: A,
}

impl<A: Allocator> NodeAllocator<A> {
    /// Creates a new `NodeAllocator` with the given allocator.
    #[inline]
    pub fn new(alloc: A) -> Self {
        Self { alloc }
    }

    pub(crate) fn alloc_node<K, V, S>(
        &self,
        key: K,
        value: V,
        stats: S,
    ) -> Result<NodeRef<K, V, S>, OutOfMemoryError> {
        let layout = Layout::new::<Node<K, V, S>>();

        let memory_block = match self.alloc.allocate(layout) {
            Ok(block) => block.cast::<Node<K, V, S>>(),
            Err(e) => return Err(OutOfMemoryError::new(layout.size(), e)),
        };

        let raw_ptr = memory_block.as_ptr();

        unsafe {
            raw_ptr.write(Node {
                key,
                value,
                stats,
                color: Color::Red, // New nodes always default to Red in Red-Black trees
                left: None,
                right: None,
                parent: None,
                _marker: core::marker::PhantomData,
            });

            Ok(NodeRef::from_raw(NonNull::new_unchecked(raw_ptr)))
        }
    }

    pub(crate) unsafe fn dealloc_node<K, V, S>(&self, node: NodeRef<K, V, S>) -> (K, V, S) {
        unsafe {
            let raw_ptr = node.ptr.as_ptr();

            // Use addr_of! to avoid creating a reference to a potentially moved-from location.
            // ptr::read performs a bitwise copy without running Drop on the source.
            let key = core::ptr::read(core::ptr::addr_of!((*raw_ptr).key));
            let value = core::ptr::read(core::ptr::addr_of!((*raw_ptr).value));
            let stats = core::ptr::read(core::ptr::addr_of!((*raw_ptr).stats));

            let layout = Layout::new::<Node<K, V, S>>();
            self.alloc.deallocate(node.ptr.cast(), layout);

            (key, value, stats)
        }
    }
}

#[cfg(all(
    test,
    any(feature = "alloc", feature = "nightly", feature = "allocator-api")
))]
mod tests {

    use super::*;

    #[test]
    #[allow(clippy::clone_on_copy)]
    fn node_ref_copy_clone_eq() {
        let allocator = NodeAllocator::new(crate::alloc_proxy::proxy::Global);
        let node_ref1 = allocator.alloc_node(1, 10, 1024).unwrap();
        let node_ref2 = node_ref1;
        let node_ref3 = node_ref1.clone();

        assert_eq!(node_ref1, node_ref2);
        assert_eq!(node_ref1, node_ref3);

        unsafe {
            allocator.dealloc_node(node_ref1);
        }
    }
}
