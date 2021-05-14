use nuclear::error::StatusError;
use nuclear::functional::{ref_handler, ref_middleware};
use nuclear::prelude::{Handler, Request, Responder, Response, Result};
use nuclear::router::{SimpleRouter, SimpleRouterExt};

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

struct App {
    count: AtomicUsize,
}

impl App {
    fn new() -> Self {
        Self {
            count: AtomicUsize::new(0),
        }
    }

    fn into_handler(self) -> impl Handler {
        let get_hello: _ = ref_handler(Self::get_hello);
        let get_world: _ = ref_handler(Self::get_world);
        let not_found: _ = ref_handler(Self::not_found);
        let recover: _ = ref_middleware(Self::recover);

        let mut router: SimpleRouter = SimpleRouter::new();

        router.at("/hello/:name").get(get_hello.boxed());
        router.at("/world").get(get_world.boxed());
        router.set_default(not_found.boxed());

        router.wrap(recover).with_state(Arc::new(self))
    }

    async fn get_hello(&self, req: Request) -> String {
        let name = req.expect_param("name");
        format!("GET /hello/{}", name)
    }

    async fn get_world(&self, _: Request) -> String {
        let count = self.count.fetch_add(1, Ordering::Relaxed) + 1;
        format!("GET /world => {}", count)
    }

    async fn not_found(&self, _: Request) -> Result<Response> {
        Err(StatusError::NOT_FOUND.into())
    }

    async fn recover(&self, req: Request, next: &dyn Handler) -> Result<Response> {
        match next.handle(req).await {
            Err(err) => {
                eprintln!("Error: {:?}", err);
                "Oops".respond().await
            }
            ret => ret,
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let h = App::new().into_handler();
    h.into_server().run("127.0.0.1:8080").await
}
