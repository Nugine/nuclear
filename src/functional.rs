use crate::handler::Handler;
use crate::internal_prelude::*;
use crate::state;

use std::any::type_name;
use std::marker::PhantomData;

mod sealed {
    use std::future::Future;

    pub trait AsyncFn<'a, A>: Send + Sync + 'a {
        type Future: Future<Output = Self::Output> + Send + 'a;
        type Output: 'a;

        fn call<'t: 'a>(&'t self, args: A) -> Self::Future;
    }

    macro_rules! impl_async_fn {
    (($($ty:tt,)+),($($id:tt,)+)) => {
            impl<'a, $($ty,)+ F, U, O> AsyncFn<'a, ($($ty,)+)> for F
            where
                $($ty:'a,)+
                F: Fn($($ty,)+) -> U + Send + Sync + 'a,
                U: Future<Output = O> + Send + 'a,
                O: 'a,
            {
                type Future = U;

                type Output = O;

                fn call<'t: 'a>(&'t self, ($($id,)+): ($($ty,)+)) -> Self::Future {
                    (self)($($id,)+)
                }
            }
        };
    }

    impl_async_fn!((A0,), (a0,));
    impl_async_fn!((A0, A1,), (a0, a1,));
    impl_async_fn!((A0, A1, A2,), (a0, a1, a2,));
}

use self::sealed::AsyncFn;

pub fn handler<F>(f: F) -> HandlerFn<F> {
    HandlerFn { f }
}

pub struct HandlerFn<F> {
    f: F,
}

impl<F, R> Handler for HandlerFn<F>
where
    F: for<'a> AsyncFn<'a, (Request,), Output = R>,
    R: Responder,
{
    fn handle<'t, 'a>(&'t self, req: Request) -> BoxFuture<'a, Result<Response>>
    where
        't: 'a,
        Self: 'a,
    {
        Box::pin(async move { AsyncFn::call(&self.f, (req,)).await.respond().await })
    }
}

pub fn ref_handler<S, F>(f: F) -> RefHandlerFn<S, F> {
    RefHandlerFn {
        f,
        _marker: PhantomData,
    }
}

pub struct RefHandlerFn<S, F> {
    f: F,
    _marker: PhantomData<fn(&S)>,
}

impl<S, F, R> Handler for RefHandlerFn<S, F>
where
    S: Send + Sync + 'static,
    F: for<'a> AsyncFn<'a, (&'a S, Request), Output = R>,
    R: Responder,
{
    #[track_caller]
    fn handle<'t, 'a>(&'t self, req: Request) -> BoxFuture<'a, Result<Response>>
    where
        't: 'a,
        Self: 'a,
    {
        let state = match state::inject::<S>() {
            Some(s) => s,
            None => panic!(
                "failed to inject state <{}> for handler <{}>",
                type_name::<S>(),
                type_name::<F>(),
            ),
        };
        Box::pin(async move { AsyncFn::call(&self.f, (&*state, req)).await.respond().await })
    }
}

pub fn middleware<F>(f: F) -> MiddlewareFn<F> {
    MiddlewareFn { f }
}

pub struct MiddlewareFn<F> {
    f: F,
}

impl<F> Middleware for MiddlewareFn<F>
where
    F: for<'a> AsyncFn<'a, (Request, &'a dyn Handler), Output = Result<Response>>,
{
    fn handle<'t, 'n, 'a>(
        &'t self,
        req: Request,
        next: &'a dyn Handler,
    ) -> BoxFuture<'a, Result<Response>>
    where
        't: 'a,
        'n: 'a,
        Self: 'a,
    {
        Box::pin(AsyncFn::call(&self.f, (req, next)))
    }
}

pub fn ref_middleware<S, F>(f: F) -> RefMiddlewareFn<S, F>
where
    S: Send + Sync + 'static,
    F: for<'a> AsyncFn<'a, (&'a S, Request, &'a dyn Handler)>,
{
    RefMiddlewareFn {
        f,
        _marker: PhantomData,
    }
}

pub struct RefMiddlewareFn<S, F> {
    f: F,
    _marker: PhantomData<fn(&S)>,
}

impl<S, F> Middleware for RefMiddlewareFn<S, F>
where
    S: Send + Sync + 'static,
    F: for<'a> AsyncFn<'a, (&'a S, Request, &'a dyn Handler), Output = Result<Response>>,
{
    fn handle<'t, 'n, 'a>(
        &'t self,
        req: Request,
        next: &'a dyn Handler,
    ) -> BoxFuture<'a, Result<Response>>
    where
        't: 'a,
        'n: 'a,
        Self: 'a,
    {
        let state = match state::inject::<S>() {
            Some(s) => s,
            None => panic!(
                "failed to inject state <{}> for middleware <{}>",
                type_name::<S>(),
                type_name::<F>(),
            ),
        };
        Box::pin(async move { AsyncFn::call(&self.f, (&*state, req, next)).await })
    }
}
