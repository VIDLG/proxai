use std::sync::Arc;

use super::{CompiledRoute, ModelMatchKind, Result, RoutingError};

#[derive(Debug, Clone)]
pub(super) enum CompiledModelMatcher {
    Exact(String),
    Glob(Arc<globset::GlobMatcher>),
    Regex(Arc<regex::Regex>),
}

impl CompiledModelMatcher {
    pub(super) fn build(index: usize, kind: ModelMatchKind, pattern: &str) -> Result<Self> {
        match infer_match_kind(kind, pattern) {
            ModelMatchKind::Auto => unreachable!("auto should be resolved before compilation"),
            ModelMatchKind::Exact => Ok(Self::Exact(pattern.to_string())),
            ModelMatchKind::Glob => Ok(Self::Glob(Arc::new(
                globset::GlobBuilder::new(pattern)
                    .case_insensitive(true)
                    .backslash_escape(false)
                    .build()
                    .map_err(|source| RoutingError::InvalidGlob {
                        index,
                        pattern: pattern.to_string(),
                        source,
                    })?
                    .compile_matcher(),
            ))),
            ModelMatchKind::Regex => Ok(Self::Regex(Arc::new(
                regex::RegexBuilder::new(pattern)
                    .case_insensitive(true)
                    .build()
                    .map_err(|source| RoutingError::InvalidRegex {
                        index,
                        pattern: pattern.to_string(),
                        source,
                    })?,
            ))),
        }
    }
}

impl CompiledRoute {
    pub(super) fn match_model(&self, model: &str) -> Option<String> {
        let matched = match &self.matcher {
            CompiledModelMatcher::Exact(pattern) => pattern.eq_ignore_ascii_case(model),
            CompiledModelMatcher::Glob(matcher) => matcher.is_match(model),
            CompiledModelMatcher::Regex(regex) => {
                if !regex.is_match(model) {
                    return None;
                }
                if let Some(template) = &self.upstream_model {
                    return Some(regex.replace(model, template.as_str()).to_string());
                }
                true
            }
        };

        matched.then(|| {
            self.upstream_model
                .clone()
                .unwrap_or_else(|| model.to_string())
        })
    }
}

fn infer_match_kind(kind: ModelMatchKind, pattern: &str) -> ModelMatchKind {
    match kind {
        ModelMatchKind::Auto => {
            if looks_like_regex(pattern) {
                ModelMatchKind::Regex
            } else if pattern.contains('*') || pattern.contains('?') {
                ModelMatchKind::Glob
            } else {
                ModelMatchKind::Exact
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
