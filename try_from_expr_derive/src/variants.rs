use crate::generators::{
    ArgSource, FieldSource, generate_arg_processing, generate_struct_field_parsing,
};
use heck::ToSnakeCase;
use proc_macro2::TokenStream;
use quote::quote;
use std::collections::HashMap;
use syn::{DataEnum, Error, Fields, Ident, ext::IdentExt};

/// Parse arms for every variant of an enum, shared by the leaf and wrapper impls.
pub struct VariantArms<'a> {
    pub unit_names: Vec<String>,
    pub unit_idents: Vec<&'a Ident>,
    /// `"Variant" => ..` arms for `Enum::Variant(..)`, with the call bound as `call_expr`.
    pub call_arms: Vec<TokenStream>,
    /// `"Variant" => ..` arms for `Enum::Variant { .. }`, with the literal bound as `struct_expr`.
    pub struct_arms: Vec<TokenStream>,
    /// `parse_meta_variant`, `meta_type_wrapper_arg` and `names_this_enum`.
    pub shared_fns: TokenStream,
    /// `unknown_meta_variant`, which only the leaf impl reports through.
    pub unknown_meta_fn: TokenStream,
    pub param_list: String,
}

impl<'a> VariantArms<'a> {
    pub fn new(enum_name: &Ident, data: &'a DataEnum) -> syn::Result<Self> {
        let mut snake_owners: HashMap<String, &Ident> = HashMap::new();
        let mut unit_names = Vec::new();
        let mut unit_idents = Vec::new();
        let mut unit_snakes = Vec::new();
        let mut param_names = Vec::new();
        let mut tuple_snakes = Vec::new();
        let mut struct_snakes = Vec::new();
        let mut call_arms = Vec::new();
        let mut struct_arms = Vec::new();
        let mut meta_tuple_arms = Vec::new();
        let mut meta_struct_arms = Vec::new();
        let type_snake = enum_name.unraw().to_string().to_snake_case();
        // Fields of a struct variant that shares the enum's meta name
        let mut self_named_fields: Option<Vec<&Ident>> = None;

        for variant in &data.variants {
            let ident = &variant.ident;
            let name = ident.to_string();
            let snake = ident.unraw().to_string().to_snake_case();
            if let Some(owner) = snake_owners.insert(snake.clone(), ident) {
                return Err(Error::new(
                    ident.span(),
                    format!("Variants `{owner}` and `{ident}` both use the meta name `{snake}`"),
                ));
            }

            match &variant.fields {
                Fields::Unit => {
                    unit_names.push(name);
                    unit_idents.push(ident);
                    unit_snakes.push(snake);
                }
                Fields::Unnamed(fields) => {
                    let call_parser =
                        generate_arg_processing(ident, &fields.unnamed, ArgSource::Call);
                    let assign_parser =
                        generate_arg_processing(ident, &fields.unnamed, ArgSource::Assign);
                    call_arms.push(quote! { #name => #call_parser, });
                    meta_tuple_arms
                        .push(quote! { Some(#snake) => Result::map(#assign_parser, Some), });
                    param_names.push(name);
                    tuple_snakes.push(snake);
                }
                Fields::Named(fields) => {
                    let literal_parser =
                        generate_struct_field_parsing(ident, fields, FieldSource::StructLiteral)?;
                    let meta_parser =
                        generate_struct_field_parsing(ident, fields, FieldSource::MetaCall)?;
                    if snake == type_snake {
                        self_named_fields = Some(
                            fields
                                .named
                                .iter()
                                .filter_map(|field| field.ident.as_ref())
                                .collect(),
                        );
                    }
                    struct_arms.push(quote! { #name => #literal_parser, });
                    meta_struct_arms
                        .push(quote! { Some(#snake) => Result::map(#meta_parser, Some), });
                    struct_snakes.push(snake);
                }
            }
        }

        let enum_name_str = enum_name.to_string();

        // `name(field = ..)` reads as the same-named struct variant when every
        // argument is one of its fields, so a field sharing a tuple variant's
        // meta name would make `name(field = ..)` ambiguous.
        let struct_call_check = match &self_named_fields {
            Some(fields) => {
                let mut field_names = Vec::new();
                for field in fields {
                    let field_name = field.unraw().to_string();
                    if tuple_snakes.contains(&field_name) {
                        return Err(Error::new(
                            field.span(),
                            format!(
                                "Field `{field_name}` shares its meta name with a tuple variant, \
                                 so `{type_snake}({field_name} = ..)` is ambiguous"
                            ),
                        ));
                    }
                    field_names.push(field_name);
                }
                quote! {
                    let reads_as_struct = call_expr.args.iter().all(|arg| {
                        let ::syn::Expr::Assign(assign) = arg else { return false };
                        let ::syn::Expr::Path(path_expr) = &*assign.left else { return false };
                        Self::meta_ident_name(&path_expr.path)
                            .is_some_and(|name| [#(#field_names),*].contains(&name.as_str()))
                    });
                    if reads_as_struct {
                        return Ok(None);
                    }
                }
            }
            None => TokenStream::new(),
        };

        let shared_fns = quote! {
            fn parse_meta_variant(expr: &::syn::Expr) -> Result<Option<Self>, ::syn::Error> {
                match Self::unwrap_expr(expr) {
                    ::syn::Expr::Path(path_expr) => match Self::meta_ident_name(&path_expr.path).as_deref() {
                        #(Some(#unit_snakes) => Ok(Some(Self::#unit_idents)),)*
                        _ => Ok(None),
                    },
                    ::syn::Expr::Assign(assign) => {
                        let ::syn::Expr::Path(path_expr) = &*assign.left else { return Ok(None) };
                        match Self::meta_ident_name(&path_expr.path).as_deref() {
                            #(#meta_tuple_arms)*
                            _ => Ok(None),
                        }
                    }
                    ::syn::Expr::Call(call_expr) => {
                        let ::syn::Expr::Path(path_expr) = &*call_expr.func else { return Ok(None) };
                        match Self::meta_ident_name(&path_expr.path).as_deref() {
                            #(#meta_struct_arms)*
                            _ => Ok(None),
                        }
                    }
                    _ => Ok(None),
                }
            }

            fn meta_type_wrapper_arg(expr: &::syn::Expr) -> Result<Option<&::syn::Expr>, ::syn::Error> {
                let ::syn::Expr::Call(call_expr) = expr else { return Ok(None) };
                let ::syn::Expr::Path(path_expr) = &*call_expr.func else { return Ok(None) };
                if Self::meta_ident_name(&path_expr.path).as_deref() != Some(#type_snake) {
                    return Ok(None);
                }
                #struct_call_check
                let mut args = call_expr.args.iter();
                match (args.next(), args.next()) {
                    (Some(arg), None) => Ok(Some(arg)),
                    _ => Err(::syn::Error::new_spanned(
                        call_expr,
                        format!("`{}(..)` takes exactly one variant", #type_snake)
                    )),
                }
            }

            // Keyword names such as `type` are written `r#type`
            fn meta_ident_name(path: &::syn::Path) -> Option<String> {
                path.get_ident().map(|ident| ::syn::ext::IdentExt::unraw(ident).to_string())
            }

            fn names_this_enum(path: &::syn::Path) -> bool {
                path.segments
                    .iter()
                    .rev()
                    .nth(1)
                    .is_some_and(|segment| segment.ident == #enum_name_str || segment.ident == "Self")
            }
        };

        let unit_valid = format!(
            "{}; in meta form: {}",
            list_or_none(&unit_names),
            list_or_none(&unit_snakes)
        );
        let tuple_valid = list_or_none(&tuple_snakes);
        let struct_valid = list_or_none(&struct_snakes);
        let unknown_meta_fn = quote! {
            fn unknown_meta_variant(expr: &::syn::Expr) -> ::syn::Error {
                let expected = || ::syn::Error::new_spanned(
                    expr,
                    format!("Expected `variant`, `variant = value` or `variant(field = value, ..)` for enum '{}'", #enum_name_str)
                );
                let (kind, path, valid) = match expr {
                    ::syn::Expr::Path(path_expr) => ("unit variant", &path_expr.path, #unit_valid),
                    ::syn::Expr::Assign(assign) => match &*assign.left {
                        ::syn::Expr::Path(path_expr) => ("tuple variant", &path_expr.path, #tuple_valid),
                        _ => return expected(),
                    },
                    ::syn::Expr::Call(call_expr) => match &*call_expr.func {
                        ::syn::Expr::Path(path_expr) => ("struct variant", &path_expr.path, #struct_valid),
                        _ => return expected(),
                    },
                    _ => return expected(),
                };
                match path.get_ident() {
                    Some(ident) => ::syn::Error::new(
                        ident.span(),
                        format!("Unknown {} '{}' for enum '{}'. Valid options: {}", kind, ident, #enum_name_str, valid)
                    ),
                    None => expected(),
                }
            }
        };

        Ok(Self {
            unit_names,
            unit_idents,
            call_arms,
            struct_arms,
            shared_fns,
            unknown_meta_fn,
            param_list: if param_names.is_empty() {
                "(no parametrized variants)".to_string()
            } else {
                param_names.join(", ")
            },
        })
    }
}

fn list_or_none(names: &[String]) -> String {
    if names.is_empty() {
        "(none)".to_string()
    } else {
        names.join(", ")
    }
}
