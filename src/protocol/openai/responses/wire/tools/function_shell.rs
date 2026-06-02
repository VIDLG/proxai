use async_openai::types::responses as openai;
use serde::{Deserialize, Serialize};
use structural_convert::StructuralConvert;
use strum::Display;

// ============================================================
// Tool Definition Supporting Types
// ============================================================

#[derive(Debug, Clone, PartialEq, Eq, StructuralConvert, Serialize, Deserialize)]
#[convert(from(openai::ContainerNetworkPolicyDomainSecretParam))]
pub struct ContainerNetworkPolicyDomainSecretParam {
    pub domain: String,
    pub name: String,
    pub value: String,
}

#[derive(Debug, Clone, PartialEq, Eq, StructuralConvert, Default, Serialize, Deserialize)]
#[convert(from(openai::ContainerNetworkPolicyAllowlistDetails))]
pub struct ContainerNetworkPolicyAllowlistDetails {
    pub allowed_domains: Vec<String>,
    pub domain_secrets: Option<Vec<ContainerNetworkPolicyDomainSecretParam>>,
}

#[derive(Debug, Clone, PartialEq, Eq, StructuralConvert, Serialize, Deserialize)]
#[convert(from(openai::ContainerNetworkPolicy))]
pub enum ContainerNetworkPolicy {
    Disabled,
    Allowlist(ContainerNetworkPolicyAllowlistDetails),
}

#[derive(Debug, Clone, PartialEq, Eq, StructuralConvert, Default, Serialize, Deserialize)]
#[convert(from(openai::SkillReferenceParam))]
pub struct SkillReferenceParam {
    pub skill_id: String,
    pub version: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, StructuralConvert, Serialize, Deserialize)]
#[convert(from(openai::InlineSkillSourceParam))]
pub struct InlineSkillSourceParam {
    pub media_type: String,
    pub data: String,
}

#[derive(Debug, Clone, PartialEq, Eq, StructuralConvert, Serialize, Deserialize)]
#[convert(from(openai::InlineSkillParam))]
pub struct InlineSkillParam {
    pub name: String,
    pub description: String,
    pub source: InlineSkillSourceParam,
}

#[derive(Debug, Clone, PartialEq, Eq, StructuralConvert, Serialize, Deserialize)]
#[convert(from(openai::SkillParam))]
pub enum SkillParam {
    SkillReference(SkillReferenceParam),
    Inline(InlineSkillParam),
}

#[derive(Debug, Clone, PartialEq, Eq, StructuralConvert, Default, Serialize, Deserialize)]
#[convert(from(openai::ContainerAutoParam))]
pub struct ContainerAutoParam {
    pub file_ids: Option<Vec<String>>,
    pub network_policy: Option<ContainerNetworkPolicy>,
    pub skills: Option<Vec<SkillParam>>,
}

#[derive(Debug, Clone, PartialEq, Eq, StructuralConvert, Serialize, Deserialize)]
#[convert(from(openai::LocalSkillParam))]
pub struct LocalSkillParam {
    pub name: String,
    pub description: String,
    pub path: String,
}

#[derive(Debug, Clone, PartialEq, Eq, StructuralConvert, Default, Serialize, Deserialize)]
#[convert(from(openai::LocalEnvironmentParam))]
pub struct LocalEnvironmentParam {
    pub skills: Option<Vec<LocalSkillParam>>,
}

#[derive(Debug, Clone, PartialEq, Eq, StructuralConvert, Serialize, Deserialize)]
#[convert(from(openai::ContainerReferenceParam))]
pub struct ContainerReferenceParam {
    pub container_id: String,
}

// ============================================================
// Tool Definition
// ============================================================

#[derive(Debug, Clone, PartialEq, Eq, StructuralConvert, Serialize, Deserialize)]
#[convert(from(openai::FunctionShellEnvironment))]
pub enum FunctionShellEnvironment {
    ContainerAuto(ContainerAutoParam),
    Local(LocalEnvironmentParam),
    ContainerReference(ContainerReferenceParam),
}

#[derive(Debug, Clone, PartialEq, Eq, StructuralConvert, Serialize, Deserialize)]
#[convert(from(openai::FunctionShellToolParam))]
pub struct FunctionShellToolParam {
    pub environment: Option<FunctionShellEnvironment>,
}

// ============================================================
// Input / Context Item Supporting Types
// ============================================================

#[derive(Debug, Clone, PartialEq, Eq, StructuralConvert, Serialize, Deserialize)]
#[convert(from(openai::FunctionShellActionParam))]
pub struct FunctionShellActionParam {
    pub commands: Vec<String>,
    pub timeout_ms: Option<u64>,
    pub max_output_length: Option<u64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, StructuralConvert, Display, Serialize, Deserialize)]
#[convert(from(openai::FunctionShellCallItemStatus))]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum FunctionShellCallItemStatus {
    InProgress,
    Completed,
    Incomplete,
}

