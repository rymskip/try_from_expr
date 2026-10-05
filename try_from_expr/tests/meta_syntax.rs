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
    RegexLiteral(Format),
    MaxLength(Option<usize>),
    Length { min: usize, max: Option<usize> },
}

#[derive(Debug, PartialEq, TryFromExpr)]
enum NumberValidator {
    Positive,
    Between(i64, i64),
}

#[derive(Debug, PartialEq, TryFromExpr)]
enum Validator {
    String(StringValidator),
    Number(NumberValidator),
}

fn validators(attr: &Attribute) -> Vec<Validator> {
    attr.parse_args_with(Punctuated::<Expr, Token![,]>::parse_terminated)
        .expect("attribute arguments are expressions")
        .iter()
        .map(Validator::try_from)
        .collect::<Result<_, _>>()
        .expect("every validator parses")
}

fn string_validator(expr: Expr) -> StringValidator {
    StringValidator::try_from(&expr).expect("string validator parses")
}

fn string_validator_error(expr: Expr) -> String {
    StringValidator::try_from(&expr)
        .expect_err("string validator is rejected")
        .to_string()
}

fn validator(expr: Expr) -> Validator {
    Validator::try_from(&expr).expect("validator parses")
}

#[test]
fn test_meta_attribute_matches_path_attribute() {
    let path_form: Attribute = parse_quote!(#[validators(
        StringValidator::Trim,
        StringValidator::RegexLiteral(Format::Custom(r"^/(?:[^/\\].*)?$"))
    )]);
    let meta_form: Attribute = parse_quote!(#[validators(
        string_validator(trim),
        string_validator(regex_literal = format(custom = r"^/(?:[^/\\].*)?$")),
    )]);

    let expected = vec![
        Validator::String(StringValidator::Trim),
        Validator::String(StringValidator::RegexLiteral(Format::Custom(
            r"^/(?:[^/\\].*)?$".to_string(),
        ))),
    ];
    assert_eq!(validators(&path_form), expected);
    assert_eq!(validators(&meta_form), expected);
}

#[test]
fn test_leaf_unit_variants() {
    assert_eq!(string_validator(parse_quote!(trim)), StringValidator::Trim);
    assert_eq!(
        string_validator(parse_quote!(string_validator(trim))),
        StringValidator::Trim
    );
    assert_eq!(string_validator(parse_quote!(Trim)), StringValidator::Trim);
}

#[test]
fn test_leaf_tuple_variants() {
    assert_eq!(
        string_validator(parse_quote!(regex_literal = format(email))),
        StringValidator::RegexLiteral(Format::Email)
    );
    assert_eq!(
        string_validator(parse_quote!(regex_literal = Format::Email)),
        StringValidator::RegexLiteral(Format::Email)
    );
    assert_eq!(
        string_validator(parse_quote!(max_length = None)),
        StringValidator::MaxLength(None)
    );
    assert_eq!(
        string_validator(parse_quote!(max_length = 5)),
        StringValidator::MaxLength(Some(5))
    );
    assert_eq!(
        string_validator(parse_quote!(string_validator(max_length = Some(5)))),
        StringValidator::MaxLength(Some(5))
    );
}

#[test]
fn test_leaf_multi_field_tuple_variant() {
    assert_eq!(
        NumberValidator::try_from(&parse_quote!(between = (1, 10))).expect("between parses"),
        NumberValidator::Between(1, 10)
    );
    assert_eq!(
        NumberValidator::try_from(&parse_quote!(number_validator(between = (-5, 5))))
            .expect("wrapped between parses"),
        NumberValidator::Between(-5, 5)
    );

    let not_a_tuple = NumberValidator::try_from(&parse_quote!(between = 1))
        .expect_err("a bare value is rejected")
        .to_string();
    assert!(
        not_a_tuple.contains("expects a tuple of 2 values"),
        "{not_a_tuple}"
    );

    let wrong_arity = NumberValidator::try_from(&parse_quote!(between = (1, 2, 3)))
        .expect_err("three values are rejected")
        .to_string();
    assert!(
        wrong_arity.contains("expects 2 argument(s), but 3"),
        "{wrong_arity}"
    );
}

