use try_from_expr::TryFromExpr;

#[derive(TryFromExpr)]
#[try_from_expr(wrapper)]
enum Childless {
    Unit,
    Named { value: i32 },
}

fn main() {}
