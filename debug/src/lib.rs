use proc_macro::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput, parse_macro_input, DataStruct, Fields, FieldsNamed, Field};

#[proc_macro_derive(CustomDebug)]
pub fn derive(input: TokenStream) -> TokenStream {
    let derive_input = parse_macro_input!(input as DeriveInput);

    if let DeriveInput {
        ident: struct_name,
        data: Data::Struct(
            DataStruct {
                fields: Fields::Named(FieldsNamed { named: fields, .. }), ..
            }
        ), ..
    } = derive_input {
        let debug_struct_fields: proc_macro2::TokenStream = fields.iter().map(|field| {
            if let Field { ident: Some(field_name), .. } = &field {
                let field_name_string = field_name.to_string();
                quote! {
                    .field(#field_name_string, &self.#field_name)
                }
            } else {
                quote! {}
            }
        }).collect();

        let struct_name_string = struct_name.to_string();

        TokenStream::from(quote! {
            impl std::fmt::Debug for #struct_name {
                fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                    fmt.debug_struct(#struct_name_string)
                        #debug_struct_fields
                        .finish()
                }
            }
        })
    } else {
        TokenStream::new()
    }
}
