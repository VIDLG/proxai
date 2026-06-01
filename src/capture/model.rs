use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use axum::http::{HeaderMap, Method, Uri};
use serde_json::Value;

use crate::upstream::ContentType;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CaptureRecord {
    pub request_id: u64,
    pub prefix: String,
    pub inbound_request: Option<InboundRequestArtifacts>,
    pub forwarded_request: Option<ForwardedRequestArtifacts>,
    pub upstream_response: Option<UpstreamResponseArtifacts>,
    pub outbound_response: Option<OutboundResponseArtifacts>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InboundRequestArtifacts {
    pub metadata_path: PathBuf,
    pub body_path: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ForwardedRequestArtifacts {
    pub metadata_path: PathBuf,
    pub body_path: PathBuf,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct UpstreamResponseArtifacts {
    pub headers_path: Option<PathBuf>,
    pub body_path: Option<PathBuf>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct OutboundResponseArtifacts {
    pub headers_path: Option<PathBuf>,
    pub body_path: Option<PathBuf>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaptureShowTarget {
    InboundRequest,
    ForwardedRequest,
    UpstreamResponse,
    OutboundResponse,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CaptureQuery {
    Show(Option<CaptureShowTarget>),
    List(Option<usize>),
}

#[derive(Debug, Clone)]
pub(crate) struct CapturePrefix(String);

#[derive(Debug, Clone)]
pub(crate) struct CaptureDestination {
    dir: PathBuf,
    prefix: CapturePrefix,
}

impl CaptureDestination {
    pub(crate) fn new(dir: PathBuf, prefix: CapturePrefix) -> Self {
        Self { dir, prefix }
    }

    pub(crate) fn prefix_string(&self) -> String {
        self.prefix.0.clone()
    }

    pub(crate) fn inbound_request_metadata_path(&self) -> PathBuf {
        self.dir
            .join(format!("{}-inbound-request.metadata.json", self.prefix.0))
    }

    pub(crate) fn inbound_request_body_path(&self) -> PathBuf {
        self.dir
            .join(format!("{}-inbound-request.body.json", self.prefix.0))
    }

    pub(crate) fn forwarded_request_metadata_path(&self) -> PathBuf {
        self.dir
            .join(format!("{}-forwarded-request.metadata.json", self.prefix.0))
    }

    pub(crate) fn forwarded_request_body_path(&self) -> PathBuf {
        self.dir
            .join(format!("{}-forwarded-request.body.json", self.prefix.0))
    }

    pub(crate) fn upstream_response_headers_path(&self) -> PathBuf {
        self.dir
            .join(format!("{}-upstream-response.headers.json", self.prefix.0))
    }

    pub(crate) fn upstream_response_body_path(
        &self,
        content_type: Option<&ContentType>,
    ) -> PathBuf {
        let suffix = match content_type {
            Some(ContentType::EventStream) => "upstream-response.body.sse",
            _ => "upstream-response.body.bin",
        };
        self.dir.join(format!("{}-{suffix}", self.prefix.0))
    }

    pub(crate) fn outbound_response_headers_path(&self) -> PathBuf {
        self.dir
            .join(format!("{}-outbound-response.headers.json", self.prefix.0))
    }
}

impl CapturePrefix {
    pub(crate) fn new(request_id: u64) -> Self {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        Self(format!("{timestamp}-{request_id:06}"))
    }
}

pub(crate) struct InboundRequestCapture<'a> {
    pub(crate) request_id: u64,
    pub(crate) method: &'a Method,
    pub(crate) uri: &'a Uri,
    pub(crate) headers: &'a HeaderMap,
    pub(crate) body: &'a [u8],
}

pub(crate) struct ForwardedRequestCapture<'a> {
    pub(crate) request_id: u64,
    pub(crate) method: &'a Method,
    pub(crate) url: &'a str,
    pub(crate) headers: &'a HeaderMap,
    pub(crate) body: &'a [u8],
    pub(crate) normalized_payload: Option<&'a Value>,
}
