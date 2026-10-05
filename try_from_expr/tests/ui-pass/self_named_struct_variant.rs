use syn::{Expr, parse_quote};
use try_from_expr::TryFromExpr;

fn main() {
    #[derive(Debug, PartialEq, TryFromExpr)]
    enum Config {
        Config { host: String, port: Option<u16> },
        Verbose,
        Level(u8),
    }

    let parse = |expr: Expr| Config::try_from(&expr).expect("config parses");
    let localhost = Config::Config {
        host: "localhost".to_string(),
        port: None,
    };

    assert_eq!(parse(parse_quote!(config(host = "localhost"))), localhost);
    assert_eq!(
        parse(parse_quote!(config(config(host = "localhost")))),
        localhost
    );
    assert_eq!(
        parse(parse_quote!(config(host = "localhost", port = 8080))),
        Config::Config {
            host: "localhost".to_string(),
            port: Some(8080)
        }
    );
    assert_eq!(parse(parse_quote!(config(verbose))), Config::Verbose);
    assert_eq!(parse(parse_quote!(config(level = 3))), Config::Level(3));

    let typo = Config::try_from(&parse_quote!(config(hots = "localhost")))
        .expect_err("an unknown field is rejected")
        .to_string();
    assert_eq!(
        typo,
        "Unknown tuple variant 'hots' for enum 'Config'. Valid options: level"
    );
}
