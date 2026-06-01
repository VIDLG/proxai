import json
import sys

from .checks import (
    comment_shape_diffs,
    enum_literal_diffs,
    field_suppress_diffs,
    proxai_internal_diffs,
    required_nullable_accepts_missing_diffs,
    serde_field_diffs,
    serde_wire_diffs,
    untagged_union_diffs,
)
from .common import PROTO_DIR, SDK_FILE, SDK_PKG, h2, hr, norm, out
from .rust import px_enum_variants, px_types, rust_sdk_markers
from .sdk import sdk_tool_union, sdk_types


def run_report(level=2, only_marked=False):
    sdk_ver = ""
    if SDK_PKG.exists():
        try:
            sdk_ver = json.loads(SDK_PKG.read_text()).get("version", "")
        except Exception:
            pass

    text = SDK_FILE.read_text(encoding="utf-8")
    sdk_raw = sdk_types(text)
    px = px_types()
    comment_diffs = comment_shape_diffs(text, only_marked=only_marked)
    serde_diffs = serde_wire_diffs(text, only_marked=only_marked)
    serde_field_semantic_diffs = serde_field_diffs(text, only_marked=only_marked)
    required_nullable_missing_diffs, required_nullable_missing_marked = (
        required_nullable_accepts_missing_diffs(text, only_marked=only_marked)
    )
    field_suppress_marker_diffs = field_suppress_diffs(text, only_marked=only_marked)
    enum_semantic_diffs = enum_literal_diffs(text)
    union_semantic_diffs = untagged_union_diffs(text, only_marked=only_marked)
    proxai_only_diffs = proxai_internal_diffs(text, only_marked=only_marked)
    sdk_markers = rust_sdk_markers()
    naming_map = sdk_markers["aliases"]
    external_types = sdk_markers["externals"]
    skip_types = sdk_markers["internals"]
    union_variant_map = sdk_markers["union_variants"]

    sn = {}
    for tag, info in sdk_raw.items():
        base = tag.split(".")[-1]
        sn[norm(base)] = (base, info, tag)

    pn = {}
    for name, info in px.items():
        pn[norm(name)] = (name, info)

    sk = set(sn)
    pk = set(pn)
    matched = sk & pk

    reverse_alias = {}
    for tag, info in sdk_raw.items():
        alias_of = info.get("alias_of")
        if alias_of:
            reverse_alias.setdefault(norm(alias_of), []).append(
                info.get("alias_name", "")
            )

    covered = set()
    for nk in sorted(sk - pk):
        base, info, tag = sn[nk]
        if (
            "." in tag
            or info["kind"] == "class"
            or base in external_types
            or base in skip_types
        ):
            continue
        alias_names = reverse_alias.get(nk, [])
        for alias_name in alias_names:
            if alias_name and norm(alias_name) in pk:
                covered.add(nk)
                break
        else:
            alias_of = info.get("alias_of")
            if alias_of and norm(alias_of) in pk:
                covered.add(nk)
            elif base.startswith("Raw") and len(base) > 3 and norm(base[3:]) in pk:
                covered.add(nk)
            elif base in naming_map and norm(naming_map[base]) in pk:
                covered.add(nk)

    missing_schema = []
    namespaced = []
    external = []
    api_class = []
    aliased = []
    skipped = []

    for nk in sorted(sk - pk):
        base, info, tag = sn[nk]
        if info["kind"] == "class":
            api_class.append((base, info, tag))
        elif base in external_types:
            external.append((base, info, tag))
        elif base in skip_types:
            skipped.append((base, info, tag))
        elif "." in tag:
            parent = tag.rsplit(".", 1)[0]
            parent_base = parent.split(".")[-1]
            parent_nk = norm(parent_base)
            if parent_nk in pk or parent_nk in covered:
                namespaced.append((base, info, tag, parent_base))
            else:
                missing_schema.append((base, info, tag))
        elif nk in covered:
            matched_name = ""
            for alias_name in reverse_alias.get(nk, []):
                if alias_name and norm(alias_name) in pk:
                    matched_name = alias_name
                    break
            if not matched_name and base.startswith("Raw") and len(base) > 3:
                matched_name = base[3:]
            if not matched_name and base in naming_map:
                matched_name = naming_map[base]
            if not matched_name:
                alias_of = info.get("alias_of", "")
                if alias_of and norm(alias_of) in pk:
                    matched_name = alias_of
            aliased.append((base, info, tag, matched_name))
        else:
            missing_schema.append((base, info, tag))

    struct_diffs, matched_ok = _structural_diffs(matched, sn, pn, sdk_markers)
    sdk_tu = sdk_tool_union(text)
    px_tu_raw = px_enum_variants("tools/mod.rs", "ToolUnion")
    tu_missing = _missing_tool_union_variants(sdk_tu, px_tu_raw, union_variant_map)

    has_missing_fields = any(missing for _, _, missing, _, _ in struct_diffs)
    has_gaps = bool(
        missing_schema
        or tu_missing
        or has_missing_fields
        or comment_diffs
        or serde_diffs
        or serde_field_semantic_diffs
        or required_nullable_missing_diffs
        or field_suppress_marker_diffs
        or enum_semantic_diffs
        or union_semantic_diffs
        or proxai_only_diffs
    )

    hr()
    out(f"  Anthropic Messages Protocol  vs  SDK {sdk_ver}")
    out(f"  SDK:  {SDK_FILE}")
    out(f"  Ours: {PROTO_DIR}/")
    out()
    px_only = len(px) - len(matched)
    out(f"  SDK types: {len(sdk_raw)}  |  Ours: {len(px)}  |  Matched: {len(matched)}")
    if level >= 2:
        other = len(sk - pk)
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
            "Serde wire semantics",
            "Serde discriminator handling matches SDK shape comments",
            serde_diffs,
            level,
            drift_word="drift",
        )
        _print_diff_section(
            "Serde field semantics",
            "Serde field names and optional/null semantics match SDK shape comments",
            serde_field_semantic_diffs,
            level,
            drift_word="drift",
        )
        _print_diff_section(
            "Required-nullable Option markers",
            "Required-nullable Option fields are explicitly marked",
            required_nullable_missing_diffs,
            level,
            drift_word="missing marker",
        )
        _print_required_nullable_accepts_missing_section(
            required_nullable_missing_marked
        )
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
            sys.exit(1)
        out("\n  ✅  Anthropic protocol coverage complete — no gaps")
        hr()
        out()


