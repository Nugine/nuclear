use crate::body::BodyError;
use crate::internal_prelude::*;

use std::ops;

use bytes::{BufMut, Bytes, BytesMut};
use futures::stream::StreamExt;

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

    pub async fn body_bytes(&mut self, length_limit: usize) -> Result<Bytes> {
        let body = self.body_mut();

        let mut bufs: Vec<Bytes> = Vec::new();
        let mut total: usize = 0;

        while let Some(bytes) = body.next().await.transpose()? {
            total = match total.checked_add(bytes.len()) {
                Some(t) if t <= length_limit => t,
                _ => return Err(BodyError::LengthLimitExceeded.into()),
            };

            bufs.push(bytes);
        }

        let mut buf: BytesMut = BytesMut::with_capacity(total);
        for bytes in bufs {
            buf.put(bytes);
        }

        Ok(buf.freeze())
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
