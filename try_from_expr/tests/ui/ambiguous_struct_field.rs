use try_from_expr::TryFromExpr;

#[derive(TryFromExpr)]
enum Config {
    Config { level: u8 },
    Level(u8),
}

fn main() {}
