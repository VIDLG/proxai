# /// script
# requires-python = ">=3.12"
# dependencies = [
#   "tree-sitter",
#   "tree-sitter-rust",
# ]
# ///

"""
Compare proxai OpenAI protocol types against the vendored async-openai Rust crate.

Uses tree-sitter AST parsing. Run from project root:
  pixi run python tools/compare_openai_protocol.py
  pixi run python tools/compare_openai_protocol.py responses    # only Responses API
  pixi run python tools/compare_openai_protocol.py chat         # only Chat Completions

Comparison levels:
  1. Type name alignment: every SDK type has a matching proxai type.
  2. Structural alignment: matched local protocol structs/enums are checked
     against the SDK shape — struct fields match, enum variants match.
"""

import json
import re
import sys
from pathlib import Path

import tree_sitter
import tree_sitter_rust as ts_rust

# ── Paths ─────────────────────────────────────────────────────
SDK_DIR = Path("contrib/async-openai/async-openai/src/types")
PROTO_DIR = Path("src/protocol/openai")
SDK_PKG = Path("contrib/async-openai/async-openai/Cargo.toml")


# ── Manual mappings ───────────────────────────────────────────
# Intentional exclusions: SDK types proxai doesn't implement.
# Each entry maps type_name → category shown in -d output.
# Categories are short labels; details are in comments above each group.
INTENTIONAL_EXCLUSIONS = {
    # ── Conversation CRUD ──
    # Types for the conversation management endpoints (GET/POST/DELETE
    # /v1/responses/{id}/conversation*).  proxai handles items inline via
    # InputItem/OutputItem and has no separate conversation RUD endpoint.
    "ComputerScreenContent": "conversation CRUD",
    "ConversationItem": "conversation CRUD",
    "ConversationItemList": "conversation CRUD",
    "ConversationResource": "conversation CRUD",
    "ConversationParam": "conversation CRUD",
    "CreateConversationItemsRequest": "conversation CRUD",
    "CreateConversationRequest": "conversation CRUD",
    "DeleteConversationResponse": "conversation CRUD",
    "UpdateConversationRequest": "conversation CRUD",
    "ListOrder": "conversation CRUD",
    "Message": "conversation CRUD",
    "MessageContent": "conversation CRUD",
    "MessageRole": "conversation CRUD",
    "MessageStatus": "conversation CRUD",
    "TextContent": "conversation CRUD",
    # ── Item list endpoint ──
    # Types for GET /v1/responses/{id}/items.  proxai doesn't expose item
    # list/query operations — items are returned inline in the Response.
    "AnyItemReference": "item list API",
    "ItemResource": "item list API",
    "ItemResourceItem": "item list API",
    "ResponseItemList": "item list API",
    # ── Compact API ──
    # Request/response types for POST /v1/responses/compact.  proxai handles
    # compaction data shapes (CompactionSummaryItemParam, CompactionBody) but
    # doesn't expose the compaction endpoint itself.
    "CompactResource": "compact API",
    "CompactResponseRequest": "compact API",
    # ── Token count API ──
    # Request/response types for POST /v1/responses/token-count.
    "TokenCountsBody": "token count API",
    "TokenCountsResource": "token count API",
    # ── Delete API ──
    # Response type for DELETE /v1/responses/{id}.
    "DeleteResponse": "delete API",
    # ── Stream wrapper ──
    # Type alias for StreamResponse<ResponseStreamEvent>. proxai defines the
    # concrete SSE event payload types but doesn't use the SDK's generic stream
    # wrapper — it handles SSE at the byte/event level.
    "ResponseStream": "stream wrapper",
    # ── Request body (pass-through) ──
    # CreateResponse: request-body type for POST /v1/responses.  proxai forwards
    # the raw JSON for same-protocol routes and hasn't implemented cross-protocol
    # translation yet.  Add a proxai-owned CreateResponse when translation starts.
    "CreateResponse": "request body",
    # ── Dead code in SDK ──
    # CodeInterpreterFile, LocalShellOutput: defined in async-openai but never
    #   referenced by any other SDK type.  Unused in the SDK itself.
    "CodeInterpreterFile": "dead code",
    "LocalShellOutput": "dead code",
    # ── Legacy function calling (superseded by tools) ──
    "FunctionCall": "legacy function",
    "FunctionName": "legacy function",
    "FunctionObject": "legacy function",
    # ── Shared utility types ──
    # Token-detail and format types in the SDK's shared/ module.  proxai has
    # its own equivalents (InputTokenDetails, OutputTokenDetails,
    # TextResponseFormatConfiguration) for the Responses protocol.
    "CompletionTokensDetails": "shared utility",
    "PromptTokensDetails": "shared utility",
    "ImageUrl": "shared utility",
    "ResponseFormat": "shared utility",
    # ── Chat Completions: request/param types ──
    "ChatCompletionAllowedTools": "chat request",
    "ChatCompletionAllowedToolsChoice": "chat request",
    "ChatCompletionAudio": "chat request",
    "ChatCompletionAudioFormat": "chat request",
    "ChatCompletionAudioVoice": "chat request",
    "ChatCompletionDeleted": "chat request",
    "ChatCompletionFunctionCall": "legacy function",
    "ChatCompletionFunctions": "legacy function",
    "ChatCompletionList": "chat request",
    "ChatCompletionMessageList": "chat request",
    "ChatCompletionMessageListItem": "chat request",
    "ChatCompletionNamedToolChoice": "chat request",
    "ChatCompletionNamedToolChoiceCustom": "chat request",
    "ChatCompletionRequestAssistantMessage": "chat request",
    "ChatCompletionRequestAssistantMessageAudio": "chat request",
    "ChatCompletionRequestAssistantMessageContent": "chat request",
    "ChatCompletionRequestAssistantMessageContentPart": "chat request",
    "ChatCompletionRequestDeveloperMessage": "chat request",
    "ChatCompletionRequestDeveloperMessageContent": "chat request",
    "ChatCompletionRequestDeveloperMessageContentPart": "chat request",
    "ChatCompletionRequestFunctionMessage": "chat request",
    "ChatCompletionRequestMessage": "chat request",
    "ChatCompletionRequestMessageContentPartAudio": "chat request",
    "ChatCompletionRequestMessageContentPartFile": "chat request",
    "ChatCompletionRequestMessageContentPartImage": "chat request",
    "ChatCompletionRequestMessageContentPartRefusal": "chat request",
    "ChatCompletionRequestMessageContentPartText": "chat request",
    "ChatCompletionRequestSystemMessage": "chat request",
    "ChatCompletionRequestSystemMessageContent": "chat request",
    "ChatCompletionRequestSystemMessageContentPart": "chat request",
    "ChatCompletionRequestToolMessage": "chat request",
    "ChatCompletionRequestToolMessageContent": "chat request",
    "ChatCompletionRequestToolMessageContentPart": "chat request",
    "ChatCompletionRequestUserMessage": "chat request",
    "ChatCompletionRequestUserMessageContent": "chat request",
    "ChatCompletionRequestUserMessageContentPart": "chat request",
    "ChatCompletionResponseStream": "stream wrapper",
    "ChatCompletionStreamOptions": "chat request",
    "ChatCompletionTool": "chat request",
    "ChatCompletionToolChoiceOption": "chat request",
    "ChatCompletionTools": "chat request",
    "Choice": "response detail",
    "CompletionFinishReason": "response detail",
    "ContentPart": "response detail",
    "CreateChatCompletionRequest": "chat request",
    "CustomName": "chat request",
    "CustomToolChatCompletions": "chat request",
    "CustomToolProperties": "chat request",
    "CustomToolPropertiesFormat": "chat request",
    "FileObject": "chat request",
    "InputAudio": "chat request",
    "InputAudioFormat": "chat request",
    "Logprobs": "response detail",
    "PredictionContent": "chat request",
    "PredictionContentContent": "chat request",
    "Prompt": "chat request",
    "ResponseModalities": "chat request",
    "StopConfiguration": "chat request",
    "ToolChoiceAllowedMode": "chat request",
    "ToolChoiceOptions": "chat request",
    "Verbosity": "chat request",
    "WebSearchContextSize": "chat request",
    "WebSearchLocation": "chat request",
    "WebSearchOptions": "chat request",
    "WebSearchUserLocation": "chat request",
    "WebSearchUserLocationType": "chat request",
    # Chat Completions: shared types only in responses protocol
    "CustomGrammarFormatParam": "responses-protocol only",
    "GrammarSyntax": "responses-protocol only",
    "ImageDetail": "responses-protocol only",
    "ReasoningEffort": "responses-protocol only",
    "ResponseFormatJsonSchema": "responses-protocol only",
}

