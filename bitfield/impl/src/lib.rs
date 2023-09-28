use proc_macro::TokenStream;
use quote::{quote, format_ident};
use syn::{
    parse::{Parse, ParseStream}, LitInt, Token
};

#[proc_macro_attribute]
pub fn bitfield(args: TokenStream, input: TokenStream) -> TokenStream {
    let _ = args;
    let _ = input;

    unimplemented!()
}

#[proc_macro]
pub fn gen_bit_width_types(input: TokenStream) -> TokenStream {
    match gen_bit_width_types_impl(input) {
        Ok(output) => output,
        Err(err) => err.to_compile_error().into()
    }
}

fn gen_bit_width_types_impl(input: TokenStream) -> syn::Result<TokenStream> {
    let GenBitWidthTypesInput { start, end } = syn::parse(input)?;
    assert!(end >= start);
    let type_count = end - start + 1;

    let mut types = Vec::new();
    types.reserve(type_count.into());

    for bit_width in start..=end {
        let type_name = format_ident!("B{}", bit_width);
        types.push(quote!{
            pub enum #type_name {}

            impl Specifier for #type_name {
                const BITS: u16 = #bit_width;
            }
        });
    }

    Ok(quote! {
        #(#types)*
    }.into())
}

struct GenBitWidthTypesInput {
    start: u16,
    end: u16,
}

impl Parse for GenBitWidthTypesInput {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let start = input.parse::<LitInt>()?.base10_parse::<u16>()?;

        let _: Token![..=] = input.parse()?;

        let end = input.parse::<LitInt>()?.base10_parse::<u16>()?;

        if end < start {
            return Err(input.error("'end' must be greater than or equal to 'start'"));
        }

        Ok(Self { start, end })
    }
}