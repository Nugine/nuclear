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
#[error("StatusError: {}", .status.as_str())]
pub struct StatusError {
    pub status: StatusCode,
}

impl StatusError {
    pub const NOT_FOUND: Self = Self {
        status: StatusCode::NOT_FOUND,
    };

    pub fn new(status: StatusCode) -> Self {
        Self { status }
    }
}

impl From<StatusError> for Response {
    fn from(e: StatusError) -> Self {
        let body = match e.status.canonical_reason() {
            Some(s) => s.into(),
            None => Body::empty(),
        };
        Response::new(e.status, body)
    }
}

impl Responder for StatusError {
    type Future = Ready<Result<Response>>;

    fn respond(self) -> Self::Future {
        future::ready(Ok(self.into()))
    }
}
