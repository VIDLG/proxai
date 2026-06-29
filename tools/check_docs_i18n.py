#!/usr/bin/env python3
from __future__ import annotations

import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
SITE = ROOT / "site"
DOCS = SITE / "src" / "content" / "docs"
EN = DOCS / "en"
ZH = DOCS / "zh"
ASTRO_CONFIG = SITE / "astro.config.mjs"

OLD_PATTERNS = [
    r"site/app",
    r"docs/site",
    r"configuration-example",
    r"proxai-docs",
    r"vercel",
    r"\]\(/docs/en",
    r"\]\(/docs/zh",
    r"href=[\"']/docs/en",
    r"href=[\"']/docs/zh",
    r"nextra",
    r"next\.config",
    r"/protocols-overview",
    r"protocols-overview",
    r"/protocol-openai-responses",
    r"protocol-openai-responses",
    r"/protocol-openai-chat-completions",
    r"protocol-openai-chat-completions",
    r"/protocol-anthropic-messages",
    r"protocol-anthropic-messages",
    r"/en/(quick-start|recipes|configuration|routing-and-providers|observability|troubleshooting)([#\)\"'\s]|$)",
    r"/zh/(quick-start|recipes|configuration|routing-and-providers|observability|troubleshooting)([#\)\"'\s]|$)",
    r"/en/(architecture|protocol-conversion|streaming-internals|error-handling-internals)([#\)\"'\s]|$)",
    r"/zh/(architecture|protocol-conversion|streaming-internals|error-handling-internals)([#\)\"'\s]|$)",
    r"site/src/content/docs/zh/streaming-internals\.mdx",
    r"\]/error-handling\)",
    r"/error-handling\s",
    r"/error-handling$",
    r"/error-handling\.",
    r"/reference/conversion-pairs",
    r"/reference/protocol-values",
    r"/reference/error-response-payload",
    r"/reference/cli-flags",
]

def requires_noindex(rel: str) -> bool:
    return rel.startswith("developer/")


FENCED_CODE_RE = re.compile(r"(^|\n)```.*?(?=\n```)(?:\n```)", re.DOTALL)
FRONTMATTER_RE = re.compile(r"\A---\n(.*?)\n---", re.DOTALL)
MD_LINK_RE = re.compile(r"(?<!\!)\[[^\]]+\]\((/[^)\s#]+)(?:#[^)\s]+)?\)")
JSX_LINK_RE = re.compile(r"(?:href|to)=[\"'](/[^\"'#]+)(?:#[^\"']*)?[\"']")
MD_LINK_WITH_ANCHOR_RE = re.compile(r"(?<!\!)\[[^\]]+\]\((/[^)\s#]+)#([^)\s]+)\)")
JSX_LINK_WITH_ANCHOR_RE = re.compile(r"(?:href|to)=[\"'](/[^\"'#]+)#([^\"']+)[\"']")
HEADING_RE = re.compile(r"^\s{0,3}(#{1,6})\s+(.+?)\s*#*\s*$", re.MULTILINE)
FENCE_LINE_RE = re.compile(r"^\s*```([^`\n]*)\s*$")
SIDEBAR_SLUG_RE = re.compile(r"slug:\s*[\"']([^\"']+)[\"']")
CONTRACT_ID_RE = re.compile(r"id:\s*[\"']C(\d+)[\"']")
PROTOCOL_VALUES = {"openai_responses", "openai_chat_completions", "anthropic_messages"}
PHASE_VALUES = {"inbound_request", "provider_request", "upstream_response", "outbound_response"}
CONFIG_SECTION_RE = re.compile(r"^\s*(\[\[?)([^\]\n]+)(\]\]?)", re.MULTILINE)
ERROR_TYPES = {
    "invalid_request_error",
    "internal_error",
    "upstream_request_error",
    "upstream_error",
    "upstream_response_body_read_error",
    "upstream_error_body_empty",
    "upstream_error_body_non_json",
    "upstream_error_body_unknown_shape",
    "stream_translation_error",
}

errors: list[str] = []


def mdx_files(path: Path) -> set[str]:
    return {p.relative_to(path).as_posix() for p in path.rglob("*.mdx")}


def read_text(path: Path) -> str:
    return path.read_text(encoding="utf-8-sig")


def frontmatter_block(path: Path) -> str:
    match = FRONTMATTER_RE.match(read_text(path))
    return match.group(1) if match else ""


def frontmatter_has(path: Path, key: str) -> bool:
    fm = frontmatter_block(path)
    return bool(re.search(rf"^\s*{re.escape(key)}\s*:", fm, re.MULTILINE))


