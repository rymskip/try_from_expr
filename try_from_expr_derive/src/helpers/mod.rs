pub mod types;

use syn::{Fields, GenericArgument, PathArguments, Type};

// Extract the inner type from Option<T>
pub fn extract_option_inner_type(type_path: &syn::TypePath) -> Option<&Type> {
    let seg = type_path.path.segments.last()?;

    if seg.ident != "Option" {
        return None;
    }

    let PathArguments::AngleBracketed(args) = &seg.arguments else {
        return None;
    };
    let GenericArgument::Type(inner_ty) = args.args.first()? else {
        return None;
    };

    Some(inner_ty)
}

pub fn is_option_type(type_path: &syn::TypePath) -> bool {
    type_path
        .path
        .segments
        .last()
        .map(|seg| seg.ident == "Option")
        .unwrap_or(false)
}

// Extract the type name from a Type for matching
pub fn extract_type_name(ty: &Type) -> Option<String> {
    let Type::Path(type_path) = ty else {
        return None;
    };
    type_path
        .path
        .segments
        .last()
        .map(|segment| segment.ident.to_string())
}

// Detect if this is a wrapper enum (all variants are single-field tuples)
pub fn detect_wrapper_enum(data: &syn::DataEnum) -> bool {
    if data.variants.is_empty() {
        return false;
    }

    // A wrapper enum should only have single-field tuple variants
    // If there are any unit variants or struct variants, it's not a wrapper enum
    data.variants.iter().all(
        |variant| matches!(&variant.fields, Fields::Unnamed(fields) if fields.unnamed.len() == 1),
    )
}
