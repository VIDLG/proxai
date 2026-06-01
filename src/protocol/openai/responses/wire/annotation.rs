use async_openai::types::responses as openai;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use structural_convert::StructuralConvert;

// Keep these local citation/path bodies typed even though the upstream SDK keeps
// their fields private. We cannot use `StructuralConvert` directly for them, so
// we deserialize from their serialized JSON shape instead.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct FileCitationBody {
    pub file_id: String,
    pub filename: String,
    pub index: u32,
}

impl From<openai::FileCitationBody> for FileCitationBody {
    fn from(value: openai::FileCitationBody) -> Self {
        serde_json::from_value(serde_json::to_value(value).unwrap_or(Value::Null))
            .expect("FileCitationBody should match local protocol shape")
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct UrlCitationBody {
    pub end_index: u32,
    pub start_index: u32,
    pub title: String,
    pub url: String,
}

impl From<openai::UrlCitationBody> for UrlCitationBody {
    fn from(value: openai::UrlCitationBody) -> Self {
        serde_json::from_value(serde_json::to_value(value).unwrap_or(Value::Null))
            .expect("UrlCitationBody should match local protocol shape")
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct ContainerFileCitationBody {
    pub container_id: String,
    pub end_index: u32,
    pub file_id: String,
    pub filename: String,
    pub start_index: u32,
}

impl From<openai::ContainerFileCitationBody> for ContainerFileCitationBody {
    fn from(value: openai::ContainerFileCitationBody) -> Self {
        serde_json::from_value(serde_json::to_value(value).unwrap_or(Value::Null))
            .expect("ContainerFileCitationBody should match local protocol shape")
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct FilePath {
    pub file_id: String,
    pub index: u32,
}

impl From<openai::FilePath> for FilePath {
    fn from(value: openai::FilePath) -> Self {
        serde_json::from_value(serde_json::to_value(value).unwrap_or(Value::Null))
            .expect("FilePath should match local protocol shape")
    }
}

#[derive(Debug, Clone, PartialEq, Eq, StructuralConvert, Serialize, Deserialize)]
#[convert(from(openai::Annotation))]
pub enum Annotation {
    FileCitation(FileCitationBody),
    UrlCitation(UrlCitationBody),
    ContainerFileCitation(ContainerFileCitationBody),
    FilePath(FilePath),
}
