# Contributing to augmented-rbtree

Thank you for your interest in improving `augmented-rbtree`! We welcome contributions from the community, whether they are bug fixes, feature additions, or documentation improvements.

Because this crate is built for high-performance and low-latency environments (including `no_std` bare-metal microcontrollers), all contributions must satisfy strict architectural constraints.

---

## Architectural Guidelines

This crate uses a specialized four-tier allocation framework. All code must be strictly `#![no_std]` compliant. Use the following breakdown to determine how to gate your modifications or feature flags:

| Feature Flag | Target Environment | Internal Behavior & Restrictions |
| :--- | :--- | :--- |
| **`--no-default-features`** | 100% stack-only / zero-heap | The allocation proxy uses a dummy ZST `Global` that returns a safe `Err(AllocError)` on allocation. Structure sizes must be bounded. |
| **`--features alloc`** | System-wide global heap | The proxy handles tree memory blocks by forwarding straight to the standard system functions `alloc::alloc::alloc` and `dealloc`. |
| **`--features allocator-api`** | Custom allocators on Stable Rust | The proxy pulls in the `allocator-api2` polyfill dependency and enforces the stable `allocator_api2::alloc::Allocator` trait. |
| **`--features nightly`** | Custom allocators on Nightly Rust | The proxy activates `#![feature(allocator_api)]` and enforces the native compiler `core::alloc::Allocator` trait. |

* **Heap Gating**: If your contribution relies on heap-allocation mechanics, gate the code using:
  ```rust
  #[cfg(any(feature = "alloc", feature = "allocator-api", feature = "nightly"))]
  ```

### Understanding Allocator Feature Combinations

Because Cargo features are **additive**, users or downstream crates can accidentally activate multiple allocation flags at once. The crate's internal `alloc_proxy` module resolves combinations using a strict priority ladder:

1. **`nightly` vs `allocator-api` (Mutually Exclusive)**: 
   The `nightly` and `allocator-api` features are completely incompatible because they implement entirely different backend traits for custom allocators. If a build attempts to activate both simultaneously, the crate triggers a hard compile-time failure (`compile_error!`).
2. **`alloc` combined with `allocator-api`**: 
   This is a valid and common combination. The crate will prioritize `allocator-api`. The core collection structs will be generic (`AugmentedRBTree<..., A>`) and accept custom stable allocators. The `alloc` feature simply ensures that the standard global heap fallback (`alloc::alloc::alloc`) is wired up inside the dummy `Global` allocator type.
3. **`alloc` combined with `nightly`**: 
   This is a valid combination. The crate will prioritize `nightly`. The collection structures switch entirely over to the unstable standard library `core::alloc::Allocator` trait, while `alloc` provides the underlying system-wide global allocator implementation.

---

## Development Workflow

### 1. Prerequisites
Ensure you have the full Rust toolchain installed, along with the required target for testing embedded compliance and the Miri validation interpreter:

```bash
rustup component add clippy miri
rustup target add thumbv7m-none-eabi
```

### 2. Guarding `no_std` Compatibility
This library must compile cleanly on platforms without an operating system.
* **Do not use `std::` components** inside core modules. Use `core::` or `alloc::` instead.
* When writing documentation examples (doc-tests), you must add `extern crate std;` inside the example snippet block so that test binaries compile smoothly without breaking `#![no_std]` rules.

### 3. Coding Guidelines & Lints
We enforce clean, idiomatic, modern Rust. Your code must not trigger any warnings or lints.
* Run Clippy to check your code safety boundaries and code quality across all features:
  ```bash
  cargo clippy  --features alloc,interval-tree,serde -- -D warnings
  ```
* All public APIs, structs, and methods **must have documentation comments** along with simple usage examples.

---

## Pre-PR Testing Checklist

Do not use `cargo test --all-features`. Because the `nightly` and `allocator-api` features are mutually exclusive, they must be tested in isolation. Run the following matrix locally before submitting your PR:

Install `thumbv7m-none-eabi` target for bare-metal compilation:
```
rustup target add thumbv7m-none-eabi 
rustup target add thumbv7m-none-eabi --toolchain nightly
```

Checklist:
```bash
./run_tests.sh core
```

Run tests inside the Miri interpreter to guarantee raw pointer safety and verify there is no undefined behavior or memory leaks:
```bash
./run_tests.sh miri
```

Run the full suite test for CI
```bash
./run_tests.sh ci
```

### 4. Code coverage 
Coverage is already part of the [CI pipeline](.github/workflows/coverage.yml), but you can also run it locally.

Install the nightly toolchain and the `llvm-tools-preview` component:
```
rustup toolchain install nightly
rustup component add llvm-tools-preview --toolchain nightly

# Install cargo-llvm-cov for coverage reporting:
```
cargo install cargo-llvm-cov

# to use grcov instead of cargo-llvm-cov, install it with:
cargo install grcov
```

Collect coverage data and generate a report:
```
# run with cargo-llvm-cov
./run_tests.sh cov

# run with grcov
./run_tests.sh cov2
```

## Pull Request Process

1. Fork the repository and create your feature branch from `main`.
2. Commit your changes with clear, descriptive commit messages.
3. Update the `README.md` if your change introduces or modifies a public API or configuration flag.
4. Ensure the pre-PR testing checklist runs completely green on your machine.
5. Submit your PR against the `main` branch. A maintainer will review your implementation shortly!