# Category descriptions shown as section headers in -d output.
# Shared files to skip per protocol.  These contain types Chat Completions
# needs but Responses API doesn't (or vice versa).
SHARED_FILE_EXCLUSIONS = {
    "responses": {
        "completion_tokens_details.rs",
        "image_url.rs",
        "prompt_tokens_details.rs",
    },
    "chat": set(),
}

# Specific SDK type names to skip during comparison (not file-level).
# Types here still appear in SDK scanned data so types sharing their file
# (like ResponseFormatJsonSchema in response_format.rs) can be matched.
IGNORED_SDK_TYPES = {
    "ResponseFormat",
}


CATEGORY_DESCRIPTIONS = {
    "conversation CRUD": "Types for conversation management endpoints. proxai handles items inline"
    " via InputItem/OutputItem, no separate conversation CRUD.",
    "item list API": "Types for GET /v1/responses/{id}/items. proxai returns items inline"
    " in the Response.",
    "compact API": "Types for POST /v1/responses/compact. proxai handles compaction data"
    " shapes but does not expose the endpoint.",
    "token count API": "Types for POST /v1/responses/token-count.",
    "delete API": "Types for DELETE /v1/responses/{id}.",
    "stream wrapper": "SDK stream type alias StreamResponse<ResponseStreamEvent>. proxai"
    " handles SSE at byte/event level.",
    "request body": "CreateResponse request-body type. proxai forwards raw JSON for"
    " same-protocol routes; add when cross-protocol translation starts.",
    "dead code": "LocalShellOutput. Defined in async-openai but never referenced by"
    " any other SDK type.",
    "legacy function": "Legacy function-calling types superseded by the tools system.",
    "shared utility": "Token-detail and format types in SDK shared/ module. proxai has"
    " Responses-specific equivalents (InputTokenDetails, OutputTokenDetails,"
    " TextResponseFormatConfiguration).",
    "chat request": "Chat Completions request/param/message types. proxai Responses-API-focused"
    " and does not implement Chat Completions protocol.",
    "response detail": "Chat Completions response detail types.",
    "responses-protocol only": "Shared SDK types only present in Responses protocol. proxai has them"
    " for Responses but not Chat Completions.",
}

