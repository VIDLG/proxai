use serde::Serialize;

use async_openai::types::responses as openai;
use structural_convert::StructuralConvert;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, StructuralConvert)]
#[convert(from(openai::ErrorObject))]
pub struct ErrorObject {
    pub code: String,
    pub message: String,
}
