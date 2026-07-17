/// A trait for augmenting a tree with additional data.
pub trait Augment<K, V>: Sized {
    /// The type of data that will be stored in the augmented tree.
    type Stats;

    /// Computes the augmented data for a node given its key, value, and the augmented data of its left and right children.
    fn compute(
        key: &K,
        value: &V,
        left: Option<(&K, &V, &Self::Stats)>,
        right: Option<(&K, &V, &Self::Stats)>,
    ) -> Self::Stats;

    /// Returns the identity element for the augmented data.
    fn identity() -> Self::Stats;
}