def without_code(text: str) -> str:
    return FENCED_CODE_RE.sub("\n", text)


def page_slugs(locale_dir: Path) -> set[str]:
    slugs: set[str] = set()
    for rel in mdx_files(locale_dir):
        if rel == "index.mdx":
            slugs.add("")
        elif rel.endswith("/index.mdx"):
            slugs.add(rel.removesuffix("/index.mdx"))
        else:
            slugs.add(rel.removesuffix(".mdx"))
    return slugs


def route_exists(path: str, en_slugs: set[str], zh_slugs: set[str]) -> bool:
    clean = path.strip("/")
    if clean in {"", "en", "zh"}:
        return True
    parts = clean.split("/", 1)
    if parts[0] == "en":
        return (parts[1] if len(parts) > 1 else "") in en_slugs
    if parts[0] == "zh":
        return (parts[1] if len(parts) > 1 else "") in zh_slugs
    # Starlight default locale routes are unprefixed English routes.
    return clean in en_slugs


def slugify_heading(text: str) -> str:
    text = re.sub(r"<[^>]+>", "", text)
    text = re.sub(r"[`*_~]", "", text).strip().lower()
    text = re.sub(r"[^\w\s\-\u4e00-\u9fff]", "", text, flags=re.UNICODE)
    text = re.sub(r"\s+", "-", text)
    text = re.sub(r"-+", "-", text).strip("-")
    return text


def anchors_for(path: Path) -> set[str]:
    text = without_code(read_text(path))
    anchors: set[str] = set()
    counts: dict[str, int] = {}
    for _, heading in HEADING_RE.findall(text):
        slug = slugify_heading(heading)
        if not slug:
            continue
        count = counts.get(slug, 0)
        counts[slug] = count + 1
        anchors.add(slug if count == 0 else f"{slug}-{count}")
    return anchors


def path_for_route(path: str, en_slugs: set[str], zh_slugs: set[str]) -> Path | None:
    clean = path.strip("/")
    if clean in {"", "en", "zh"}:
        locale_dir = EN if clean != "zh" else ZH
        return locale_dir / "index.mdx"
    parts = clean.split("/", 1)
    if parts[0] == "en":
        slug = parts[1] if len(parts) > 1 else ""
        locale_dir = EN
    elif parts[0] == "zh":
        slug = parts[1] if len(parts) > 1 else ""
        locale_dir = ZH
    else:
        slug = clean
        locale_dir = EN
    slugs = en_slugs if locale_dir == EN else zh_slugs
    if slug not in slugs:
        return None
    if slug == "":
        return locale_dir / "index.mdx"
    index_path = locale_dir / slug / "index.mdx"
    if index_path.exists():
        return index_path
    return locale_dir / f"{slug}.mdx"


def check_anchor_link(link_path: str, anchor: str, source: Path, en_slugs: set[str], zh_slugs: set[str]) -> None:
    target = path_for_route(link_path, en_slugs, zh_slugs)
    if target is None or not target.exists():
        return
    normalized_anchor = anchor.strip().lower()
    if normalized_anchor not in anchors_for(target):
        errors.append(
            f"broken internal anchor `{link_path}#{anchor}` in {source.relative_to(ROOT)}"
        )


def check_heading_quality(path: Path) -> None:
    rel = path.relative_to(ROOT)
    text = without_code(read_text(path))
    headings = [(len(level), heading) for level, heading in HEADING_RE.findall(text)]
    h1_count = sum(1 for level, _ in headings if level == 1)
    if h1_count != 1:
        errors.append(f"expected exactly one H1 in {rel}, found {h1_count}")

    previous = 0
    seen: set[str] = set()
    for level, heading in headings:
        if previous and level > previous + 1:
            errors.append(f"heading level jumps from H{previous} to H{level} in {rel}: {heading}")
        previous = level
        slug = slugify_heading(heading)
        if slug in seen:
            errors.append(f"duplicate heading slug `{slug}` in {rel}")
        seen.add(slug)


def check_hub_coverage(locale_dir: Path, locale: str) -> None:
    groups = ["using", "protocol", "developer", "reference"]
    for group in groups:
        group_dir = locale_dir / group
        index_path = group_dir / "index.mdx"
        if not group_dir.exists() or not index_path.exists():
            continue
        index_text = without_code(read_text(index_path))
        for page in sorted(group_dir.rglob("*.mdx")):
            rel = page.relative_to(group_dir).as_posix()
            if rel == "index.mdx":
                continue
            if rel.endswith("/index.mdx"):
                slug = rel.removesuffix("/index.mdx")
                # Nested topic index pages are represented by their directory route.
                expected = f"/{locale}/{group}/{slug}"
            else:
                slug = rel.removesuffix(".mdx")
                # Deep topic pages are covered by their own nested index pages.
                if group == "developer" and (slug.startswith("protocol-conversion/") or slug.startswith("architecture/")):
                    continue
                if group == "using" and slug.startswith("recipes/"):
                    continue
                expected = f"/{locale}/{group}/{slug}"
            if expected not in index_text:
                errors.append(f"missing hub link `{expected}` in {index_path.relative_to(ROOT)}")


