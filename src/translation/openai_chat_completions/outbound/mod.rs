pub(crate) mod request;
pub(crate) mod response;
pub(crate) mod streaming;

pub(crate) const CHAT_COMPLETION_OBJECT: &str = "chat.completion";
pub(crate) const CHAT_COMPLETION_CHUNK_OBJECT: &str = "chat.completion.chunk";

pub(crate) use request::*;
pub(crate) use response::*;
pub(crate) use streaming::*;
