use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct FileCitationBody {
    pub file_id: String,
    pub filename: String,
    pub index: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct UrlCitationBody {
    pub end_index: u32,
    pub start_index: u32,
    pub title: String,
    pub url: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct ContainerFileCitationBody {
    pub container_id: String,
    pub end_index: u32,
    pub file_id: String,
    pub filename: String,
    pub start_index: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct FilePath {
    pub file_id: String,
    pub index: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Annotation {
    FileCitation(FileCitationBody),
    UrlCitation(UrlCitationBody),
    ContainerFileCitation(ContainerFileCitationBody),
    FilePath(FilePath),
}