#[test]
fn test_leaf_struct_variants() {
    assert_eq!(
        string_validator(parse_quote!(length(min = 1, max = 5))),
        StringValidator::Length {
            min: 1,
            max: Some(5)
        }
    );
    assert_eq!(
        string_validator(parse_quote!(string_validator(length(min = 1)))),
        StringValidator::Length { min: 1, max: None }
    );

    let missing = string_validator_error(parse_quote!(length(max = 2)));
    assert!(
        missing.contains("Missing required field 'min'"),
        "{missing}"
    );

    let unknown = string_validator_error(parse_quote!(length(min = 1, mx = 2)));
    assert!(unknown.contains("Unknown field 'mx'"), "{unknown}");

    let duplicate = string_validator_error(parse_quote!(length(min = 1, min = 2)));
    assert!(duplicate.contains("Duplicate field 'min'"), "{duplicate}");

    let positional = string_validator_error(parse_quote!(length(1)));
    assert!(
        positional.contains("Expected `field = value`"),
        "{positional}"
    );
}

#[test]
fn test_leaf_unknown_meta_variants() {
    let unit = string_validator_error(parse_quote!(string_validator(trimm)));
    assert!(
        unit.contains("Unknown unit variant 'trimm' for enum 'StringValidator'. Valid options: Trim; in meta form: trim"),
        "{unit}"
    );

    let tuple = string_validator_error(parse_quote!(regex = format(email)));
    assert!(
        tuple.contains("Unknown tuple variant 'regex'")
            && tuple.contains("regex_literal, max_length"),
        "{tuple}"
    );

    let structure = string_validator_error(parse_quote!(lenght(min = 1)));
    assert!(
        structure.contains("Unknown struct variant 'lenght'"),
        "{structure}"
    );

    let arity = string_validator_error(parse_quote!(string_validator(trim, trim)));
    assert!(
        arity.contains("`string_validator(..)` takes exactly one variant"),
        "{arity}"
    );

    let mixed = string_validator_error(parse_quote!(string_validator(StringValidator::Trim)));
    assert!(
        mixed.contains("Expected `variant`, `variant = value`"),
        "{mixed}"
    );
}

#[test]
fn test_wrapper_meta_forms() {
    let trim = Validator::String(StringValidator::Trim);
    assert_eq!(validator(parse_quote!(string_validator(trim))), trim);
    assert_eq!(validator(parse_quote!(string = trim)), trim);
    assert_eq!(
        validator(parse_quote!(validator(string_validator(trim)))),
        trim
    );
    assert_eq!(validator(parse_quote!(validator(string = trim))), trim);
    assert_eq!(
        validator(parse_quote!(string = string_validator(trim))),
        trim
    );
    assert_eq!(
        validator(parse_quote!(positive)),
        Validator::Number(NumberValidator::Positive)
    );
    assert_eq!(
        validator(parse_quote!(between = (1, 2))),
        Validator::Number(NumberValidator::Between(1, 2))
    );

    let unknown = Validator::try_from(&parse_quote!(string_validator(trimm)))
        .expect_err("an unknown child variant is rejected")
        .to_string();
    assert!(
        unknown.contains("Failed to parse StringValidator: Unknown unit variant 'trimm'"),
        "{unknown}"
    );
}

#[test]
fn test_struct_literal_rejects_unknown_fields_and_update_syntax() {
    let unknown = string_validator_error(parse_quote!(StringValidator::Length { min: 1, mx: 2 }));
    assert!(unknown.contains("Unknown field 'mx'"), "{unknown}");

    let update = string_validator_error(parse_quote!(StringValidator::Length {
        min: 1,
        ..Default::default()
    }));
    assert!(
        update.contains("Struct update syntax `..` is not supported"),
        "{update}"
    );
}

#[test]
fn test_keyword_names_use_raw_identifiers() {
    #[derive(Debug, PartialEq, TryFromExpr)]
    enum Kind {
        Type(String),
        Ref { r#type: u8 },
    }

    assert_eq!(
        Kind::try_from(&parse_quote!(r#type = "text")).expect("raw tuple variant parses"),
        Kind::Type("text".to_string())
    );
    assert_eq!(
        Kind::try_from(&parse_quote!(kind(r#ref(r#type = 3)))).expect("raw struct variant parses"),
        Kind::Ref { r#type: 3 }
    );
    assert_eq!(
        Kind::try_from(&parse_quote!(Kind::Ref { r#type: 4 })).expect("raw field literal parses"),
        Kind::Ref { r#type: 4 }
    );
}

#[test]
fn test_derive_ui() {
    let cases = trybuild::TestCases::new();
    cases.compile_fail("tests/ui/*.rs");
    // Enums that clippy's `enum_variant_names` flags, built and run by rustc alone
    cases.pass("tests/ui-pass/*.rs");
}
