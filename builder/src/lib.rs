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
            let fields = &fields.named;

            let mut builder_struct_members = Vec::with_capacity(fields.len());
            let mut builder_function_initializers = Vec::with_capacity(fields.len());

            for field in fields {
                let Field { ident: field_name, ty, .. } = &field;

                builder_struct_members.push(
                    quote! {
                        #field_name: Option<#ty>,
                    }
                );

                builder_function_initializers.push(
                    quote! {
                        #field_name: None,
                    }
                );
            }

            let builder_name = format_ident!("{}Builder", struct_name);

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
