use if_chain::if_chain;
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
    PathArguments,
    PathSegment,
    punctuated::Punctuated,
    token::Comma,
    Type::{Path, self},
    TypePath,
    visit::{self, Visit}, TypeParam,
};

fn custom_format_from_debug_attribute(attr: &Attribute) -> syn::Result<Option<String>> {
    if_chain! {
        if let Attribute { meta, .. } = attr;
        if let Meta::NameValue(meta) = meta;
        let MetaNameValue { path, value, .. } = meta;
        if path.is_ident("debug");
        if let Expr::Lit(lit) = value;
        let ExprLit { lit, .. } = lit;
        if let Lit::Str(lit_str) = lit;
        then {
            Ok(Some(lit_str.value()))
        } else {
            Err(syn::Error::new_spanned(&attr.meta, "expected `debug = \"...\"`"))
        }
    }
}

fn custom_format_from_field_attributes(attrs: &Vec<Attribute>) -> syn::Result<Option<String>> {
    let mut custom_format: Option<_> = None;

    for attr in attrs {
        let this_custom_format = custom_format_from_debug_attribute(attr)?;
        if this_custom_format.is_some() {
            if custom_format.is_some() {
                return Err(syn::Error::new_spanned(&attr.meta, "only one 'debug' custom format attribute should be specified"));
            } else {
                custom_format = this_custom_format;
            }
        }
    }
    
    Ok(custom_format)
}

// A visitor that enumerates any types that use a certain set of generic type parameters
struct TypeParamVisitor<'ast> {
    type_params: Vec<&'ast TypeParam>,
    related_types: Vec<Type>,
}

impl<'ast> TypeParamVisitor<'ast> {
    fn new(type_params: Vec<&'ast TypeParam>) -> Self {
        Self {
            type_params,
            related_types: Vec::new(),
        }
    }
}

impl<'ast> Visit<'ast> for TypeParamVisitor<'ast> {
    fn visit_type(&mut self, ty: &'ast Type) {
        if_chain! {
            if let Path(TypePath { qself: None, path: syn::Path { segments, leading_colon: None } }) = ty;
            if segments.len() > 1;
            if let Some(PathSegment { ident, arguments: PathArguments::None }) = segments.first();
            if self.type_params.iter().find(|type_param| type_param.ident == *ident).is_some();
            then { self.related_types.push(ty.clone()) }
        }

        // Delegate to the default impl so that we get type parameters as well.
        visit::visit_type(self, ty);
    }
}

#[proc_macro_derive(CustomDebug, attributes(debug))]
pub fn derive(input: TokenStream) -> TokenStream {
    let derive_input = parse_macro_input!(input as DeriveInput);

    if_chain! {
        if let DeriveInput { ident: struct_name, generics, data, .. } = &derive_input;
        if let Data::Struct(data_struct) = data;
        if let DataStruct { fields, .. } = data_struct;
        if let Fields::Named(fields) = fields;
        if let FieldsNamed { named: fields, .. } = fields;
        then {
            let debug_struct_fields: proc_macro2::TokenStream = fields.iter().map(|field| {
                if let Field { ident: Some(field_name), attrs, .. } = &field {
                    let field_name_string = field_name.to_string();

                    let custom_format = match custom_format_from_field_attributes(attrs) {
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

            let struct_type_parameters: Vec<_> = generics.params
                .iter()
                .filter_map(|param| {
                    if let GenericParam::Type(type_param) = param {
                        Some(type_param)
                    } else {
                        None
                    }
                })
                .collect();

            let mut type_param_visitor = TypeParamVisitor::new(struct_type_parameters);
            type_param_visitor.visit_data_struct(&data_struct);

            let associated_type_bounds: Vec<_> = type_param_visitor.related_types
                .iter()
                .map(|ty| {
                    quote!(#ty : Debug)
                })
                .collect();

            let where_clauses =
                if associated_type_bounds.len() > 0 {
                    quote!(where #(#associated_type_bounds)*)
                } else {
                    quote!()
                };

            let struct_name_string = struct_name.to_string();

            let generics = add_trait_bounds(generics.clone(), &fields);
            let (impl_generics, struct_generics, _) = generics.split_for_impl();

            TokenStream::from(quote! {
                impl #impl_generics std::fmt::Debug for #struct_name #struct_generics
                    #where_clauses {
                    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        fmt.debug_struct(#struct_name_string)
                            #debug_struct_fields
                            .finish()
                    }
                }
            })
        }
        else { TokenStream::new() }
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
                if_chain! {
                    if let Path(path) = &f.ty;
                    if let TypePath { qself: None, path } = path;
                    if let syn::Path { segments, leading_colon: None } = path;
                    if segments.len() == 1;
                    if let Some(segment) = segments.first();
                    if let PathSegment { ident, arguments: PathArguments::None } = segment;
                    if *ident == type_param.ident;
                    then {
                        true
                    } else {
                        false
                    }
                }
            }).is_some();

            if used_outside_phantom_data {
                type_param.bounds.push(parse_quote!(std::fmt::Debug));
            }
        }
    }
    generics
}
