use serde::{Deserialize, Serialize};
use strum::Display;

// ============================================================
// Tool Definition Supporting Types
// ============================================================

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContainerNetworkPolicyDomainSecretParam {
    pub domain: String,
    pub name: String,
    pub value: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct ContainerNetworkPolicyAllowlistDetails {
    pub allowed_domains: Vec<String>,
    pub domain_secrets: Option<Vec<ContainerNetworkPolicyDomainSecretParam>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ContainerNetworkPolicy {
    Disabled,
    Allowlist(ContainerNetworkPolicyAllowlistDetails),
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct SkillReferenceParam {
    pub skill_id: String,
    pub version: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InlineSkillSourceParam {
    pub media_type: String,
    pub data: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InlineSkillParam {
    pub name: String,
    pub description: String,
    pub source: InlineSkillSourceParam,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SkillParam {
    SkillReference(SkillReferenceParam),
    Inline(InlineSkillParam),
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct ContainerAutoParam {
    pub file_ids: Option<Vec<String>>,
    pub network_policy: Option<ContainerNetworkPolicy>,
    pub skills: Option<Vec<SkillParam>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LocalSkillParam {
    pub name: String,
    pub description: String,
    pub path: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct LocalEnvironmentParam {
    pub skills: Option<Vec<LocalSkillParam>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContainerReferenceParam {
    pub container_id: String,
}

// ============================================================
// Tool Definition
// ============================================================

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FunctionShellEnvironment {
    ContainerAuto(ContainerAutoParam),
    Local(LocalEnvironmentParam),
    ContainerReference(ContainerReferenceParam),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FunctionShellToolParam {
    pub environment: Option<FunctionShellEnvironment>,
}

// ============================================================
// Input / Context Item Supporting Types
// ============================================================

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FunctionShellActionParam {
    pub commands: Vec<String>,
    pub timeout_ms: Option<u64>,
    pub max_output_length: Option<u64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Display, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum FunctionShellCallItemStatus {
    InProgress,
    Completed,
    Incomplete,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FunctionShellCallItemEnvironment {
    Local(LocalEnvironmentParam),
    ContainerReference(ContainerReferenceParam),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FunctionShellCallOutputExitOutcomeParam {
    pub exit_code: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FunctionShellCallOutputOutcomeParam {
    Timeout,
    Exit(FunctionShellCallOutputExitOutcomeParam),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FunctionShellCallOutputContentParam {
    pub stdout: String,
    pub stderr: String,
    pub outcome: FunctionShellCallOutputOutcomeParam,
}

// ============================================================
// Input / Context Item Shapes
// ============================================================

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FunctionShellCallItemParam {
    pub id: Option<String>,
    pub call_id: String,
    pub action: FunctionShellActionParam,
    pub status: Option<FunctionShellCallItemStatus>,
    pub environment: Option<FunctionShellCallItemEnvironment>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FunctionShellCallOutputItemParam {
    pub id: Option<String>,
    pub call_id: String,
    pub output: Vec<FunctionShellCallOutputContentParam>,
    pub max_output_length: Option<u64>,
}

// ============================================================
// Function Shell Output Supporting Types
// ============================================================

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FunctionShellAction {
    pub commands: Vec<String>,
    pub timeout_ms: Option<u64>,
    pub max_output_length: Option<u64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Display, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum LocalShellCallStatus {
    InProgress,
    Completed,
    Incomplete,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Display, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum FunctionShellCallStatus {
    InProgress,
    Completed,
    Incomplete,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Display, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum FunctionShellCallOutputStatusEnum {
    InProgress,
    #[default]
    Completed,
    Incomplete,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContainerReferenceResource {
    pub container_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FunctionShellCallEnvironment {
    Local,
    ContainerReference(ContainerReferenceResource),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FunctionShellCallOutputExitOutcome {
    pub exit_code: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FunctionShellCallOutputOutcome {
    Timeout,
    Exit(FunctionShellCallOutputExitOutcome),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FunctionShellCallOutputContent {
    pub stdout: String,
    pub stderr: String,
    pub outcome: FunctionShellCallOutputOutcome,
    pub created_by: Option<String>,
}

// ============================================================
// Function Shell Output Shapes
// ============================================================

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FunctionShellCall {
    pub id: String,
    pub call_id: String,
    pub action: FunctionShellAction,
    pub status: FunctionShellCallStatus,
    pub environment: Option<FunctionShellCallEnvironment>,
    pub created_by: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FunctionShellCallOutput {
    pub id: String,
    pub call_id: String,
    #[serde(default)]
    pub status: FunctionShellCallOutputStatusEnum,
    pub output: Vec<FunctionShellCallOutputContent>,
    pub max_output_length: Option<u64>,
    pub created_by: Option<String>,
}
