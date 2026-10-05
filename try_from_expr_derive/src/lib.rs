//! Derive macro implementation for `try_from_expr`.
//!
//! This crate provides the procedural macro implementation for generating
//! `TryFrom<&syn::Expr>` implementations. It should be used through the
//! main `try_from_expr` crate.

mod generators;
mod helpers;
mod variants;

use crate::{
    generators::{generate_helper_functions, generate_type_parser},
    helpers::{detect_wrapper_enum, extract_type_name},
    variants::VariantArms,
};
use heck::ToSnakeCase;
use proc_macro::TokenStream;
use quote::quote;
use std::collections::HashSet;
use syn::{Attribute, Data, DataEnum, DeriveInput, Error, Fields, Ident, parse_macro_input};

/// Derives `TryFrom<&syn::Expr>` for enums.
///
/// This macro generates an implementation that can parse `syn::Expr` values
/// into your enum type. It automatically detects whether the enum is a
/// "wrapper" enum or a "leaf" enum and generates appropriate parsing logic.
///
/// Every variant parses from its path form (`MyEnum::Tuple("x")`) and from
/// its snake_case meta form (`my_enum(tuple = "x")`, or just `tuple = "x"`).
///
/// # Attributes
///
/// - `#[try_from_expr(wrapper)]` - Force wrapper enum mode
/// - `#[try_from_expr(leaf)]` - Force leaf enum mode
///
/// # Example
///
/// ```rust,ignore
/// #[derive(TryFromExpr)]
/// enum MyEnum {
///     Unit,
///     Tuple(String),
///     Struct { field: i32 },
/// }
/// ```
#[proc_macro_derive(TryFromExpr, attributes(try_from_expr))]
pub fn derive_try_from_expr(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    expand(&input)
        .unwrap_or_else(Error::into_compile_error)
        .into()
}

enum Mode {
    Wrapper,
    Leaf,
}

fn expand(input: &DeriveInput) -> syn::Result<proc_macro2::TokenStream> {
    let Data::Enum(data) = &input.data else {
        return Err(Error::new_spanned(
            input,
            "TryFromExpr can only be applied to enums",
        ));
    };

    let mode = match parse_mode(&input.attrs)? {
        Some(mode) => mode,
        None if detect_wrapper_enum(data) => Mode::Wrapper,
        None => Mode::Leaf,
    };
    let arms = VariantArms::new(&input.ident, data)?;

    match mode {
        Mode::Wrapper => generate_wrapper_enum_impl(&input.ident, data, &arms),
        Mode::Leaf => Ok(generate_leaf_enum_impl(&input.ident, &arms)),
    }
}

fn parse_mode(attrs: &[Attribute]) -> syn::Result<Option<Mode>> {
    let mut mode = None;
    for attr in attrs
        .iter()
        .filter(|attr| attr.path().is_ident("try_from_expr"))
    {
        attr.parse_nested_meta(|meta| {
            let parsed = if meta.path.is_ident("wrapper") {
                Mode::Wrapper
            } else if meta.path.is_ident("leaf") {
                Mode::Leaf
            } else {
                return Err(meta.error("expected `wrapper` or `leaf`"));
            };
            if mode.replace(parsed).is_some() {
                return Err(meta.error("the enum mode is already set"));
            }
            Ok(())
        })?;
    }
    Ok(mode)
}

