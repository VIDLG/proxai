#![allow(
    dead_code,
    unused_imports,
    clippy::enum_variant_names,
    reason = "Anthropic Messages cross-reference types shared by tools/ and content/."
)]

use crate::protocol::OptionalNullable;
use crate::protocol::RequiredNullable;
use serde::{Deserialize, Serialize};
use strum::{AsRefStr, Display};

use super::{
    citations::{CitationsConfig, CitationsConfigParam, TextCitationParam},
    common::CacheControlEphemeral,
};

/// @sdk(proxai_internal = "discriminator")
/// 🎯 @use: shared discriminator for text blocks.
/// Used by: request
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TextBlockType {
    Text,
}

/// @sdk(proxai_internal = "discriminator")
/// Discriminator value used by `DocumentBlock.type`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DocumentBlockType {
    Document,
}

/// @sdk(proxai_internal = "field_literal_wrapper")
/// Media type enum used by `Base64ImageSource.media_type`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, AsRefStr, Display, Serialize, Deserialize)]
pub enum ImageMediaType {
    #[strum(serialize = "image/jpeg")]
    #[serde(rename = "image/jpeg")]
    Jpeg,
    #[strum(serialize = "image/png")]
    #[serde(rename = "image/png")]
    Png,
    #[strum(serialize = "image/gif")]
    #[serde(rename = "image/gif")]
    Gif,
    #[strum(serialize = "image/webp")]
    #[serde(rename = "image/webp")]
    Webp,
}

/// @sdk(proxai_internal = "field_literal_wrapper")
/// Media type enum used by `Base64PDFSource.media_type`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, AsRefStr, Display, Serialize, Deserialize)]
pub enum PdfMediaType {
    #[strum(serialize = "application/pdf")]
    #[serde(rename = "application/pdf")]
    ApplicationPdf,
}

/// @sdk(proxai_internal = "field_literal_wrapper")
/// Media type enum used by `PlainTextSource.media_type`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, AsRefStr, Display, Serialize, Deserialize)]
pub enum PlainTextMediaType {
    #[strum(serialize = "text/plain")]
    #[serde(rename = "text/plain")]
    TextPlain,
}

/// @sdk(shape = "Base64ImageSource")
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Base64ImageSource {
    pub data: String,
    pub media_type: ImageMediaType,
}

/// @sdk(shape = "Base64PDFSource")
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Base64PdfSource {
    pub data: String,
    pub media_type: PdfMediaType,
}

/// @sdk(shape = "PlainTextSource")
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlainTextSource {
    pub data: String,
    pub media_type: PlainTextMediaType,
}

/// @sdk(shape = "URLImageSource")
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UrlImageSource {
    pub url: String,
}

/// @sdk(shape = "URLPDFSource")
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UrlPdfSource {
    pub url: String,
}

/// ImageBlockParam.source: `Base64ImageSource | URLImageSource`.
/// @sdk(proxai_internal = "union_wrapper")
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ImageBlockSource {
    Base64(Base64ImageSource),
    Url(UrlImageSource),
}

// ── Cross-reference param types ──────────────────────────────────────────

/// @sdk(shape = "TextBlockParam")
/// 🎯 @use: text content block param.
/// Used by: content, search, self, tool_use
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TextBlockParam {
    pub text: String,
    #[serde(default, skip_serializing_if = "OptionalNullable::is_missing")]
    pub cache_control: OptionalNullable<CacheControlEphemeral>,
    #[serde(default, skip_serializing_if = "OptionalNullable::is_missing")]
    pub citations: OptionalNullable<Vec<TextCitationParam>>,
}

/// @sdk(shape = "ImageBlockParam")
/// 🎯 @use: image content block param.
/// Used by: content, self, tool_use
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImageBlockParam {
    pub source: ImageBlockSource,
    #[serde(default, skip_serializing_if = "OptionalNullable::is_missing")]
    pub cache_control: OptionalNullable<CacheControlEphemeral>,
}

/// @sdk(shape = "ContentBlockSourceContent")
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ContentBlockSourceContent {
    Text(TextBlockParam),
    Image(ImageBlockParam),
}

/// @sdk(proxai_internal = "union_wrapper")
/// ContentBlockSource.content: `string | Array<ContentBlockSourceContent>`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ContentBlockSourceContentUnion {
    Text(String),
    Blocks(Vec<ContentBlockSourceContent>),
}

/// @sdk(shape = "ContentBlockSource")
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContentBlockSource {
    pub content: ContentBlockSourceContentUnion,
}

/// @sdk(proxai_internal = "union_wrapper")
/// DocumentBlockParam.source: `Base64PDFSource | PlainTextSource | ContentBlockSource | URLPDFSource`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum DocumentBlockParamSource {
    #[serde(rename = "base64")]
    Base64(Base64PdfSource),
    #[serde(rename = "text")]
    PlainText(PlainTextSource),
    #[serde(rename = "content")]
    Content(ContentBlockSource),
    #[serde(rename = "url")]
    Url(UrlPdfSource),
}

/// @sdk(shape = "DocumentBlockParam")
/// 🎯 @use: document content block param.
/// Used by: content, tool_use, web
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DocumentBlockParam {
    pub source: DocumentBlockParamSource,
    #[serde(default, skip_serializing_if = "OptionalNullable::is_missing")]
    pub cache_control: OptionalNullable<CacheControlEphemeral>,
    #[serde(default, skip_serializing_if = "OptionalNullable::is_missing")]
    pub citations: OptionalNullable<CitationsConfigParam>,
    #[serde(default, skip_serializing_if = "OptionalNullable::is_missing")]
    pub context: OptionalNullable<String>,
    #[serde(default, skip_serializing_if = "OptionalNullable::is_missing")]
    pub title: OptionalNullable<String>,
}

/// @sdk(proxai_internal = "union_wrapper")
/// DocumentBlock.source: `Base64PDFSource | PlainTextSource`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum DocumentBlockSource {
    #[serde(rename = "base64")]
    Base64(Base64PdfSource),
    #[serde(rename = "text")]
    PlainText(PlainTextSource),
}

/// @sdk(shape = "DocumentBlock")
/// 🎯 @use: response-side document block.
/// Used by: web
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DocumentBlock {
    pub citations: RequiredNullable<CitationsConfig>,
    pub source: DocumentBlockSource,
    pub title: RequiredNullable<String>,
    #[serde(rename = "type")]
    pub type_: DocumentBlockType,
}
