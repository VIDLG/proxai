use std::collections::BTreeMap;

use crate::formatting::{format_count_map, format_count_map_with, join_non_empty, map_count_keys};

pub(crate) fn string_count_map<K>(values: &BTreeMap<K, u64>) -> BTreeMap<String, u64>
where
    K: ToString,
{
    map_count_keys(values, |key| key.to_string())
}

pub(crate) fn merge_count_maps<const N: usize>(
    maps: [BTreeMap<String, u64>; N],
) -> BTreeMap<String, u64> {
    let mut merged = BTreeMap::new();
    for map in maps {
        for (key, value) in map {
            *merged.entry(key).or_insert(0) += value;
        }
    }
    merged
}

pub(crate) fn source_count_maps<const N: usize>(
    maps: [(&'static str, BTreeMap<String, u64>); N],
) -> BTreeMap<String, BTreeMap<String, u64>> {
    maps.into_iter()
        .filter(|(_, values)| !values.is_empty())
        .map(|(source, values)| (source.to_string(), values))
        .collect()
}

pub(crate) fn compact_output_items<K>(values: &BTreeMap<K, u64>) -> String
where
    K: ToString,
{
    let rendered: Vec<_> = values
        .iter()
        .filter_map(|(key, value)| {
            let key_str = key.to_string();
            let alias = super::compact_output_item_kind(&key_str);
            if alias.is_empty() {
                None
            } else {
                Some(format!("{alias}:{value}"))
            }
        })
        .collect();
    rendered.join(" ")
}

pub(crate) fn compact_output_items_for_human<K>(
    values: &BTreeMap<K, u64>,
    default_kind: K,
) -> String
where
    K: Copy + Ord + ToString,
{
    if values.len() == 1 && values.get(&default_kind) == Some(&1) {
        String::new()
    } else {
        compact_output_items(values)
    }
}

pub(crate) fn compact_tool_calls<K>(values: &BTreeMap<K, u64>) -> String
where
    K: AsRef<str>,
{
    format_count_map_with(values, |key| super::compact_tool_call_name(key.as_ref()))
}

pub(crate) fn join_call_maps<const N: usize>(values: [String; N]) -> String {
    join_non_empty(values)
}

pub(crate) fn full_count_map<K>(values: &BTreeMap<K, u64>) -> String
where
    K: std::fmt::Display,
{
    format_count_map(values)
}

#[cfg(test)]
#[path = "counts_tests.rs"]
mod tests;
