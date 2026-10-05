use try_from_expr::TryFromExpr;

#[derive(TryFromExpr)]
enum Server {
    HttpServer,
    HTTPServer,
}

fn main() {}