# Fields that proxai intentionally omits (simplified protocol types)
INTENTIONAL_FIELD_EXCLUSIONS = {
    # Toplevel type → set of field names
    "Response": {
        "object",  # serialization marker, not a semantic field
        "incomplete_details",  # proxai uses status == Incomplete instead
        "modalities",  # request-time field
        "parallel_tool_calls",  # moved to feature level
    },
    "OutputItem": {
        "type",  # serde tag, not a semantic field
    },
    "InputItem": {
        "type",  # serde tag
    },
    "OutputContent": {
        "type",  # serde tag
    },
    "InputContent": {
        "type",  # serde tag
    },
    "Tool": {
        "type",  # serde tag
    },
    "CreateChatCompletionRequest": {
        "metadata",  # SDK uses Metadata newtype with private .0 access
    },
}


# ── tree-sitter helpers ───────────────────────────────────────


def _parser():
    return tree_sitter.Parser(tree_sitter.Language(ts_rust.language()))


RS = _parser()


# ── SDK type extraction (full detail) ──────────────────────────


def sdk_type_index(protocol="responses"):
    """Build a dict of SDK type_name → {kind, fields, variants, file, line}."""
    files = _sdk_files(protocol)
    index = {}
    for f in files:
        if not f.exists():
            continue
        rel = str(f.relative_to(SDK_DIR.parent.parent))
        buf = f.read_bytes()
        tree = RS.parse(buf)
        _index_sdk_types(tree.root_node, buf, index, rel)
    return index


