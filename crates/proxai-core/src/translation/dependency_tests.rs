use std::fs;
use std::path::{Path, PathBuf};

const FORBIDDEN_DEPENDENCIES: &[&str] = &[
    "axum::",
    "http::",
    "reqwest::",
    "tracing::",
    "crate::config",
    "crate::http_support",
    "crate::ingress",
    "crate::mcp",
    "crate::pipeline",
    "crate::provider",
    "crate::request",
    "crate::routing",
    "crate::sse",
    "crate::upstream",
];

#[test]
fn translation_sources_do_not_depend_on_application_layers() {
    let translation_dir = translation_dir();
    let mut violations = Vec::new();
    inspect_directory(&translation_dir, &mut violations);

    assert!(
        violations.is_empty(),
        "translation core has application-layer dependencies:\n{}",
        violations.join("\n")
    );
}

#[test]
fn stream_primitives_do_not_depend_on_protocol_pairs_or_translator() {
    let source = fs::read_to_string(translation_dir().join("stream.rs")).unwrap();
    for forbidden in [
        "crate::protocol",
        "crate::translation::anthropic_messages",
        "crate::translation::openai_chat_completions",
        "crate::translation::openai_responses",
        "crate::translation::translator",
    ] {
        assert!(
            !source.contains(forbidden),
            "translation/stream.rs must remain dependency-free but contains `{forbidden}`"
        );
    }
}

#[test]
fn translation_context_does_not_depend_on_pairs_or_facade() {
    let source = fs::read_to_string(translation_dir().join("context.rs")).unwrap();
    for forbidden in [
        "crate::translation::anthropic_messages",
        "crate::translation::openai_chat_completions",
        "crate::translation::openai_responses",
        "crate::translation::translator",
    ] {
        assert!(
            !source.contains(forbidden),
            "translation/context.rs contains upper-layer dependency `{forbidden}`"
        );
    }
}

#[test]
fn dispatchers_do_not_depend_on_translator_facade() {
    for file in ["request.rs", "response.rs"] {
        let source = fs::read_to_string(translation_dir().join(file)).unwrap();
        assert!(
            !source.contains("Translator"),
            "translation/{file} must receive TranslationScope rather than the Translator façade"
        );
    }
}

#[test]
fn protocol_pairs_do_not_depend_on_translator_facade() {
    for protocol in [
        "anthropic_messages",
        "openai_chat_completions",
        "openai_responses",
    ] {
        let directory = translation_dir().join(protocol);
        for path in rust_sources(&directory)
            .into_iter()
            .filter(|path| !is_test_rust_source(path))
        {
            let source = fs::read_to_string(&path).unwrap();
            let forbidden = "crate::translation::translator";
            assert!(
                !source.contains(forbidden),
                "{} contains forbidden upper-layer dependency `{forbidden}`",
                path.strip_prefix(env!("CARGO_MANIFEST_DIR"))
                    .unwrap()
                    .display()
            );
        }
    }
}

fn translation_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("src/translation")
}

fn rust_sources(directory: &Path) -> Vec<PathBuf> {
    let mut sources = Vec::new();
    collect_rust_sources(directory, &mut sources);
    sources
}

fn collect_rust_sources(directory: &Path, sources: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(directory).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            collect_rust_sources(&path, sources);
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            sources.push(path);
        }
    }
}

fn inspect_directory(directory: &Path, violations: &mut Vec<String>) {
    for path in rust_sources(directory)
        .into_iter()
        .filter(|path| is_translation_rust_source(path))
    {
        inspect_file(&path, violations);
    }
}

fn is_translation_rust_source(path: &Path) -> bool {
    path.extension().is_some_and(|extension| extension == "rs")
        && path
            .file_name()
            .is_some_and(|name| name != "dependency_tests.rs")
}

fn is_test_rust_source(path: &Path) -> bool {
    path.file_name().is_some_and(|name| {
        let name = name.to_string_lossy();
        name == "tests.rs" || name.ends_with("_tests.rs")
    })
}

fn inspect_file(path: &Path, violations: &mut Vec<String>) {
    let source = fs::read_to_string(path).unwrap();
    for dependency in FORBIDDEN_DEPENDENCIES {
        for (index, line) in source.lines().enumerate() {
            if line.contains(dependency) {
                violations.push(format!(
                    "{}:{} contains `{dependency}`",
                    path.strip_prefix(env!("CARGO_MANIFEST_DIR"))
                        .unwrap()
                        .display(),
                    index + 1
                ));
            }
        }
    }
}
