use proc_macro::TokenStream;
use syn::{
    parse_macro_input,
    Item,
};

#[proc_macro_attribute]
pub fn sorted(args: TokenStream, input: TokenStream) -> TokenStream {
    let _ = args;

    let input_clone = input.clone();
    let _item: Item = parse_macro_input!(input_clone);
    input
}
