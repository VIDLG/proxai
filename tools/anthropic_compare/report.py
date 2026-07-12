"""Human-readable rendering for Anthropic protocol comparison results."""

from .common import PROTO_DIR, SDK_FILE, h2, hr, norm, out


def render_report(result, level=2):
    sdk_ver = result.sdk_version
    sdk_raw = result.sdk_types
    px = result.rust_types
    matched = result.matched
    sn = result.sdk_index
    sk_minus_pk_count = result.sdk_only_count
    missing_schema = result.missing_types
    namespaced = result.namespaced_types
    external = result.external_types
    api_class = result.api_classes
    aliased = result.aliases
    skipped = result.skipped_types
    struct_diffs = result.structural_diffs
    has_missing_fields = result.has_missing_fields
    sdk_tu = result.sdk_tool_union
    px_tu_raw = result.rust_tool_union
    tu_missing = result.missing_tool_variants
    comment_diffs = result.comment_diffs
    provenance_diffs = result.provenance_diffs
    serde_diffs = result.serde_diffs
    serde_field_semantic_diffs = result.serde_field_diffs
    field_carrier_semantic_diffs = result.field_carrier_diffs
    required_nullable_modeled = result.required_nullable_fields
    field_suppress_marker_diffs = result.field_suppress_diffs
    enum_semantic_diffs = result.enum_diffs
    union_semantic_diffs = result.union_diffs
    proxai_only_diffs = result.proxai_only_diffs
    has_gaps = result.has_gaps
    hr()
    out(f"  Anthropic Messages Protocol  vs  SDK {sdk_ver}")
    out(f"  SDK:  {SDK_FILE}")
    out(f"  Ours: {PROTO_DIR}/")
    out()
    px_only = len(px) - len(matched)
    out(f"  SDK types: {len(sdk_raw)}  |  Ours: {len(px)}  |  Matched: {len(matched)}")
    if level >= 2:
        other = sk_minus_pk_count
        out(
            f"  (SDK {len(sdk_raw)} = matched {len(matched)} + namespaced/class/external {other})"
        )
        out(f"  (Ours {len(px)}  = matched {len(matched)} + proxai-internal {px_only})")
    out()

    if level >= 2:
        _print_schema_sections(missing_schema, tu_missing, aliased, skipped, level)
        _print_structural_section(struct_diffs, has_missing_fields, level)
        _print_diff_section(
            "SDK doc annotations",
            "SDK doc annotations are structured and valid",
            comment_diffs,
            level,
            drift_word="drift",
        )
        _print_diff_section(
            "SDK provenance",
            "Every public wire type has explicit Anthropic SDK provenance",
            provenance_diffs,
            level,
            drift_word="missing provenance",
        )
        _print_diff_section(
            "Serde wire semantics",
            "Serde discriminator handling matches SDK shape comments",
            serde_diffs,
            level,
            drift_word="drift",
        )
        _print_diff_section(
            "Serde field structure",
            "Serde field names and structured unions match SDK shape comments",
            serde_field_semantic_diffs,
            level,
            drift_word="drift",
        )
        _print_diff_section(
            "Field carrier semantics",
            "Required, optional, and required-nullable fields use the enforced carriers",
            field_carrier_semantic_diffs,
            level,
            drift_word="carrier mismatch",
        )
        _print_required_nullable_section(required_nullable_modeled, level)
        _print_diff_section(
            "Field suppress markers",
            "Field suppress markers correspond to real SDK/Rust shape differences",
            field_suppress_marker_diffs,
            level,
            drift_word="stale",
        )
        _print_diff_section(
            "Enum literal semantics",
            "Enum literals match SDK string literal unions",
            enum_semantic_diffs,
            level,
            drift_word="drift",
        )
        _print_diff_section(
            "Untagged union semantics",
            "Untagged union payloads match SDK union aliases",
            union_semantic_diffs,
            level,
            drift_word="drift",
        )
        _print_diff_section(
            "Proxai-only classification",
            "Proxai-only types carry structured internal classification",
            proxai_only_diffs,
            level,
            drift_word="missing",
        )
        h2("ToolUnion")
        out(f"  SDK: {len(sdk_tu)}  |  Ours: {len(px_tu_raw)}")
        out()
        _print_informational_sections(namespaced, api_class, external, matched, sn)

    if level >= 3:
        _print_proxai_only_types(px, sdk_raw)

    if level >= 1:
        hr()
        if has_gaps:
            out("\n  ⚠  Gaps found — see sections above")
        else:
            out("\n  ✅  Anthropic protocol coverage complete — no gaps")
        hr()
        out()




