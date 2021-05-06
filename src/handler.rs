use crate::internal_prelude::*;
use crate::middleware::Middleware;
use crate::server::Server;
use crate::state;

use std::sync::Arc;

use futures::future;

pub trait Handler: Send + Sync {
    fn handle<'t, 'a>(&'t self, req: Request) -> BoxFuture<'a, Result<Response>>
    where
        't: 'a,
        Self: 'a;

    fn with_state<S>(self, state: Arc<S>) -> WithState<Self, S>
    where
        Self: Sized,
        S: Send + Sync + 'static,
    {
        WithState { h: self, s: state }
    }

    fn wrap<M>(self, middleware: M) -> Wrap<Self, M>
    where
        Self: Sized,
        M: Middleware,
    {
        Wrap {
            h: self,
            m: middleware,
        }
    }

    fn boxed(self) -> Box<dyn Handler>
    where
        Self: Sized + 'static,
    {
        Box::new(self)
    }

    fn into_server(self) -> Server
    where
        Self: Sized + 'static,
    {
        Server::new(Box::new(self))
    }
}

impl Handler for Box<dyn Handler> {
    fn handle<'t, 'a>(&'t self, req: Request) -> BoxFuture<'a, Result<Response>>
    where
        't: 'a,
        Self: 'a,
    {
        Handler::handle(&**self, req)
    }
}

pub struct WithState<H, S> {
    h: H,
    s: Arc<S>,
}

impl<H, S> Handler for WithState<H, S>
where
    H: Handler,
    S: Send + Sync + 'static,
{
    fn handle<'t, 'a>(&'t self, req: Request) -> BoxFuture<'a, Result<Response>>
    where
        't: 'a,
        Self: 'a,
    {
        let mut fut = state::enter(self.s.clone(), || self.h.handle(req));
        Box::pin(future::poll_fn(move |cx| {
            state::enter(self.s.clone(), || fut.as_mut().poll(cx))
        }))
    }
}

pub struct Wrap<H, M> {
    h: H,
    m: M,
}

impl<H, M> Handler for Wrap<H, M>
where
    H: Handler,
    M: Middleware,
{
    fn handle<'t, 'a>(&'t self, req: Request) -> BoxFuture<'a, Result<Response>>
    where
        't: 'a,
        Self: 'a,
    {
        self.m.handle(req, &self.h)
    }
}
