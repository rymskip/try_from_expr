use try_from_expr::TryFromExpr;
use std::collections::{HashMap, BTreeMap};
use syn::{Expr, parse_quote};

#[test]
fn test_unit_variants() {
    #[derive(Debug, PartialEq, TryFromExpr)]
    enum Status {
        Active,
        Inactive,
        Pending,
    }

    let expr: Expr = parse_quote!(Status::Active);
    let status = Status::try_from(&expr).unwrap();
    assert_eq!(status, Status::Active);

    let expr: Expr = parse_quote!(Status::Inactive);
    let status = Status::try_from(&expr).unwrap();
    assert_eq!(status, Status::Inactive);
}

#[test]
fn test_tuple_variants_primitives() {
    #[derive(Debug, PartialEq, TryFromExpr)]
    enum Primitives {
        Ints(u8, i16, u32, i64),
        Floats(f32, f64),
        BoolChar(bool, char),
    }

    let expr: Expr = parse_quote!(Primitives::Ints(1, -2, 3, -4));
    let val = Primitives::try_from(&expr).unwrap();
    if let Primitives::Ints(a, b, c, d) = val {
        assert_eq!(a, 1);
        assert_eq!(b, -2);
        assert_eq!(c, 3);
        assert_eq!(d, -4);
    } else {
        panic!("Wrong variant");
    }

    let expr: Expr = parse_quote!(Primitives::Floats(1.5, -2.5));
    let val = Primitives::try_from(&expr).unwrap();
    if let Primitives::Floats(a, b) = val {
        assert_eq!(a, 1.5);
        assert_eq!(b, -2.5);
    } else {
        panic!("Wrong variant");
    }

    let expr: Expr = parse_quote!(Primitives::BoolChar(true, 'x'));
    let val = Primitives::try_from(&expr).unwrap();
    if let Primitives::BoolChar(a, b) = val {
        assert_eq!(a, true);
        assert_eq!(b, 'x');
    } else {
        panic!("Wrong variant");
    }
}

#[test]
fn test_struct_variants() {
    #[derive(Debug, PartialEq, TryFromExpr)]
    enum Config {
        Server { host: String, port: u16, debug: bool },
    }

    let expr: Expr = parse_quote!(Config::Server { host: "localhost", port: 8080, debug: true });
    let config = Config::try_from(&expr).unwrap();
    
    match config {
        Config::Server { host, port, debug } => {
            assert_eq!(host, "localhost");
            assert_eq!(port, 8080);
            assert!(debug);
        }
    }
}

#[test]
fn test_collections() {
    #[derive(Debug, PartialEq, TryFromExpr)]
    enum Collections {
        List(Vec<i32>),
        Map(HashMap<String, i32>),
    }

    // Test Vec with vec! macro
    let expr: Expr = parse_quote!(Collections::List(vec![1, 2, 3]));
    let val = Collections::try_from(&expr).unwrap();
    match val {
        Collections::List(v) => assert_eq!(v, vec![1, 2, 3]),
        _ => panic!("Wrong variant"),
    }

    // Test Vec with array literal
    let expr: Expr = parse_quote!(Collections::List([4, 5, 6]));
    let val = Collections::try_from(&expr).unwrap();
    match val {
        Collections::List(v) => assert_eq!(v, vec![4, 5, 6]),
        _ => panic!("Wrong variant"),
    }

    // Test HashMap
    let expr: Expr = parse_quote!(Collections::Map([("a", 1), ("b", 2)]));
    let val = Collections::try_from(&expr).unwrap();
    match val {
        Collections::Map(m) => {
            assert_eq!(m.get("a"), Some(&1));
            assert_eq!(m.get("b"), Some(&2));
        },
        _ => panic!("Wrong variant"),
    }
}

