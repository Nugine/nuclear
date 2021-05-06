use crate::handler::Handler;
use crate::internal_prelude::*;

pub trait Middleware: Send + Sync {
    fn handle<'t, 'n, 'a>(
        &'t self,
        req: Request,
        next: &'n dyn Handler,
    ) -> BoxFuture<'a, Result<Response>>
    where
        't: 'a,
        'n: 'a,
        Self: 'a;

    fn boxed(self) -> Box<dyn Middleware>
    where
        Self: Sized + 'static,
    {
        Box::new(self)
    }
}

impl Middleware for Box<dyn Middleware> {
    fn handle<'t, 'n, 'a>(
        &'t self,
        req: Request,
        next: &'n dyn Handler,
    ) -> BoxFuture<'a, Result<Response>>
    where
        't: 'a,
        'n: 'a,
        Self: 'a,
    {
        Middleware::handle(&**self, req, next)
    }
}
