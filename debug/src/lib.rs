use proc_macro::TokenStream;
use quote::quote;
use syn::{
    Attribute,
    Data,
    DataStruct,
    DeriveInput,
    Expr,
    ExprLit,
    Field,
    Fields,
    FieldsNamed,
    Lit,
    Meta,
    MetaNameValue,
    parse_macro_input,
};

fn custom_format_from_debug_attribute(attrs: &Vec<Attribute>) -> syn::Result<Option<String>> {
    match attrs.as_slice() {
        [attr @ Attribute { meta: Meta::NameValue(MetaNameValue { path, value, .. }), .. }] if path.is_ident("debug") => {
            if let Expr::Lit(ExprLit { lit: Lit::Str(lit_str), .. }) = value {
                Ok(Some(lit_str.value()))
            } else {
                Err(syn::Error::new_spanned(attr.meta.clone(), "expected `debug = \"...\"`"))
            }
        },
        _ => Ok(None),
    }
}

#[proc_macro_derive(CustomDebug, attributes(debug))]
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
            if let Field { ident: Some(field_name), attrs, .. } = &field {
                let field_name_string = field_name.to_string();

                let custom_format = match custom_format_from_debug_attribute(attrs) {
                    Ok(custom_format) => custom_format,
                    Err(error) => {
                        return error.to_compile_error().into();
                    }
                };

                let format = match custom_format {
                    Some(custom_format) => quote! { &format_args!(#custom_format, &self.#field_name) },
                    None => quote! { &self.#field_name },
                };

                quote! {
                    .field(#field_name_string, #format)
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
