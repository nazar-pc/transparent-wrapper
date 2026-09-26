#![no_std]
#![cfg_attr(feature = "const-trait", feature(const_trait_impl))]

//! Zero-cost conversions between transparent wrappers and their inner types.
//!
//! A newtype like `struct Hash([u8; 32])` is often needed as `&[u8; 32]`, `&[[u8; 32]]` or
//! `&mut [MaybeUninit<[u8; 32]>]` and back, which usually ends up being done with
//! [`mem::transmute()`](core::mem::transmute) and a hand-written safety proof at every call site.
//! This crate moves the proof into a single `unsafe` trait implementation, which can be derived,
//! and provides safe conversions on top of it for values, references, slices, arrays and
//! [`MaybeUninit`].
//!
//! ```
//! use core::mem::MaybeUninit;
//! use transparent_wrapper::TransparentWrapper;
//!
//! #[derive(TransparentWrapper)]
//! #[repr(transparent)]
//! struct Hash([u8; 32]);
//!
//! let bytes = [[1; 32], [2; 32]];
//!
//! // References and slices
//! let hash = Hash::wrap_ref(&bytes[0]);
//! assert_eq!(hash.peel_ref(), &bytes[0]);
//! let hashes = Hash::wrap_slice(&bytes);
//! assert_eq!(Hash::peel_slice(hashes), &bytes);
//! // Arrays and `MaybeUninit` are transparent wrappers too if their elements are
//! let hashes = <[Hash; 2]>::wrap_ref(&bytes);
//! let mut buffer = [const { MaybeUninit::<Hash>::uninit() }; 2];
//! let buffer_bytes = MaybeUninit::<Hash>::peel_slice_mut(&mut buffer);
//! # let _ = (hash, hashes, buffer_bytes);
//! ```
//!
//! # `const fn` support
//!
//! Trait methods can't be called in `const fn` on stable Rust, which is why every conversion is
//! also available as a free `const fn` with the same name:
//!
//! ```
//! use transparent_wrapper::TransparentWrapper;
//!
//! #[derive(TransparentWrapper)]
//! #[repr(transparent)]
//! struct Hash([u8; 32]);
//!
//! impl Hash {
//!     const fn slice_from_repr(value: &[[u8; 32]]) -> &[Self] {
//!         transparent_wrapper::wrap_slice(value)
//!     }
//! }
//!
//! const HASHES: &[Hash] = Hash::slice_from_repr(&[[1; 32], [2; 32]]);
//! assert_eq!(HASHES.len(), 2);
//! ```
//!
//! With the `const-trait` feature, which requires nightly Rust, [`TransparentWrapper`] becomes a
//! `const trait`, so its methods can be called in `const fn` directly. Crates calling them in
//! `const fn` need to enable `const_trait_impl` feature themselves, while crates that only derive
//! [`TransparentWrapper`] or call its methods at runtime don't need any changes.
//!
//! ```ignore
//! #![feature(const_trait_impl)]
//!
//! use transparent_wrapper::TransparentWrapper;
//!
//! #[derive(TransparentWrapper)]
//! #[repr(transparent)]
//! struct Hash([u8; 32]);
//!
//! const HASHES: &[Hash] = Hash::wrap_slice(&[[1; 32], [2; 32]]);
//! assert_eq!(HASHES.len(), 2);
//! ```
//!
//! # Deriving
//!
//! `#[derive(TransparentWrapper)]` implements [`TransparentWrapperBase`] (and through it
//! [`TransparentWrapper`]) for structs with exactly one field that are `#[repr(transparent)]` or
//! `#[repr(C)]`. Any value of the field becomes accessible as the wrapper and the other way
//! around, including through `&mut`, so only derive it for types that don't have any invariants
//! on top of their field.
//!
//! Structs without an appropriate `#[repr]` are rejected:
//!
//! ```compile_fail
//! use transparent_wrapper::TransparentWrapper;
//!
//! #[derive(TransparentWrapper)]
//! struct Hash([u8; 32]);
//! ```
//!
//! As well as structs with more than one field:
//!
//! ```compile_fail
//! use transparent_wrapper::TransparentWrapper;
//!
//! #[derive(TransparentWrapper)]
//! #[repr(C)]
//! struct Pair(u32, u32);
//! ```

use core::mem::{ManuallyDrop, MaybeUninit};
use core::{mem, ptr, slice};
pub use transparent_wrapper_derive::TransparentWrapper;

