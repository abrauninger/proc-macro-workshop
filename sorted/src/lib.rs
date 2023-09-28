use proc_macro::TokenStream;
use proc_macro2::Span;
use syn::{
    Item::{self, Enum}, Ident,
};

#[proc_macro_attribute]
pub fn sorted(_args: TokenStream, input: TokenStream) -> TokenStream {
    match sorted_impl(input) {
        Ok(output) => output,
        Err(err) => err.to_compile_error().into()
    }
}

fn sorted_impl(input: TokenStream) -> syn::Result<TokenStream> {
    let item: Item = syn::parse(input.clone())?;

    if let Enum(item_enum) = item {
        let mut previous_variant_ident: Option<&Ident> = None;

        for variant in &item_enum.variants {
            if let Some(previous_variant_ident) = previous_variant_ident {
                if variant.ident < *previous_variant_ident {
                    let sort_before_variant = item_enum.variants.iter().find(|v| { v.ident > variant.ident }).unwrap();
                    return Err(syn::Error::new_spanned(&variant.ident, format!("{} should sort before {}", variant.ident, sort_before_variant.ident)));
                }
            }

            previous_variant_ident = Some(&variant.ident);
        }

        Ok(input)
    } else {
        Err(syn::Error::new(Span::call_site(), "expected enum or match expression"))
    }
}