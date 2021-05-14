#![deny(clippy::all)]
#![deny(unsafe_code)]

pub mod body;
pub mod error;
pub mod functional;
pub mod handler;
pub mod http;
pub mod middleware;
pub mod request;
pub mod response;
pub mod router;
pub mod server;
pub mod state;

pub(crate) mod internal_prelude {
    pub use crate::error::{Error, Result};
    pub use crate::handler::Handler;
    pub use crate::middleware::Middleware;
    pub use crate::request::Request;
    pub use crate::response::Response;

    pub use std::future::Future;
    pub use std::pin::Pin;
    pub use std::task::{Context, Poll};

    pub use futures::future::BoxFuture;

    pub use hyper::Body;
    pub type HyperRequest = hyper::Request<Body>;
    pub type HyperResponse = hyper::Response<Body>;
    pub type HyperServer<I, S> = hyper::server::Server<I, S>;
}

pub use crate::error::{Error, Result};

pub mod prelude {
    pub use crate::error::{Error, Result};
    pub use crate::request::Request;
    pub use crate::response::Response;

    pub use crate::handler::*;
    pub use crate::middleware::*;

    pub use futures::future::BoxFuture;
}