def _norm_field(name):
    value = name.lower().replace("_", "").replace("-", "").replace("#", "").rstrip(";")
    if value.startswith("r"):
        value = value.lstrip("r")
    return value


def _structural_diffs(matched, sn, pn, sdk_markers):
    struct_diffs = []
    matched_ok = 0
    field_suppressed = sdk_markers.get("field_suppressed", {})
    for nk in sorted(matched):
        base_sdk, sdk_info, _tag = sn[nk]
        name_px, px_info = pn[nk]
        if sdk_info["kind"] != "interface" or px_info["kind"] != "struct":
            matched_ok += 1
            continue

        sdk_fields = sdk_info.get("fields", [])
        px_fields = px_info.get("fields", [])
        sdk_norm = {_norm_field(f): f for f in sdk_fields}
        px_norm = {_norm_field(f): f for f in px_fields}
        sdk_nk = set(sdk_norm)
        px_nk = set(px_norm)
        exclusions = {
            _norm_field(f)
            for f in sdk_info.get("deprecated_fields", set())
            | px_info.get("deprecated_fields", set())
        }
        missing_f = [sdk_norm[nk] for nk in sorted(sdk_nk - px_nk - exclusions)]
        extra_f = [px_norm[nk] for nk in sorted(px_nk - sdk_nk)]

        common_sdk = []
        common_px = []
        for common_nk in sorted(sdk_nk & px_nk):
            common_sdk.extend(f for f in sdk_fields if _norm_field(f) == common_nk)
            common_px.extend(f for f in px_fields if _norm_field(f) == common_nk)
        sdk_order = [_norm_field(f) for f in common_sdk]
        px_order = [_norm_field(f) for f in common_px]
        order_mismatch = (
            (common_sdk, common_px)
            if common_sdk and common_px and sdk_order != px_order
            else None
        )

        suppressed = {_norm_field(f) for f in field_suppressed.get(name_px, set())}
        missing_f = [
            f
            for f in missing_f
            if not _is_structural_discriminator_gap(f)
            and _norm_field(f) not in suppressed
        ]
        extra_f = [f for f in extra_f if _norm_field(f) not in suppressed]

        if missing_f or extra_f or order_mismatch:
            struct_diffs.append((base_sdk, name_px, missing_f, extra_f, order_mismatch))
        else:
            matched_ok += 1
    return struct_diffs, matched_ok


def _is_structural_discriminator_gap(field):
    return _norm_field(field) == "type"


def _missing_tool_union_variants(sdk_tu, px_tu_raw, union_variant_map):
    tu_sdk_n = {norm(v): v for v in sdk_tu}
    tu_px_n = {norm(v): v for v in px_tu_raw}
    tu_missing = []
    for key in sorted(set(tu_sdk_n) - set(tu_px_n)):
        variant = tu_sdk_n[key]
        if variant in union_variant_map and norm(union_variant_map[variant]) in tu_px_n:
            continue
        tu_missing.append(variant)
    return tu_missing


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


def _print_required_nullable_accepts_missing_section(marked):
    if not marked:
        return
    h2(f"Required-nullable fields accepting missing ({len(marked)})")
    for item, field, file, line in marked:
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
