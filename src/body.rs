use crate::http::{self, Mime};
use crate::internal_prelude::*;

use std::mem;

use async_trait::async_trait;
use bytes::{BufMut, Bytes, BytesMut};
use futures::stream::StreamExt;
use serde::Deserialize;

#[derive(Debug, thiserror::Error)]
pub enum BodyError {
    #[error("LengthLimitExceeded")]
    LengthLimitExceeded,
    #[error("InvalidFormat: {}", .source)]
    InvalidFormat { source: Error },
    #[error("ContentTypeMismatch")]
    ContentTypeMismatch,
}

async fn to_bytes(mut body: Body, length_limit: usize) -> Result<Bytes> {
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

fn parse_mime(req: &Request) -> Option<Mime> {
    req.headers()
        .get(http::header::CONTENT_TYPE)?
        .to_str()
        .ok()?
        .parse()
        .ok()
}

fn take_body(hreq: &mut HyperRequest) -> Body {
    mem::take(hreq.body_mut())
}

pub struct FullBody(Bytes);

#[derive(Debug, Clone)]
pub struct JsonParser {
    length_limit: usize,
}

impl Default for JsonParser {
    fn default() -> Self {
        Self {
            length_limit: Self::DEFAULT_LENGTH_LIMIT,
        }
    }
}

impl JsonParser {
    const DEFAULT_LENGTH_LIMIT: usize = 32 * 1024;

    pub fn length_limit(&mut self, limit: usize) {
        self.length_limit = limit;
    }

    pub async fn parse<'r, T>(&self, req: &'r mut Request) -> Result<T>
    where
        T: Deserialize<'r>,
    {
        let ct_check = parse_mime(&req)
            .map(|mime| mime.type_() == mime::APPLICATION && mime.subtype() == mime::JSON)
            .unwrap_or(false);

        if !ct_check {
            return Err(BodyError::ContentTypeMismatch.into());
        }

        {
            let hreq = &mut **req;
            if hreq.extensions().get::<FullBody>().is_none() {
                let full_body = FullBody(to_bytes(take_body(hreq), self.length_limit).await?);
                hreq.extensions_mut().insert(full_body);
            }
        }

        let full_body = req.extensions().get::<FullBody>().unwrap();

        match serde_json::from_slice(&*full_body.0) {
            Ok(value) => Ok(value),
            Err(e) => Err(BodyError::InvalidFormat { source: e.into() }.into()),
        }
    }
}

#[async_trait]
pub trait JsonExt {
    async fn parse_json<'r, T: Deserialize<'r>>(&'r mut self, parser: &JsonParser) -> Result<T>;
    async fn json<'r, T: Deserialize<'r>>(&'r mut self) -> Result<T>;
}

#[async_trait]
impl JsonExt for Request {
    async fn parse_json<'r, T: Deserialize<'r>>(&'r mut self, parser: &JsonParser) -> Result<T> {
        parser.parse(self).await
    }

    async fn json<'r, T: Deserialize<'r>>(&'r mut self) -> Result<T> {
        let parser = match self.extensions().get::<JsonParser>() {
            Some(p) => p.clone(),
            None => JsonParser::default(),
        };
        self.parse_json(&parser).await
    }
}
