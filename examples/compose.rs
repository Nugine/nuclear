use nuclear::functional::{handler, middleware};
use nuclear::prelude::{Handler, Request, Response, Result};

async fn outer(req: Request, next: &dyn Handler) -> Result<Response> {
    println!("outer: before next");
    let res = next.handle(req).await;
    println!("outer: after next\n");
    res
}

async fn inner(req: Request, next: &dyn Handler) -> Result<Response> {
    println!("inner: before next");
    let res = next.handle(req).await;
    println!("inner: after next");
    res
}

fn compose(h: impl Handler) -> impl Handler {
    let inner = middleware(inner);
    let outer = middleware(outer);
    h.wrap(inner).wrap(outer)
}

#[tokio::main]
async fn main() -> Result<()> {
    let h = compose(handler(|_| async { println!("hello") }));
    h.into_server().run("127.0.0.1:8080").await
}
