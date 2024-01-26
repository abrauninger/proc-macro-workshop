use if_chain::if_chain;
use proc_macro::TokenStream;
use quote::quote;
use std::collections::HashMap;
use syn::{
    Data,
    DeriveInput,
    Expr,
    Ident,
    Lit,
    Variant,
    punctuated::Punctuated,
    token::Comma,
};

pub fn bitfield_specifier_derive_impl(input: TokenStream) -> syn::Result<TokenStream> {
    let DeriveInput { ident: enum_name, data, .. }: DeriveInput = syn::parse(input)?;

    if let Data::Enum(data) = data {
        let variants = enum_variants(data.variants)?;

        let maximum_discriminant: &u32 = variants
            .iter()
            .max_by(|a, b| { a.1.cmp(b.1) })
            .unwrap()
            .1;

        let size_bits = (std::mem::size_of::<u32>() * 8) - (maximum_discriminant.leading_zeros() as usize);

        let size_bytes = (size_bits + 7) / 8;

        let deserialize_match_arms: Vec<_> = variants
            .iter()
            .map(|(ident, value)| {
                quote! {
                    #value => #enum_name::#ident,
                }
            })
            .collect();

        let panic_string = format!("unexpected value for `{}`: {{}}", enum_name);

        Ok(quote! {
            impl ::bitfield::Specifier for #enum_name {
                const BITS: usize = #size_bits;
                type ACCESSOR = #enum_name;
            }

            impl ::bitfield::Serialize<#size_bytes> for #enum_name {
                type Type = #enum_name;

                fn serialize(t: #enum_name) -> [u8; #size_bytes] {
                    [t as u8]
                }

                fn deserialize(bytes: [u8; #size_bytes]) -> #enum_name {
                    match bytes[0] as u32 {
                        #(#deserialize_match_arms)*
                        value => panic!(#panic_string, value)
                    }
                }
            }
        }.into())
    } else {
        Err(syn::Error::new(enum_name.span(), "`BitfieldSpecifier` should only be used on enums"))
    }
}

fn enum_variants(variants: Punctuated<Variant, Comma>) -> syn::Result<HashMap<Ident, u32>> {
    let mut hashmap = HashMap::new();

    for variant in variants.iter() {
        if_chain! {
            if let Some((_, discriminant)) = &variant.discriminant;
            if let Expr::Lit(discriminant) = discriminant;
            if let Lit::Int(discriminant) = &discriminant.lit;
            if let Ok(value) = discriminant.base10_parse::<u32>();
            then {
                hashmap.insert(variant.ident.clone(), value);
            } else {
                return Err(syn::Error::new(variant.ident.span(), "every variant in an `BitfieldSpecifier` enum must have an explicit integer discriminant"));
            }
        }
    }

    Ok(hashmap)
}