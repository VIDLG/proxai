use crate::protocol::openai::chat_completions as chat;

use crate::protocol::openai::responses;

use crate::translation::openai_chat_completions::compatibility::ChatRequestExtensions;
use crate::translation::openai_chat_completions::outbound::{
    assistant_message, developer_message, developer_text_message, system_message, tool_message,
    user_message, user_text_message,
};
use crate::translation::{TranslationError, TranslationResult, TranslationScope};

use super::content::{
    assistant_content_from_easy_input, assistant_content_from_output,
    assistant_reasoning_content_from_item, developer_content_from_easy_input,
    developer_content_from_input, system_content_from_easy_input, system_content_from_input,
    tool_content_from_custom_output, tool_content_from_function_output,
    user_content_from_easy_input, user_content_from_input,
};

/// Consecutive Responses reasoning and tool-call items that Chat represents
/// together with at most one explicit assistant message.
#[derive(Default)]
struct PendingAssistantTurn {
    content: Option<chat::ChatCompletionRequestAssistantMessageContent>,
    reasoning: String,
    tool_calls: Vec<chat::ChatCompletionMessageToolCalls>,
}

impl PendingAssistantTurn {
    fn with_content(
        self,
        content: chat::ChatCompletionRequestAssistantMessageContent,
    ) -> (Self, Option<ProjectedMessage>) {
        if self.content.is_some() {
            return (
                Self {
                    content: Some(content),
                    ..Self::default()
                },
                self.finish(),
            );
        }

        (
            Self {
                content: Some(content),
                ..self
            },
            None,
        )
    }

    fn with_reasoning(mut self, reasoning: &str) -> Self {
        self.reasoning.push_str(reasoning);
        self
    }

    fn with_tool_call(mut self, tool_call: chat::ChatCompletionMessageToolCalls) -> Self {
        self.tool_calls.push(tool_call);
        self
    }

    fn take_finished(&mut self) -> Option<ProjectedMessage> {
        std::mem::take(self).finish()
    }

    fn finish(self) -> Option<ProjectedMessage> {
        let Self {
            content,
            reasoning,
            tool_calls,
        } = self;
        let tool_calls = (!tool_calls.is_empty()).then_some(tool_calls);
        if content.is_none() && reasoning.is_empty() && tool_calls.is_none() {
            return None;
        }

        Some(ProjectedMessage {
            message: assistant_message(content, tool_calls),
            reasoning: (!reasoning.is_empty()).then_some(reasoning),
        })
    }
}

/// Message plus its non-schema reasoning extension before the final message
/// index is known.
struct ProjectedMessage {
    message: chat::ChatCompletionRequestMessage,
    reasoning: Option<String>,
}

