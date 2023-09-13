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
            let mut builder_function_members = Vec::with_capacity(fields.len());
            let mut build_member_validation_checks = Vec::with_capacity(fields.len());
            let mut build_struct_member_initializers = Vec::with_capacity(fields.len());

            for field in fields {
                let Field { ident: field_name, ty, .. } = &field;

                if let Some(field_name) = field_name {
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

                    builder_function_members.push(
                        quote! {
                            fn #field_name(&mut self, #field_name: #ty) -> &mut Self {
                                self.#field_name = Some(#field_name);
                                self
                            }
                        }
                    );

                    let error_message = format!("{} has not been set", field_name);

                    build_member_validation_checks.push(
                        quote! {
                            let #field_name = match self.#field_name.take() {
                                Some(#field_name) => #field_name,
                                None => return Err(#error_message.to_string().into()),
                            };
                        }
                    );

                    build_struct_member_initializers.push(
                        quote! {
                            #field_name,
                        }
                    );
                }
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

                impl #builder_name {
                    #(#builder_function_members)*

                    pub fn build(&mut self) -> Result<#struct_name, Box<dyn std::error::Error>> {
                        #(#build_member_validation_checks)*

                        Ok(#struct_name {
                            #(#build_struct_member_initializers)*
                        })
                    }
                }
            };

            return TokenStream::from(expanded)
        }
    }
    
    TokenStream::new()
}
