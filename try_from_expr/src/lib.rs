//! # try_from_expr
//!
//! A procedural macro for generating `TryFrom<&syn::Expr>` implementations for enums.
//! This allows you to parse Rust syntax expressions into strongly-typed enum values.
//!
//! ## Features
//!
//! - Automatically generates parsers for enum variants from `syn::Expr`
//! - Supports unit variants, tuple variants, and struct variants
//! - Handles both leaf enums (with direct values) and wrapper enums (containing other types)
//! - Provides helpful error messages for parse failures
//! - Works with primitive types and custom types that implement `TryFrom<&syn::Expr>`
//!
//! ## Usage
//!
//! Add the derive macro to your enum:
//!
//! ```rust,ignore
//! use try_from_expr::TryFromExpr;
//! use syn::Expr;
//!
//! #[derive(TryFromExpr)]
//! enum MyEnum {
//!     Unit,
//!     Tuple(String, i32),
//!     Struct { field: bool },
//! }
//!
//! // Parse from a syn::Expr
//! let expr: Expr = syn::parse_str("MyEnum::Tuple(\"hello\", 42)").unwrap();
//! let value = MyEnum::try_from(&expr).unwrap();
//! ```
//!
//! ## Meta Syntax
//!
//! Every variant also parses from a snake_case form suited to attribute
//! arguments, so the variants above read as `unit`, `tuple = ("hello", 42)`
//! and `r#struct(field = true)`. A name that is a keyword takes the raw
//! identifier prefix. Any of these can be wrapped in the enum's own
//! snake_case name, as in `my_enum(unit)`.
//!
//! ## Wrapper vs Leaf Enums
//!
//! The macro automatically detects whether your enum is a "wrapper" (contains other enums)
//! or a "leaf" (contains direct values). You can override this with attributes:
//!
//! ```rust,ignore
//! #[derive(TryFromExpr)]
//! #[try_from_expr(wrapper)]  // Force wrapper mode
//! enum WrapperEnum {
//!     Variant1(OtherEnum),
//!     Variant2(AnotherEnum),
//! }
//!
//! #[derive(TryFromExpr)]
//! #[try_from_expr(leaf)]  // Force leaf mode
//! enum LeafEnum {
//!     Simple,
//!     WithValue(String),
//! }
//! ```
//!
//! ## Children Sharing a Name
//!
//! A wrapper routes a bare name like `trim` to the one child that accepts it,
//! using the names each child publishes through [`meta_names::MetaNames`]. A
//! bare name that two children accept is a parse error asking for the
//! qualified form, such as `string_validator(trim)`. Add
//! `#[try_from_expr(all_unique)]` to a wrapper to make any overlap a compile
//! error instead. A hand-written child type implements `MetaNames` itself.

pub mod meta_names;

/// The derive macro for generating `TryFrom<&syn::Expr>` implementations.
///
/// This macro can be applied to enums to automatically generate parsing logic
/// from `syn::Expr` expressions. It supports:
///
/// - Unit variants (e.g., `MyEnum::Simple`)
/// - Tuple variants (e.g., `MyEnum::Tuple(value1, value2)`)
/// - Struct variants (e.g., `MyEnum::Struct { field: value }`)
///
/// The macro intelligently determines whether the enum is a "wrapper" enum
/// (containing other enum types) or a "leaf" enum (containing direct values),
/// and generates optimized parsing code accordingly.
pub use try_from_expr_derive::TryFromExpr;

// Re-export dependencies that the macro needs
pub use ordered_float;
pub use syn;
