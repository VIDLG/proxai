use axum::body::Bytes;
use futures_util::{Stream, StreamExt};
use std::error::Error;
use std::pin::Pin;

pub(crate) type ByteStreamError = Box<dyn Error + Send + Sync>;
pub(crate) type ByteStream = Pin<Box<dyn Stream<Item = Result<Bytes, ByteStreamError>> + Send>>;

pub(crate) fn into_byte_stream<S, E>(stream: S) -> ByteStream
where
    S: Stream<Item = Result<Bytes, E>> + Send + 'static,
    E: Error + Send + Sync + 'static,
{
    Box::pin(stream.map(|chunk| chunk.map_err(Into::into)))
}