def check_sidebar_slugs(en_slugs: set[str], zh_slugs: set[str]) -> None:
    text = read_text(ASTRO_CONFIG)
    for slug in SIDEBAR_SLUG_RE.findall(text):
        if slug not in en_slugs:
            errors.append(f"sidebar slug missing English page: {slug}")
        if slug not in zh_slugs:
            errors.append(f"sidebar slug missing Chinese page: {slug}")


def check_reference_value_coverage() -> None:
    protocol_text = read_text(EN / "reference" / "protocols.mdx") + read_text(ZH / "reference" / "protocols.mdx")
    phase_text = read_text(EN / "reference" / "capture-phases.mdx") + read_text(ZH / "reference" / "capture-phases.mdx")
    for value in sorted(PROTOCOL_VALUES):
        if value not in protocol_text:
            errors.append(f"protocol value `{value}` missing from reference/protocols")
    for value in sorted(PHASE_VALUES):
        if value not in phase_text:
            errors.append(f"phase value `{value}` missing from reference/capture-phases")


def normalize_config_section(section: str) -> str:
    if section.startswith("providers."):
        return "providers.<name>"
    return section


def check_config_section_coverage() -> None:
    example_sections = {
        normalize_config_section(section.strip())
        for _, section, _ in CONFIG_SECTION_RE.findall(read_text(ROOT / "config.example.toml"))
    }
    expected = {f"[{section}]" for section in sorted(example_sections)}
    expected.discard("[routing.routes]")
    expected.add("[[routing.routes]]")
    for locale, path in [("en", EN / "reference" / "configuration.mdx"), ("zh", ZH / "reference" / "configuration.mdx")]:
        text = read_text(path)
        if "config.example.toml" not in text:
            errors.append(f"{locale}/reference/configuration does not mention config.example.toml")
        for section in sorted(expected):
            if section not in text:
                errors.append(f"config section `{section}` missing from {locale}/reference/configuration")


def check_compatibility_matrix() -> None:
    pairs = {(inbound, provider) for inbound in PROTOCOL_VALUES for provider in PROTOCOL_VALUES}
    for locale, path in [("en", EN / "reference" / "compatibility-matrix.mdx"), ("zh", ZH / "reference" / "compatibility-matrix.mdx")]:
        text = read_text(path)
        for inbound, provider in sorted(pairs):
            if inbound not in text or provider not in text:
                errors.append(f"compatibility matrix {locale} missing protocol value in pair {inbound}->{provider}")
        for token in ["supported", "unsupported", "pass-through"]:
            if token not in text and {"supported": "支持", "unsupported": "不支持", "pass-through": "透传"}[token] not in text:
                errors.append(f"compatibility matrix {locale} missing `{token}` legend/value")


def check_error_type_coverage() -> None:
    for locale, path in [("en", EN / "reference" / "error-responses.mdx"), ("zh", ZH / "reference" / "error-responses.mdx")]:
        text = read_text(path)
        for error_type in sorted(ERROR_TYPES):
            if error_type not in text:
                errors.append(f"error type `{error_type}` missing from {locale}/reference/error-responses")


def check_contract_ids() -> None:
    for locale, path in [("en", EN / "reference" / "behavior-contracts.mdx"), ("zh", ZH / "reference" / "behavior-contracts.mdx")]:
        ids = [int(value) for value in CONTRACT_ID_RE.findall(read_text(path))]
        if not ids:
            errors.append(f"no behavior contract ids found in {locale}/reference/behavior-contracts")
            continue
        duplicates = sorted({value for value in ids if ids.count(value) > 1})
        if duplicates:
            errors.append(f"duplicate behavior contract ids in {locale}: {duplicates}")
        expected = list(range(1, max(ids) + 1))
        if sorted(ids) != expected:
            errors.append(f"behavior contract ids are not contiguous in {locale}: found C{min(ids)}..C{max(ids)} with gaps")


def check_fenced_code_languages(path: Path, text: str) -> None:
    rel = path.relative_to(ROOT)
    in_fence = False
    for line_no, line in enumerate(text.splitlines(), start=1):
        match = FENCE_LINE_RE.match(line)
        if not match:
            continue
        if in_fence:
            in_fence = False
            continue
        info = match.group(1).strip()
        if not info:
            errors.append(f"fenced code block missing language in {rel}:{line_no}")
        in_fence = True


