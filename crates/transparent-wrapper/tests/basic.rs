//! Tests for conversions provided by `transparent-wrapper`

use core::cell::Cell;
use core::marker::PhantomData;
use core::mem::MaybeUninit;
use transparent_wrapper::TransparentWrapper;

#[derive(Debug, Copy, Clone, Eq, PartialEq, TransparentWrapper)]
#[repr(transparent)]
struct Hash([u8; 32]);

#[derive(Debug, Copy, Clone, Eq, PartialEq, TransparentWrapper)]
#[repr(C)]
struct Index {
    value: u64,
}

#[derive(Debug, Eq, PartialEq, TransparentWrapper)]
#[repr(transparent)]
struct Generic<T>(T);

#[derive(Debug, Eq, PartialEq, TransparentWrapper)]
#[repr(transparent)]
struct Borrowed<'a, T>(&'a T)
where
    T: ?Sized;

#[derive(Debug, Eq, PartialEq, TransparentWrapper)]
#[repr(transparent)]
struct Nested(Hash);

#[derive(Debug, Eq, PartialEq, TransparentWrapper)]
#[repr(transparent)]
struct ZeroSized(PhantomData<u8>);

#[test]
fn values() {
    assert_eq!(Hash::wrap([1; 32]), Hash([1; 32]));
    assert_eq!(Hash([2; 32]).peel(), [2; 32]);
    assert_eq!(Index::wrap(3), Index { value: 3 });
    assert_eq!(Index { value: 4 }.peel(), 4);
    assert_eq!(Generic::wrap(5_u16), Generic(5));
    assert_eq!(Nested::wrap(Hash([6; 32])), Nested(Hash([6; 32])));
    assert_eq!(ZeroSized::wrap(PhantomData), ZeroSized(PhantomData));

    assert_eq!(transparent_wrapper::wrap::<Hash>([7; 32]), Hash([7; 32]));
    assert_eq!(transparent_wrapper::peel(Hash([8; 32])), [8; 32]);
}

#[test]
fn references() {
    let mut bytes = [1; 32];
    assert_eq!(Hash::wrap_ref(&bytes), &Hash([1; 32]));
    Hash::wrap_mut(&mut bytes).0[0] = 2;
    assert_eq!(bytes[0], 2);

    let mut hash = Hash([3; 32]);
    assert_eq!(hash.peel_ref(), &[3; 32]);
    hash.peel_mut()[1] = 4;
    assert_eq!(hash.0[1], 4);

    let value = String::from("value");
    let value_ref = &value;
    let borrowed = Borrowed::wrap_ref(&value_ref);
    assert_eq!(borrowed.0, "value");
}

#[test]
fn slices() {
    let mut bytes = [[1; 32], [2; 32], [3; 32]];
    assert_eq!(
        Hash::wrap_slice(&bytes),
        &[Hash([1; 32]), Hash([2; 32]), Hash([3; 32])]
    );
    Hash::wrap_slice_mut(&mut bytes)[1].0[0] = 4;
    assert_eq!(bytes[1][0], 4);

    let mut hashes = [Hash([5; 32]), Hash([6; 32])];
    assert_eq!(Hash::peel_slice(&hashes), &[[5; 32], [6; 32]]);
    Hash::peel_slice_mut(&mut hashes)[0][0] = 7;
    assert_eq!(hashes[0].0[0], 7);

    assert_eq!(Hash::wrap_slice(&[]), []);
    assert_eq!(ZeroSized::wrap_slice(&[PhantomData; 3]).len(), 3);
}

#[test]
fn arrays() {
    let mut bytes = [[1; 32], [2; 32]];
    assert_eq!(
        <[Hash; 2]>::wrap_ref(&bytes),
        &[Hash([1; 32]), Hash([2; 32])]
    );
    <[Hash; 2]>::wrap_mut(&mut bytes)[0].0[0] = 3;
    assert_eq!(bytes[0][0], 3);

    let hashes = <[Hash; 2]>::wrap(bytes);
    assert_eq!(hashes, [Hash(bytes[0]), Hash(bytes[1])]);
    assert_eq!(hashes.peel(), bytes);

    let nested_bytes = [[[4; 32]; 2]; 3];
    assert_eq!(
        <[[Hash; 2]; 3]>::wrap_slice(&[nested_bytes]),
        &[[[Hash([4; 32]); 2]; 3]]
    );
}

#[test]
fn maybe_uninit() {
    let mut hashes = [const { MaybeUninit::<Hash>::uninit() }; 2];
    for (hash_bytes, byte) in MaybeUninit::<Hash>::peel_slice_mut(&mut hashes)
        .iter_mut()
        .zip([1, 2])
    {
        hash_bytes.write([byte; 32]);
    }
    // SAFETY: Just initialized
    let hashes = hashes.map(|hash| unsafe { hash.assume_init() });
    assert_eq!(hashes, [Hash([1; 32]), Hash([2; 32])]);

    let mut bytes = [const { MaybeUninit::<[u8; 32]>::uninit() }; 2];
    let wrapped = MaybeUninit::<Hash>::wrap_slice_mut(&mut bytes);
    wrapped[0].write(Hash([3; 32]));
    wrapped[1].write(Hash([4; 32]));
    // SAFETY: Just initialized
    let bytes = bytes.map(|hash_bytes| unsafe { hash_bytes.assume_init() });
    assert_eq!(bytes, [[3; 32], [4; 32]]);
}

#[test]
fn values_are_dropped_exactly_once() {
    struct DropCounter<'a>(&'a Cell<usize>);

    impl Drop for DropCounter<'_> {
        fn drop(&mut self) {
            self.0.set(self.0.get() + 1);
        }
    }

    let drops = Cell::new(0);

    let wrapped = Generic::wrap(DropCounter(&drops));
    assert_eq!(drops.get(), 0);
    let peeled = wrapped.peel();
    assert_eq!(drops.get(), 0);
    drop(peeled);
    assert_eq!(drops.get(), 1);

    let wrapped = <[Generic<DropCounter<'_>>; 2]>::wrap([DropCounter(&drops), DropCounter(&drops)]);
    assert_eq!(drops.get(), 1);
    drop(wrapped);
    assert_eq!(drops.get(), 3);
}

#[test]
fn const_fns() {
    const BYTES: &[[u8; 32]] = &[[1; 32], [2; 32]];
    const HASHES: &[Hash] = transparent_wrapper::wrap_slice(BYTES);
    const HASH: Hash = transparent_wrapper::wrap([3; 32]);
    const HASH_BYTES: &[u8; 32] = transparent_wrapper::peel_ref(&Hash([4; 32]));
    const INDEX: u64 = transparent_wrapper::peel(Index { value: 5 });
    const HASH_ARRAY: [Hash; 2] = transparent_wrapper::wrap([[6; 32], [7; 32]]);
    const MODIFIED: [u8; 32] = {
        let mut hash = Hash([8; 32]);
        transparent_wrapper::peel_slice_mut(core::slice::from_mut(&mut hash))[0][0] = 9;
        transparent_wrapper::peel_mut(&mut hash)[1] = 10;
        hash.0
    };

    assert_eq!(HASHES, &[Hash([1; 32]), Hash([2; 32])]);
    assert_eq!(HASH, Hash([3; 32]));
    assert_eq!(HASH_BYTES, &[4; 32]);
    assert_eq!(INDEX, 5);
    assert_eq!(HASH_ARRAY, [Hash([6; 32]), Hash([7; 32])]);
    assert_eq!(MODIFIED[..3], [9, 10, 8]);
}
