use std::borrow::Cow;

pub const TOOL_NAME_ALIASES: &[(&str, &str)] = &[
    ("cp", "copy_path"),
    ("mkdir", "create_directory"),
    ("rm", "delete_path"),
    ("diag", "diagnostics"),
    ("e", "edit_file"),
    ("fetch", "fetch"),
    ("fd", "find_path"),
    ("rg", "grep"),
    ("ls", "list_directory"),
    ("mv", "move_path"),
    ("t", "now"),
    ("o", "open"),
    ("r", "read_file"),
    ("restore", "restore_file_from_disk"),
    ("w", "save_file"),
    ("spawn", "spawn_agent"),
    ("sh", "terminal"),
];

pub(crate) fn compact_tool_call_name(name: &str) -> String {
    compact_alias(name, TOOL_NAME_ALIASES).into_owned()
}

fn compact_alias<'a>(value: &'a str, aliases: &[(&'static str, &'static str)]) -> Cow<'a, str> {
    aliases
        .iter()
        .find_map(|(alias, full)| (*full == value).then_some(Cow::Borrowed(*alias)))
        .unwrap_or(Cow::Borrowed(value))
}
