use serde::{Deserialize, Serialize};

// ============================================================
// Input / Context Item Shapes
// ============================================================

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompactionSummaryItemParam {
    pub id: Option<String>,
    pub encrypted_content: String,
}

// ============================================================
// Output / Resource Shapes
// ============================================================

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompactionBody {
    pub id: String,
    pub encrypted_content: String,
    pub created_by: Option<String>,
}

// ============================================================
// Request Parameters
// ============================================================

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct ContextManagementParam {
    #[serde(rename = "type")]
    pub type_: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub compact_threshold: Option<u32>,
}
