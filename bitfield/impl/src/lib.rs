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

                            let current_field_bit_end_index_exclusive = current_field_bit_start_index + current_field_bit_count;

                            let current_field_byte_start_index = current_field_bit_start_index / 8;
                            let current_field_byte_end_index_exclusive = (current_field_bit_end_index_exclusive + 7) / 8;

                            if current_field_byte_end_index_exclusive <= current_field_byte_start_index {
                                panic!("Unexpected");
                            }

                            let source_data = &self.data[current_field_byte_start_index .. current_field_byte_end_index_exclusive];

                            // Currently all fields are u64
                            let mut field_data: [u8; 8] = [0; 8];

                            let bit_start_index_within_each_byte = current_field_bit_start_index % 8;

                            let second_part_shift_left_bit_count = bit_start_index_within_each_byte;
                            let first_part_shift_right_bit_count = 8 - bit_start_index_within_each_byte;

                            let first_part_mask = 2 ^ bit_start_index_within_each_byte - 1;
                            let second_part_mask = (2 ^ (8 - bit_start_index_within_each_byte) - 1) >> second_part_shift_left_bit_count;

                            for (byte_index, source_data_byte) in source_data.iter().enumerate() {
                                if bit_start_index_within_each_byte == 0 {
                                    let masked_byte: u8 = source_data_byte & second_part_mask;
                                    field_data[byte_index] = masked_byte;
                                } else {
                                    let mut field_data_byte: u8 = (source_data_byte & second_part_mask) << second_part_shift_left_bit_count;

                                    if byte_index + 1 < source_data.len() {
                                        // OR in the first part of the next byte
                                        let first_part_of_next_byte: u8 = source_data[byte_index + 1] & first_part_mask;
                                        field_data_byte = field_data_byte | (first_part_of_next_byte >> first_part_shift_right_bit_count);
                                    }

                                    field_data[byte_index] = field_data_byte;
                                }
                            }
                        }

                        fn #setter_name(&mut self, val: u64) {
                            // let field_data = val.to_le_bytes();

                            // let previous_fields_bits = 0 #previous_bit_widths;
                            // let current_field_bits = #current_field_bits;
                            // let source_data = &mut self.data[previous_fields_size .. previous_fields_size + current_field_size];
                            // source_data.copy_from_slice(&field_data[..current_field_size]);
                            todo!();
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