def _sdk_files(protocol):
    """List SDK source files to scan for a given protocol."""
    shared_dir = SDK_DIR / "shared"
    shared_files = sorted(shared_dir.rglob("*.rs")) if shared_dir.exists() else []

    if protocol == "responses":
        files = [
            SDK_DIR / "responses" / "response.rs",
            SDK_DIR / "responses" / "stream.rs",
            SDK_DIR / "responses" / "conversation.rs",
        ]
        rel = [
            f
            for f in shared_files
            if _has_feature_gate(f, "response-types")
            or _has_feature_gate(f, "chat-completion-types")
            if f.name not in SHARED_FILE_EXCLUSIONS.get(protocol, set())
        ]
        files.extend(rel)
        mcp_dir = SDK_DIR / "mcp"
        if mcp_dir.exists():
            files.extend(sorted(mcp_dir.rglob("*.rs")))
    else:
        files = [SDK_DIR / "chat" / "chat_.rs"]
        rel = [
            f
            for f in shared_files
            if _has_feature_gate(f, "chat-completion-types")
            if f.name not in SHARED_FILE_EXCLUSIONS.get(protocol, set())
        ]
        files.extend(rel)
    return files


def _has_feature_gate(file_path, feature):
    """Check if a shared module file has a cfg gate matching the given feature.

    Uses tree-sitter to parse mod.rs AST — handles multi-line/nested cfg
    attributes correctly.
    Returns True if the file should be included (no gate, or gate matches).
    """
    mod_rs = file_path.parent / "mod.rs"
    if not mod_rs.exists():
        return True
    stem = file_path.stem
    buf = mod_rs.read_bytes()
    tree = RS.parse(buf)

    # Walk pairs of (attribute?, mod) siblings
    mod_found = False
    cfg_attrs = []
    for child in tree.root_node.children:
        if child.type == "attribute_item":
            cfg_attrs.append(child)
        elif child.type == "mod_item":
            # Check if this mod matches our stem
            mod_name = _mod_item_name(child)
            if mod_name == stem:
                mod_found = True
                # Check all accumulated cfg attributes
                if not cfg_attrs:
                    return True
                for attr in cfg_attrs:
                    text = attr.text.decode("utf-8", errors="replace")
                    if feature in text:
                        return True
                # Has cfg but none match the feature → exclude
                return False
            # Non-matching mod: clear cfg accumulation (attributes belong to this mod)
            cfg_attrs = []

    return True  # mod not found in mod.rs, include by default


def _mod_item_name(node):
    """Extract the module name from a mod_item AST node."""
    for c in node.children:
        if c.type == "identifier":
            return c.text.decode()
    return None


def _index_sdk_types(node, buf, index, rel):
    for child in node.children:
        if _has_pub(child):
            if child.type == "struct_item":
                name = _type_name(child)
                if name:
                    fields, deprecated = _struct_fields(child, buf)
                    index[name] = dict(
                        kind="struct",
                        fields=fields,
                        deprecated_fields=deprecated,
                        line=child.start_point[0] + 1,
                        file=rel,
                    )
            elif child.type == "enum_item":
                name = _type_name(child)
                if name:
                    variants = _enum_variants_ast(child)
                    index[name] = dict(
                        kind="enum",
                        variants=variants,
                        line=child.start_point[0] + 1,
                        file=rel,
                    )
            elif child.type == "type_item":
                name = _type_name(child)
                if name:
                    index[name] = dict(
                        kind="type", line=child.start_point[0] + 1, file=rel
                    )
        _index_sdk_types(child, buf, index, rel)


