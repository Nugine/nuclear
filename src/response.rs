use crate::http::{self, HeaderValue, Mime, StatusCode};
use crate::internal_prelude::*;

use std::convert::TryFrom;
use std::ops;

// use futures::future::{self, Either, Ready};
// use pin_project::pin_project;
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

    pub fn text(s: impl Into<String>) -> Self {
        let mut res = Self::new_ok(Body::from(s.into()));
        res.set_static_mime(&mime::TEXT_PLAIN_UTF_8);
        res
    }

    pub fn json<T>(value: T) -> Result<Response, serde_json::Error>
    where
        T: Serialize,
    {
        let bytes_vec = serde_json::to_vec(&value)?;
        let mut res = Response::new_ok(Body::from(bytes_vec));
        res.set_static_mime(&mime::APPLICATION_JSON);
        Ok(res)
    }

    pub fn with_status(mut self, status: StatusCode) -> Self {
        *self.status_mut() = status;
        self
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

impl From<()> for Response {
    fn from(_: ()) -> Self {
        Response::new_ok(Body::empty())
    }
}

impl From<&'_ str> for Response {
    fn from(s: &'_ str) -> Self {
        Response::text(s)
    }
}

impl From<String> for Response {
    fn from(s: String) -> Self {
        Response::text(s)
    }
}

impl TryFrom<Result<Response>> for Response {
    type Error = Error;

    fn try_from(ret: Result<Response>) -> Result<Self, Self::Error> {
        ret
    }
}

pub struct Json<T>(pub T);

impl<T> TryFrom<Json<T>> for Response
where
    T: Serialize,
{
    type Error = serde_json::Error;

    fn try_from(Json(value): Json<T>) -> Result<Self, Self::Error> {
        Response::json(value)
    }
}
