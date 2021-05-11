use crate::http::{self, HeaderValue, Mime, StatusCode};
use crate::internal_prelude::*;

use std::convert::TryFrom;
use std::ops;

use futures::future::{self, Either, Ready};
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

pub trait Responder: Send + Sync {
    type Future: Future<Output = Result<Response>> + Send;

    fn respond(self) -> Self::Future;
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

fn json<T>(status: StatusCode, value: T) -> Result<Response, serde_json::Error>
where
    T: Serialize,
{
    let bytes_vec = serde_json::to_vec(&value)?;
    let mut res = Response::new(status, Body::from(bytes_vec));
    res.set_static_mime(&mime::APPLICATION_JSON);
    Ok(res)
}

pub struct Json<T> {
    status: StatusCode,
    value: T,
}

impl<T> Json<T>
where
    T: Serialize,
{
    pub fn new(status: StatusCode, value: T) -> Self {
        Self { status, value }
    }

    pub fn ok(value: T) -> Self {
        Self {
            status: StatusCode::OK,
            value,
        }
    }
}

impl<T> From<Json<T>> for Result<Response>
where
    T: Serialize,
{
    fn from(this: Json<T>) -> Self {
        json(this.status, this.value).map_err(Into::into)
    }
}

impl<T> TryFrom<Json<T>> for Response
where
    T: Serialize,
{
    type Error = serde_json::Error;

    fn try_from(j: Json<T>) -> Result<Self, Self::Error> {
        json(j.status, j.value)
    }
}

impl<T> Responder for Json<T>
where
    T: Serialize + Send + Sync,
{
    type Future = Ready<Result<Response>>;

    fn respond(self) -> Self::Future {
        future::ready(json(self.status, self.value).map_err(Into::into))
    }
}

impl From<StatusCode> for Response {
    fn from(status: StatusCode) -> Self {
        Response::new(status, Body::empty())
    }
}
