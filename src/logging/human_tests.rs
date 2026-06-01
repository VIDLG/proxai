use super::{base36, short_request_id};

#[test]
fn short_request_id_compacts_millisecond_timestamp() {
    assert_eq!(short_request_id("1778228311188"), "wn82ac");
}

#[test]
fn short_request_id_falls_back_to_last_six_chars_for_non_numeric_ids() {
    assert_eq!(short_request_id("request-abcdef"), "abcdef");
}

#[test]
fn base36_encodes_zero_and_regular_values() {
    assert_eq!(base36(0), "0");
    assert_eq!(base36(35), "z");
    assert_eq!(base36(36), "10");
}
