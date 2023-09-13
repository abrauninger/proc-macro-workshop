use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{DeriveInput, Data, Field, Fields, parse_macro_input};

#[proc_macro_derive(Builder)]
pub fn derive(input: TokenStream) -> TokenStream {
    let derive_input = parse_macro_input!(input as DeriveInput);

    let DeriveInput { ident: struct_name, data, .. } = derive_input;

    if let Data::Struct(data_struct) = data {
        let fields = &data_struct.fields;

        if let Fields::Named(fields) = fields {
            let builder_name = format_ident!("{}Builder", struct_name);

            let builder_struct_members: Vec<_> = fields.named.iter().map(|field| {
                let Field { ident: field_name, ty, .. } = &field;

                quote! {
                    #field_name: Option<#ty>,
                }
            }).collect();

            let builder_function_initializers: Vec<_> = fields.named.iter().map(|field| {
                let Field { ident: field_name, .. } = &field;

                quote! {
                    #field_name: None,
                }
            }).collect();

            let expanded = quote! {
                impl #struct_name {
                    pub fn builder() -> #builder_name {
                        #builder_name {
                            #(#builder_function_initializers)*
                        }
                    }
                }

                pub struct #builder_name {
                    #(#builder_struct_members)*
                }
            };

            return TokenStream::from(expanded)
        }
    }
    
    TokenStream::new()
}
