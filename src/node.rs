use core::ptr::NonNull;
/// The color of a node in the red-black tree.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Color {
    /// A red node.
    Red,
    /// A black node.
    Black,
}

/// Tracks which side of the parent a nil node is on.
/// This is necessary for `delete_fixup` when x is None.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum NilSide {
    Left,
    Right,
}

pub(crate) struct Node<K, V, S> {
    pub(crate) left: Option<NonNull<Node<K, V, S>>>,
    pub(crate) right: Option<NonNull<Node<K, V, S>>>,
    pub(crate) parent: Option<NonNull<Node<K, V, S>>>,
    pub(crate) key: K,
    pub(crate) value: V,
    pub(crate) stats: S,
    pub(crate) color: Color,
    pub(crate) _marker: core::marker::PhantomData<(K, V, S)>,
}

pub mod internal_details {
    use crate::{Color, node::Node};
    use core::ptr::NonNull;

    #[derive(Debug)]
    pub struct NodeRef<K, V, S> {
        pub(crate) ptr: NonNull<Node<K, V, S>>,
    }

    impl<K, V, S> Clone for NodeRef<K, V, S> {
        #[inline]
        fn clone(&self) -> Self {
            *self
        }
    }

    impl<K, V, S> Copy for NodeRef<K, V, S> {}

    impl<K, V, S> PartialEq for NodeRef<K, V, S> {
        #[inline]
        fn eq(&self, other: &Self) -> bool {
            // Two handles are equal if they point to the exact same heap node block
            self.ptr == other.ptr
        }
    }

    impl<K, V, S> Eq for NodeRef<K, V, S> {}

    impl<K, V, S> NodeRef<K, V, S> {
        #[inline]
        pub(crate) unsafe fn from_raw(ptr: NonNull<Node<K, V, S>>) -> Self {
            Self { ptr }
        }

        #[inline]
        pub(crate) fn parent(self) -> Option<Self> {
            unsafe { (*self.ptr.as_ptr()).parent.map(|p| NodeRef { ptr: p }) }
        }

        #[inline]
        pub(crate) fn set_parent(self, parent: Option<Self>) {
            unsafe { (*self.ptr.as_ptr()).parent = parent.map(|p| p.ptr) }
        }

        #[inline]
        pub(crate) fn left(self) -> Option<Self> {
            unsafe { (*self.ptr.as_ptr()).left.map(|p| NodeRef { ptr: p }) }
        }

        #[inline]
        pub(crate) fn set_left(self, left: Option<Self>) {
            unsafe { (*self.ptr.as_ptr()).left = left.map(|l| l.ptr) }
        }

        #[inline]
        pub(crate) fn right(self) -> Option<Self> {
            unsafe { (*self.ptr.as_ptr()).right.map(|p| NodeRef { ptr: p }) }
        }

        #[inline]
        pub(crate) fn set_right(self, right: Option<Self>) {
            unsafe { (*self.ptr.as_ptr()).right = right.map(|r| r.ptr) }
        }

        #[inline]
        pub(crate) unsafe fn key<'a>(self) -> &'a K {
            unsafe { &(*self.ptr.as_ptr()).key }
        }

        #[inline]
        pub(crate) unsafe fn value<'a>(self) -> &'a V {
            unsafe { &(*self.ptr.as_ptr()).value }
        }

        #[inline]
        pub(crate) unsafe fn stats<'a>(self) -> &'a S {
            unsafe { &(*self.ptr.as_ptr()).stats }
        }

        #[inline]
        pub(crate) fn color(self) -> Color {
            unsafe { (*self.ptr.as_ptr()).color }
        }

        #[inline]
        pub(crate) fn set_color(self, color: Color) {
            unsafe { (*self.ptr.as_ptr()).color = color }
        }

        #[inline]
        pub(crate) unsafe fn value_mut<'a>(self) -> &'a mut V {
            unsafe { &mut (*self.ptr.as_ptr()).value }
        }

        #[inline]
        pub(crate) fn is_black(node: Option<Self>) -> bool {
            match node {
                Some(ptr) => ptr.color() == Color::Black,
                None => true,
            }
        }

        #[inline]
        #[allow(dead_code)]
        pub(crate) fn next_node(self) -> Option<Self> {
            if let Some(right) = self.right() {
                let mut current = right;
                while let Some(left) = current.left() {
                    current = left;
                }
                return Some(current);
            }

            let mut current = self;
            while let Some(parent) = current.parent() {
                if parent.left() == Some(current) {
                    return Some(parent);
                }
                current = parent;
            }

            None
        }

        #[allow(dead_code)]
        #[inline]
        pub(crate) fn prev_node(self) -> Option<Self> {
            if let Some(left) = self.left() {
                let mut current = left;
                while let Some(right) = current.right() {
                    current = right;
                }
                return Some(current);
            }

            let mut current = self;
            while let Some(parent) = current.parent() {
                if parent.right() == Some(current) {
                    return Some(parent);
                }
                current = parent;
            }

            None
        }

        #[inline]
        pub(crate) fn leftmost(self) -> Self {
            let mut node = self;
            while let Some(left) = node.left() {
                node = left;
            }
            node
        }

        #[allow(dead_code)]
        #[inline]
        pub(crate) fn rightmost(self) -> Self {
            let mut node = self;
            while let Some(right) = node.right() {
                node = right;
            }
            node
        }
    }
}
