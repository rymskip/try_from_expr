use try_from_expr::TryFromExpr;

#[derive(TryFromExpr)]
enum TextRule {
    Required,
}

#[derive(TryFromExpr)]
enum NumberRule {
    Required,
}

#[derive(TryFromExpr)]
#[try_from_expr(all_unique)]
enum Rule {
    Text(TextRule),
    Number(NumberRule),
}

fn main() {}
