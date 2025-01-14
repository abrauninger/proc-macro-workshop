use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{
    Attribute,
    DeriveInput,
    Data,
    Field,
    Fields,
    Ident,
    LitStr,
    MacroDelimiter,
    MetaList,
    parse::{
        Parse,
        ParseStream,
    },
    parse_macro_input,
    parse_quote,
    PathArguments,
    Token,
    Type,
    TypePath, spanned::Spanned,
};

fn inner_type<'a>(ty: &'a Type, outer_type_name: &'static str) -> Option<&'a Type> {
    match ty {
        Type::Path(TypePath { qself: None, path }) => {
            let segments = &path.segments;
            if segments.len() == 1 {
                let segment = &segments[0];
                if segment.ident == outer_type_name {
                    match &segment.arguments {
                        PathArguments::AngleBracketed(generic_args) => {
                            if generic_args.args.len() == 1 {
                                let arg = &generic_args.args[0];
                                match arg {
                                    syn::GenericArgument::Type(inner_type) => Some(inner_type),
                                    _ => None,
                                }
                            } else {
                                None
                            }
                        },
                        _ => None,
                    }
                } else {
                    None
                }
            } else {
                None
            }
        },
        _ => None,
    }
}

struct VecBuilderInfo {
    each_name: String,
}

impl Parse for VecBuilderInfo {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let each_ident: Ident = input.parse()?;
        if each_ident != "each" {
            return Err(input.error("expected 'each'"));
        }

        let _: Token![=] = input.parse()?;

        let each_name: LitStr = input.parse()?;

        Ok(VecBuilderInfo { each_name: each_name.value() })
    }
}

fn vec_builder_name_from_attr(attr: &Attribute) -> syn::Result<Option<String>> {
    match &attr.meta {
        syn::Meta::List(MetaList { path, delimiter: MacroDelimiter::Paren(_), tokens, .. }) if path.is_ident("builder") => {
            match syn::parse2::<VecBuilderInfo>(tokens.clone()) {
                Ok(builder_info) => Ok(Some(builder_info.each_name)),
                Err(_) => Err(syn::Error::new_spanned(&attr.meta, "expected `builder(each = \"...\")`")),
            }
        },
        _ => Ok(None),
    }
}

fn vec_builder_name(attrs: &Vec<Attribute>) -> syn::Result<Option<String>> {
    let mut unique_builder_name = None;

    for attr in attrs {
        let builder_name = vec_builder_name_from_attr(&attr)?;
        if builder_name.is_some() && unique_builder_name.is_some() {
            return Err(syn::Error::new_spanned(&attr.meta, "expected only one `builder` attribute"));
        }
        unique_builder_name = builder_name;
    }

    Ok(unique_builder_name)
}

#[proc_macro_derive(Builder, attributes(builder))]
pub fn derive(input: TokenStream) -> TokenStream {
    let derive_input = parse_macro_input!(input as DeriveInput);

    let DeriveInput { ident: struct_name, data, .. } = derive_input;

    if let Data::Struct(data) = data {
        if let Fields::Named(fields) = data.fields {
            let fields = fields.named;

            let mut builder_struct_members = Vec::with_capacity(fields.len());
            let mut builder_function_initializers = Vec::with_capacity(fields.len());
            let mut builder_function_members = Vec::with_capacity(fields.len());
            let mut build_member_variable_inits = Vec::with_capacity(fields.len());
            let mut build_struct_member_initializers = Vec::with_capacity(fields.len());

            for field in fields {
                let Field { ident: field_name, ty: field_type, attrs, .. } = field;

                if let Some(field_name) = field_name {
                    let vec_builder_name_value = match vec_builder_name(&attrs) {
                        Ok(builder_name) => builder_name,
                        Err(error) => {
                            return error
                                .to_compile_error()
                                .into();
                        },
                    };

                    let vec_builder_name_ident = vec_builder_name_value.map(|value| { format_ident!("{}", value) });

                    let is_built_vec = vec_builder_name_ident.is_some();

                    let option_inner_type = inner_type(&field_type, "Option");
                    let vec_inner_type = inner_type(&field_type, "Vec");

                    let builder_member_type =
                        if is_built_vec {
                            match vec_inner_type {
                                Some(_) => parse_quote! { std::option::Option<#field_type> },
                                None => {
                                    return syn::Error::new(field_type.span(), "the `builder` attribute should only be used on fields of type `Vec<_>`")
                                        .to_compile_error()
                                        .into();
                                }
                            }
                        } else {
                            match option_inner_type {
                                Some(_) => field_type.clone(),
                                None => parse_quote! { std::option::Option<#field_type> },
                            }
                        };

                    let builder_function_arg_type = match (is_built_vec, option_inner_type) {
                        (false, Some(inner_type)) => inner_type.clone(),
                        _ => field_type.clone(),
                    };

                    let vec_builder_function_arg_type = match (is_built_vec, vec_inner_type) {
                        (true, Some(inner_type)) => Some(inner_type.clone()),
                        _ => None,
                    };

                    let is_optional =
                        if is_built_vec {
                            match vec_inner_type {
                                Some(_) => true,
                                None => false,
                            }
                        } else {
                            match option_inner_type {
                                Some(_) => true,
                                None => false,
                            }
                        };

                    builder_struct_members.push(
                        quote! {
                            #field_name: #builder_member_type,
                        }
                    );

                    builder_function_initializers.push(
                        quote! {
                            #field_name: None,
                        }
                    );

                    let generate_all_at_once_member_builder = match vec_builder_name_ident {
                        Some(ref builder_name) => builder_name != &field_name,
                        None => true,
                    };

                    if generate_all_at_once_member_builder {
                        builder_function_members.push(
                            quote! {
                                fn #field_name(&mut self, #field_name: #builder_function_arg_type) -> &mut Self {
                                    self.#field_name = Some(#field_name);
                                    self
                                }
                            }
                        );
                    }

                    let none_arm = match vec_builder_function_arg_type {
                        Some(_) => quote! { vec![] },
                        None => {
                            let error_message = format!("{} has not been set", field_name);
                            quote! { return Err(#error_message.to_string().into()) }
                        }
                    };

                    build_member_variable_inits.push(
                        if is_optional && vec_builder_function_arg_type.is_none() {
                            quote! {
                                let #field_name = self.#field_name.take();
                            }
                        } else {
                            quote! {
                                let #field_name = match self.#field_name.take() {
                                    Some(#field_name) => #field_name,
                                    None => #none_arm,
                                };
                            }
                        }
                    );

                    build_struct_member_initializers.push(
                        quote! {
                            #field_name,
                        }
                    );

                    if let Some(vec_builder_name) = &vec_builder_name_ident {
                        let vec_builder_function_arg_type = vec_builder_function_arg_type.unwrap();

                        builder_function_members.push(
                            quote! {
                                fn #vec_builder_name(&mut self, item: #vec_builder_function_arg_type) -> &mut Self {
                                    match &mut self.#field_name {
                                        Some(#field_name) => {
                                            #field_name.push(item)
                                        },
                                        None => {
                                            self.#field_name = Some(vec![item])
                                        }
                                    }
                                    self
                                }
                            }
                        );
                    }
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

                    pub fn build(&mut self) -> std::result::Result<#struct_name, std::boxed::Box<dyn std::error::Error>> {
                        #(#build_member_variable_inits)*

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
