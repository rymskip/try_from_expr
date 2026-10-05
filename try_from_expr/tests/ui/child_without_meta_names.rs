use try_from_expr::TryFromExpr;

struct Point;

impl TryFrom<&syn::Expr> for Point {
    type Error = syn::Error;

    fn try_from(expr: &syn::Expr) -> Result<Self, Self::Error> {
        Err(syn::Error::new_spanned(expr, "never"))
    }
}

#[derive(TryFromExpr)]
enum Geometry {
    Point(Point),
    Count(u32),
}

fn main() {}
