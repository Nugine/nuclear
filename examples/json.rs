use nuclear::body::JsonExt;
use nuclear::functional::{handler, middleware};
use nuclear::http::StatusCode;
use nuclear::prelude::{Handler, Request, Response, Result};
use nuclear::response::{Json, Responder};

use serde_json::Value;

async fn json_echo(mut req: Request) -> Result<Json<Value>> {
    let body = req.json::<Value>().await?;
    println!("{}", body);
    Ok(Json(body))
}

async fn recover(req: Request, next: &dyn Handler) -> Result<Response> {
    match next.handle(req).await {
        Err(err) => {
            let value = serde_json::json!({
                "code": 1000,
                "message": err.to_string(),
            });
            eprintln!("{}", value);

            Json(value)
                .with_status(StatusCode::INTERNAL_SERVER_ERROR)
                .respond()
                .await
        }
        ret => ret,
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let h: _ = handler(json_echo).wrap(middleware(recover));
    h.into_server().run("127.0.0.1:8080").await
}
