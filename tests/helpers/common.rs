use rand::SeedableRng;

pub(crate) fn test_rng() -> rand::rngs::SmallRng {
    rand::rngs::SmallRng::seed_from_u64(0xDEAD_BEEF)
}

pub(crate) mod custom_augment_a {
    use std::{
        fmt::Display,
        hash::{DefaultHasher, Hash, Hasher},
    };

    use augmented_rbtree::Augment;
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
    pub(crate) struct CustomKey(pub(crate) i32);

    #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
    pub(crate) struct CustomValue(pub(crate) String);

    pub(crate) struct CustomAugment;

    pub(crate) struct CustomStats {
        pub(crate) data: String,
    }

    impl Display for CustomStats {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "{}", self.data)
        }
    }

    impl Display for CustomKey {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "{}", self.0)
        }
    }

    impl Display for CustomValue {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "{}", self.0)
        }
    }
    thread_local! {
        pub(crate) static DROP_CUSTOM_KEY_LOGGER: std::cell::RefCell<Vec<String>> = const { std::cell::RefCell::new(Vec::new()) };
    }

    thread_local! {
        pub(crate) static DROP_CUSTOM_VALUE_LOGGER: std::cell::RefCell<Vec<String>> = const { std::cell::RefCell::new(Vec::new()) };
    }

    thread_local! {
        pub(crate) static DROP_CUSTOM_STATS_LOGGER: std::cell::RefCell<Vec<String>> = const { std::cell::RefCell::new(Vec::new()) };
    }

    pub(crate) fn reset_drop_loggers() {
        DROP_CUSTOM_KEY_LOGGER.with(|logger| logger.borrow_mut().clear());
        DROP_CUSTOM_VALUE_LOGGER.with(|logger| logger.borrow_mut().clear());
        DROP_CUSTOM_STATS_LOGGER.with(|logger| logger.borrow_mut().clear());
    }

    impl Drop for CustomKey {
        fn drop(&mut self) {
            DROP_CUSTOM_KEY_LOGGER.with(|logger| {
                logger.borrow_mut().push(format!("{}", self.0));
            });
        }
    }

    impl Drop for CustomValue {
        fn drop(&mut self) {
            DROP_CUSTOM_VALUE_LOGGER.with(|logger| {
                logger.borrow_mut().push(self.0.clone());
            });
        }
    }

    impl Drop for CustomStats {
        fn drop(&mut self) {
            DROP_CUSTOM_STATS_LOGGER.with(|logger| {
                logger.borrow_mut().push(self.data.clone());
            });
        }
    }

    impl Augment<CustomKey, CustomValue> for CustomAugment {
        type Stats = CustomStats;

        fn compute(
            _key: &CustomKey,
            value: &CustomValue,
            left: Option<(&CustomKey, &CustomValue, &Self::Stats)>,
            right: Option<(&CustomKey, &CustomValue, &Self::Stats)>,
        ) -> Self::Stats {
            let mut result = DefaultHasher::new();
            if let Some((_, _, left_data)) = left {
                left_data.data.hash(&mut result);
            }
            value.0.hash(&mut result);
            if let Some((_, _, right_data)) = right {
                right_data.data.hash(&mut result);
            }
            CustomStats {
                data: result.finish().to_string(),
            }
        }
    }
}
