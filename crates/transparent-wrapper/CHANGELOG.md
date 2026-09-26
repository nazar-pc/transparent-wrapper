# 0.1.0

Initial release.

Features:

* `#[derive(TransparentWrapper)]` for `#[repr(transparent)]` and `#[repr(C)]` structs with exactly one field
* `TransparentWrapperBase` trait that the derive implements, also implemented for arrays and `MaybeUninit` of
  transparent wrappers
* `TransparentWrapper` trait implemented for all `TransparentWrapperBase` implementations with conversions between
  wrappers and their inner types for values, shared/exclusive references and shared/exclusive slices
* The same conversions as free `const fn`s that work in `const` context on stable Rust
* `const-trait` feature that makes `TransparentWrapper` a `const trait` on nightly Rust
