use try_from_expr::TryFromExpr;

#[derive(TryFromExpr)]
#[try_from_expr(all_unique)]
enum Shape {
    Square,
    Circle,
}

fn main() {}
