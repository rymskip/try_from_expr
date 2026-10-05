//! Every example in the README's Meta Syntax section, so the docs cannot drift.
//! The self-named struct variant examples live in `tests/ui-pass` and
//! `tests/ui/ambiguous_struct_field.rs`, since clippy flags such enums.

use std::collections::HashMap;
use syn::{Attribute, Expr, Token, parse_quote, punctuated::Punctuated};
use try_from_expr::TryFromExpr;

#[derive(Debug, PartialEq, TryFromExpr)]
enum Format {
    Email,
    Custom(String),
}

#[derive(Debug, PartialEq, TryFromExpr)]
enum StringValidator {
    Trim,
    MaxLength(Option<usize>),
    OneOf(Vec<String>),
    RegexLiteral(Format),
    Length { min: usize, max: Option<usize> },
}

#[derive(Debug, PartialEq, TryFromExpr)]
enum NumberValidator {
    Positive,
    Between(i64, i64),
    Labels(HashMap<String, i64>),
}

#[derive(Debug, PartialEq, TryFromExpr)]
enum Validator {
    String(StringValidator),
    Number(NumberValidator),
}

fn string(expr: Expr) -> StringValidator {
    StringValidator::try_from(&expr).expect("string validator parses")
}

fn number(expr: Expr) -> NumberValidator {
    NumberValidator::try_from(&expr).expect("number validator parses")
}

fn validator(expr: Expr) -> Validator {
    Validator::try_from(&expr).expect("validator parses")
}

#[test]
fn test_unit_variants() {
    assert_eq!(string(parse_quote!(trim)), StringValidator::Trim);
    assert_eq!(string(parse_quote!(Trim)), StringValidator::Trim);
}

#[test]
fn test_tuple_variants() {
    assert_eq!(
        string(parse_quote!(max_length = 64)),
        StringValidator::MaxLength(Some(64))
    );
    assert_eq!(
        string(parse_quote!(max_length = Some(64))),
        StringValidator::MaxLength(Some(64))
    );
    assert_eq!(
        string(parse_quote!(max_length = None)),
        StringValidator::MaxLength(None)
    );
    assert_eq!(
        string(parse_quote!(one_of = ["draft", "published"])),
        StringValidator::OneOf(vec!["draft".into(), "published".into()])
    );
    assert_eq!(
        string(parse_quote!(one_of = vec!["draft"])),
        StringValidator::OneOf(vec!["draft".into()])
    );
    assert_eq!(
        string(parse_quote!(regex_literal = format(email))),
        StringValidator::RegexLiteral(Format::Email)
    );
    assert_eq!(
        string(parse_quote!(regex_literal = Format::Email)),
        StringValidator::RegexLiteral(Format::Email)
    );
    assert_eq!(
        string(parse_quote!(regex_literal = format(custom = "^a"))),
        StringValidator::RegexLiteral(Format::Custom("^a".into()))
    );
}

#[test]
fn test_multi_field_and_map_variants() {
    assert_eq!(
        number(parse_quote!(between = (1, 10))),
        NumberValidator::Between(1, 10)
    );
    assert_eq!(
        number(parse_quote!(between = (-5, 5))),
        NumberValidator::Between(-5, 5)
    );
    assert_eq!(
        number(parse_quote!(labels = [("low", 0), ("high", 100)])),
        NumberValidator::Labels(HashMap::from([("low".into(), 0), ("high".into(), 100)]))
    );
}

#[test]
fn test_struct_variants() {
    let bounded = StringValidator::Length {
        min: 1,
        max: Some(64),
    };
    assert_eq!(string(parse_quote!(length(min = 1, max = 64))), bounded);
    assert_eq!(string(parse_quote!(length(max = 64, min = 1))), bounded);
    assert_eq!(
        string(parse_quote!(length(min = 1))),
        StringValidator::Length { min: 1, max: None }
    );
}

#[test]
fn test_naming_the_enum() {
    assert_eq!(
        string(parse_quote!(string_validator(trim))),
        StringValidator::Trim
    );
    assert_eq!(
        string(parse_quote!(string_validator(length(min = 1)))),
        StringValidator::Length { min: 1, max: None }
    );
    assert_eq!(
        number(parse_quote!(number_validator(between = (1, 10)))),
        NumberValidator::Between(1, 10)
    );
}

#[test]
fn test_wrapper_enums() {
    let trim = Validator::String(StringValidator::Trim);
    assert_eq!(validator(parse_quote!(trim)), trim);
    assert_eq!(
        validator(parse_quote!(between = (1, 10))),
        Validator::Number(NumberValidator::Between(1, 10))
    );
    assert_eq!(
        validator(parse_quote!(string_validator(max_length = 64))),
        Validator::String(StringValidator::MaxLength(Some(64)))
    );
    assert_eq!(
        validator(parse_quote!(number = positive)),
        Validator::Number(NumberValidator::Positive)
    );
    assert_eq!(validator(parse_quote!(validator(string = trim))), trim);
    assert_eq!(validator(parse_quote!(StringValidator::Trim)), trim);
}

#[test]
fn test_parsing_an_attribute() -> syn::Result<()> {
    let attr: Attribute = syn::parse_quote!(#[validators(
        trim,
        length(min = 1, max = 64),
        string_validator(regex_literal = format(custom = r"^/(?:[^/\\].*)?$")),
    )]);

    let validators = attr
        .parse_args_with(Punctuated::<Expr, Token![,]>::parse_terminated)?
        .iter()
        .map(Validator::try_from)
        .collect::<syn::Result<Vec<_>>>()?;

    let path_form: Attribute = parse_quote!(#[validators(
        StringValidator::Trim,
        StringValidator::Length { min: 1, max: Some(64) },
        StringValidator::RegexLiteral(Format::Custom(r"^/(?:[^/\\].*)?$")),
    )]);
    let path_validators = path_form
        .parse_args_with(Punctuated::<Expr, Token![,]>::parse_terminated)?
        .iter()
        .map(Validator::try_from)
        .collect::<syn::Result<Vec<_>>>()?;

    assert_eq!(validators, path_validators);
    assert_eq!(
        validators,
        vec![
            Validator::String(StringValidator::Trim),
            Validator::String(StringValidator::Length {
                min: 1,
                max: Some(64)
            }),
            Validator::String(StringValidator::RegexLiteral(Format::Custom(
                r"^/(?:[^/\\].*)?$".into()
            ))),
        ]
    );
    Ok(())
}

#[test]
fn test_mistakes() {
    let error = |expr: Expr| {
        StringValidator::try_from(&expr)
            .expect_err("the form is rejected")
            .to_string()
    };
    assert_eq!(
        error(parse_quote!(length(min = 1, mx = 64))),
        "Unknown field 'mx' for variant 'Length'. Valid fields: min, max"
    );
    assert_eq!(
        error(parse_quote!(string_validator(trimm))),
        "Unknown unit variant 'trimm' for enum 'StringValidator'. Valid options: Trim; in meta form: trim"
    );
    assert_eq!(
        NumberValidator::try_from(&parse_quote!(between = 1))
            .expect_err("a bare value is rejected")
            .to_string(),
        "Variant 'Between' expects a tuple of 2 values"
    );
}
