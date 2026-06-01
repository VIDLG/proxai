use indexmap::IndexMap;
use owo_colors::{OwoColorize, Style};
use std::fmt;
use std::io::Write;
use tracing::Level;
use tracing_core::span::{Attributes, Id, Record};
use tracing_core::{Event, Subscriber};
use tracing_subscriber::field::Visit;
use tracing_subscriber::layer::{Context, Layer};
use tracing_subscriber::registry::LookupSpan;

use crate::formatting::{
    format_bytes, format_count, format_millis, format_minutes_as_duration,
    format_seconds_as_duration,
};
use serde::Deserialize;

const DEFAULT_WARN_MS: u64 = 5_000;
const DEFAULT_ERROR_MS: u64 = 15_000;

#[derive(Debug, Clone, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct DurationThresholds {
    pub warn_ms: u64,
    pub error_ms: u64,
}

impl Default for DurationThresholds {
    fn default() -> Self {
        Self {
            warn_ms: DEFAULT_WARN_MS,
            error_ms: DEFAULT_ERROR_MS,
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct HumanLayer {
    duration_thresholds: DurationThresholds,
    color: bool,
}

impl HumanLayer {
    pub(crate) fn new(duration_thresholds: DurationThresholds, color: bool) -> Self {
        Self {
            duration_thresholds,
            color,
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum ColorToken {
    LevelInfo,
    LevelWarn,
    LevelError,
    LevelDebug,
    LevelTrace,
    EventForward,
    EventHeaders,
    EventEnd,
    EventClosed,
    EventError,
    Method,
    StatusOk,
    StatusRedirect,
    StatusError,
    TrafficUp,
    TrafficDown,
    TimeFast,
    TimeWarn,
    TimeSlow,
    Token,
    Flag,
    Tool,
    RateLimit,
    Meta,
}

#[derive(Debug, Default)]
struct LogFields {
    fields: IndexMap<String, String>,
}

impl LogFields {
    fn insert(&mut self, name: &str, value: impl Into<String>) {
        self.fields.insert(name.to_string(), value.into());
    }

    fn extend(&mut self, other: &Self) {
        for (key, value) in &other.fields {
            self.fields.insert(key.clone(), value.clone());
        }
    }

    fn text(&self, name: &str) -> Option<String> {
        self.fields
            .get(name)
            .cloned()
            .filter(|value| !value.is_empty())
    }

    fn u64(&self, name: &str) -> Option<u64> {
        self.text(name)?.parse().ok()
    }

    fn f64(&self, name: &str) -> Option<f64> {
        self.text(name)?.parse().ok()
    }
}

impl Visit for LogFields {
    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn fmt::Debug) {
        self.insert(
            field.name(),
            normalize_debug_field_value(&format!("{value:?}")),
        );
    }

    fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
        self.insert(field.name(), value);
    }

    fn record_bool(&mut self, field: &tracing::field::Field, value: bool) {
        self.insert(field.name(), value.to_string());
    }

    fn record_i64(&mut self, field: &tracing::field::Field, value: i64) {
        self.insert(field.name(), value.to_string());
    }

    fn record_u64(&mut self, field: &tracing::field::Field, value: u64) {
        self.insert(field.name(), value.to_string());
    }

    fn record_error(
        &mut self,
        field: &tracing::field::Field,
        value: &(dyn std::error::Error + 'static),
    ) {
        self.insert(field.name(), value.to_string());
    }
}

fn normalize_debug_field_value(value: &str) -> String {
    let mut value = value.trim();

    loop {
        if value == "None" {
            return String::new();
        }
        if let Some(inner) = value
            .strip_prefix("Some(")
            .and_then(|value| value.strip_suffix(')'))
        {
            value = inner.trim();
            continue;
        }
        return value.trim_matches('"').to_string();
    }
}

impl<S> Layer<S> for HumanLayer
where
    S: Subscriber + for<'span> LookupSpan<'span>,
{
    fn on_new_span(&self, attrs: &Attributes<'_>, id: &Id, ctx: Context<'_, S>) {
        let Some(span) = ctx.span(id) else {
            return;
        };
        let mut fields = LogFields::default();
        attrs.record(&mut fields);
        span.extensions_mut().insert(fields);
    }

    fn on_record(&self, id: &Id, values: &Record<'_>, ctx: Context<'_, S>) {
        let Some(span) = ctx.span(id) else {
            return;
        };
        let mut extensions = span.extensions_mut();
        if let Some(fields) = extensions.get_mut::<LogFields>() {
            values.record(fields);
        }
    }

    fn on_event(&self, event: &Event<'_>, ctx: Context<'_, S>) {
        let meta = event.metadata();
        let mut fields = LogFields::default();
        if let Some(scope) = ctx.event_scope(event) {
            for span in scope.from_root() {
                if let Some(span_fields) = span.extensions().get::<LogFields>() {
                    fields.extend(span_fields);
                }
            }
        }
        event.record(&mut fields);
        let mut line = String::new();
        let _ = format_event_line(
            &mut line,
            meta.level(),
            &fields,
            self.color,
            &self.duration_thresholds,
        );
        let mut stdout = std::io::stdout().lock();
        let _ = writeln!(stdout, "{line}");
    }
}

fn format_event_line(
    writer: &mut dyn fmt::Write,
    level: &Level,
    fields: &LogFields,
    color: bool,
    duration_thresholds: &DurationThresholds,
) -> fmt::Result {
    write!(
        writer,
        "{} {:>5} ",
        paint(
            color,
            chrono::Local::now().format("%H:%M:%S%.3f"),
            style(ColorToken::Meta)
        ),
        paint(color, level, level_style(level))
    )?;
    if let Some(id) = fields.text("id") {
        write!(
            writer,
            "{} ",
            paint(
                color,
                format!("#{}", short_request_id(&id)),
                style(ColorToken::Meta)
            )
        )?;
    }
    if let Some(event) = fields.text("event") {
        write!(writer, "{} ", paint(color, &event, event_style(&event)))?;
    }

    match fields.text("event").as_deref() {
        Some("fwd") => format_forward(writer, fields, color)?,
        Some("hdr") => format_headers(writer, fields, color, duration_thresholds)?,
        Some("end" | "closed") => format_stream_end(writer, fields, color, duration_thresholds)?,
        Some("hdr-error" | "stream-error" | "timeout" | "unfinished-tool") => {
            format_error(writer, fields, color, duration_thresholds)?
        }
        _ => format_remaining_fields(writer, fields, color)?,
    }

    Ok(())
}

fn short_request_id(id: &str) -> String {
    id.parse::<u64>()
        .map(|value| {
            let encoded = base36(value);
            let keep = encoded.len().saturating_sub(6);
            encoded[keep..].to_string()
        })
        .unwrap_or_else(|_| {
            id.chars()
                .rev()
                .take(6)
                .collect::<String>()
                .chars()
                .rev()
                .collect()
        })
}

fn base36(mut value: u64) -> String {
    const DIGITS: &[u8; 36] = b"0123456789abcdefghijklmnopqrstuvwxyz";
    if value == 0 {
        return "0".to_string();
    }

    let mut encoded = Vec::new();
    while value > 0 {
        encoded.push(DIGITS[(value % 36) as usize] as char);
        value /= 36;
    }
    encoded.iter().rev().collect()
}

fn format_forward(writer: &mut dyn fmt::Write, fields: &LogFields, color: bool) -> fmt::Result {
    for name in ["method", "path"] {
        if let Some(value) = fields.text(name) {
            let style = if name == "method" {
                style(ColorToken::Method)
            } else {
                Style::new()
            };
            write!(writer, " {}", paint(color, value, style))?;
        }
    }
    if let Some(value) = fields.u64("forwarded_request_bytes") {
        write_traffic(writer, color, TrafficDirection::Up, value)?;
    }
    if let Some(model) = fields.text("model") {
        if let Some(effort) = fields.text("reasoning_effort") {
            write!(writer, " {model}/{effort}")?;
        } else {
            write!(writer, " {model}")?;
        }
    }
    if fields.text("stream").as_deref() == Some("true") {
        write!(
            writer,
            " {}",
            paint(color, "stream", style(ColorToken::Flag))
        )?;
    }
    if let Some(max) = fields.u64("max_output_tokens") {
        write!(
            writer,
            " {}",
            paint(
                color,
                format!("max={}", format_count(max)),
                style(ColorToken::Token)
            )
        )?;
    }
    if let Some(params) = fields.text("request_hints") {
        for param in params.split_whitespace() {
            let token = if param.starts_with("tools[") {
                paint(color, param, style(ColorToken::Tool))
            } else {
                paint(color, param, style(ColorToken::Flag))
            };
            write!(writer, " {token}")?;
        }
    }
    if fields.text("capture").as_deref() == Some("true") {
        write!(writer, " capture")?;
    }
    Ok(())
}

fn format_headers(
    writer: &mut dyn fmt::Write,
    fields: &LogFields,
    color: bool,
    _duration_thresholds: &DurationThresholds,
) -> fmt::Result {
    if let Some(value) = fields.text("status") {
        write!(writer, " {}", paint(color, &value, status_style(&value)))?;
    }
    for name in ["ct", "te"] {
        if let Some(value) = fields.text(name) {
            write!(writer, " {value}")?;
        }
    }
    Ok(())
}

fn format_stream_end(
    writer: &mut dyn fmt::Write,
    fields: &LogFields,
    color: bool,
    duration_thresholds: &DurationThresholds,
) -> fmt::Result {
    if let Some(value) = fields.text("status") {
        write!(writer, " {}", paint(color, &value, status_style(&value)))?;
    }
    match (fields.u64("ttfb_ms"), fields.u64("duration_ms")) {
        (Some(ttfb), Some(duration)) => {
            write!(
                writer,
                " {}",
                paint(
                    color,
                    format!("lat[{}/{}]", format_millis(ttfb), format_millis(duration)),
                    duration_style(duration.max(ttfb), duration_thresholds)
                )
            )?;
        }
        (None, Some(duration)) => {
            write!(
                writer,
                " {}",
                paint(
                    color,
                    format!("dur={}", format_millis(duration)),
                    duration_style(duration, duration_thresholds)
                )
            )?;
        }
        (Some(ttfb), None) => {
            write!(
                writer,
                " {}",
                paint(
                    color,
                    format!("ttfb={}", format_millis(ttfb)),
                    duration_style(ttfb, duration_thresholds)
                )
            )?;
        }
        (None, None) => {}
    }
    if let Some(value) = fields.u64("forwarded_request_bytes") {
        write_traffic(writer, color, TrafficDirection::Up, value)?;
    }
    if let Some(value) = fields.u64("down") {
        write_traffic(writer, color, TrafficDirection::Down, value)?;
    }
    format_tokens(writer, fields, color)?;
    if let Some(model) = fields.text("model") {
        if let Some(effort) = fields.text("reasoning_effort") {
            write!(writer, " {model}/{effort}")?;
        } else {
            write!(writer, " {model}")?;
        }
    }
    if let Some(value) = fields.text("ct") {
        write!(writer, " {value}")?;
    }
    if let Some(chunks) = fields.u64("chunks") {
        let mut parts = vec![format_count(chunks)];
        if let Some(average) = fields.u64("avg_chunk_bytes").filter(|value| *value != 0) {
            parts.push(format_bytes(average));
        }
        write!(
            writer,
            " {}",
            paint(
                color,
                format!("stream[{}]", parts.join("/")),
                style(ColorToken::Flag)
            )
        )?;
    }
    format_response_summary(writer, fields, color)?;
    format_rate_limits(writer, fields, color)?;
    format_codex_limits(writer, fields, color)?;
    format_response_tail(writer, fields, color)
}

fn format_error(
    writer: &mut dyn fmt::Write,
    fields: &LogFields,
    color: bool,
    duration_thresholds: &DurationThresholds,
) -> fmt::Result {
    format_stream_end(writer, fields, color, duration_thresholds)?;
    if let Some(error) = fields.text("err") {
        write!(
            writer,
            " {}",
            paint(color, error, style(ColorToken::EventError))
        )?;
    }
    Ok(())
}

fn format_remaining_fields(
    writer: &mut dyn fmt::Write,
    fields: &LogFields,
    color: bool,
) -> fmt::Result {
    for (key, value) in &fields.fields {
        if key != "event" {
            write!(
                writer,
                " {}={}",
                paint(color, key, style(ColorToken::Meta)),
                value.trim_matches('"')
            )?;
        }
    }
    Ok(())
}

fn format_response_summary(
    writer: &mut dyn fmt::Write,
    fields: &LogFields,
    color: bool,
) -> fmt::Result {
    if let Some(value) = fields.text("response_status") {
        write!(
            writer,
            " {}",
            paint(
                color,
                format!("state={value}"),
                response_status_style(&value)
            )
        )?;
    }
    if let Some(value) = fields.text("service_tier") {
        write!(writer, " tier={value}")?;
    }
    if let Some(value) = fields.text("incomplete_reason") {
        write!(
            writer,
            " {}",
            paint(
                color,
                format!("inc={value}"),
                style(ColorToken::EventClosed)
            )
        )?;
    }
    if let Some(value) = fields.text("error_code") {
        write!(
            writer,
            " {}",
            paint(color, format!("err={value}"), style(ColorToken::EventError))
        )?;
    }
    if let Some(value) = fields.text("error_param").filter(|value| !value.is_empty()) {
        write!(
            writer,
            " {}",
            paint(
                color,
                format!("param={value}"),
                style(ColorToken::EventError)
            )
        )?;
    }
    Ok(())
}

fn format_response_tail(
    writer: &mut dyn fmt::Write,
    fields: &LogFields,
    color: bool,
) -> fmt::Result {
    if let Some(value) = fields.text("response_id") {
        write!(writer, " rid={value}")?;
    }
    if let Some(value) = fields.u64("seq") {
        write!(writer, " seq={value}")?;
    }
    if let Some(value) = fields.u64("timeout_ms") {
        write!(
            writer,
            " {}",
            paint(
                color,
                format!("timeout={}ms", value),
                style(ColorToken::EventError)
            )
        )?;
    }
    if let Some(value) = fields.text("diagnostic_path") {
        write!(writer, " diag={value}")?;
    }
    if let Some(value) = fields.text("output_items") {
        write!(writer, " out[{value}]")?;
    }
    if let Some(value) = fields.text("function_calls") {
        write!(
            writer,
            " {}",
            paint(color, format!("funcs[{value}]"), style(ColorToken::Tool))
        )?;
    }
    if let Some(value) = fields.text("mcp_calls") {
        write!(
            writer,
            " {}",
            paint(color, format!("mcp[{value}]"), style(ColorToken::Tool))
        )?;
    }
    Ok(())
}

fn format_tokens(writer: &mut dyn fmt::Write, fields: &LogFields, color: bool) -> fmt::Result {
    let token_values = [
        ("input", "↑"),
        ("output", "↓"),
        ("tok", "Σ"),
        ("cache", "$"),
        ("reasoning", "?"),
    ]
    .into_iter()
    .filter_map(|(field, label)| {
        fields
            .u64(field)
            .filter(|value| *value != 0)
            .map(|value| format!("{label}{}", format_count(value)))
    })
    .collect::<Vec<_>>();

    if token_values.is_empty() {
        Ok(())
    } else {
        write!(
            writer,
            " {}",
            paint(
                color,
                format!("tok[{}]", token_values.join(" ")),
                style(ColorToken::Token)
            )
        )
    }
}

fn format_rate_limits(writer: &mut dyn fmt::Write, fields: &LogFields, color: bool) -> fmt::Result {
    let mut parts = Vec::new();

    if let Some(value) = format_limit_pair(
        fields,
        "rate_limit_remaining_requests",
        "rate_limit_limit_requests",
    ) {
        parts.push(format!("r:{value}"));
    }
    if let Some(value) = format_limit_pair(
        fields,
        "rate_limit_remaining_tokens",
        "rate_limit_limit_tokens",
    ) {
        parts.push(format!("t:{value}"));
    }
    if let Some(ms) = fields.u64("rate_limit_reset_requests_ms") {
        parts.push(format!("rr:{}", format_millis(ms)));
    }
    if let Some(ms) = fields.u64("rate_limit_reset_tokens_ms") {
        parts.push(format!("rt:{}", format_millis(ms)));
    }

    if !parts.is_empty() {
        write!(
            writer,
            " {}",
            paint(
                color,
                format!("rl[{}]", parts.join(" ")),
                style(ColorToken::RateLimit)
            )
        )?;
    }
    Ok(())
}

fn format_codex_limits(
    writer: &mut dyn fmt::Write,
    fields: &LogFields,
    color: bool,
) -> fmt::Result {
    let mut parts = Vec::new();

    if let Some(value) = format_codex_window(
        fields,
        "p",
        "codex_primary_used_percent",
        "codex_primary_reset_after_secs",
        "codex_primary_window_minutes",
    ) {
        parts.push(value);
    }
    if let Some(value) = format_codex_window(
        fields,
        "s",
        "codex_secondary_used_percent",
        "codex_secondary_reset_after_secs",
        "codex_secondary_window_minutes",
    ) {
        parts.push(value);
    }
    if let Some(value) = fields.f64("codex_primary_over_secondary_percent") {
        parts.push(format!("p/s:{value:.1}%"));
    }

    if !parts.is_empty() {
        write!(
            writer,
            " {}",
            paint(
                color,
                format!("codex[{}]", parts.join(" ")),
                style(ColorToken::RateLimit)
            )
        )?;
    }
    Ok(())
}

fn format_codex_window(
    fields: &LogFields,
    label: &str,
    used_field: &str,
    reset_field: &str,
    window_field: &str,
) -> Option<String> {
    let used = fields.f64(used_field)?;
    let mut value = format!("{label}:{used:.1}%");
    if let Some(minutes) = fields.u64(window_field) {
        value.push('/');
        value.push_str(&format_minutes_as_duration(minutes));
    }
    if let Some(secs) = fields.u64(reset_field) {
        value.push('@');
        value.push_str(&format_seconds_as_duration(secs));
    }
    Some(value)
}

fn format_limit_pair(fields: &LogFields, remaining: &str, limit: &str) -> Option<String> {
    match (fields.u64(remaining), fields.u64(limit)) {
        (Some(remaining), Some(limit)) => Some(format!(
            "{}/{}",
            format_count(remaining),
            format_count(limit)
        )),
        (Some(remaining), None) => Some(format_count(remaining)),
        (None, Some(limit)) => Some(format!("-/{}", format_count(limit))),
        (None, None) => None,
    }
}

#[derive(Debug, Clone, Copy)]
enum TrafficDirection {
    Up,
    Down,
}

fn write_traffic(
    writer: &mut dyn fmt::Write,
    color: bool,
    direction: TrafficDirection,
    value: u64,
) -> fmt::Result {
    let (symbol, token) = match direction {
        TrafficDirection::Up => ("↑", ColorToken::TrafficUp),
        TrafficDirection::Down => ("↓", ColorToken::TrafficDown),
    };
    write!(
        writer,
        " {}",
        paint(
            color,
            format!("{symbol}{}", format_bytes(value)),
            style(token)
        )
    )
}

fn paint(value_color: bool, value: impl fmt::Display, style: Style) -> String {
    if value_color {
        value.style(style).to_string()
    } else {
        value.to_string()
    }
}

fn event_style(event: &str) -> Style {
    style(match event {
        "fwd" => ColorToken::EventForward,
        "hdr" => ColorToken::EventHeaders,
        "end" => ColorToken::EventEnd,
        "closed" => ColorToken::EventClosed,
        _ => ColorToken::EventError,
    })
}

fn level_style(level: &Level) -> Style {
    style(match *level {
        Level::ERROR => ColorToken::LevelError,
        Level::WARN => ColorToken::LevelWarn,
        Level::INFO => ColorToken::LevelInfo,
        Level::DEBUG => ColorToken::LevelDebug,
        Level::TRACE => ColorToken::LevelTrace,
    })
}

fn status_style(status: &str) -> Style {
    style(match status.parse::<u16>().ok() {
        Some(200..=299) => ColorToken::StatusOk,
        Some(300..=399) => ColorToken::StatusRedirect,
        Some(400..) => ColorToken::StatusError,
        _ => ColorToken::Meta,
    })
}

fn response_status_style(status: &str) -> Style {
    style(match status {
        "completed" => ColorToken::StatusOk,
        "incomplete" | "cancelled" => ColorToken::EventClosed,
        "failed" => ColorToken::StatusError,
        _ => ColorToken::Meta,
    })
}

fn duration_style(ms: u64, duration_thresholds: &DurationThresholds) -> Style {
    style(if ms >= duration_thresholds.error_ms {
        ColorToken::TimeSlow
    } else if ms >= duration_thresholds.warn_ms {
        ColorToken::TimeWarn
    } else {
        ColorToken::TimeFast
    })
}

fn style(token: ColorToken) -> Style {
    match token {
        ColorToken::LevelInfo => Style::new().green().bold(),
        ColorToken::LevelWarn => Style::new().yellow().bold(),
        ColorToken::LevelError => Style::new().red().bold(),
        ColorToken::LevelDebug => Style::new().blue(),
        ColorToken::LevelTrace => Style::new().dimmed(),
        ColorToken::EventForward => Style::new().cyan().bold(),
        ColorToken::EventHeaders => Style::new().blue().bold(),
        ColorToken::EventEnd => Style::new().green().bold(),
        ColorToken::EventClosed => Style::new().yellow().bold(),
        ColorToken::EventError => Style::new().red().bold(),
        ColorToken::Method => Style::new().green().bold(),
        ColorToken::StatusOk => Style::new().green().bold(),
        ColorToken::StatusRedirect => Style::new().yellow().bold(),
        ColorToken::StatusError => Style::new().red().bold(),
        ColorToken::TrafficUp => Style::new().blue().bold(),
        ColorToken::TrafficDown => Style::new().green().bold(),
        ColorToken::TimeFast => Style::new().green(),
        ColorToken::TimeWarn => Style::new().yellow().bold(),
        ColorToken::TimeSlow => Style::new().red().bold(),
        ColorToken::Token => Style::new().cyan(),
        ColorToken::Flag => Style::new().purple().bold(),
        ColorToken::Tool => Style::new().bright_blue().bold(),
        ColorToken::RateLimit => Style::new().yellow(),
        ColorToken::Meta => Style::new().dimmed(),
    }
}

#[cfg(test)]
#[path = "human_tests.rs"]
mod tests;
