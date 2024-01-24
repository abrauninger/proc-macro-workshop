use proc_macro::TokenStream;
use syn::DeriveInput;

pub fn bitfield_specifier_derive_impl(input: TokenStream) -> syn::Result<TokenStream> {
    let derive_input: DeriveInput = syn::parse(input)?;

    let DeriveInput { ident: _enum_name, data: _a, .. } = derive_input;

    Ok(TokenStream::new())
}