def check_mdx_file(path: Path, en_slugs: set[str], zh_slugs: set[str]) -> None:
    rel = path.relative_to(ROOT)
    text = read_text(path)
    scan_text = without_code(text)

    if not frontmatter_has(path, "title"):
        errors.append(f"missing frontmatter title: {rel}")
    if not frontmatter_has(path, "description"):
        errors.append(f"missing frontmatter description: {rel}")

    for pattern in OLD_PATTERNS:
        if re.search(pattern, scan_text, re.IGNORECASE):
            errors.append(f"old docs reference `{pattern}` in {rel}")

    if re.search(r"\b(TODO:|TBD:)\b", scan_text):
        errors.append(f"unfinished placeholder TODO/TBD in {rel}")

    check_fenced_code_languages(path, text)

    for line_no, line in enumerate(scan_text.splitlines(), start=1):
        stripped = line.strip()
        if stripped.startswith("|") and stripped.endswith("|") and stripped.count("|") >= 2:
            errors.append(f"markdown table row remains in {rel}:{line_no}")

    for link in MD_LINK_RE.findall(scan_text) + JSX_LINK_RE.findall(scan_text):
        if link.startswith(("//", "/_", "/assets/")):
            continue
        if not route_exists(link, en_slugs, zh_slugs):
            errors.append(f"broken internal link `{link}` in {rel}")

    for link, anchor in MD_LINK_WITH_ANCHOR_RE.findall(scan_text) + JSX_LINK_WITH_ANCHOR_RE.findall(scan_text):
        if link.startswith(("//", "/_", "/assets/")):
            continue
        check_anchor_link(link, anchor, path, en_slugs, zh_slugs)

    locale_root = EN if "site/src/content/docs/en" in path.as_posix() else ZH
    page_rel = path.relative_to(locale_root).as_posix()
    fm = frontmatter_block(path)
    if requires_noindex(page_rel) and "robots: noindex" not in fm:
        errors.append(f"missing `robots: noindex` in internals page: {rel}")

    if page_rel.startswith("using/") and page_rel != "using/index.mdx" and "/reference/" not in scan_text:
        errors.append(f"using page does not link to reference docs: {rel}")

    if page_rel.startswith("developer/architecture/"):
        if not re.search(r"\b(src/|config\.example\.toml|AGENTS\.md|AppState)\b", scan_text):
            errors.append(f"developer architecture page lacks source/file reference: {rel}")


def check_text_file(path: Path) -> None:
    if not path.exists():
        return
    rel = path.relative_to(ROOT)
    scan_text = without_code(read_text(path))
    for pattern in OLD_PATTERNS:
        if re.search(pattern, scan_text, re.IGNORECASE):
            errors.append(f"old docs reference `{pattern}` in {rel}")


def heading_levels(path: Path) -> list[int]:
    text = without_code(read_text(path))
    return [len(level) for level, _ in HEADING_RE.findall(text)]


def check_heading_parity(en_files: set[str], zh_files: set[str]) -> None:
    for rel in sorted(en_files & zh_files):
        en_levels = heading_levels(EN / rel)
        zh_levels = heading_levels(ZH / rel)
        if en_levels != zh_levels:
            errors.append(f"heading structure differs between en/{rel} and zh/{rel}: {en_levels} != {zh_levels}")


def main() -> int:
    en = mdx_files(EN)
    zh = mdx_files(ZH)
    en_slugs = page_slugs(EN)
    zh_slugs = page_slugs(ZH)

    for name in sorted(en - zh):
        errors.append(f"missing zh pair for en/{name}")
    for name in sorted(zh - en):
        errors.append(f"missing en pair for zh/{name}")

    check_sidebar_slugs(en_slugs, zh_slugs)

    for path in sorted(DOCS.rglob("*.mdx")):
        check_mdx_file(path, en_slugs, zh_slugs)
        check_heading_quality(path)

    check_hub_coverage(EN, "en")
    check_hub_coverage(ZH, "zh")
    check_reference_value_coverage()
    check_config_section_coverage()
    check_compatibility_matrix()
    check_error_type_coverage()
    check_contract_ids()

    for path in [ROOT / "AGENTS.md", ROOT / "README.md", ROOT / "README_CN.md", SITE / "README.md"]:
        check_text_file(path)

    if errors:
        for error in errors:
            print(f"docs-check: {error}", file=sys.stderr)
        return 1

    print("docs-check: ok")
    return 0


if __name__ == "__main__":
    sys.exit(main())