def _struct_fields(struct_node, buf):
    """Extract field names and detect deprecated fields from a struct_item AST node.
    Returns (field_names, deprecated_set).
    """
    fields = []
    deprecated = set()
    for child in struct_node.children:
        if child.type == "field_declaration_list":
            for field in child.children:
                if field.type == "field_declaration":
                    # Check if this field has a #[deprecated] attribute
                    is_deprecated = False
                    cursor = field.prev_named_sibling
                    while cursor and cursor.type == "attribute_item":
                        if cursor.text.decode().startswith("#[deprecated"):
                            is_deprecated = True
                            break
                        cursor = cursor.prev_named_sibling
                    for ident in field.children:
                        if ident.type == "field_identifier":
                            name = ident.text.decode()
                            fields.append(name)
                            if is_deprecated:
                                deprecated.add(name)
                            break
    return fields, deprecated


def _enum_variants_ast(enum_node):
    """Extract variant names from an enum_item AST node."""
    variants = []
    for child in enum_node.children:
        if child.type == "enum_variant_list":
            for variant in child.children:
                if variant.type == "enum_variant":
                    for ident in variant.children:
                        if ident.type == "identifier":
                            variants.append(ident.text.decode())
                            break
    return variants


# ── Proxai type extraction (full detail + convert annotations) ─


def px_type_index(protocol="responses"):
    """Build dict of proxai type_name → {kind, fields, variants, file, line}."""
    if protocol == "responses":
        proto_dir = PROTO_DIR / "responses" / "wire"
        extra_dirs = []
    else:
        proto_dir = PROTO_DIR / "chat_completions" / "wire"
        extra_dirs = [PROTO_DIR / "chat_completions" / "request" / "wire"]

    index = {}
    for d in [proto_dir] + extra_dirs:
        for f in sorted(d.rglob("*.rs")):
            rel = str(f.relative_to(PROTO_DIR))
            buf = f.read_bytes()
            tree = RS.parse(buf)
            _index_px_types(tree.root_node, buf, index, rel)
    return index


def _index_px_types(node, buf, index, rel):
    for child in node.children:
        if _has_pub(child):
            info = dict(line=child.start_point[0] + 1, file=rel)

            if child.type == "struct_item":
                name = _type_name(child)
                if name:
                    info["kind"] = "struct"
                    px_fields, _ = _struct_fields(child, buf)
                    info["fields"] = px_fields
                    index[name] = info
            elif child.type == "enum_item":
                name = _type_name(child)
                if name:
                    info["kind"] = "enum"
                    info["variants"] = _enum_variants_ast(child)
                    index[name] = info
            elif child.type == "type_item":
                name = _type_name(child)
                if name:
                    info["kind"] = "type"
                    index[name] = info
            elif child.type == "use_declaration":
                # pub use re-exports (e.g. pub use crate::protocol::ErrorObject;)
                text = child.text.decode("utf-8", errors="replace")
                m = re.search(r"pub\s+use\s+(?:\w+::)+?(\w+);", text)
                if m:
                    name = m.group(1)
                    index[name] = dict(
                        kind="struct",
                        line=child.start_point[0] + 1,
                        file=rel,
                        is_re_export=True,
                    )
        _index_px_types(child, buf, index, rel)


# ── Common helpers ────────────────────────────────────────────


def _has_pub(node):
    for c in node.children:
        if c.type == "visibility_modifier" and c.text.decode() == "pub":
            return True
    return False


def _type_name(node):
    for c in node.children:
        if c.type == "type_identifier":
            return c.text.decode()
    return None


# ── Normalization ──────────────────────────────────────────────


def norm(n):
    return n.lower().replace("_", "").replace("-", "")


# ── Output ─────────────────────────────────────────────────────


def out(t=""):
    sys.stdout.buffer.write((t + "\n").encode("utf-8", errors="replace"))


