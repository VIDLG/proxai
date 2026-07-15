use serde::{Deserialize, Serialize};

/// OpenAPI schema: `#/components/schemas/FileCitationBody`
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct FileCitationBody {
    pub file_id: String,
    pub index: u32,
    pub filename: String,
}

/// OpenAPI schema: `#/components/schemas/UrlCitationBody`
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct UrlCitationBody {
    pub url: String,
    pub start_index: u32,

    pub end_index: u32,
    pub title: String,
}

/// OpenAPI schema: `#/components/schemas/ContainerFileCitationBody`
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct ContainerFileCitationBody {
    pub container_id: String,
    pub file_id: String,
    pub start_index: u32,
    pub end_index: u32,
    pub filename: String,
}

/// OpenAPI schema: `#/components/schemas/FilePath`
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct FilePath {
    pub file_id: String,
    pub index: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Annotation {
    FileCitation(FileCitationBody),
    UrlCitation(UrlCitationBody),
    ContainerFileCitation(ContainerFileCitationBody),
    FilePath(FilePath),
}
