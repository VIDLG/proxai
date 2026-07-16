use crate::protocol::RequiredNullable;
use crate::protocol::{OptionalNullable, deserialize_present};
use serde::{Deserialize, Serialize};
use strum::Display;

use super::{CallableToolAllowedCaller, ToolCallCaller, ToolCallCallerParam};

// ============================================================
// Tool Definition Supporting Types
// ============================================================

/// OpenAPI schema: `#/components/schemas/ContainerNetworkPolicyDomainSecretParam`
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContainerNetworkPolicyDomainSecretParam {
    pub domain: String,
    pub name: String,
    pub value: String,
}

/// OpenAPI schema: `#/components/schemas/ContainerNetworkPolicyAllowlistParam`
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct ContainerNetworkPolicyAllowlistParam {
    pub allowed_domains: Vec<String>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_present"
    )]
    pub domain_secrets: Option<Vec<ContainerNetworkPolicyDomainSecretParam>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ContainerNetworkPolicy {
    Disabled,
    Allowlist(ContainerNetworkPolicyAllowlistParam),
}

/// OpenAPI schema: `#/components/schemas/SkillReferenceParam`
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct SkillReferenceParam {
    pub skill_id: String,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_present"
    )]
    pub version: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Display, Serialize, Deserialize)]
#[strum(serialize_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum InlineSkillSourceType {
    Base64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Display, Serialize, Deserialize)]
pub enum InlineSkillSourceMediaType {
    #[serde(rename = "application/zip")]
    #[strum(to_string = "application/zip")]
    ApplicationZip,
}

/// OpenAPI schema: `#/components/schemas/InlineSkillSourceParam`
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InlineSkillSourceParam {
    pub r#type: InlineSkillSourceType,
    pub media_type: InlineSkillSourceMediaType,
    pub data: String,
}

/// OpenAPI schema: `#/components/schemas/InlineSkillParam`
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InlineSkillParam {
    pub name: String,
    pub description: String,
    pub source: InlineSkillSourceParam,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SkillParam {
    SkillReference(SkillReferenceParam),
    Inline(InlineSkillParam),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Display, Serialize, Deserialize)]
pub enum ContainerMemoryLimit {
    #[serde(rename = "1g")]
    #[strum(to_string = "1g")]
    OneG,
    #[serde(rename = "4g")]
    #[strum(to_string = "4g")]
    FourG,
    #[serde(rename = "16g")]
    #[strum(to_string = "16g")]
    SixteenG,
    #[serde(rename = "64g")]
    #[strum(to_string = "64g")]
    SixtyFourG,
}

/// OpenAPI schema: `#/components/schemas/ContainerAutoParam`
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct ContainerAutoParam {
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_present"
    )]
    pub file_ids: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "OptionalNullable::is_missing")]
    pub memory_limit: OptionalNullable<ContainerMemoryLimit>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_present"
    )]
    pub network_policy: Option<ContainerNetworkPolicy>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_present"
    )]
    pub skills: Option<Vec<SkillParam>>,
}

/// OpenAPI schema: `#/components/schemas/LocalSkillParam`
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LocalSkillParam {
    pub name: String,
    pub description: String,
    pub path: String,
}

/// OpenAPI schema: `#/components/schemas/LocalEnvironmentParam`
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct LocalEnvironmentParam {
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_present"
    )]
    pub skills: Option<Vec<LocalSkillParam>>,
}

/// OpenAPI schema: `#/components/schemas/ContainerReferenceParam`
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContainerReferenceParam {
    pub container_id: String,
}

// ============================================================
// Tool Definition
// ============================================================

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum FunctionShellEnvironment {
    ContainerAuto(ContainerAutoParam),
    Local(LocalEnvironmentParam),
    ContainerReference(ContainerReferenceParam),
}

/// OpenAPI schema: `#/components/schemas/FunctionShellToolParam`
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FunctionShellToolParam {
    #[serde(default, skip_serializing_if = "OptionalNullable::is_missing")]
    pub environment: OptionalNullable<FunctionShellEnvironment>,
    #[serde(default, skip_serializing_if = "OptionalNullable::is_missing")]
    pub allowed_callers: OptionalNullable<Vec<CallableToolAllowedCaller>>,
}

// ============================================================
// Input / Context Item Supporting Types
// ============================================================

