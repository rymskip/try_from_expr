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
enum Rule {
    Text(TextRule),
    Number(NumberRule),
}

#[derive(TryFromExpr)]
enum Shape {
    Square,
}

#[derive(TryFromExpr)]
#[try_from_expr(all_unique)]
enum Field {
    Rule(Rule),
    Shape(Shape),
}

fn main() {}
