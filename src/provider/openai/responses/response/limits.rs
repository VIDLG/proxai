use axum::http::HeaderMap;
use std::time::Duration;

#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct ResponseLimits {
    pub(crate) rate: RateLimit,
    pub(crate) codex: CodexLimits,
}

impl ResponseLimits {
    pub(crate) fn from_headers(headers: &HeaderMap) -> Self {
        Self {
            rate: RateLimit::from_headers(headers),
            codex: CodexLimits::from_headers(headers),
        }
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct RateLimit {
    pub(crate) limit_requests: Option<u64>,
    pub(crate) limit_tokens: Option<u64>,
    pub(crate) remaining_requests: Option<u64>,
    pub(crate) remaining_tokens: Option<u64>,
    pub(crate) reset_requests: Option<Duration>,
    pub(crate) reset_tokens: Option<Duration>,
}

impl RateLimit {
    pub(crate) fn from_headers(headers: &HeaderMap) -> Self {
        let mut value = Self::default();
        for (name, header_value) in headers {
            let name = name.as_str().to_ascii_lowercase();
            let Ok(header_value) = header_value.to_str() else {
                continue;
            };
            match name.as_str() {
                "x-ratelimit-limit-requests" => value.limit_requests = header_value.parse().ok(),
                "x-ratelimit-limit-tokens" => value.limit_tokens = header_value.parse().ok(),
                "x-ratelimit-remaining-requests" => {
                    value.remaining_requests = header_value.parse().ok()
                }
                "x-ratelimit-remaining-tokens" => {
                    value.remaining_tokens = header_value.parse().ok()
                }
                "x-ratelimit-reset-requests" => {
                    value.reset_requests = humantime::parse_duration(header_value).ok()
                }
                "x-ratelimit-reset-tokens" => {
                    value.reset_tokens = humantime::parse_duration(header_value).ok()
                }
                _ => {}
            }
        }
        value
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct CodexLimits {
    pub(crate) primary_used_percent: Option<f64>,
    pub(crate) primary_reset_after_secs: Option<u64>,
    pub(crate) primary_window_minutes: Option<u64>,
    pub(crate) secondary_used_percent: Option<f64>,
    pub(crate) secondary_reset_after_secs: Option<u64>,
    pub(crate) secondary_window_minutes: Option<u64>,
    pub(crate) primary_over_secondary_percent: Option<f64>,
}

impl CodexLimits {
    pub(crate) fn from_headers(headers: &HeaderMap) -> Self {
        let mut value = Self::default();
        for (name, header_value) in headers {
            let name = name.as_str().to_ascii_lowercase();
            let Ok(header_value) = header_value.to_str() else {
                continue;
            };
            match name.as_str() {
                "x-codex-primary-used-percent" => {
                    value.primary_used_percent = header_value.parse().ok()
                }
                "x-codex-primary-reset-after-seconds" => {
                    value.primary_reset_after_secs = header_value.parse().ok()
                }
                "x-codex-primary-window-minutes" => {
                    value.primary_window_minutes = header_value.parse().ok()
                }
                "x-codex-secondary-used-percent" => {
                    value.secondary_used_percent = header_value.parse().ok()
                }
                "x-codex-secondary-reset-after-seconds" => {
                    value.secondary_reset_after_secs = header_value.parse().ok()
                }
                "x-codex-secondary-window-minutes" => {
                    value.secondary_window_minutes = header_value.parse().ok()
                }
                "x-codex-primary-over-secondary-limit-percent" => {
                    value.primary_over_secondary_percent = header_value.parse().ok()
                }
                _ => {}
            }
        }
        value
    }
}