pub(super) fn project_request_messages(
    instructions: Option<&str>,
    input: Option<&responses::InputParam>,
    scope: &TranslationScope,
) -> TranslationResult<(
    Vec<chat::ChatCompletionRequestMessage>,
    ChatRequestExtensions,
)> {
    let mut projected = Vec::new();
    if let Some(instructions) = instructions
        .map(str::trim)
        .filter(|instructions| !instructions.is_empty())
    {
        projected.push(ProjectedMessage {
            message: developer_text_message(instructions),
            reasoning: None,
        });
    }
    let mut assistant = PendingAssistantTurn::default();

    match input {
        Some(responses::InputParam::Text(text)) => projected.push(ProjectedMessage {
            message: user_text_message(text),
            reasoning: None,
        }),
        Some(responses::InputParam::Items(items)) => {
            for input_item in items {
                match input_item {
                    responses::InputItem::EasyMessage(message) => match message.role {
                        responses::Role::Assistant => {
                            if let Some(content) =
                                assistant_content_from_easy_input(&message.content, scope)
                            {
                                let (next, completed) = assistant.with_content(content);
                                assistant = next;
                                if let Some(message) = completed {
                                    projected.push(message);
                                }
                            }
                        }
                        responses::Role::System => {
                            if let Some(message) = assistant.take_finished() {
                                projected.push(message);
                            }
                            if let Some(content) =
                                system_content_from_easy_input(&message.content, scope)
                            {
                                projected.push(ProjectedMessage {
                                    message: system_message(content),
                                    reasoning: None,
                                });
                            }
                        }
                        responses::Role::Developer => {
                            if let Some(message) = assistant.take_finished() {
                                projected.push(message);
                            }
                            if let Some(content) =
                                developer_content_from_easy_input(&message.content, scope)
                            {
                                projected.push(ProjectedMessage {
                                    message: developer_message(content),
                                    reasoning: None,
                                });
                            }
                        }
                        responses::Role::User => {
                            if let Some(message) = assistant.take_finished() {
                                projected.push(message);
                            }
                            if let Some(content) =
                                user_content_from_easy_input(&message.content, scope)
                            {
                                projected.push(ProjectedMessage {
                                    message: user_message(content),
                                    reasoning: None,
                                });
                            }
                        }
                    },
                    responses::InputItem::ItemReference(reference) => {
                        return Err(TranslationError::InvalidPayload(format!(
                            "OpenAI Responses item_reference `{}` cannot be translated to Chat Completions because the referenced item content is not available",
                            reference.id
                        )));
                    }
                    responses::InputItem::Item(item) => match item {
                        responses::Item::Message(responses::MessageItem::Output(output)) => {
                            if let Some(content) = assistant_content_from_output(output, scope) {
                                let (next, completed) = assistant.with_content(content);
                                assistant = next;
                                if let Some(message) = completed {
                                    projected.push(message);
                                }
                            }
                        }
                        responses::Item::Message(responses::MessageItem::Input(input)) => {
                            if let Some(message) = assistant.take_finished() {
                                projected.push(message);
                            }
                            let message = match input.role {
                                responses::InputRole::System => {
                                    system_content_from_input(&input.content, scope)
                                        .map(system_message)
                                }
                                responses::InputRole::Developer => {
                                    developer_content_from_input(&input.content, scope)
                                        .map(developer_message)
                                }
                                responses::InputRole::User => {
                                    user_content_from_input(&input.content, scope).map(user_message)
                                }
                            };
                            if let Some(message) = message {
                                projected.push(ProjectedMessage {
                                    message,
                                    reasoning: None,
                                });
                            }
                        }
                        responses::Item::FunctionCall(call) => {
                            assistant = assistant.with_tool_call(call.into());
                        }
                        responses::Item::FunctionCallOutput(output) => {
                            if let Some(message) = assistant.take_finished() {
                                projected.push(message);
                            }
                            projected.push(ProjectedMessage {
                                message: tool_message(
                                    tool_content_from_function_output(&output.output, scope),
                                    output.call_id.clone(),
                                ),
                                reasoning: None,
                            });
                        }
                        responses::Item::CustomToolCall(call) => {
                            assistant = assistant.with_tool_call(call.into());
                        }
                        responses::Item::CustomToolCallOutput(output) => {
                            if let Some(message) = assistant.take_finished() {
                                projected.push(message);
                            }
                            projected.push(ProjectedMessage {
                                message: tool_message(
                                    tool_content_from_custom_output(&output.output, scope),
                                    output.call_id.clone(),
                                ),
                                reasoning: None,
                            });
                        }
                        responses::Item::Reasoning(reasoning) => {
                            if let Some(reasoning) =
                                assistant_reasoning_content_from_item(reasoning, scope)
                            {
                                assistant = assistant.with_reasoning(&reasoning);
                            }
                        }
                        item @ responses::Item::AdditionalTools(_) => {
                            if let Some(message) = assistant.take_finished() {
                                projected.push(message);
                            }
                            scope.dropped(
                                format!("Responses input item `{}`", item.as_ref()),
                                "Chat Completions cannot represent per-item developer tool declarations",
                            );
                        }
                        responses::Item::Compaction(_) => {
                            return Err(TranslationError::InvalidPayload(
                                "OpenAI Responses compaction item cannot be translated to Chat Completions because its summary content is encrypted"
                                    .to_string(),
                            ));
                        }
                        item @ (responses::Item::FileSearchCall(_)
                        | responses::Item::ComputerCall(_)
                        | responses::Item::WebSearchCall(_)
                        | responses::Item::ToolSearchCall(_)
                        | responses::Item::ImageGenerationCall(_)
                        | responses::Item::CodeInterpreterCall(_)
                        | responses::Item::LocalShellCall(_)
                        | responses::Item::ShellCall(_)
                        | responses::Item::ApplyPatchCall(_)
                        | responses::Item::McpListTools(_)
                        | responses::Item::McpApprovalRequest(_)
                        | responses::Item::McpCall(_)) => {
                            scope.dropped(
                                format!("Responses input item `{}`", item.as_ref()),
                                "Chat Completions has no matching assistant artifact",
                            );
                        }
                        item @ responses::Item::ToolSearchOutput(output) => {
                            if output.execution == Some(responses::ToolSearchExecutionType::Client)
                                && let Some(message) = assistant.take_finished()
                            {
                                projected.push(message);
                            }
                            scope.dropped(
                                format!("Responses input item `{}`", item.as_ref()),
                                "Chat Completions cannot represent dynamic tool-search results",
                            );
                        }
                        item @ (responses::Item::ComputerCallOutput(_)
                        | responses::Item::LocalShellCallOutput(_)
                        | responses::Item::ShellCallOutput(_)
                        | responses::Item::ApplyPatchCallOutput(_)
                        | responses::Item::McpApprovalResponse(_)) => {
                            if let Some(message) = assistant.take_finished() {
                                projected.push(message);
                            }
                            scope.dropped(
                                format!("Responses input item `{}`", item.as_ref()),
                                "Chat Completions has no matching user/tool-result message",
                            );
                        }
                    },
                }
            }
        }
        None => {}
    }
    if let Some(message) = assistant.finish() {
        projected.push(message);
    }
    if projected.is_empty() {
        scope.adapted(
            "Responses request without representable messages",
            "Chat Completions requires at least one message; emitting an empty user message",
        );
        projected.push(ProjectedMessage {
            message: user_text_message(""),
            reasoning: None,
        });
    }

    let mut extensions = ChatRequestExtensions::default();
    let messages = projected
        .into_iter()
        .enumerate()
        .map(|(index, projected)| {
            if let Some(reasoning) = projected.reasoning {
                extensions.insert(index, reasoning);
            }
            projected.message
        })
        .collect();
    Ok((messages, extensions))
}
