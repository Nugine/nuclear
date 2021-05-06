pub use anyhow::{Error, Result};

use crate::http::{Body, StatusCode};
use crate::response::{Responder, Response};

use futures::future::{self, Ready};

pub trait CatchExt {
    type Value;
    type Error;
    fn catch<E>(self) -> Result<Result<Self::Value, E>, Self::Error>
    where
        E: std::error::Error + Send + Sync + 'static;
}

impl<T> CatchExt for Result<T> {
    type Value = T;
    type Error = Error;

    fn catch<E>(self) -> Result<Result<Self::Value, E>, Self::Error>
    where
        E: std::error::Error + Send + Sync + 'static,
    {
        match self {
            Ok(value) => Ok(Ok(value)),
            Err(err) => match err.downcast::<E>() {
                Ok(e) => Ok(Err(e)),
                Err(err) => Err(err),
            },
        }
    }
}

#[derive(Debug, thiserror::Error)]
#[error("Not Found")]
pub struct NotFound;

impl From<NotFound> for Response {
    fn from(_: NotFound) -> Self {
        let status = StatusCode::NOT_FOUND;
        let body = Body::from("Not Found");
        Response::new(status, body)
    }
}

impl Responder for NotFound {
    type Future = Ready<Result<Response>>;

    fn respond(self) -> Self::Future {
        future::ready(Ok(self.into()))
    }
}
