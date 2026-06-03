use std::collections::BTreeMap;

use super::*;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum OutputKind {
    Message,
    FunctionCall,
}

impl std::fmt::Display for OutputKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Message => write!(f, "message"),
            Self::FunctionCall => write!(f, "function_call"),
        }
    }
}

#[test]
fn compact_output_items_for_human_omits_single_default_item() {
    let values = BTreeMap::from([(OutputKind::Message, 1)]);

    assert_eq!(
        compact_output_items_for_human(&values, OutputKind::Message),
        ""
    );
}

#[test]
fn compact_output_items_for_human_keeps_non_default_items() {
    let values = BTreeMap::from([(OutputKind::Message, 1), (OutputKind::FunctionCall, 1)]);

    assert_eq!(
        compact_output_items_for_human(&values, OutputKind::Message),
        "m:1 fn:1"
    );
}
