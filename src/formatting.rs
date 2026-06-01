use std::collections::BTreeMap;
use std::fmt::Display;
use unit_prefix::NumberPrefix;

pub(crate) fn format_count(value: u64) -> String {
    format_prefixed_number(value, NumberPrefix::decimal)
}

pub(crate) fn format_bytes(value: u64) -> String {
    format!("{}B", format_prefixed_number(value, NumberPrefix::binary))
}

pub(crate) fn format_millis(ms: u64) -> String {
    let seconds = ms / 1_000;
    let millis = ms % 1_000;
    if millis == 0 {
        return format!("{seconds}s");
    }
    let fraction = format!("{millis:03}").trim_end_matches('0').to_string();
    format!("{seconds}.{fraction}s")
}

pub(crate) fn format_minutes_as_duration(minutes: u64) -> String {
    format_seconds_as_duration(minutes.saturating_mul(60))
}

pub(crate) fn format_seconds_as_duration(seconds: u64) -> String {
    let hours = seconds / 3600;
    let minutes = (seconds % 3600) / 60;
    let seconds = seconds % 60;

    match (hours, minutes, seconds) {
        (0, 0, s) => format!("{s}s"),
        (0, m, 0) => format!("{m}m"),
        (0, m, s) => format!("{m}m{s}s"),
        (h, 0, 0) => format!("{h}h"),
        (h, m, 0) => format!("{h}h{m}m"),
        (h, m, s) => format!("{h}h{m}m{s}s"),
    }
}

pub(crate) fn format_count_map<K>(values: &BTreeMap<K, u64>) -> String
where
    K: Display,
{
    values
        .iter()
        .map(|(key, value)| format!("{key}:{value}"))
        .collect::<Vec<_>>()
        .join(" ")
}

pub(crate) fn compact_tail(value: &str, keep: usize) -> String {
    let chars = value.chars().collect::<Vec<_>>();
    if chars.len() <= keep {
        value.to_string()
    } else {
        format!(
            "...{}",
            chars[chars.len() - keep..].iter().collect::<String>()
        )
    }
}

pub(crate) fn truncate_chars(value: &str, max_chars: usize) -> String {
    let mut chars = value.chars();
    let truncated = chars.by_ref().take(max_chars).collect::<String>();
    if chars.next().is_some() {
        format!("{truncated}...")
    } else {
        truncated
    }
}

fn format_prefixed_number(value: u64, prefixer: impl FnOnce(f64) -> NumberPrefix<f64>) -> String {
    match prefixer(value as f64) {
        NumberPrefix::Standalone(value) => format!("{value:.0}"),
        NumberPrefix::Prefixed(prefix, value) => {
            format!("{value:.1}{}", format_prefix_label(prefix.to_string()))
        }
    }
}

fn format_prefix_label(prefix: String) -> String {
    prefix.trim_end_matches('i').replace('k', "K")
}