/// Base trait of [`TransparentWrapper`], describing a type that has the same layout and validity
/// as its inner type.
///
/// This is what `#[derive(TransparentWrapper)]` implements, [`TransparentWrapper`] with all the
/// conversions is then implemented automatically.
///
/// # Safety
/// `Self` must have the same size and alignment as [`Self::Inner`], which is the case for
/// `#[repr(transparent)]` and `#[repr(C)]` structs with a single field of type [`Self::Inner`].
/// Any valid value of [`Self::Inner`] must be a valid value of `Self` and the other way around,
/// meaning `Self` can't have any additional invariants that code relies on for soundness.
pub unsafe trait TransparentWrapperBase: Sized {
    /// The wrapped type
    type Inner: Sized;
}

// SAFETY: Arrays of transparent wrappers have the same layout and validity as arrays of their
// inner types
unsafe impl<W, const N: usize> TransparentWrapperBase for [W; N]
where
    W: TransparentWrapperBase,
{
    type Inner = [W::Inner; N];
}

// SAFETY: `MaybeUninit<T>` has the same layout as `T`, so the layout of transparent wrappers is
// preserved, and any bit pattern is valid for any `MaybeUninit`
unsafe impl<W> TransparentWrapperBase for MaybeUninit<W>
where
    W: TransparentWrapperBase,
{
    type Inner = MaybeUninit<W::Inner>;
}

/// Checks the part of [`TransparentWrapperBase`] contract that can be checked at compile time
#[inline(always)]
const fn assert_same_layout<W>()
where
    W: TransparentWrapperBase,
{
    const {
        assert!(
            size_of::<W>() == size_of::<W::Inner>(),
            "Transparent wrapper must have the same size as its inner type"
        );
        assert!(
            align_of::<W>() == align_of::<W::Inner>(),
            "Transparent wrapper must have the same alignment as its inner type"
        );
    }
}

/// Wrap a value
#[inline(always)]
pub const fn wrap<W>(inner: W::Inner) -> W
where
    W: TransparentWrapperBase,
{
    assert_same_layout::<W>();
    let inner = ManuallyDrop::new(inner);
    // SAFETY: Same layout and validity according to `TransparentWrapperBase` contract, the
    // original value is not dropped
    unsafe { mem::transmute_copy::<ManuallyDrop<W::Inner>, W>(&inner) }
}

/// Unwrap a value
#[inline(always)]
pub const fn peel<W>(wrapper: W) -> W::Inner
where
    W: TransparentWrapperBase,
{
    assert_same_layout::<W>();
    let wrapper = ManuallyDrop::new(wrapper);
    // SAFETY: Same layout and validity according to `TransparentWrapperBase` contract, the
    // original value is not dropped
    unsafe { mem::transmute_copy::<ManuallyDrop<W>, W::Inner>(&wrapper) }
}

/// Wrap a shared reference
#[inline(always)]
pub const fn wrap_ref<W>(inner: &W::Inner) -> &W
where
    W: TransparentWrapperBase,
{
    assert_same_layout::<W>();
    // SAFETY: Same layout and validity according to `TransparentWrapperBase` contract
    unsafe { &*ptr::from_ref(inner).cast::<W>() }
}

/// Unwrap a shared reference
#[inline(always)]
pub const fn peel_ref<W>(wrapper: &W) -> &W::Inner
where
    W: TransparentWrapperBase,
{
    assert_same_layout::<W>();
    // SAFETY: Same layout and validity according to `TransparentWrapperBase` contract
    unsafe { &*ptr::from_ref(wrapper).cast::<W::Inner>() }
}

/// Wrap an exclusive reference
#[inline(always)]
pub const fn wrap_mut<W>(inner: &mut W::Inner) -> &mut W
where
    W: TransparentWrapperBase,
{
    assert_same_layout::<W>();
    // SAFETY: Same layout and validity according to `TransparentWrapperBase` contract
    unsafe { &mut *ptr::from_mut(inner).cast::<W>() }
}

/// Unwrap an exclusive reference
#[inline(always)]
pub const fn peel_mut<W>(wrapper: &mut W) -> &mut W::Inner
where
    W: TransparentWrapperBase,
{
    assert_same_layout::<W>();
    // SAFETY: Same layout and validity according to `TransparentWrapperBase` contract
    unsafe { &mut *ptr::from_mut(wrapper).cast::<W::Inner>() }
}