#[derive(Debug, Clone, PartialEq, Eq, StructuralConvert, Serialize, Deserialize)]
#[convert(from(openai::FunctionShellCallItemEnvironment))]
pub enum FunctionShellCallItemEnvironment {
    Local(LocalEnvironmentParam),
    ContainerReference(ContainerReferenceParam),
}

#[derive(Debug, Clone, PartialEq, Eq, StructuralConvert, Serialize, Deserialize)]
#[convert(from(openai::FunctionShellCallOutputExitOutcomeParam))]
pub struct FunctionShellCallOutputExitOutcomeParam {
    pub exit_code: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, StructuralConvert, Serialize, Deserialize)]
#[convert(from(openai::FunctionShellCallOutputOutcomeParam))]
pub enum FunctionShellCallOutputOutcomeParam {
    Timeout,
    Exit(FunctionShellCallOutputExitOutcomeParam),
}

#[derive(Debug, Clone, PartialEq, Eq, StructuralConvert, Serialize, Deserialize)]
#[convert(from(openai::FunctionShellCallOutputContentParam))]
pub struct FunctionShellCallOutputContentParam {
    pub stdout: String,
    pub stderr: String,
    pub outcome: FunctionShellCallOutputOutcomeParam,
}

// ============================================================
// Input / Context Item Shapes
// ============================================================

#[derive(Debug, Clone, PartialEq, Eq, StructuralConvert, Serialize, Deserialize)]
#[convert(from(openai::FunctionShellCallItemParam))]
pub struct FunctionShellCallItemParam {
    pub id: Option<String>,
    pub call_id: String,
    pub action: FunctionShellActionParam,
    pub status: Option<FunctionShellCallItemStatus>,
    pub environment: Option<FunctionShellCallItemEnvironment>,
}

#[derive(Debug, Clone, PartialEq, Eq, StructuralConvert, Serialize, Deserialize)]
#[convert(from(openai::FunctionShellCallOutputItemParam))]
pub struct FunctionShellCallOutputItemParam {
    pub id: Option<String>,
    pub call_id: String,
    pub output: Vec<FunctionShellCallOutputContentParam>,
    pub max_output_length: Option<u64>,
}

// ============================================================
// Function Shell Output Supporting Types
// ============================================================

#[derive(Debug, Clone, PartialEq, Eq, StructuralConvert, Serialize, Deserialize)]
#[convert(from(openai::FunctionShellAction))]
pub struct FunctionShellAction {
    pub commands: Vec<String>,
    pub timeout_ms: Option<u64>,
    pub max_output_length: Option<u64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, StructuralConvert, Display, Serialize, Deserialize)]
#[convert(from(openai::LocalShellCallStatus))]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum LocalShellCallStatus {
    InProgress,
    Completed,
    Incomplete,
}

#[derive(Debug, Clone, PartialEq, Eq, StructuralConvert, Serialize, Deserialize)]
#[convert(from(openai::ContainerReferenceResource))]
pub struct ContainerReferenceResource {
    pub container_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, StructuralConvert, Serialize, Deserialize)]
#[convert(from(openai::FunctionShellCallEnvironment))]
pub enum FunctionShellCallEnvironment {
    Local,
    ContainerReference(ContainerReferenceResource),
}

#[derive(Debug, Clone, PartialEq, Eq, StructuralConvert, Serialize, Deserialize)]
#[convert(from(openai::FunctionShellCallOutputExitOutcome))]
pub struct FunctionShellCallOutputExitOutcome {
    pub exit_code: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, StructuralConvert, Serialize, Deserialize)]
#[convert(from(openai::FunctionShellCallOutputOutcome))]
pub enum FunctionShellCallOutputOutcome {
    Timeout,
    Exit(FunctionShellCallOutputExitOutcome),
}

#[derive(Debug, Clone, PartialEq, Eq, StructuralConvert, Serialize, Deserialize)]
#[convert(from(openai::FunctionShellCallOutputContent))]
pub struct FunctionShellCallOutputContent {
    pub stdout: String,
    pub stderr: String,
    pub outcome: FunctionShellCallOutputOutcome,
    pub created_by: Option<String>,
}

// ============================================================
// Function Shell Output Shapes
// ============================================================

#[derive(Debug, Clone, PartialEq, Eq, StructuralConvert, Serialize, Deserialize)]
#[convert(from(openai::FunctionShellCall))]
pub struct FunctionShellCall {
    pub id: String,
    pub call_id: String,
    pub action: FunctionShellAction,
    pub status: LocalShellCallStatus,
    pub environment: Option<FunctionShellCallEnvironment>,
    pub created_by: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, StructuralConvert, Serialize, Deserialize)]
#[convert(from(openai::FunctionShellCallOutput))]
pub struct FunctionShellCallOutput {
    pub id: String,
    pub call_id: String,
    pub output: Vec<FunctionShellCallOutputContent>,
    pub max_output_length: Option<u64>,
    pub created_by: Option<String>,
}
