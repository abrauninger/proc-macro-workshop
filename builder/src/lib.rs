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
    TypePath,
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
    each_name: LitStr,
}

impl Parse for VecBuilderInfo {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let each_ident: Ident = input.parse()?;
        if each_ident != "each" {
            return Err(input.error("expected 'each'"));
        }

        let _: Token![=] = input.parse()?;

        let each_name: LitStr = input.parse()?;

        Ok(VecBuilderInfo { each_name })
    }
}

fn builder_each_name(attrs: &Vec<Attribute>) -> syn::Result<Option<String>> {
    if let [attr] = attrs.as_slice() {
        match &attr.meta {
            syn::Meta::List(MetaList { path, delimiter, tokens, .. }) => {
                if path.is_ident("builder") {
                    if let MacroDelimiter::Paren(_) = delimiter {
                        let builder_info = syn::parse2::<VecBuilderInfo>(tokens.clone())?;
                        Ok(Some(builder_info.each_name.value()))
                    } else {
                        Ok(None)
                    }
                } else {
                    Ok(None)
                }
            },
            _ => Ok(None),
        }
    } else {
        Ok(None)
    }
}

struct EffectiveTypes {
    builder_member_type: Type,
    builder_function_arg_type: Type,
    vec_builder_function_arg_type: Option<Type>,
    is_optional: bool,
}

fn effective_types(field_type: &Type, is_built_vec: bool) -> EffectiveTypes {
    if is_built_vec {
        match inner_type(field_type, "Vec") {
            Some(inner_type) => EffectiveTypes {
                builder_member_type: parse_quote! { Option<#field_type> },
                //builder_member_type: field_type.clone(),
                builder_function_arg_type: field_type.clone(),
                vec_builder_function_arg_type: Some(inner_type.clone()),
                is_optional: true,
            },
            None => EffectiveTypes {
                builder_member_type: parse_quote! { Option<#field_type> },
                builder_function_arg_type: field_type.clone(),
                vec_builder_function_arg_type: None,
                is_optional: false,
            }
        }
    } else {
        match inner_type(field_type, "Option") {
            Some(inner_type) => EffectiveTypes {
                builder_member_type: field_type.clone(),
                builder_function_arg_type: inner_type.clone(),
                vec_builder_function_arg_type: None,
                is_optional: true,
            },
            None => EffectiveTypes {
                builder_member_type: parse_quote! { Option<#field_type> },
                builder_function_arg_type: field_type.clone(),
                vec_builder_function_arg_type: None,
                is_optional: false,
            }
        }
    }
}

#[proc_macro_derive(Builder, attributes(builder))]
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
            let mut build_member_variable_inits = Vec::with_capacity(fields.len());
            let mut build_struct_member_initializers = Vec::with_capacity(fields.len());

            for field in fields {
                let Field { ident: field_name, ty: field_type, attrs, .. } = &field;

                let builder_each_name_value = match builder_each_name(attrs) {
                    Ok(each_name) => each_name,
                    Err(error) => {
                        return error.to_compile_error().into();
                    },
                };

                let builder_each_name_ident = builder_each_name_value.map(|value| { format_ident!("{}", value) });

                let EffectiveTypes { builder_member_type, builder_function_arg_type, vec_builder_function_arg_type, is_optional } = effective_types(field_type, builder_each_name_ident.is_some());

                if let Some(field_name) = field_name {
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

                    let generate_all_at_once_member_builder = match &builder_each_name_ident {
                        Some(each_name) => each_name != field_name,
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

                    if let Some(each_name) = &builder_each_name_ident {
                        let vec_builder_function_arg_type = vec_builder_function_arg_type.unwrap();

                        builder_function_members.push(
                            quote! {
                                fn #each_name(&mut self, item: #vec_builder_function_arg_type) -> &mut Self {
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

                    pub fn build(&mut self) -> Result<#struct_name, Box<dyn std::error::Error>> {
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
