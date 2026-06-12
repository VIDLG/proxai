#![allow(
    dead_code,
    unused_imports,
    clippy::enum_variant_names,
    reason = "Anthropic Messages cross-reference types shared by tools/ and content/."
)]

use serde::{Deserialize, Serialize};

use super::{
    citations::{CitationsConfig, CitationsConfigParam, TextCitationParam},
    common::CacheControlEphemeral,
};

/// 🎯 @use: shared discriminator for text blocks.
/// Used by: request
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TextBlockType {
    Text,
}

/// Discriminator value used by `DocumentBlock.type`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DocumentBlockType {
    Document,
}

/// Media type enum used by `Base64ImageSource.media_type`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ImageMediaType {
    #[serde(rename = "image/jpeg")]
    Jpeg,
    #[serde(rename = "image/png")]
    Png,
    #[serde(rename = "image/gif")]
    Gif,
    #[serde(rename = "image/webp")]
    Webp,
}

/// Media type enum used by `Base64PDFSource.media_type`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PdfMediaType {
    #[serde(rename = "application/pdf")]
    ApplicationPdf,
}

/// Media type enum used by `PlainTextSource.media_type`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PlainTextMediaType {
    #[serde(rename = "text/plain")]
    TextPlain,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Base64ImageSource {
    pub data: String,
    pub media_type: ImageMediaType,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Base64PdfSource {
    pub data: String,
    pub media_type: PdfMediaType,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlainTextSource {
    pub data: String,
    pub media_type: PlainTextMediaType,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UrlImageSource {
    pub url: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UrlPdfSource {
    pub url: String,
}

/// ImageBlockParam.source: `Base64ImageSource | URLImageSource`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ImageBlockSource {
    Base64(Base64ImageSource),
    Url(UrlImageSource),
}

// ── Cross-reference param types ──────────────────────────────────────────

/// 🎯 @use: text content block param.
/// Used by: content, search, self, tool_use
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TextBlockParam {
    pub text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_control: Option<CacheControlEphemeral>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub citations: Option<Vec<TextCitationParam>>,
}

/// 🎯 @use: image content block param.
/// Used by: content, self, tool_use
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImageBlockParam {
    pub source: ImageBlockSource,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_control: Option<CacheControlEphemeral>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ContentBlockSourceContent {
    Text(TextBlockParam),
    Image(ImageBlockParam),
}

/// ContentBlockSource.content: `string | Array<ContentBlockSourceContent>`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ContentBlockSourceContentUnion {
    Text(String),
    Blocks(Vec<ContentBlockSourceContent>),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContentBlockSource {
    pub content: ContentBlockSourceContentUnion,
}

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

/// 🎯 @use: document content block param.
/// Used by: content, tool_use, web
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DocumentBlockParam {
    pub source: DocumentBlockParamSource,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_control: Option<CacheControlEphemeral>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub citations: Option<CitationsConfigParam>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
}

/// DocumentBlock.source: `Base64PDFSource | PlainTextSource`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum DocumentBlockSource {
    #[serde(rename = "base64")]
    Base64(Base64PdfSource),
    #[serde(rename = "text")]
    PlainText(PlainTextSource),
}

/// 🎯 @use: response-side document block.
/// Used by: web
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DocumentBlock {
    /// @sdk(required_nullable_accepts_missing)
    pub citations: Option<CitationsConfig>,
    pub source: DocumentBlockSource,
    /// @sdk(required_nullable_accepts_missing)
    pub title: Option<String>,
    #[serde(rename = "type")]
    pub type_: DocumentBlockType,
}
