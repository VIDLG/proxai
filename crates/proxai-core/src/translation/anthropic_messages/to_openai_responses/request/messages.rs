//! Message and content block translation for `anthropic_messages -> openai_responses`.

use crate::protocol::anthropic::messages as anthropic;
use crate::protocol::openai_responses as responses;
use crate::translation::openai_responses::outbound::easy_message;
use crate::translation::{TranslationError, TranslationResult};

pub(super) fn translate_message_param(
    message: anthropic::MessageParam,
) -> TranslationResult<Vec<responses::InputItem>> {
    let role = message.role.into();

    match message.content {
        anthropic::MessageParamContent::Text(text) => Ok(vec![easy_message(
            role,
            responses::EasyInputContent::Text(text),
        )]),
        anthropic::MessageParamContent::Blocks(blocks) => {
            let mut items = Vec::new();
            for block in blocks {
                items.extend(translate_content_block(block, role)?);
            }
            Ok(items)
        }
    }
}

// ── Content block → InputItem ──────────────────────────────────────────

fn translate_content_block(
    block: anthropic::ContentBlockParam,
    role: responses::Role,
) -> TranslationResult<Vec<responses::InputItem>> {
    match block {
        anthropic::ContentBlockParam::ToolUse(tool_use) => {
            Ok(vec![responses::InputItem::Item(tool_use.try_into()?)])
        }
        anthropic::ContentBlockParam::ToolResult(tool_result) => {
            Ok(vec![responses::InputItem::Item(tool_result.try_into()?)])
        }
        anthropic::ContentBlockParam::Text(block) => {
            Ok(vec![easy_input_item(role, block.into())])
        }
        anthropic::ContentBlockParam::Image(block) => {
            Ok(vec![easy_input_item(role, block.into())])
        }
        anthropic::ContentBlockParam::Document(block) => {
            Ok(vec![easy_input_item(role, block.try_into()?)])
        }
        anthropic::ContentBlockParam::ContainerUpload(_) => Err(TranslationError::InvalidPayload(
            "Anthropic container_upload content cannot be translated to OpenAI Responses input_file; container file IDs are provider-scoped and are not safely interchangeable with Responses file_id"
                .to_string(),
        )),
        other => Err(TranslationError::InvalidPayload(format!(
            "Anthropic content block `{}` is a response-side type that cannot appear \
             in OpenAI Responses input; only text, image, and document blocks are \
             supported in request translation",
            other.as_ref()
        ))),
    }
}

fn easy_input_item(
    role: responses::Role,
    content: responses::InputContent,
) -> responses::InputItem {
    let content = match content {
        responses::InputContent::InputText(t) => responses::EasyInputContent::Text(t.text),
        other => responses::EasyInputContent::ContentList(vec![other]),
    };
    easy_message(role, content)
}
