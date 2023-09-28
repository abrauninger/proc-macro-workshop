use std::cmp::Ordering;

use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::quote;
use syn::{
    ExprMatch,
    Ident,
    Item::{self, Enum},
    ItemFn,
    Meta,
    Path,
    visit_mut::{self, VisitMut}, Arm, parse_macro_input, parse_quote, Pat,
};

#[proc_macro_attribute]
pub fn sorted(_args: TokenStream, input: TokenStream) -> TokenStream {
    match sorted_impl(input) {
        Ok(output) => output,
        Err(err) => err.to_compile_error().into()
    }
}

fn sorted_impl(input: TokenStream) -> syn::Result<TokenStream> {
    let item: Item = syn::parse(input.clone())?;

    if let Enum(item_enum) = item {
        let mut previous_variant_ident: Option<&Ident> = None;

        for variant in &item_enum.variants {
            if let Some(previous_variant_ident) = previous_variant_ident {
                if variant.ident < *previous_variant_ident {
                    let sort_before_variant = item_enum.variants.iter().find(|v| v.ident > variant.ident).unwrap();
                    return Err(syn::Error::new_spanned(&variant.ident, format!("{} should sort before {}", variant.ident, sort_before_variant.ident)));
                }
            }

            previous_variant_ident = Some(&variant.ident);
        }

        Ok(input)
    } else {
        Err(syn::Error::new(Span::call_site(), "expected enum or match expression"))
    }
}

#[proc_macro_attribute]
pub fn check(_args: TokenStream, input: TokenStream) -> TokenStream {
    let mut item = parse_macro_input!(input as ItemFn);

    let mut check_visitor = CheckVisitor::new();
    check_visitor.visit_item_fn_mut(&mut item);

    let error_tokens = match check_visitor.error {
        Some(error) => error.to_compile_error(),
        None => proc_macro2::TokenStream::new(),
    };

    quote! {
        #item
        #error_tokens
    }.into()
}

struct CheckVisitor {
    error: Option<syn::Error>,
}

impl CheckVisitor {
    fn new() -> Self {
        Self { error: None }
    }

    fn add_error(&mut self, error: syn::Error) {
        match &mut self.error {
            Some(existing_error) => existing_error.combine(error),
            None => self.error = Some(error),
        }
    }
}

impl VisitMut for CheckVisitor {
    fn visit_expr_match_mut(self: &mut Self, expr_match: &mut ExprMatch) {
        let mut previous_arm_path: Option<Path> = None;
        let mut wildcard_pat: Option<&Pat> = None;

        for arm in &expr_match.arms {
            if let Some(wildcard_pat) = &wildcard_pat {
                self.add_error(syn::Error::new_spanned(wildcard_pat, "wildcard pattern should be last"));
            }

            if let Some(path) = path_from_match_arm(arm) {
                if let Some(previous_arm_path) = previous_arm_path {
                    if compare_paths(&path, &previous_arm_path) == Ordering::Less {
                        let sort_before_arm_path: Path = expr_match.arms
                            .iter()
                            .map(path_from_match_arm)
                            .find(|possible_sort_before_path| {
                                if let Some(possible_sort_before_path) = possible_sort_before_path {
                                    if compare_paths(possible_sort_before_path, &path) == Ordering::Greater {
                                        true
                                    } else {
                                        false
                                    }
                                } else {
                                    false
                                }
                            })
                            .unwrap()
                            .unwrap();

                        self.add_error(syn::Error::new_spanned(&path, format!("{} should sort before {}", path_to_string(&path), path_to_string(&sort_before_arm_path))));
                    }
                }

                previous_arm_path = Some(path);
            } else {
                if let Pat::Wild(_) = &arm.pat {
                    wildcard_pat = Some(&arm.pat);
                } else {
                    self.add_error(syn::Error::new_spanned(&arm.pat, "unsupported by #[sorted]"));
                }
            }
        }

        // Remove the #[sorted] attribute (which would otherwise cause a compile error)
        expr_match.attrs.retain(|attr| {
            if let Meta::Path(path) = &attr.meta {
                if path.is_ident("sorted") {
                    false
                } else {
                    true
                }
            } else {
                true
            }
        });

        visit_mut::visit_expr_match_mut(self, expr_match)
    }
}

fn path_from_match_arm(arm: &Arm) -> Option<Path> {
    match &arm.pat {
        Pat::Ident(ident) => {
            let path: Path = parse_quote!(#ident);
            Some(path)
        },
        Pat::TupleStruct(tuple_struct) => Some(tuple_struct.path.clone()),
        Pat::Path(expr_path) => Some(expr_path.path.clone()),
        Pat::Struct(pat_struct) => Some(pat_struct.path.clone()),
        _ => None,
    }
}

fn compare_paths(a: &Path, b: &Path) -> Ordering {
    let mut a_iter = a.segments.iter();
    let mut b_iter = b.segments.iter();

    loop {
        let a_segment = a_iter.next();
        let b_segment = b_iter.next();

        match (a_segment, b_segment) {
            (Some(a), Some(b)) => {
                match a.ident.cmp(&b.ident) {
                    Ordering::Greater => return Ordering::Greater,
                    Ordering::Less => return Ordering::Less,
                    _ => ()
                }
            },
            (Some(_), None) => return Ordering::Greater,
            (None, Some(_)) => return Ordering::Less,
            (None, None) => return Ordering::Equal,
        }
    }
}

fn path_to_string(path: &Path) -> String {
    let mut output = String::new();

    if let Some(_) = path.leading_colon {
        output.push_str("::");
    }

    let mut add_colon = false;

    for segment in &path.segments {
        if add_colon {
            output.push_str("::");
        }
        add_colon = true;

        output.push_str(&segment.ident.to_string());
    }

    output
}