/// OpenAPI schema: `#/components/schemas/FunctionShellActionParam`
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FunctionShellActionParam {
    pub commands: Vec<String>,
    #[serde(default, skip_serializing_if = "OptionalNullable::is_missing")]
    pub timeout_ms: OptionalNullable<u64>,
    #[serde(default, skip_serializing_if = "OptionalNullable::is_missing")]
    pub max_output_length: OptionalNullable<u64>,
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
#[serde(tag = "type", rename_all = "snake_case")]
pub enum FunctionShellCallItemEnvironment {
    Local(LocalEnvironmentParam),
    ContainerReference(ContainerReferenceParam),
}

/// OpenAPI schema: `#/components/schemas/FunctionShellCallOutputExitOutcomeParam`
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FunctionShellCallOutputExitOutcomeParam {
    pub exit_code: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum FunctionShellCallOutputOutcomeParam {
    Timeout,
    Exit(FunctionShellCallOutputExitOutcomeParam),
}

/// OpenAPI schema: `#/components/schemas/FunctionShellCallOutputContentParam`
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FunctionShellCallOutputContentParam {
    pub stdout: String,
    pub stderr: String,
    pub outcome: FunctionShellCallOutputOutcomeParam,
}

// ============================================================
// Input / Context Item Shapes
// ============================================================

/// OpenAPI schema: `#/components/schemas/FunctionShellCallItemParam`
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FunctionShellCallItemParam {
    #[serde(default, skip_serializing_if = "OptionalNullable::is_missing")]
    pub id: OptionalNullable<String>,
    pub call_id: String,
    #[serde(default, skip_serializing_if = "OptionalNullable::is_missing")]
    pub caller: OptionalNullable<ToolCallCallerParam>,
    pub action: FunctionShellActionParam,
    #[serde(default, skip_serializing_if = "OptionalNullable::is_missing")]
    pub status: OptionalNullable<FunctionShellCallItemStatus>,
    #[serde(default, skip_serializing_if = "OptionalNullable::is_missing")]
    pub environment: OptionalNullable<FunctionShellCallItemEnvironment>,
}

/// OpenAPI schema: `#/components/schemas/FunctionShellCallOutputItemParam`
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FunctionShellCallOutputItemParam {
    #[serde(default, skip_serializing_if = "OptionalNullable::is_missing")]
    pub id: OptionalNullable<String>,
    pub call_id: String,
    #[serde(default, skip_serializing_if = "OptionalNullable::is_missing")]
    pub caller: OptionalNullable<ToolCallCallerParam>,
    pub output: Vec<FunctionShellCallOutputContentParam>,
    #[serde(default, skip_serializing_if = "OptionalNullable::is_missing")]
    pub status: OptionalNullable<FunctionShellCallItemStatus>,
    #[serde(default, skip_serializing_if = "OptionalNullable::is_missing")]
    pub max_output_length: OptionalNullable<u64>,
}

// ============================================================
// Function Shell Output Supporting Types
// ============================================================

/// OpenAPI schema: `#/components/schemas/FunctionShellAction`
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FunctionShellAction {
    pub commands: Vec<String>,
    pub timeout_ms: RequiredNullable<u64>,
    pub max_output_length: RequiredNullable<u64>,
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

/// OpenAPI schema: `#/components/schemas/ContainerReferenceResource`
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContainerReferenceResource {
    pub container_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum FunctionShellCallEnvironment {
    Local,
    ContainerReference(ContainerReferenceResource),
}

/// OpenAPI schema: `#/components/schemas/FunctionShellCallOutputExitOutcome`
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FunctionShellCallOutputExitOutcome {
    pub exit_code: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum FunctionShellCallOutputOutcome {
    Timeout,
    Exit(FunctionShellCallOutputExitOutcome),
}

/// OpenAPI schema: `#/components/schemas/FunctionShellCallOutputContent`
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FunctionShellCallOutputContent {
    pub stdout: String,
    pub stderr: String,
    pub outcome: FunctionShellCallOutputOutcome,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_present"
    )]
    pub created_by: Option<String>,
}

// ============================================================
// Function Shell Output Shapes
// ============================================================

/// OpenAPI schema: `#/components/schemas/FunctionShellCall`
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FunctionShellCall {
    pub id: String,
    pub call_id: String,
    #[serde(default, skip_serializing_if = "OptionalNullable::is_missing")]
    pub caller: OptionalNullable<ToolCallCaller>,
    pub action: FunctionShellAction,
    pub status: FunctionShellCallStatus,
    pub environment: RequiredNullable<FunctionShellCallEnvironment>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_present"
    )]
    pub created_by: Option<String>,
}

/// OpenAPI schema: `#/components/schemas/FunctionShellCallOutput`
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FunctionShellCallOutput {
    pub id: String,
    pub call_id: String,
    #[serde(default, skip_serializing_if = "OptionalNullable::is_missing")]
    pub caller: OptionalNullable<ToolCallCaller>,
    pub status: FunctionShellCallOutputStatusEnum,
    pub output: Vec<FunctionShellCallOutputContent>,
    pub max_output_length: RequiredNullable<u64>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_present"
    )]
    pub created_by: Option<String>,
}