#[test]
fn test_options() {
    #[derive(Debug, PartialEq, TryFromExpr)]
    enum Options {
        MaybeInt(Option<i32>),
        Struct { opt: Option<String> },
    }

    // Explicit Some
    let expr: Expr = parse_quote!(Options::MaybeInt(Some(42)));
    let val = Options::try_from(&expr).unwrap();
    assert_eq!(val, Options::MaybeInt(Some(42)));

    // Explicit None
    let expr: Expr = parse_quote!(Options::MaybeInt(None));
    let val = Options::try_from(&expr).unwrap();
    assert_eq!(val, Options::MaybeInt(None));

    // Implicit Some
    let expr: Expr = parse_quote!(Options::MaybeInt(42));
    let val = Options::try_from(&expr).unwrap();
    assert_eq!(val, Options::MaybeInt(Some(42)));

    // Struct optional field present
    let expr: Expr = parse_quote!(Options::Struct { opt: "hello" });
    let val = Options::try_from(&expr).unwrap();
    match val {
        Options::Struct { opt } => assert_eq!(opt, Some("hello".to_string())),
        _ => panic!("Wrong variant"),
    }

    // Struct optional field missing
    let expr: Expr = parse_quote!(Options::Struct {});
    let val = Options::try_from(&expr).unwrap();
    match val {
        Options::Struct { opt } => assert_eq!(opt, None),
        _ => panic!("Wrong variant"),
    }
}

#[test]
fn test_wrapper_enums() {
    #[derive(Debug, PartialEq, TryFromExpr)]
    enum InnerA { Variant(i32) }
    
    #[derive(Debug, PartialEq, TryFromExpr)]
    enum InnerB { Variant(String) }

    #[derive(Debug, PartialEq, TryFromExpr)]
    enum Wrapper {
        A(InnerA),
        B(InnerB),
    }

    // Test parsing InnerA through Wrapper
    let expr: Expr = parse_quote!(Wrapper::A(InnerA::Variant(123)));
    let val = Wrapper::try_from(&expr).unwrap();
    match val {
        Wrapper::A(InnerA::Variant(i)) => assert_eq!(i, 123),
        _ => panic!("Wrong variant"),
    }

    // Test direct inner type parsing (if implemented by macro optimization)
    let expr: Expr = parse_quote!(InnerB::Variant("hello"));
    let val = Wrapper::try_from(&expr).unwrap();
    match val {
        Wrapper::B(InnerB::Variant(s)) => assert_eq!(s, "hello"),
        _ => panic!("Wrong variant"),
    }
}

#[test]
fn test_error_handling() {
    #[derive(Debug, PartialEq, TryFromExpr)]
    enum Simple { A(i32) }

    // Wrong type
    let expr: Expr = parse_quote!(Simple::A("not an int"));
    let err = Simple::try_from(&expr).unwrap_err();
    assert!(err.to_string().contains("Expected an integer literal"));

    // Wrong variant count
    let expr: Expr = parse_quote!(Simple::A(1, 2));
    let err = Simple::try_from(&expr).unwrap_err();
    assert!(err.to_string().contains("expects 1 argument(s)"));

    // Unknown variant
    let expr: Expr = parse_quote!(Simple::Unknown(1));
    let err = Simple::try_from(&expr).unwrap_err();
    assert!(err.to_string().contains("Unknown parametrized variant"));
}

#[test]
fn test_complex_nested() {
    #[derive(Debug, PartialEq)]
    struct CustomPoint { x: i32, y: i32 }

    // Custom impl for CustomPoint to use in enum
    impl TryFrom<&Expr> for CustomPoint {
        type Error = syn::Error;
        fn try_from(expr: &Expr) -> Result<Self, Self::Error> {
             if let Expr::Struct(s) = expr {
                 // Simplified parsing for test
                 Ok(CustomPoint { x: 10, y: 20 })
             } else {
                 Err(syn::Error::new_spanned(expr, "Expected struct"))
             }
        }
    }

    #[derive(Debug, PartialEq, TryFromExpr)]
    enum Shape {
        Circle { center: CustomPoint, radius: f64 },
        Polygon(Vec<CustomPoint>),
    }

    let expr: Expr = parse_quote!(Shape::Circle { center: CustomPoint { x: 10, y: 20 }, radius: 5.0 });
    let shape = Shape::try_from(&expr).unwrap();
    match shape {
        Shape::Circle { center, radius } => {
            assert_eq!(center.x, 10);
            assert_eq!(center.y, 20);
            assert_eq!(radius, 5.0);
        }
        _ => panic!("Wrong variant"),
    }
}