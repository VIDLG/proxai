use serde_json::json;

use super::{
    ChatCompletionAllowedTools, ChatCompletionAllowedToolsChoice, ChatCompletionToolChoiceOption,
    CustomGrammarFormatParam, CustomToolPropertiesFormat, GrammarSyntax, ToolChoiceAllowedMode,
};

#[test]
fn serializes_allowed_tools_choice_with_official_object_shape() {
    let choice = ChatCompletionToolChoiceOption::AllowedTools(ChatCompletionAllowedToolsChoice {
        allowed_tools: ChatCompletionAllowedTools {
            mode: ToolChoiceAllowedMode::Auto,
            tools: vec![json!({ "type": "function", "function": { "name": "lookup" } })],
        },
    });

    assert_eq!(
        serde_json::to_value(choice).unwrap(),
        json!({
            "type": "allowed_tools",
            "allowed_tools": {
                "mode": "auto",
                "tools": [
                    { "type": "function", "function": { "name": "lookup" } }
                ],
            },
        })
    );
}

#[test]
fn serializes_custom_grammar_format_as_a_flat_tagged_object() {
    let format = CustomToolPropertiesFormat::Grammar(CustomGrammarFormatParam {
        definition: "start: WORD".to_string(),
        syntax: GrammarSyntax::Lark,
    });

    assert_eq!(
        serde_json::to_value(format).unwrap(),
        json!({
            "type": "grammar",
            "definition": "start: WORD",
            "syntax": "lark",
        })
    );
}