def hr():
    out("=" * 66)


def h2(title):
    out(f"\n  {title}")
    out(f"  {'─' * min(64, len(title) + 2)}")


# ── Main ───────────────────────────────────────────────────────


def main():
    args = sys.argv[1:]
    protocols = []
    level = 2  # default detail level
    i = 0
    while i < len(args):
        if args[i] in ("responses", "chat"):
            protocols.append(args[i])
        elif args[i] in ("--level", "-l") and i + 1 < len(args):
            level = int(args[i + 1])
            i += 1
        elif args[i] in ("--quiet", "-q"):
            level = 1
        elif args[i] in ("--detail", "-d"):
            level = 2
        elif args[i] in ("--verbose", "-v"):
            level = 3
        i += 1
    if not protocols:
        protocols = ["responses", "chat"]

    has_any_gaps = False

    for protocol in protocols:
        has_any_gaps |= _check_protocol(protocol, level)

    if level >= 1:
        hr()
        if has_any_gaps:
            out(f"\n  ⚠  Gaps found — see MISSING sections above")
            sys.exit(1)
        else:
            out(f"\n  ✅  OpenAI protocol coverage complete — no gaps")
        hr()


def _check_protocol(protocol, level=2):
    label = "Responses API" if protocol == "responses" else "Chat Completions"

    sdk_ver = ""
    if SDK_PKG.exists():
        try:
            import tomllib

            sdk_ver = (
                tomllib.loads(SDK_PKG.read_text()).get("package", {}).get("version", "")
            )
        except Exception:
            pass

    if protocol == "responses":
        proto_dir = PROTO_DIR / "responses" / "wire"
    else:
        proto_dir = PROTO_DIR / "chat_completions" / "wire"

    # ── Build indexes ─────────────────────────────────────────
    sdk = sdk_type_index(protocol)  # name → {kind, fields, variants, file, line}
    px = px_type_index(protocol)  # name → {kind, fields, variants, file, line}

    # Normalized name lookup for fuzzy matching
    sn = {norm(name): (name, info) for name, info in sdk.items()}
    pn = {norm(name): (name, info) for name, info in px.items()}

    sk = set(sn)
    pk = set(pn)

    # ── Classify type-level gaps ──────────────────────────────
    sdk_only = sk - pk
    missing_types = []
    excluded_types = []
    for nk in sorted(sdk_only):
        name, info = sn[nk]
        if name in IGNORED_SDK_TYPES:
            continue
        if name in INTENTIONAL_EXCLUSIONS:
            excluded_types.append((name, info))
        else:
            missing_types.append((name, info))

    px_only = pk - sk
    px_extra = [(pn[nk][0], pn[nk][1]) for nk in sorted(px_only)]

    # ── Structural alignment by matched protocol type name ──────────
    struct_diffs = []  # list of (px_type, sdk_type, missing_fields, extra_fields)
    enum_diffs = []  # list of (px_type, sdk_type, missing_variants, extra_variants)
    aligned_ok = 0

    for nk in sorted(sk & pk):
        sdk_name, sdk_info = sn[nk]
        px_name, px_info = pn[nk]

        if px_info["kind"] != sdk_info["kind"]:
            struct_diffs.append(
                (
                    px_name,
                    sdk_name,
                    f"kind mismatch: {px_info['kind']} vs {sdk_info['kind']}",
                    [],
                    None,
                )
            )
            continue

        if px_info["kind"] == "struct":
            # Skip structural comparison for re-exports (no actual struct to scan).
            if px_info.get("is_re_export"):
                aligned_ok += 1
                continue
            sdk_fields = sdk_info.get("fields", [])
            px_fields = px_info.get("fields", [])
            exclusions = INTENTIONAL_FIELD_EXCLUSIONS.get(px_name, set())
            # Auto-exclude #[deprecated] SDK fields.
            deprecated = sdk_info.get("deprecated_fields", set())
            exclusions = exclusions | deprecated

            sdk_set = set(sdk_fields)
            px_set = set(px_fields)

            missing_f = sorted(sdk_set - px_set - exclusions)
            extra_f = sorted(px_set - sdk_set)

            # Check field order (only if same fields and both non-empty).
            order_mismatch = None
            common = [f for f in sdk_fields if f in px_set]
            px_common = [f for f in px_fields if f in sdk_set]
            if common and px_common and common != px_common:
                order_mismatch = (common, px_common)

            if missing_f or extra_f or order_mismatch:
                struct_diffs.append(
                    (px_name, sdk_name, missing_f, extra_f, order_mismatch)
                )
            else:
                aligned_ok += 1

        elif px_info["kind"] == "enum":
            sdk_variants = set(sdk_info.get("variants", []))
            px_variants = set(px_info.get("variants", []))

            missing_v = sorted(sdk_variants - px_variants)
            extra_v = sorted(px_variants - sdk_variants)

            if missing_v or extra_v:
                enum_diffs.append((px_name, sdk_name, missing_v, extra_v))
            else:
                aligned_ok += 1
        else:
            aligned_ok += 1

    # ── Print ─────────────────────────────────────────────────
    has_gaps = bool(missing_types)
    has_missing_fields = any(
        m_f
        for _, _, m_f, _, _ in struct_diffs
        if not isinstance(m_f, str)  # skip kind-mismatch entries
    )
    has_struct_gaps = has_missing_fields
    has_gaps = has_gaps or has_struct_gaps

    structural_checked = aligned_ok + len(struct_diffs) + len(enum_diffs)

    # ── Level 1: header + summary ──────────────────────────────────
    hr()
    out(f"  OpenAI {label}  vs  async-openai {sdk_ver}")
    out(
        f"  SDK:  {', '.join(str(f.relative_to(SDK_DIR.parent.parent)) for f in _sdk_files(protocol))}"
    )
    out(f"  Ours: {proto_dir}/")
    out()
    ignored_names = {n for n in (sk - pk) if sn[n][0] in IGNORED_SDK_TYPES}
    out(f"  SDK types: {len(sdk)}  |  Ours: {len(px)}  |  Matched: {len(sk & pk)}")

    # Accounting: SDK = matched + excluded (so user can verify)
    if level >= 2:
        excluded_cnt = len(sdk_only) - len(ignored_names)
        out(
            f"  (SDK {len(sdk)} = matched {len(sk & pk)} + excluded {excluded_cnt}"
            f"{' + ignored ' + str(len(ignored_names)) if ignored_names else ''})"
        )

    out(f"  Structural checks: {structural_checked}  |  Aligned: {aligned_ok}")

    deprecated_skipped = sum(
        len(sn[nk][1].get("deprecated_fields", set())) for nk in (sk & pk)
    )
    if level >= 2 and deprecated_skipped:
        out(f"  (skipped {deprecated_skipped} deprecated SDK fields)")
    out()

    # ── Level 2: gaps, exclusions, extra types ─────────────────────
    if level >= 2:
        # Type-level gaps
        if missing_types:
            h2(
                f"MISSING ({len(missing_types)}): SDK types not found in proxai ({label})"
            )
            for i, (name, info) in enumerate(missing_types, 1):
                out(f"  {i:3d}. ✗ {name:<42s} @ {info['file']}:{info['line']}")
        else:
            out(f"  ✅  Type coverage — no missing types")
        out()

        # Structural diffs
        if struct_diffs or enum_diffs:
            has_gaps = True
            if has_missing_fields:
                h2("Structural alignment (field-level)")
            else:
                h2("Structural alignment — no missing fields (extras only, not gaps)")

            if struct_diffs:
                for (
                    px_name,
                    sdk_name,
                    missing_f,
                    extra_f,
                    order_mismatch,
                ) in struct_diffs:
                    if isinstance(missing_f, str):
                        if level >= 2:
                            out(f"\n  ✗ {px_name} (→ {sdk_name}): {missing_f}")
                        continue
                    has_real_gap = bool(missing_f)
                    if has_real_gap or level >= 3:
                        out(f"\n      {px_name} → {sdk_name}")
                    if missing_f:
                        out(f"        Missing fields:  {', '.join(missing_f)}")
                    if level >= 3 and extra_f:
                        out(f"        Extra fields:    {', '.join(extra_f)}")
                    if level >= 3 and order_mismatch:
                        sdk_order, px_order = order_mismatch
                        out(f"        Order mismatch:")
                        out(f"          SDK: {', '.join(sdk_order)}")
                        out(f"          Ours: {', '.join(px_order)}")

            if enum_diffs and level >= 3:
                out(f"\n  ✗ Enum variant mismatches:")
                for px_name, sdk_name, missing_v, extra_v in enum_diffs:
                    out(f"      {px_name} → {sdk_name}")
                    if missing_v:
                        out(f"        Missing variants: {', '.join(missing_v)}")
                    if extra_v:
                        out(f"        Extra variants:   {', '.join(extra_v)}")

            if not struct_diffs and not enum_diffs:
                out(f"  ✅  All struct fields and enum variants match")
            out()

        # Intentional exclusions
        if excluded_types:
            h2(
                f"Intentional exclusions ({len(excluded_types)} types not needed in proxai)"
            )
            # Group by category
            cat_order = sorted(
                set(INTENTIONAL_EXCLUSIONS.get(n, "") for n, _ in excluded_types)
            )
            idx = 0
            for cat in cat_order:
                cat_types = [
                    (n, i)
                    for n, i in excluded_types
                    if INTENTIONAL_EXCLUSIONS.get(n, "") == cat
                ]
                desc = CATEGORY_DESCRIPTIONS.get(cat, cat)
                out("=" * 66)
                out(f"  {cat}")
                for line in desc.split("\n"):
                    out(f"  {line}")
                out("=" * 66)
                for name, info in cat_types:
                    idx += 1
                    out(f"  {idx:3d}. ~ {name:<42s} @ {info['file']}:{info['line']}")
                out()

    # ── Level 3: proxai-specific type classification ──────────────
    if level >= 3 and px_extra:
        truly_extra = px_extra
        if truly_extra:
            h2(f"Proxai-specific types ({len(truly_extra)} — no matching SDK type)")
            for i, (name, info) in enumerate(sorted(truly_extra), 1):
                kind = info.get("kind", "?")
                file = info.get("file", "?")
                # Classify the type
                tag = "other"
                src = ""
                try:
                    p = PROTO_DIR.parent.parent.parent / file
                    if p.exists():
                        src = p.read_text(encoding="utf-8", errors="replace")
                        lines = src.split("\n")
                        ln = info.get("line", 1)
                        ctx = lines[ln : ln + 5] if ln < len(lines) else []
                        ctx_text = "\n".join(ctx)[:200]
                        if src.split("\n")[ln - 1].strip().startswith("pub type"):
                            tag = "type alias"
                        elif kind == "enum":
                            tag = "internal enum"
                        elif "pub struct" in ctx_text and "Deserialize" in src:
                            tag = "serialization helper"
                        else:
                            tag = "proxai internal"
                except Exception:
                    pass
                out(f"  + {kind:<6s} {name:<40s} [{tag}]  @ {file}:{info['line']}")

    # ── Level 3: ignored SDK types ─────────────────────────
    if level >= 3 and ignored_names:
        h2(
            f"Ignored SDK types ({len(ignored_names)} — no proxai equivalent, silently skipped)"
        )
        for i, nk in enumerate(sorted(ignored_names), 1):
            name, info = sn[nk]
            out(f"  {i:3d}. ~ {name:<42s} @ {info['file']}:{info['line']}")
        out()

    if level >= 2 and not has_gaps:
        out(f"\n  ✅  No gaps — {label} protocol fully aligned")
        out()

    if level >= 1:
        hr()
        out()

    return has_gaps


if __name__ == "__main__":
    main()
