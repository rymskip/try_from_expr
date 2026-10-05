use try_from_expr::TryFromExpr;

#[derive(TryFromExpr)]
#[try_from_expr(wrap)]
enum Unknown {
    Unit,
}

fn main() {}
