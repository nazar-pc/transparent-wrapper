//! Tests for `const-trait` feature, which makes `TransparentWrapper` methods callable in `const fn`

#![cfg(feature = "const-trait")]
#![cfg_attr(feature = "const-trait", feature(const_trait_impl))]

// `[const]` bounds are rejected by stable Rust even in code disabled with `#[cfg]`, hence a
// separate module, which is not loaded at all without `const-trait` feature
#[path = "const_trait/generic.rs"]
mod generic;

use crate::generic::generic_peel;
use core::mem::MaybeUninit;
use transparent_wrapper::TransparentWrapper;

#[derive(Debug, Copy, Clone, Eq, PartialEq, TransparentWrapper)]
#[repr(transparent)]
struct Hash([u8; 32]);

impl Hash {
    const fn slice_from_repr(value: &[[u8; 32]]) -> &[Self] {
        Self::wrap_slice(value)
    }
}

#[test]
fn methods_in_const_fn() {
    const HASHES: &[Hash] = Hash::slice_from_repr(&[[1; 32], [2; 32]]);
    const HASH: Hash = Hash::wrap([3; 32]);
    const HASH_PEELED: [u8; 32] = Hash([7; 32]).peel();
    const HASH_BYTES: &[u8; 32] = generic_peel(&Hash([4; 32]));
    const ARRAY: [Hash; 2] = <[Hash; 2]>::wrap([[5; 32], [6; 32]]);
    const UNINIT_LEN: usize = {
        let mut buffer = [const { MaybeUninit::<Hash>::uninit() }; 3];
        MaybeUninit::<Hash>::peel_slice_mut(&mut buffer).len()
    };

    assert_eq!(HASHES, &[Hash([1; 32]), Hash([2; 32])]);
    assert_eq!(HASH, Hash([3; 32]));
    assert_eq!(HASH_PEELED, [7; 32]);
    assert_eq!(HASH_BYTES, &[4; 32]);
    assert_eq!(ARRAY, [Hash([5; 32]), Hash([6; 32])]);
    assert_eq!(UNINIT_LEN, 3);
}
