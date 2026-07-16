mod normalize;

pub(crate) use normalize::{normalize_response_payload, normalize_stream_event_payload};

#[cfg(test)]
mod tests;
