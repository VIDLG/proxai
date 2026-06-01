use std::sync::Arc;

use crate::config::MatchKind;
use crate::error::{InternalError, Result};

use super::EffectiveRoute;

#[derive(Debug, Clone)]
pub(super) enum CompiledModelMatcher {
    Exact(String),
    Glob(Arc<globset::GlobMatcher>),
    Regex(Arc<regex::Regex>),
}

impl CompiledModelMatcher {
    pub(super) fn build(kind: MatchKind, pattern: &str) -> Result<Self, InternalError> {
        let effective_kind = infer_match_kind(kind, pattern);
        match effective_kind {
            MatchKind::Auto => unreachable!("auto should be resolved before compilation"),
            MatchKind::Exact => Ok(Self::Exact(pattern.to_string())),
            MatchKind::Glob => Ok(Self::Glob(Arc::new(
                globset::GlobBuilder::new(pattern)
                    .case_insensitive(true)
                    .backslash_escape(false)
                    .build()
                    .map_err(|error| InternalError::InvalidRoute(format!("{pattern}: {error}")))?
                    .compile_matcher(),
            ))),
            MatchKind::Regex => Ok(Self::Regex(Arc::new(
                regex::RegexBuilder::new(pattern)
                    .case_insensitive(true)
                    .build()
                    .map_err(|error| InternalError::InvalidRoute(format!("{pattern}: {error}")))?,
            ))),
        }
    }
}

impl EffectiveRoute {
    pub(super) fn match_model(&self, model: &str) -> Result<Option<String>, InternalError> {
        let matched = match &self.matcher {
            CompiledModelMatcher::Exact(pattern) => pattern.eq_ignore_ascii_case(model),
            CompiledModelMatcher::Glob(matcher) => matcher.is_match(model),
            CompiledModelMatcher::Regex(regex) => {
                if !regex.is_match(model) {
                    return Ok(None);
                }
                if let Some(template) = &self.upstream_model {
                    let rewritten = regex.replace(model, template.as_str()).to_string();
                    return Ok(Some(rewritten));
                }
                true
            }
        };

        if matched {
            Ok(Some(
                self.upstream_model
                    .clone()
                    .unwrap_or_else(|| model.to_string()),
            ))
        } else {
            Ok(None)
        }
    }
}

fn infer_match_kind(kind: MatchKind, pattern: &str) -> MatchKind {
    match kind {
        MatchKind::Auto => {
            if pattern.contains('*') || pattern.contains('?') {
                MatchKind::Glob
            } else if looks_like_regex(pattern) {
                MatchKind::Regex
            } else {
                MatchKind::Exact
            }
        }
        other => other,
    }
}

fn looks_like_regex(pattern: &str) -> bool {
    pattern.starts_with('^')
        || pattern.ends_with('$')
        || pattern.contains('(')
        || pattern.contains('[')
        || pattern.contains('|')
        || pattern.contains('+')
        || pattern.contains('\\')
}

#[cfg(test)]
#[path = "matcher_tests.rs"]
mod tests;
