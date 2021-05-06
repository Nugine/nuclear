use crate::http;
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

    pub(crate) fn as_ref_hyper(&self) -> &HyperRequest {
        &*self.inner
    }

    pub(crate) fn as_mut_hyper(&mut self) -> &mut HyperRequest {
        &mut *self.inner
    }

    pub fn method(&self) -> &http::Method {
        self.as_ref_hyper().method()
    }

    pub fn uri(&self) -> &http::Uri {
        self.as_ref_hyper().uri()
    }
}
