use nuclear::functional::handler;
use nuclear::prelude::{Handler, Request, Result};

async fn hello(_req: Request) -> &'static str {
    "hello"
}

#[tokio::main]
async fn main() -> Result<()> {
    handler(hello).into_server().run("127.0.0.1:8080").await
}
