use proc_macro::TokenStream;
use quote::{quote, format_ident};
use syn::{
    parse::{Parse, ParseStream}, LitInt, Token,
};

pub fn gen_bit_width_types_impl(input: TokenStream) -> syn::Result<TokenStream> {
    let GenBitWidthTypesInput { start, end } = syn::parse(input)?;
    assert!(end >= start);
    let type_count = end - start + 1;

    let mut types = Vec::new();
    types.reserve(type_count.into());

    for bit_width in start..=end {
        let type_name = format_ident!("B{}", bit_width);
        let accessor_type_size = std::cmp::max(bit_width.next_power_of_two(), 8);
        let accessor_type_name = format_ident!("u{}", accessor_type_size);

        types.push(quote!{
            pub enum #type_name {}

            impl Specifier for #type_name {
                const BITS: usize = #bit_width;
                type ACCESSOR = #accessor_type_name;
            }
        });
    }

    Ok(quote! {
        #(#types)*
    }.into())
}

struct GenBitWidthTypesInput {
    start: usize,
    end: usize,
}

impl Parse for GenBitWidthTypesInput {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let start = input.parse::<LitInt>()?.base10_parse::<usize>()?;

        let _: Token![..=] = input.parse()?;

        let end = input.parse::<LitInt>()?.base10_parse::<usize>()?;

        if end < start {
            return Err(input.error("'end' must be greater than or equal to 'start'"));
        }

        Ok(Self { start, end })
    }
}