def _print_schema_sections(missing_schema, tu_missing, aliased, skipped, level):
    if missing_schema:
        h2(f"MISSING ({len(missing_schema)}): schema types not found in proxai")
        for i, (base, info, _tag) in enumerate(missing_schema, 1):
            out(f"  {i:3d}. ✗ {base:<35s} @ messages.ts:{info['line']}")
    if tu_missing:
        h2("MISSING: ToolUnion variants (SDK has, we don't)")
        for variant in tu_missing:
            out(f"  ✗ {variant}")
    if not missing_schema and not tu_missing:
        out("  ✅  Type coverage — no missing types")
    out()

    if aliased:
        h2(f"SDK type aliases ({len(aliased)}) — wrapped types already matched")
        for base, info, _tag, alias_of in aliased:
            out(f"  ~ {base:<35s} = {alias_of:<35s} @ messages.ts:{info['line']}")

    if skipped:
        h2(f"SDK-internal types ({len(skipped)}) — intentionally not mirrored")
        for base, info, _tag in skipped:
            out(f"  - {base:<35s} @ messages.ts:{info['line']}")


def _print_structural_section(struct_diffs, has_missing_fields, level):
    if struct_diffs:
        only_extra = (
            all(not missing for _, _, missing, _, _ in struct_diffs)
            and not has_missing_fields
        )
        title = "Structural alignment — proxai has extra fields (enrichments, not gaps)"
        if not only_extra:
            title = "Structural alignment (field-level)"
        h2(title)
        for sdk_name, px_name, missing_f, extra_f, order_mismatch in struct_diffs:
            has_real_gap = bool(missing_f)
            if has_real_gap or level >= 3:
                out(f"      {px_name} → {sdk_name}")
            if missing_f:
                out(f"        ✗ Missing fields:  {', '.join(missing_f)}")
            if level >= 3 and extra_f:
                out(f"        + Extra fields:    {', '.join(extra_f)}")
            if level >= 3 and order_mismatch:
                sdk_order, px_order = order_mismatch
                out("        Order mismatch:")
                out(f"          SDK: {', '.join(sdk_order)}")
                out(f"          Ours: {', '.join(px_order)}")
    else:
        out("  ✅  All struct fields and order match")
    out()


def _print_diff_section(title, ok_message, diffs, level, drift_word):
    if diffs:
        h2(f"{title} ({len(diffs)} {drift_word})")
        for name, where, details in diffs:
            out(f"  ✗ {name:<35s} @ {where}")
            if level >= 3:
                for diff in details:
                    out(f"      - {diff}")
    else:
        out(f"  ✅  {ok_message}")
    out()


def _print_required_nullable_section(modeled, level):
    if not modeled:
        return
    if level < 3:
        out(f"  Required-nullable carriers modeled: {len(modeled)}")
        out()
        return
    h2(f"Explicit required-nullable carriers ({len(modeled)})")
    for item, field, file, line in modeled:
        out(f"  ~ {item}.{field} @ {file}:{line}")
    out()


def _print_informational_sections(namespaced, api_class, external, matched, sn):
    if namespaced:
        h2("Namespaced SDK types")
        for base, info, tag, parent in namespaced:
            out(f"  ~ {tag:<35s} @ messages.ts:{info['line']}  (member of {parent})")
    if api_class:
        h2("SDK resource classes")
        for base, info, _tag in api_class:
            out(f"  ~ {base:<30s} @ messages.ts:{info['line']}  (class)")
    if external:
        h2("External re-exports")
        for base, info, _tag in external:
            out(f"  ~ {base:<30s} @ messages.ts:{info['line']}")

    total_skipped = 0
    for nk in matched:
        _base_sdk, sdk_info, _ = sn[nk]
        total_skipped += len(sdk_info.get("deprecated_fields", set()))
    if total_skipped:
        out(f"  (skipped {total_skipped} deprecated SDK fields)")


def _print_proxai_only_types(px, sdk_raw):
    sk_base = {norm(tag.split(".")[-1]) for tag in sdk_raw}
    px_only_names = [name for name in sorted(px, key=norm) if norm(name) not in sk_base]
    if not px_only_names:
        return

    h2(f"Proxai-only types ({len(px_only_names)} — classification)")
    for i, name in enumerate(px_only_names, 1):
        info = px.get(name, {})
        kind = info.get("kind", "?")
        file = info.get("file", "?")
        tag = _classify_proxai_only_type(info, kind, file)
        tag_suffix = f" [{tag}]" if tag != kind else ""
        out(
            f"  {i:3d}. {kind:<6s} {name:<45s}{tag_suffix:<12s} @ {file}:{info.get('line', '?')}"
        )
    out()


def _classify_proxai_only_type(info, kind, file):
    tag = "other"
    try:
        p = PROTO_DIR.parent.parent.parent / file
        if not p.exists():
            return tag
        src = p.read_text(encoding="utf-8", errors="replace")
        lines = src.split("\n")
        line = info.get("line", 1)
        ctx = lines[line : line + 5] if line < len(lines) else []
        ctx_text = "\n".join(ctx)[:200]
        if "impl From" in ctx_text and "serde_json" in ctx_text:
            return "manual From"
        if src.split("\n")[line - 1].strip().startswith("pub type"):
            return "alias"
        if kind == "enum":
            return "enum"
        if "pub struct" in ctx_text and "Deserialize" in src:
            return "helper"
        return "struct"
    except Exception:
        return tag
