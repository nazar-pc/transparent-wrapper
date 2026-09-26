//! Implements the `#[derive(TransparentWrapper)]` macro, see `transparent-wrapper` crate for public
//! documentation and usage

#[cfg(test)]
mod tests;

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{Data, DeriveInput, Error};

/// See `transparent-wrapper` crate for documentation.
#[proc_macro_derive(TransparentWrapper)]
pub fn transparent_wrapper(input: TokenStream) -> TokenStream {
    expand(TokenStream2::from(input))
        .unwrap_or_else(Error::into_compile_error)
        .into()
}

fn expand(input: TokenStream2) -> syn::Result<TokenStream2> {
    let input = syn::parse2::<DeriveInput>(input)?;

    let fields = match &input.data {
        Data::Struct(data) => &data.fields,
        Data::Enum(_) | Data::Union(_) => {
            return Err(Error::new_spanned(
                &input.ident,
                "`#[derive(TransparentWrapper)]` only supports structs",
            ));
        }
    };

    check_repr(&input)?;

    let mut fields_iter = fields.iter();
    let (Some(field), None) = (fields_iter.next(), fields_iter.next()) else {
        return Err(Error::new_spanned(
            &input.ident,
            "`#[derive(TransparentWrapper)]` requires a struct with exactly one field",
        ));
    };

    let ident = &input.ident;
    let inner = &field.ty;
    let (impl_generics, type_generics, where_clause) = input.generics.split_for_impl();

    Ok(quote! {
        unsafe impl #impl_generics ::transparent_wrapper::TransparentWrapperBase
            for #ident #type_generics #where_clause
        {
            type Inner = #inner;
        }
    })
}

/// Checks that the struct is `#[repr(transparent)]` or `#[repr(C)]` without any other modifiers
/// that could change its layout, which (together with a single field) guarantees the same layout
/// as the field
fn check_repr(input: &DeriveInput) -> syn::Result<()> {
    let mut supported_repr = false;

    for attr in &input.attrs {
        if !attr.path().is_ident("repr") {
            continue;
        }

        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("transparent") || meta.path.is_ident("C") {
                supported_repr = true;
                Ok(())
            } else {
                Err(meta.error(
                    "`#[derive(TransparentWrapper)]` only supports `#[repr(transparent)]` and \
                    `#[repr(C)]` without other modifiers",
                ))
            }
        })?;
    }

    if supported_repr {
        Ok(())
    } else {
        Err(Error::new_spanned(
            &input.ident,
            "`#[derive(TransparentWrapper)]` requires `#[repr(transparent)]` or `#[repr(C)]`",
        ))
    }
}
