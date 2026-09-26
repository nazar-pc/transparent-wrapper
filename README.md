# transparent-wrapper

[![Crates.io](https://img.shields.io/crates/v/transparent-wrapper.svg)](https://crates.io/crates/transparent-wrapper)
[![Docs](https://docs.rs/transparent-wrapper/badge.svg)](https://docs.rs/transparent-wrapper)

Zero-cost conversions between transparent wrappers and their inner types, usable in `const fn`.

A newtype like `struct Hash([u8; 32])` is often needed as `&[u8; 32]`, `&[[u8; 32]]` or `&mut [MaybeUninit<[u8; 32]>]`
and back, which usually ends up being done with `mem::transmute()` and a hand-written safety proof at every call site.
This crate moves the proof into a single `unsafe` trait implementation, which can be derived, and provides safe
conversions on top of it for values, references, slices, arrays and `MaybeUninit`.

```rust
use core::mem::MaybeUninit;
use transparent_wrapper::TransparentWrapper;

#[derive(TransparentWrapper)]
#[repr(transparent)]
struct Hash([u8; 32]);

let bytes = [[1; 32], [2; 32]];

// References and slices
let hash = Hash::wrap_ref(&bytes[0]);
assert_eq!(hash.peel_ref(), &bytes[0]);
let hashes = Hash::wrap_slice(&bytes);
assert_eq!(Hash::peel_slice(hashes), &bytes);
// Arrays and `MaybeUninit` are transparent wrappers too if their elements are
let hashes = <[Hash; 2]>::wrap_ref(&bytes);
let mut buffer = [const { MaybeUninit::<Hash>::uninit() }; 2];
let buffer_bytes = MaybeUninit::<Hash>::peel_slice_mut(&mut buffer);
```

Every conversion is also available as a free `const fn` (like `transparent_wrapper::wrap_slice()`) that works in `const`
context on stable Rust. With `const-trait` feature, which requires nightly Rust, `TransparentWrapper` becomes a
`const trait`, so its methods can be called in `const fn` directly.

The crate is `no_std` and works on stable Rust 1.85 or newer.
