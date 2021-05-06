use crate::handler::Handler;
use crate::internal_prelude::*;

use std::net::{TcpListener, ToSocketAddrs};
use std::sync::Arc;
use std::task::{Context, Poll};

use futures::future;

struct ServerInner {
    handler: Box<dyn Handler>,
}

async fn hyper_call(server: &ServerInner, req: HyperRequest) -> Result<HyperResponse> {
    let req = Request::from_hyper(req);
    let res = server.handler.handle(req).await?;
    Ok(res.into_hyper())
}

pub struct Server {
    inner: Arc<ServerInner>,
}

impl Server {
    pub fn new(handler: Box<dyn Handler>) -> Self {
        Self {
            inner: Arc::new(ServerInner { handler }),
        }
    }

    pub async fn run(self, addr: impl ToSocketAddrs) -> Result<()> {
        let listener = TcpListener::bind(&addr)?;
        let builder = HyperServer::from_tcp(listener)?;
        let hyper_server = builder.serve(self);
        hyper_server.await?;
        Ok(())
    }
}

impl hyper::service::Service<HyperRequest> for Server {
    type Response = HyperResponse;
    type Error = crate::error::Error;
    type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, _: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: HyperRequest) -> Self::Future {
        let inner = Arc::clone(&self.inner);
        Box::pin(async move { hyper_call(&*inner, req).await })
    }
}

impl hyper::service::Service<&'_ hyper::server::conn::AddrStream> for Server {
    type Response = Self;
    type Error = anyhow::Error;
    type Future = future::Ready<Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, _: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, _req: &'_ hyper::server::conn::AddrStream) -> Self::Future {
        future::ready(Ok(Self {
            inner: Arc::clone(&self.inner),
        }))
    }
}
