use try_from_expr::TryFromExpr;

#[derive(TryFromExpr)]
#[try_from_expr(leaf, wrapper)]
enum Duplicate {
    Unit,
}

fn main() {}
