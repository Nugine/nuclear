use crate::http::{self, HeaderValue, Mime, StatusCode};
use crate::internal_prelude::*;

use std::ops;

use futures::future::{self, Either, Ready};
use pin_project::pin_project;
use serde::Serialize;

#[derive(Debug)]
pub struct Response {
    inner: Box<HyperResponse>,
}

impl Response {
    pub(crate) fn from_hyper(res: HyperResponse) -> Self {
        Self {
            inner: Box::new(res),
        }
    }

    pub(crate) fn into_hyper(self) -> HyperResponse {
        *self.inner
    }

    pub fn new(status: StatusCode, body: Body) -> Self {
        let mut res = HyperResponse::new(body);
        *res.status_mut() = status;
        Self::from_hyper(res)
    }

    pub fn new_ok(body: Body) -> Self {
        Self::from_hyper(HyperResponse::new(body))
    }

    pub(crate) fn set_static_mime(&mut self, mime: &'static Mime) {
        self.inner.headers_mut().insert(
            http::header::CONTENT_TYPE,
            HeaderValue::from_static(mime.as_ref()),
        );
    }
}

impl ops::Deref for Response {
    type Target = HyperResponse;

    fn deref(&self) -> &Self::Target {
        self.inner.as_ref()
    }
}

impl ops::DerefMut for Response {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.inner.as_mut()
    }
}

impl From<StatusCode> for Response {
    fn from(status: StatusCode) -> Self {
        Response::new(status, Body::empty())
    }
}

pub trait Responder: Send + Sync {
    type Future: Future<Output = Result<Response>> + Send;

    fn respond(self) -> Self::Future;

    fn with_status(self, status: StatusCode) -> WithStatus<Self>
    where
        Self: Sized,
    {
        WithStatus { r: self, status }
    }
}

pub struct WithStatus<R> {
    r: R,
    status: StatusCode,
}

impl<R> Responder for WithStatus<R>
where
    R: Responder,
{
    type Future = CustomResponderFuture<R>;

    fn respond(self) -> Self::Future {
        CustomResponderFuture {
            future: self.r.respond(),
            status: Some(self.status),
        }
    }
}

impl Responder for () {
    type Future = Ready<Result<Response>>;

    fn respond(self) -> Self::Future {
        future::ready(Ok(Response::new_ok(Body::empty())))
    }
}

impl Responder for Response {
    type Future = Ready<Result<Response>>;

    fn respond(self) -> Self::Future {
        future::ready(Ok(self))
    }
}

impl<T, E> Responder for Result<T, E>
where
    T: Responder,
    E: Into<Error> + Send + Sync,
{
    type Future = Either<T::Future, Ready<Result<Response>>>;

    fn respond(self) -> Self::Future {
        match self {
            Ok(res) => Either::Left(res.respond()),
            Err(err) => Either::Right(future::ready(Err(err.into()))),
        }
    }
}

impl Responder for StatusCode {
    type Future = Ready<Result<Response>>;

    fn respond(self) -> Self::Future {
        future::ready(Ok(Response::new(self, Body::empty())))
    }
}

impl<R> Responder for (StatusCode, R)
where
    R: Responder,
{
    type Future = CustomResponderFuture<R>;

    fn respond(self) -> Self::Future {
        CustomResponderFuture {
            future: self.1.respond(),
            status: Some(self.0),
        }
    }
}

#[pin_project]
pub struct CustomResponderFuture<R: Responder> {
    #[pin]
    future: R::Future,
    status: Option<StatusCode>,
}

impl<R> Future for CustomResponderFuture<R>
where
    R: Responder,
{
    type Output = Result<Response>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.project();
        let mut ret = futures::ready!(this.future.poll(cx));
        if let Ok(ref mut res) = ret {
            if let Some(status) = this.status.take() {
                *res.status_mut() = status
            }
        }
        Poll::Ready(ret)
    }
}

fn text(s: impl Into<String>) -> Response {
    let mut res = Response::new_ok(Body::from(s.into()));
    res.set_static_mime(&mime::TEXT_PLAIN_UTF_8);
    res
}

impl<'a> Responder for &'a str {
    type Future = Ready<Result<Response>>;

    fn respond(self) -> Self::Future {
        future::ready(Ok(text(self)))
    }
}

impl Responder for Box<str> {
    type Future = Ready<Result<Response>>;

    fn respond(self) -> Self::Future {
        future::ready(Ok(text(self)))
    }
}

impl Responder for String {
    type Future = Ready<Result<Response>>;

    fn respond(self) -> Self::Future {
        future::ready(Ok(text(self)))
    }
}

fn json<T>(value: T) -> Result<Response, serde_json::Error>
where
    T: Serialize,
{
    let bytes_vec = serde_json::to_vec(&value)?;
    let mut res = Response::new_ok(Body::from(bytes_vec));
    res.set_static_mime(&mime::APPLICATION_JSON);
    Ok(res)
}

pub struct Json<T>(pub T);

impl<T> Responder for Json<T>
where
    T: Serialize + Send + Sync,
{
    type Future = Ready<Result<Response>>;

    fn respond(self) -> Self::Future {
        future::ready(json(self.0).map_err(Into::into))
    }
}
