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
    Generics,
    GenericParam,
    Lit,
    Meta,
    MetaNameValue,
    parse_macro_input,
    parse_quote,
    punctuated::Punctuated,
    token::Comma,
    Type::Path, TypePath, PathSegment, PathArguments,
};

fn custom_format_from_debug_attribute(attrs: &Vec<Attribute>) -> syn::Result<Option<String>> {
    match attrs.as_slice() {
        [attr @ Attribute { meta: Meta::NameValue(MetaNameValue { path, value, .. }), .. }] if path.is_ident("debug") => {
            if let Expr::Lit(ExprLit { lit: Lit::Str(lit_str), .. }) = value {
                Ok(Some(lit_str.value()))
            } else {
                Err(syn::Error::new_spanned(&attr.meta, "expected `debug = \"...\"`"))
            }
        },
        _ => Ok(None),
    }
}

#[proc_macro_derive(CustomDebug, attributes(debug))]
pub fn derive(input: TokenStream) -> TokenStream {
    let derive_input = parse_macro_input!(input as DeriveInput);

    match derive_input {
        DeriveInput {
            ident: struct_name,
            generics,
            data: Data::Struct(
                DataStruct {
                    fields: Fields::Named(FieldsNamed { named: fields, .. }), ..
                }
            ), ..
        } => {
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

            let generics = add_trait_bounds(generics, &fields);
            let (impl_generics, struct_generics, _) = generics.split_for_impl();

            TokenStream::from(quote! {
                impl #impl_generics std::fmt::Debug for #struct_name #struct_generics {
                    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        fmt.debug_struct(#struct_name_string)
                            #debug_struct_fields
                            .finish()
                    }
                }
            })
        },
        _ => TokenStream::new(),
    }
}

fn add_trait_bounds(mut generics: Generics, fields: &Punctuated<Field, Comma>) -> Generics {
    for param in &mut generics.params {
        if let GenericParam::Type(type_param) = param {
            // Special case for PhantomData, which is very common and which implements Debug
            // regardless of its type parameters.
            //
            // Only add the trait bound if this type parameter is used outside a PhantomData field.
            let used_outside_phantom_data = fields.iter().find(|&f| {
                match &f.ty {
                    Path(TypePath { qself: None, path: syn::Path { segments, leading_colon: None } }) => {
                        if segments.len() == 1 {
                            match segments.first() {
                                Some(PathSegment { ident, arguments: PathArguments::None }) => {
                                    if *ident == type_param.ident {
                                        true
                                    } else {
                                        false
                                    }
                                },
                                _ => { false },
                            }
                        } else {
                            false
                        }
                    },
                    _ => false,
                }
            }).is_some();

            if used_outside_phantom_data {
                type_param.bounds.push(parse_quote!(std::fmt::Debug));
            }
        }
    }
    generics
}
