use proc_macro::TokenStream;
use quote::quote;
use syn::{
    parse_macro_input,
    Item::{self, Enum},
};

#[proc_macro_attribute]
pub fn sorted(_args: TokenStream, input: TokenStream) -> TokenStream {
    let input_clone = input.clone();
    let item: Item = parse_macro_input!(input_clone);

    if let Enum(_) = item {
        input
    } else {
        quote! {
            // We emit 'compile_error' directly so that the span is the proc-macro usage site itself
            compile_error!("expected enum or match expression");
        }.into()
    }
}
