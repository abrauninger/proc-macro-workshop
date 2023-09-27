use proc_macro::TokenStream;
use proc_macro2::Span;
use syn::{
    parse_macro_input,
    Item::{self, Enum},
};

#[proc_macro_attribute]
pub fn sorted(_args: TokenStream, input: TokenStream) -> TokenStream {
    let input_clone = input.clone();
    let item: Item = parse_macro_input!(input_clone);

    match sorted_impl(input, item) {
        Ok(output) => output,
        Err(err) => err.to_compile_error().into()
    }
}

fn sorted_impl(input: TokenStream, item: Item) -> syn::Result<TokenStream> {
    if let Enum(_) = item {
        Ok(input)
    } else {
        Err(syn::Error::new(Span::call_site(), "expected enum or match expression"))
    }
}