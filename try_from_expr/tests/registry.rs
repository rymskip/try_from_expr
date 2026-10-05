use syn::{Expr, parse_quote};
use try_from_expr::{
    TryFromExpr,
    meta_names::{MetaNameSet, MetaNames},
};

#[derive(Debug, PartialEq, TryFromExpr)]
enum TextRule {
    Required,
    Trim,
}

#[derive(Debug, PartialEq, TryFromExpr)]
enum NumberRule {
    Required,
    Positive,
}

#[derive(Debug, PartialEq, TryFromExpr)]
enum Rule {
    Text(TextRule),
    Number(NumberRule),
}

fn rule(expr: Expr) -> Rule {
    Rule::try_from(&expr).expect("rule parses")
}

fn rule_error(expr: Expr) -> String {
    Rule::try_from(&expr)
        .expect_err("rule is rejected")
        .to_string()
}

#[test]
fn test_shared_bare_name_is_ambiguous() {
    let ambiguous = rule_error(parse_quote!(required));
    assert_eq!(
        ambiguous,
        "Ambiguous meta name `required` for enum 'Rule': accepted by TextRule, NumberRule. \
         Qualify it with `text_rule(..)` or `number_rule(..)`"
    );

    let wrapped = rule_error(parse_quote!(rule(required)));
    assert!(
        wrapped.starts_with("Ambiguous meta name `required`"),
        "{wrapped}"
    );

    let pascal = rule_error(parse_quote!(Required));
    assert!(
        pascal.starts_with("Ambiguous meta name `Required`"),
        "{pascal}"
    );
}

#[test]
fn test_shared_name_resolves_when_qualified() {
    assert_eq!(
        rule(parse_quote!(text_rule(required))),
        Rule::Text(TextRule::Required)
    );
    assert_eq!(
        rule(parse_quote!(number_rule(required))),
        Rule::Number(NumberRule::Required)
    );
    assert_eq!(
        rule(parse_quote!(NumberRule::Required)),
        Rule::Number(NumberRule::Required)
    );
    assert_eq!(
        rule(parse_quote!(number = required)),
        Rule::Number(NumberRule::Required)
    );
}

#[test]
fn test_unique_bare_names_route_to_their_child() {
    assert_eq!(rule(parse_quote!(trim)), Rule::Text(TextRule::Trim));
    assert_eq!(
        rule(parse_quote!(positive)),
        Rule::Number(NumberRule::Positive)
    );
    assert_eq!(
        rule(parse_quote!(Positive)),
        Rule::Number(NumberRule::Positive)
    );
}

#[test]
fn test_own_variant_sharing_a_child_name_is_ambiguous() {
    #[derive(Debug, PartialEq, TryFromExpr)]
    #[try_from_expr(wrapper)]
    enum Field {
        Required,
        Text(TextRule),
    }

    let ambiguous = Field::try_from(&parse_quote!(required))
        .expect_err("own and child names overlap")
        .to_string();
    assert_eq!(
        ambiguous,
        "Ambiguous meta name `required` for enum 'Field': accepted by Field, TextRule. \
         Qualify it with `text_rule(..)`"
    );
    assert_eq!(
        Field::try_from(&parse_quote!(Field::Required)).expect("path form parses"),
        Field::Required
    );
    assert_eq!(
        Field::try_from(&parse_quote!(text_rule(required))).expect("qualified child parses"),
        Field::Text(TextRule::Required)
    );
}

#[test]
fn test_nested_wrappers_and_options_route_grandchildren() {
    #[derive(Debug, PartialEq, TryFromExpr)]
    enum Shape {
        Square,
        Circle,
    }

    #[derive(Debug, PartialEq, TryFromExpr)]
    enum Outer {
        Rule(Rule),
        Shape(Option<Shape>),
    }

    assert_eq!(
        Outer::try_from(&parse_quote!(trim)).expect("grandchild parses"),
        Outer::Rule(Rule::Text(TextRule::Trim))
    );
    assert_eq!(
        Outer::try_from(&parse_quote!(square)).expect("child behind Option parses"),
        Outer::Shape(Some(Shape::Square))
    );

    // The inner wrapper reports its own overlap
    let ambiguous = Outer::try_from(&parse_quote!(required))
        .expect_err("the overlap inside Rule is still ambiguous")
        .to_string();
    assert!(
        ambiguous.contains("Ambiguous meta name `required` for enum 'Rule'"),
        "{ambiguous}"
    );
}

#[test]
fn test_hand_written_child_publishes_its_names() {
    #[derive(Debug, PartialEq)]
    struct Flag;

    impl TryFrom<&Expr> for Flag {
        type Error = syn::Error;

        fn try_from(expr: &Expr) -> Result<Self, Self::Error> {
            match expr {
                Expr::Path(path_expr) if path_expr.path.is_ident("flag") => Ok(Flag),
                other => Err(syn::Error::new_spanned(other, "Expected `flag`")),
            }
        }
    }

    impl MetaNames for Flag {
        const META_NAMES: MetaNameSet = MetaNameSet {
            path_names: &["flag"],
            ..MetaNameSet::empty("Flag")
        };
    }

    #[derive(Debug, PartialEq, TryFromExpr)]
    #[try_from_expr(all_unique)]
    enum Setting {
        Count(u32),
        Flag(Flag),
        Text(TextRule),
    }

    assert_eq!(
        Setting::try_from(&parse_quote!(flag)).expect("hand-written child parses"),
        Setting::Flag(Flag)
    );
    assert_eq!(
        Setting::try_from(&parse_quote!(3)).expect("literal falls back to the first child"),
        Setting::Count(3)
    );
}