/// Wrap a shared slice
#[inline(always)]
pub const fn wrap_slice<W>(inner: &[W::Inner]) -> &[W]
where
    W: TransparentWrapperBase,
{
    assert_same_layout::<W>();
    // SAFETY: Same layout and validity according to `TransparentWrapperBase` contract
    unsafe { slice::from_raw_parts(inner.as_ptr().cast::<W>(), inner.len()) }
}

/// Unwrap a shared slice
#[inline(always)]
pub const fn peel_slice<W>(wrapper: &[W]) -> &[W::Inner]
where
    W: TransparentWrapperBase,
{
    assert_same_layout::<W>();
    // SAFETY: Same layout and validity according to `TransparentWrapperBase` contract
    unsafe { slice::from_raw_parts(wrapper.as_ptr().cast::<W::Inner>(), wrapper.len()) }
}

/// Wrap an exclusive slice
#[inline(always)]
pub const fn wrap_slice_mut<W>(inner: &mut [W::Inner]) -> &mut [W]
where
    W: TransparentWrapperBase,
{
    assert_same_layout::<W>();
    // SAFETY: Same layout and validity according to `TransparentWrapperBase` contract
    unsafe { slice::from_raw_parts_mut(inner.as_mut_ptr().cast::<W>(), inner.len()) }
}

/// Unwrap an exclusive slice
#[inline(always)]
pub const fn peel_slice_mut<W>(wrapper: &mut [W]) -> &mut [W::Inner]
where
    W: TransparentWrapperBase,
{
    assert_same_layout::<W>();
    // SAFETY: Same layout and validity according to `TransparentWrapperBase` contract
    unsafe { slice::from_raw_parts_mut(wrapper.as_mut_ptr().cast::<W::Inner>(), wrapper.len()) }
}

/// Defines [`TransparentWrapper`], optionally as a `const trait`.
///
/// This is a macro because `const trait` syntax is rejected by stable Rust even in code that is
/// disabled with `#[cfg]`, but not in unexpanded macro invocations.
macro_rules! define_transparent_wrapper {
    ($($const:ident)?) => {
        /// Conversions between a transparent wrapper and its inner type.
        ///
        /// Implemented automatically for all types that implement [`TransparentWrapperBase`], see
        /// crate-level documentation for examples. The same conversions are available as free
        /// `const fn`s, see [`wrap()`] and others.
        pub $($const)? trait TransparentWrapper: TransparentWrapperBase {
            /// Wrap a value
            #[inline(always)]
            fn wrap(inner: Self::Inner) -> Self {
                wrap::<Self>(inner)
            }

            /// Unwrap a value
            #[inline(always)]
            fn peel(self) -> Self::Inner {
                peel::<Self>(self)
            }

            /// Wrap a shared reference
            #[inline(always)]
            fn wrap_ref(inner: &Self::Inner) -> &Self {
                wrap_ref::<Self>(inner)
            }

            /// Unwrap a shared reference
            #[inline(always)]
            fn peel_ref(&self) -> &Self::Inner {
                peel_ref::<Self>(self)
            }

            /// Wrap an exclusive reference
            #[inline(always)]
            fn wrap_mut(inner: &mut Self::Inner) -> &mut Self {
                wrap_mut::<Self>(inner)
            }

            /// Unwrap an exclusive reference
            #[inline(always)]
            fn peel_mut(&mut self) -> &mut Self::Inner {
                peel_mut::<Self>(self)
            }

            /// Wrap a shared slice
            #[inline(always)]
            fn wrap_slice(inner: &[Self::Inner]) -> &[Self] {
                wrap_slice::<Self>(inner)
            }

            /// Unwrap a shared slice
            #[inline(always)]
            fn peel_slice(wrapper: &[Self]) -> &[Self::Inner] {
                peel_slice::<Self>(wrapper)
            }

            /// Wrap an exclusive slice
            #[inline(always)]
            fn wrap_slice_mut(inner: &mut [Self::Inner]) -> &mut [Self] {
                wrap_slice_mut::<Self>(inner)
            }

            /// Unwrap an exclusive slice
            #[inline(always)]
            fn peel_slice_mut(wrapper: &mut [Self]) -> &mut [Self::Inner] {
                peel_slice_mut::<Self>(wrapper)
            }
        }

        $($const)? impl<W> TransparentWrapper for W where W: TransparentWrapperBase {}
    };
}

#[cfg(feature = "const-trait")]
define_transparent_wrapper!(const);
#[cfg(not(feature = "const-trait"))]
define_transparent_wrapper!();
