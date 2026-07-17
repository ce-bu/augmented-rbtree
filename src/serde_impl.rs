use crate::{
    Augment, AugmentedRBTree,
    alloc_proxy::proxy::{Allocator, Global},
};
use core::marker::PhantomData;
use serde::{
    Deserialize, Deserializer, Serialize, Serializer,
    de::{DeserializeSeed, SeqAccess},
    ser::SerializeSeq,
};

/// A deserialization seed that carries a custom allocator instance
/// along with the type parameters needed to construct the tree.
///
#[derive(Debug)]
pub struct AugmentedRBTreeSeedInt<K, V, G, A: Allocator> {
    pub allocator: A,
    _marker: PhantomData<(K, V, G)>,
}

/// A deserialization seed that carries a custom allocator instance
pub type AugmentedRBTreeSeed<K, V, G, A = Global> = AugmentedRBTreeSeedInt<K, V, G, A>;

impl<K, V, G, A: Allocator> AugmentedRBTreeSeedInt<K, V, G, A> {
    /// Creates a new deserialization seed with the given allocator.
    pub fn new(allocator: A) -> Self {
        Self {
            allocator,
            _marker: PhantomData,
        }
    }
}

impl<'de, K, V, G, A: Allocator> DeserializeSeed<'de> for AugmentedRBTreeSeedInt<K, V, G, A>
where
    G: Augment<K, V>,
    K: Deserialize<'de> + Ord,
    V: Deserialize<'de>,
{
    // The type produced by this seed is the tree with your custom allocator
    type Value = AugmentedRBTree<K, V, G, A>;

    fn deserialize<D: Deserializer<'de>>(self, deserializer: D) -> Result<Self::Value, D::Error> {
        struct AllocatorVisitor<K, V, G, A> {
            allocator: A,
            _marker: core::marker::PhantomData<(K, V, G)>,
        }

        impl<'de, K, V, G, A: Allocator> serde::de::Visitor<'de> for AllocatorVisitor<K, V, G, A>
        where
            K: Deserialize<'de> + Ord,
            V: Deserialize<'de>,
            G: Augment<K, V>,
        {
            type Value = AugmentedRBTree<K, V, G, A>;

            fn expecting(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                f.write_str("a sequence of sorted (key, value) pairs")
            }

            fn visit_seq<SA: SeqAccess<'de>>(self, mut seq: SA) -> Result<Self::Value, SA::Error> {
                // Initialize the tree using your custom allocator state!
                let mut tree = AugmentedRBTree::<K, V, G, A>::new_in(self.allocator);
                while let Some((k, v)) = seq.next_element::<(K, V)>()? {
                    tree.insert(k, v);
                }
                Ok(tree)
            }
        }

        deserializer.deserialize_seq(AllocatorVisitor {
            allocator: self.allocator,
            _marker: core::marker::PhantomData,
        })
    }
}

impl<K, V, G, A: Allocator> Serialize for AugmentedRBTree<K, V, G, A>
where
    G: Augment<K, V>,
    K: Serialize + Ord,
    V: Serialize,
{
    fn serialize<Se: Serializer>(&self, serializer: Se) -> Result<Se::Ok, Se::Error> {
        // Compact output format: strips internal layout metadata
        let mut seq = serializer.serialize_seq(Some(self.len()))?;
        for (k, v, _) in self {
            seq.serialize_element(&(k, v))?;
        }
        seq.end()
    }
}

// impl<'de, K, V, G, S, A, AS> Deserialize<'de> for AugmentedRBTreeInt<K, V, G, S, A, AS>
// where
//     G: Augment<K, V, Data = S>,
//     K: Deserialize<'de> + Ord,
//     V: Deserialize<'de>,
//     A: Allocator + Default,
//
//     AS: AugmentationStrategy,
// {
//     fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
//         let seed = AugmentedRBTreeSeedInt::<K, V, G, S, A, AS>::new(A::default());
//         seed.deserialize(deserializer)
//     }
// }
