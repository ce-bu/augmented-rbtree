pub mod internal_details {
    use crate::alloc_proxy::proxy::Allocator;
    use crate::node::internal_details::NodeRef;
    use crate::{Augment, Unit};

    /// A policy that defines how to compute and maintain augmented statistics for a Red-Black Tree.
    pub trait TreePolicy {
        /// The type of keys in the tree.
        type K;
        /// The type of values in the tree.
        type V;

        /// The type of augmented statistics stored in each node.
        type S;

        /// The augmentation type that defines how to compute the stats.
        type G: Augment<Self::K, Self::V, Stats = Self::S>;

        /// The allocator type used for node allocation.
        type A: Allocator;

        /// Computes the augmented statistics for a node based on its key, value, and the stats of its children.
        fn compute(
            key: &Self::K,
            value: &Self::V,
            left: Option<(&Self::K, &Self::V, &Self::S)>,
            right: Option<(&Self::K, &Self::V, &Self::S)>,
        ) -> Self::S;

        /// Updates the augmented statistics for a node based on its key, value, and the stats of its children.
        fn augment(node: NodeRef<Self::K, Self::V, Self::S>);

        /// Updates the augmented statistics for a node and all its ancestors up to the root.
        fn augment_upstream(node: NodeRef<Self::K, Self::V, Self::S>);
    }

    /// A strategy that performs full augmentation, computing stats for each node.
    /// This is the default strategy used by `AugmentedRBTree`.
    #[derive(Debug)]
    pub struct FullAugmentationStrategy;

    /// A strategy that performs no augmentation, leaving stats uninitialized.
    /// This is the default strategy used by `RBTree`.
    #[derive(Debug)]
    pub struct NullAugmentationStrategy;

    /// A default tree policy that uses the provided augmentation type `G` and allocator `A`.
    #[derive(Debug)]
    pub struct DefaultTraitPolicy<K, V, G, S, A, AS> {
        _marker: core::marker::PhantomData<(K, V, G, S, A, AS)>,
    }

    impl<K, V, G, S, A> TreePolicy for DefaultTraitPolicy<K, V, G, S, A, FullAugmentationStrategy>
    where
        K: Ord,
        G: Augment<K, V, Stats = S>,
        A: Allocator,
    {
        type K = K;
        type V = V;
        type S = S;
        type G = G;
        type A = A;

        fn compute(
            key: &Self::K,
            value: &Self::V,
            left: Option<(&Self::K, &Self::V, &Self::S)>,
            right: Option<(&Self::K, &Self::V, &Self::S)>,
        ) -> Self::S {
            G::compute(key, value, left, right)
        }

        fn augment(node: NodeRef<Self::K, Self::V, Self::S>) {
            unsafe {
                let raw_node = node.ptr.as_ptr();

                let left_args = if let Some(l) = (*raw_node).left {
                    let l_ptr = l.as_ptr();
                    Some((&(*l_ptr).key, &(*l_ptr).value, &(*l_ptr).stats))
                } else {
                    None
                };

                let right_args = if let Some(r) = (*raw_node).right {
                    let r_ptr = r.as_ptr();
                    Some((&(*r_ptr).key, &(*r_ptr).value, &(*r_ptr).stats))
                } else {
                    None
                };

                (*raw_node).stats =
                    G::compute(&(*raw_node).key, &(*raw_node).value, left_args, right_args);
            }
        }

        fn augment_upstream(node: NodeRef<Self::K, Self::V, Self::S>) {
            let mut ptr = node;
            while let Some(parent) = ptr.parent() {
                Self::augment(parent);
                ptr = parent;
            }
        }
    }

    impl<K, V, A> TreePolicy for DefaultTraitPolicy<K, V, Unit, (), A, NullAugmentationStrategy>
    where
        K: Ord,
        A: Allocator,
    {
        type K = K;
        type V = V;
        type S = ();
        type G = Unit;
        type A = A;

        #[cfg_attr(coverage_nightly, coverage(off))]
        fn compute(
            _key: &Self::K,
            _value: &Self::V,
            _left: Option<(&Self::K, &Self::V, &Self::S)>,
            _right: Option<(&Self::K, &Self::V, &Self::S)>,
        ) -> Self::S {
        }

        fn augment(_node: NodeRef<Self::K, Self::V, Self::S>) {}

        fn augment_upstream(_node: NodeRef<Self::K, Self::V, Self::S>) {}
    }
}
