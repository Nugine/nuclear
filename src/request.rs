use std::ops;

use crate::internal_prelude::*;

#[derive(Debug)]
pub struct Request {
    inner: Box<HyperRequest>,
}

impl Request {
    pub(super) fn from_hyper(req: HyperRequest) -> Self {
        Self {
            inner: Box::new(req),
        }
    }
}

impl ops::Deref for Request {
    type Target = HyperRequest;

    fn deref(&self) -> &Self::Target {
        self.inner.as_ref()
    }
}

impl ops::DerefMut for Request {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.inner.as_mut()
    }
}
