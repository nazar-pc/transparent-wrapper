use crate::expand;
use quote::quote;

fn expand_err(input: proc_macro2::TokenStream) -> String {
    expand(input).expect_err("Must fail to expand").to_string()
}

#[test]
fn transparent_tuple_struct() {
    let output = expand(quote! {
        #[repr(transparent)]
        struct Hash([u8; 32]);
    })
    .unwrap();

    assert_eq!(
        output.to_string(),
        quote! {
            unsafe impl ::transparent_wrapper::TransparentWrapperBase for Hash {
                type Inner = [u8; 32];
            }
        }
        .to_string()
    );
}

#[test]
fn c_struct_with_named_field_and_generics() {
    let output = expand(quote! {
        #[derive(Clone)]
        #[repr(C)]
        pub struct Wrapper<'a, T: Copy>
        where
            T: Default,
        {
            inner: &'a [T],
        }
    })
    .unwrap();

    assert_eq!(
        output.to_string(),
        quote! {
            unsafe impl<'a, T: Copy> ::transparent_wrapper::TransparentWrapperBase
                for Wrapper<'a, T>
            where
                T: Default,
            {
                type Inner = &'a [T];
            }
        }
        .to_string()
    );
}

#[test]
fn missing_repr() {
    assert_eq!(
        expand_err(quote! {
            struct Hash([u8; 32]);
        }),
        "`#[derive(TransparentWrapper)]` requires `#[repr(transparent)]` or `#[repr(C)]`"
    );
}

#[test]
fn layout_changing_repr() {
    let expected = "`#[derive(TransparentWrapper)]` only supports `#[repr(transparent)]` and \
        `#[repr(C)]` without other modifiers";

    assert_eq!(
        expand_err(quote! {
            #[repr(C, align(8))]
            struct Hash([u8; 32]);
        }),
        expected
    );
    assert_eq!(
        expand_err(quote! {
            #[repr(C)]
            #[repr(packed)]
            struct Hash([u16; 16]);
        }),
        expected
    );
    assert_eq!(
        expand_err(quote! {
            #[repr(Rust)]
            struct Hash([u8; 32]);
        }),
        expected
    );
}

#[test]
fn wrong_number_of_fields() {
    let expected = "`#[derive(TransparentWrapper)]` requires a struct with exactly one field";

    assert_eq!(
        expand_err(quote! {
            #[repr(transparent)]
            struct Unit;
        }),
        expected
    );
    assert_eq!(
        expand_err(quote! {
            #[repr(C)]
            struct Pair(u8, u8);
        }),
        expected
    );
}

#[test]
fn not_a_struct() {
    let expected = "`#[derive(TransparentWrapper)]` only supports structs";

    assert_eq!(
        expand_err(quote! {
            #[repr(transparent)]
            enum Single {
                Variant(u8),
            }
        }),
        expected
    );
    assert_eq!(
        expand_err(quote! {
            #[repr(C)]
            union Single {
                field: u8,
            }
        }),
        expected
    );
}
