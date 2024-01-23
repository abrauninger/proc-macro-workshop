use if_chain::if_chain;
use proc_macro::TokenStream;
use quote::{quote, format_ident};
use syn::{
    Item::{self, Struct},
    ItemStruct, Fields, FieldsNamed, Field,
};

pub fn bitfield_impl(input: TokenStream) -> syn::Result<TokenStream> {
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
                    let current_field_accessor_type_name = quote!(<#ty as ::bitfield::Specifier>::ACCESSOR);

                    let getter_name = format_ident!("get_{}", ident);
                    let setter_name = format_ident!("set_{}", ident);

                    quote! {
                        fn #getter_name(&self) -> #current_field_accessor_type_name {
                            let current_field_bit_start_index = 0 #previous_bit_widths;
                            let current_field_bit_count = #current_field_bit_count;

                            const accessor_size: usize = std::mem::size_of::<#current_field_accessor_type_name>();

                            let field_data = ::bitfield::field_data::get_field_data::<accessor_size>(&self.data, current_field_bit_start_index, current_field_bit_count);
                            #current_field_accessor_type_name::from_le_bytes(field_data)
                        }

                        fn #setter_name(&mut self, val: #current_field_accessor_type_name) {
                            let current_field_bit_start_index = 0 #previous_bit_widths;
                            let current_field_bit_count = #current_field_bit_count;

                            const accessor_size: usize = std::mem::size_of::<#current_field_accessor_type_name>();

                            let field_data = val.to_le_bytes();

                            ::bitfield::field_data::set_field_data::<accessor_size>(&mut self.data, field_data, current_field_bit_start_index, current_field_bit_count);
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

                    fn checks() -> impl ::bitfield::checks::TotalSizeIsMultipleOfEightBits {
                        const mod8: usize = (0 #bit_widths) % 8;
                        type ReturnType = <::bitfield::checks::Mod8::<mod8> as ::bitfield::checks::Mod8Check>::Type;
                        ReturnType {}
                    }

                    #accessors
                }
            }.into())
        } else {
            Ok(input)
        }
    }
}