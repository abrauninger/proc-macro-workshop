use proc_macro::TokenStream;
use proc_macro2::Span;
use syn::{
    Item::{self, Enum},
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

    if let Enum(_) = item {
        Ok(input)
    } else {
        Err(syn::Error::new(Span::call_site(), "expected enum or match expression"))
    }
}