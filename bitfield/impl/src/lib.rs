use if_chain::if_chain;
use proc_macro::TokenStream;
use quote::{quote, format_ident};
use syn::{
    Item::{self, Struct},
    parse::{Parse, ParseStream}, LitInt, Token, ItemStruct, Fields, FieldsNamed, Field,
};

#[proc_macro_attribute]
pub fn bitfield(_args: TokenStream, input: TokenStream) -> TokenStream {
    match bitfield_impl(input) {
        Ok(output) => output,
        Err(err) => err.to_compile_error().into()
    }
}

fn bitfield_impl(input: TokenStream) -> syn::Result<TokenStream> {
    let item: Item = syn::parse(input.clone())?;

    if_chain! {
        if let Struct(item_struct) = item;
        let ItemStruct { attrs, vis, struct_token, ident, generics, fields, semi_token } = item_struct;
        if let Fields::Named(fields) = fields;
        if let FieldsNamed { named: fields, .. } = fields;
        then {
            let bit_widths: proc_macro2::TokenStream = fields.iter().map(|field| {
                let Field { ty, .. } = field;
                quote! { + <#ty as ::bitfield::Specifier>::BITS }
            }).collect();

            let accessors: proc_macro2::TokenStream = fields.iter().enumerate().map(|(field_index, field)| {
                let Field { ident, ty, .. } = field;
                if let Some(ident) = ident {
                    let previous_bit_widths: proc_macro2::TokenStream = fields
                        .iter()
                        .enumerate()
                        .filter(|(previous_field_index, _)| {
                            previous_field_index < &field_index
                        })
                        .map(|(_, previous_field)| {
                            let Field { ty, .. } = previous_field;
                            quote! { + <#ty as ::bitfield::Specifier>::BITS }
                        }).collect();

                    let current_field_bit_count = quote!(<#ty as ::bitfield::Specifier>::BITS);

                    let getter_name = format_ident!("get_{}", ident);
                    let setter_name = format_ident!("set_{}", ident);

                    quote! {
                        fn #getter_name(&self) -> u64 {
                            let current_field_bit_start_index = 0 #previous_bit_widths;
                            let current_field_bit_count = #current_field_bit_count;

                            if (current_field_bit_count > 64) {
                                panic!("Unable to get a field value that is wider than 64 bits.");
                            }

                            // Currently all fields are u64
                            let field_data = ::bitfield::get_field_data::<8>(&self.data, current_field_bit_start_index, current_field_bit_count);
                            u64::from_le_bytes(field_data)
                        }

                        fn #setter_name(&mut self, val: u64) {
                            let current_field_bit_start_index = 0 #previous_bit_widths;
                            let current_field_bit_count = #current_field_bit_count;

                            if (current_field_bit_count > 64) {
                                panic!("Unable to get a field value that is wider than 64 bits.");
                            }

                            let field_data = val.to_le_bytes();

                            // Currently all fields are u64
                            ::bitfield::set_field_data::<8>(&mut self.data, field_data, current_field_bit_start_index, current_field_bit_count);
                        }
                    }
                } else {
                    quote! {}
                }
            }).collect();

            Ok(quote! {
                #(#attrs)*
                #vis #struct_token #ident #generics {
                    data: [u8; (0 #bit_widths) / 8]
                }
                #semi_token

                impl #ident {
                    fn new() -> Self {
                        Self { data: [0; (0 #bit_widths) / 8] }
                    }

                    #accessors
                }
            }.into())
        } else {
            Ok(input)
        }
    }
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
                const BITS: usize = #bit_width;
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