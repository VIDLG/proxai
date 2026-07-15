use crate::protocol::{OptionalNullable, deserialize_present};
use serde::{Deserialize, Serialize};

// ============================================================
// Input / Context Item Shapes
// ============================================================

/// OpenAPI schema: `#/components/schemas/CompactionSummaryItemParam`
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompactionSummaryItemParam {
    #[serde(default, skip_serializing_if = "OptionalNullable::is_missing")]
    pub id: OptionalNullable<String>,
    pub encrypted_content: String,
}

// ============================================================
// Output / Resource Shapes
// ============================================================

/// OpenAPI schema: `#/components/schemas/CompactionBody`
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompactionBody {
    pub id: String,
    pub encrypted_content: String,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_present"
    )]
    pub created_by: Option<String>,
}

// ============================================================
// Request Parameters
// ============================================================

/// OpenAPI schema: `#/components/schemas/ContextManagementParam`
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct ContextManagementParam {
    pub r#type: String,
    #[serde(default, skip_serializing_if = "OptionalNullable::is_missing")]
    pub compact_threshold: OptionalNullable<u32>,
}
