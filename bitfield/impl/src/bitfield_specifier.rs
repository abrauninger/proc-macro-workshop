use proc_macro::TokenStream;
use quote::quote;
use syn::DeriveInput;

pub fn bitfield_specifier_derive_impl(input: TokenStream) -> syn::Result<TokenStream> {
    let DeriveInput { ident: enum_name, data: _a, .. }: DeriveInput = syn::parse(input)?;

    Ok(quote! {
        impl ::bitfield::Specifier for #enum_name {
            const BITS: usize = 8;
            type ACCESSOR = #enum_name;
        }
    }.into())
}