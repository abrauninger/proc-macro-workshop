use proc_macro::TokenStream;
use syn::{parse_macro_input, DeriveInput};

mod bitfield;
use crate::bitfield::bitfield_impl;

mod gen_bit_width_types;
use crate::gen_bit_width_types::gen_bit_width_types_impl;

#[proc_macro_attribute]
pub fn bitfield(_args: TokenStream, input: TokenStream) -> TokenStream {
    match bitfield_impl(input) {
        Ok(output) => output,
        Err(err) => err.to_compile_error().into()
    }
}

#[proc_macro_derive(BitfieldSpecifier)]
pub fn bitfield_specifier_derive(input: TokenStream) -> TokenStream {
    let derive_input = parse_macro_input!(input as DeriveInput);

    let DeriveInput { ident: _enum_name, data: _a, .. } = derive_input;

    TokenStream::new()
}

#[proc_macro]
pub fn gen_bit_width_types(input: TokenStream) -> TokenStream {
    match gen_bit_width_types_impl(input) {
        Ok(output) => output,
        Err(err) => err.to_compile_error().into()
    }
}