fn generate_wrapper_enum_impl(
    enum_name: &Ident,
    data: &DataEnum,
    arms: &VariantArms,
) -> syn::Result<proc_macro2::TokenStream> {
    let helpers = generate_helper_functions();
    let VariantArms {
        unit_names,
        unit_idents,
        call_arms,
        struct_arms,
        shared_fns,
        param_list,
        ..
    } = arms;
    let enum_name_str = enum_name.to_string();

    // Child types are the single-field tuple variants. When two variants hold
    // the same type, dispatch by type name goes to the first.
    let mut seen_types = HashSet::new();
    let mut type_names = Vec::new();
    let mut type_snakes = Vec::new();
    let mut type_parsers = Vec::new();
    let mut fallback_attempts = Vec::new();
    for variant in &data.variants {
        let Fields::Unnamed(fields) = &variant.fields else {
            continue;
        };
        let mut field_iter = fields.unnamed.iter();
        let (Some(field), None) = (field_iter.next(), field_iter.next()) else {
            continue;
        };
        let variant_name = &variant.ident;
        let field_type = &field.ty;
        let parser = generate_type_parser(field_type, quote! { expr });

        fallback_attempts.push(quote! {
            match #parser {
                Ok(value) => return Ok(Self::#variant_name(value)),
                Err(error) => errors.push(format!(
                    "- Variant '{}': Could not parse as type '{}': {}",
                    stringify!(#variant_name),
                    stringify!(#field_type),
                    error
                )),
            }
        });

        if let Some(type_name) = extract_type_name(field_type)
            && seen_types.insert(type_name.clone())
        {
            type_snakes.push(type_name.to_snake_case());
            type_parsers.push(quote! {
                #parser
                    .map(Self::#variant_name)
                    .map_err(|error| ::syn::Error::new(expr.span(), format!("Failed to parse {}: {}", #type_name, error)))
            });
            type_names.push(type_name);
        }
    }

    if fallback_attempts.is_empty() {
        return Err(Error::new(
            enum_name.span(),
            "A wrapper enum needs at least one single-field tuple variant to hold a child type",
        ));
    }

    Ok(quote! {
        impl TryFrom<&::syn::Expr> for #enum_name {
            type Error = ::syn::Error;

            fn try_from(expr: &::syn::Expr) -> Result<Self, Self::Error> {
                use ::syn::spanned::Spanned;

                let expr = Self::unwrap_expr(expr);

                if let Some(inner) = Self::meta_type_wrapper_arg(expr)? {
                    if let Some(value) = Self::parse_meta_variant(inner)? {
                        return Ok(value);
                    }
                    return Self::parse_child(inner);
                }

                match expr {
                    ::syn::Expr::Call(call_expr) => {
                        if let ::syn::Expr::Path(path_expr) = &*call_expr.func
                            && Self::names_this_enum(&path_expr.path)
                            && let Some(variant_seg) = path_expr.path.segments.last()
                        {
                            let variant_str = variant_seg.ident.to_string();
                            return match variant_str.as_str() {
                                #(#call_arms)*
                                _ => Err(::syn::Error::new(
                                    variant_seg.span(),
                                    format!("Unknown parametrized variant '{}'. Valid options: {}", variant_str, #param_list)
                                )),
                            };
                        }
                    }
                    ::syn::Expr::Struct(struct_expr) => {
                        if Self::names_this_enum(&struct_expr.path)
                            && let Some(variant_seg) = struct_expr.path.segments.last()
                        {
                            let variant_str = variant_seg.ident.to_string();
                            return match variant_str.as_str() {
                                #(#struct_arms)*
                                _ => Err(::syn::Error::new(
                                    variant_seg.span(),
                                    format!("Unknown struct variant '{}' for enum '{}'", variant_str, #enum_name_str)
                                )),
                            };
                        }
                    }
                    ::syn::Expr::Path(path_expr) => {
                        if let Some(variant_seg) = path_expr.path.segments.last() {
                            match variant_seg.ident.to_string().as_str() {
                                #(#unit_names => return Ok(Self::#unit_idents),)*
                                _ => {}
                            }
                        }
                    }
                    _ => {}
                }

                if let Some(value) = Self::parse_meta_variant(expr)? {
                    return Ok(value);
                }
                Self::parse_child(expr)
            }
        }

        impl #enum_name {
            #helpers
            #shared_fns

            fn parse_child(expr: &::syn::Expr) -> Result<Self, ::syn::Error> {
                use ::syn::spanned::Spanned;

                // A child type named in meta form, like `child_type(..)`
                if let ::syn::Expr::Call(call_expr) = expr
                    && let ::syn::Expr::Path(path_expr) = &*call_expr.func
                    && let Some(name) = Self::meta_ident_name(&path_expr.path)
                {
                    match name.as_str() {
                        #(#type_snakes => return #type_parsers,)*
                        _ => {}
                    }
                }

                // An expression whose type cannot be determined falls back to trying every child
                if let Ok(enum_type) = Self::determine_enum_type(expr) {
                    match enum_type.as_str() {
                        #(#type_names => return #type_parsers,)*
                        _ => {}
                    }
                }

                let mut errors = Vec::new();
                #(#fallback_attempts)*

                Err(::syn::Error::new(
                    expr.span(),
                    format!("No variant of {} could parse the expression. Failures:\n{}", stringify!(#enum_name), errors.join("\n"))
                ))
            }

            /// Try to parse from any child enum without type hints
            pub fn from_any_expr(expr: &::syn::Expr) -> Result<Self, ::syn::Error> {
                <Self as TryFrom<&::syn::Expr>>::try_from(expr)
            }
        }
    })
}

fn generate_leaf_enum_impl(enum_name: &Ident, arms: &VariantArms) -> proc_macro2::TokenStream {
    let helpers = generate_helper_functions();
    let VariantArms {
        unit_names,
        unit_idents,
        call_arms,
        struct_arms,
        shared_fns,
        unknown_meta_fn,
        param_list,
    } = arms;
    let enum_name_str = enum_name.to_string();

    quote! {
        impl TryFrom<&::syn::Expr> for #enum_name {
            type Error = ::syn::Error;

            fn try_from(expr: &::syn::Expr) -> Result<Self, Self::Error> {
                use ::syn::spanned::Spanned;

                let expr = Self::unwrap_expr(expr);

                if let Some(inner) = Self::meta_type_wrapper_arg(expr)? {
                    return Self::parse_meta_variant(inner)?
                        .ok_or_else(|| Self::unknown_meta_variant(inner));
                }
                if let Some(value) = Self::parse_meta_variant(expr)? {
                    return Ok(value);
                }

                match expr {
                    // Unit variants like `StringEnum::Email`, or just `Email`
                    ::syn::Expr::Path(path_expr) => {
                        let path = &path_expr.path;
                        let Some(variant_seg) = path.segments.last() else {
                            return Err(::syn::Error::new(path.span(), "Path has no segments"));
                        };
                        let variant_str = variant_seg.ident.to_string();
                        if path.segments.len() == 1 {
                            return match variant_str.as_str() {
                                #(#unit_names => Ok(Self::#unit_idents),)*
                                _ => Err(Self::unknown_meta_variant(expr)),
                            };
                        }
                        if !Self::names_this_enum(path) {
                            return Err(::syn::Error::new(
                                path.span(),
                                format!("Expected a path to enum '{}'", #enum_name_str)
                            ));
                        }
                        match variant_str.as_str() {
                            #(#unit_names => Ok(Self::#unit_idents),)*
                            _ => Err(::syn::Error::new(
                                variant_seg.span(),
                                format!("Unknown unit variant '{}' for enum '{}'", variant_str, #enum_name_str)
                            )),
                        }
                    }

                    // Variants with parameters like `StringEnum::MinLength(5)`
                    ::syn::Expr::Call(call_expr) => {
                        let ::syn::Expr::Path(path_expr) = &*call_expr.func else {
                            return Err(::syn::Error::new(
                                call_expr.func.span(),
                                "Call expression must be a path to an enum variant"
                            ));
                        };
                        let path = &path_expr.path;
                        if path.get_ident().is_some() {
                            return Err(Self::unknown_meta_variant(expr));
                        }
                        let Some(variant_seg) = path.segments.last() else {
                            return Err(::syn::Error::new(path.span(), "Call path has no segments"));
                        };
                        if !Self::names_this_enum(path) {
                            return Err(::syn::Error::new(
                                path.span(),
                                format!("Expected a call to a variant of enum '{}'", #enum_name_str)
                            ));
                        }
                        let variant_str = variant_seg.ident.to_string();
                        match variant_str.as_str() {
                            #(#call_arms)*
                            _ => Err(::syn::Error::new(
                                variant_seg.span(),
                                format!("Unknown parametrized variant '{}'. Valid options: {}", variant_str, #param_list)
                            )),
                        }
                    }

                    // Struct variants like `StringEnum::Config { min: 5, max: 10 }`
                    ::syn::Expr::Struct(struct_expr) => {
                        let path = &struct_expr.path;
                        let Some(variant_seg) = path.segments.last() else {
                            return Err(::syn::Error::new(path.span(), "Struct path has no segments"));
                        };
                        if !Self::names_this_enum(path) {
                            return Err(::syn::Error::new(
                                path.span(),
                                format!("Expected a struct literal for a variant of enum '{}'", #enum_name_str)
                            ));
                        }
                        let variant_str = variant_seg.ident.to_string();
                        match variant_str.as_str() {
                            #(#struct_arms)*
                            _ => Err(::syn::Error::new(
                                variant_seg.span(),
                                format!("Unknown struct variant '{}' for enum '{}'", variant_str, #enum_name_str)
                            )),
                        }
                    }

                    ::syn::Expr::Assign(_) => Err(Self::unknown_meta_variant(expr)),

                    _ => Err(::syn::Error::new(
                        expr.span(),
                        format!(
                            "Unsupported expression type for enum '{}'. Expected a path, call, struct or meta expression.",
                            #enum_name_str
                        )
                    )),
                }
            }
        }

        impl #enum_name {
            #helpers
            #shared_fns
            #unknown_meta_fn

            pub fn parse_from_expr(expr: &::syn::Expr) -> Result<Self, ::syn::Error> {
                <Self as TryFrom<&::syn::Expr>>::try_from(expr)
            }
        }
    }
}
