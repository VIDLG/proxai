import re
from pathlib import Path

from .common import PROTO_DIR, RS, norm
from .sdk import _normalize_ts_type, _parse_ts_shape


def rust_comment_shapes():
    """Extract `/// export interface/type ...` snippets from Rust doc comments."""
    shapes = []
    for item in rust_doc_items():
        doc = item["doc"]
        j = 0
        while j < len(doc):
            if not re.search(r"\bexport\s+(interface|type)\s+\w+", doc[j]["text"]):
                j += 1
                continue
            snippet = [doc[j]["text"]]
            start_line = doc[j]["line"]
            if "export interface" in doc[j]["text"]:
                j += 1
                while j < len(doc):
                    snippet.append(doc[j]["text"])
                    if doc[j]["text"].strip() == "}":
                        break
                    j += 1
            else:
                j += 1
                while j < len(doc):
                    snippet.append(doc[j]["text"])
                    if doc[j]["text"].strip().endswith(";"):
                        break
                    j += 1
            shape = _parse_ts_shape("\n".join(snippet))
            if shape:
                shape["file"] = item["file"]
                shape["line"] = start_line
                shape["snippet"] = "\n".join(snippet)
                shape["item"] = item["name"]
                shapes.append(shape)
            j += 1
    return shapes


def _sdk_shape_marker(item):
    for line in item["doc"]:
        m = re.match(r'^@sdk\(shape\s*=\s*"([A-Za-z_]\w*)"\)$', line["text"])
        if m:
            return m.group(1)
        m = re.match(r"^SDK shape:\s*([A-Za-z_]\w*)\.$", line["text"])
        if m:
            return m.group(1)
    return None


def rust_item_shape_bindings(sdk_shapes, only_marked=False):
    """Bind Rust items to SDK shapes by `@sdk(shape)` or matching name."""
    bindings = []
    doc_items = {item["name"]: item for item in rust_doc_items()}
    rust_items = rust_serde_items()
    for name, rust_item in rust_items.items():
        doc_item = doc_items.get(
            name,
            {
                "name": name,
                "file": rust_item["file"],
                "line": rust_item["line"],
                "doc": [],
            },
        )
        marker = _sdk_shape_marker(doc_item)
        candidates = []
        if marker:
            candidates.append(marker)
        if not only_marked:
            candidates.extend(
                sdk_name for sdk_name in sdk_shapes if norm(sdk_name) == norm(name)
            )
        seen = set()
        for sdk_name in candidates:
            if sdk_name in seen or sdk_name not in sdk_shapes:
                continue
            seen.add(sdk_name)
            bindings.append(
                {
                    "item": name,
                    "file": rust_item["file"],
                    "line": rust_item["line"],
                    "sdk_name": sdk_name,
                    "sdk_shape": sdk_shapes[sdk_name],
                }
            )
    return bindings


def rust_sdk_markers():
    """Read SDK exception/mapping markers from Rust doc comments."""
    aliases = {}
    union_variants = {}
    internals = set()
    proxai_internals = {}
    externals = set()
    field_suppressed = {}
    required_nullable_accepts_missing = {}
    legacy = []
    for item in rust_doc_items():
        for line in item["doc"]:
            text = line["text"]
            locn = f"{item['file']}:{line['line']}"
            m = re.match(r'^@sdk\(alias\s*=\s*"([A-Za-z_]\w*)"\)$', text)
            if m:
                aliases[m.group(1)] = item["name"]
                continue
            m = re.match(
                r'^@sdk\(union_variant\s*=\s*"([A-Za-z_]\w*)",\s*rust\s*=\s*"([A-Za-z_]\w*)"\)$',
                text,
            )
            if m:
                union_variants[m.group(1)] = m.group(2)
                continue
            m = re.match(r'^@sdk\(internal\s*=\s*"([A-Za-z_]\w*)"\)$', text)
            if m:
                internals.add(m.group(1))
                continue
            m = re.match(r'^@sdk\(proxai_internal\s*=\s*"([A-Za-z0-9_-]+)"\)$', text)
            if m:
                proxai_internals[item["name"]] = m.group(1)
                continue
            m = re.match(r'^@sdk\(external\s*=\s*"([A-Za-z_]\w*)"\)$', text)
            if m:
                externals.add(m.group(1))
                continue
            m = re.match(r'^@sdk\(field_suppress\s*=\s*"([A-Za-z_]\w*)"\)$', text)
            if m:
                field_suppressed.setdefault(item["name"], set()).add(m.group(1))
                continue
            m = re.match(
                r'^@sdk\(required_nullable_accepts_missing\s*=\s*"([A-Za-z_]\w*)"\)$',
                text,
            )
            if m:
                required_nullable_accepts_missing.setdefault(item["name"], set()).add(
                    m.group(1)
                )
                continue
            if text.startswith("SDK "):
                legacy.append(
                    (
                        item["name"],
                        locn,
                        [f"use structured marker syntax instead of `{text}`"],
                    )
                )
                continue
            m = re.match(r"^SDK alias:\s*([A-Za-z_]\w*)\.$", text)
            if m:
                aliases[m.group(1)] = item["name"]
                continue
            m = re.match(
                r"^SDK union variant:\s*([A-Za-z_]\w*)\s*=\s*([A-Za-z_]\w*)\.$", text
            )
            if m:
                union_variants[m.group(1)] = m.group(2)
                continue
            m = re.match(r"^SDK internal:\s*([A-Za-z_]\w*)\.$", text)
            if m:
                internals.add(m.group(1))
                continue
            m = re.match(r"^SDK external:\s*([A-Za-z_]\w*)\.$", text)
            if m:
                externals.add(m.group(1))
                continue

    for item_name, item in rust_serde_items().items():
        for field_name, field in item.get("fields", {}).items():
            for line in field.get("doc", []):
                if line["text"] == "@sdk(required_nullable_accepts_missing)":
                    required_nullable_accepts_missing.setdefault(item_name, set()).add(
                        field_name
                    )

    return {
        "aliases": aliases,
        "internals": internals,
        "proxai_internals": proxai_internals,
        "externals": externals,
        "field_suppressed": field_suppressed,
        "required_nullable_accepts_missing": required_nullable_accepts_missing,
        "union_variants": union_variants,
        "legacy": legacy,
    }


def shape_binding_diffs(sdk_shapes):
    """Validate that SDK shape comments are attached to the intended Rust item."""
    diffs = []
    shapes_by_item = {}
    for shape in rust_comment_shapes():
        shapes_by_item.setdefault((shape["file"], shape["item"]), []).append(shape)

    for item in rust_doc_items():
        locn = f"{item['file']}:{item['line']}"
        marker = _sdk_shape_marker(item)
        item_shapes = shapes_by_item.get((item["file"], item["name"]), [])
        if marker and marker not in sdk_shapes:
            diffs.append(
                (
                    item["name"],
                    locn,
                    [f"SDK shape marker refers to unknown SDK export `{marker}`"],
                )
            )

        for shape in item_shapes:
            shape_name = shape["name"]
            if norm(shape_name) == norm(item["name"]):
                if marker and marker != shape_name:
                    diffs.append(
                        (
                            item["name"],
                            locn,
                            [
                                f"SDK shape marker `{marker}` does not match attached shape `{shape_name}`"
                            ],
                        )
                    )
                continue
            if marker != shape_name:
                diffs.append(
                    (
                        item["name"],
                        f"{shape['file']}:{shape['line']}",
                        [
                            f'attached SDK shape `{shape_name}` does not match Rust item `{item["name"]}`; add `@sdk(shape = "{shape_name}")`'
                        ],
                    )
                )

    return diffs


def _doc_comments_before(node, lines):
    doc = []
    cursor = node.prev_named_sibling
    while cursor and cursor.type in ("line_comment", "attribute_item"):
        if cursor.type == "line_comment" and cursor.text.decode(
            "utf-8", errors="replace"
        ).lstrip().startswith("///"):
            line_no = cursor.start_point[0] + 1
            text = re.sub(r"^\s*/// ?", "", lines[line_no - 1]).strip()
            doc.append({"line": line_no, "text": text})
        cursor = cursor.prev_named_sibling
    return list(reversed(doc))


def _item_attributes(item_node):
    attrs = []
    cursor = item_node.prev_named_sibling
    while cursor and cursor.type in ("line_comment", "attribute_item"):
        if cursor.type == "attribute_item":
            attrs.append(cursor.text.decode("utf-8", errors="replace"))
        cursor = cursor.prev_named_sibling
    return list(reversed(attrs))


def rust_serde_items():
    """Extract serde-relevant attributes from public Rust type items."""
    items = {}
    for f in sorted(PROTO_DIR.rglob("*.rs")):
        rel = str(f.relative_to(PROTO_DIR))
        buf = f.read_bytes()
        lines = buf.decode("utf-8", errors="replace").splitlines()
        tree = RS.parse(buf)

        def visit(node):
            for child in node.children:
                if child.type in ("struct_item", "enum_item", "type_item"):
                    has_pub = any(
                        gc.type == "visibility_modifier" and gc.text.decode() == "pub"
                        for gc in child.children
                    )
                    if has_pub:
                        name = None
                        for gc in child.children:
                            if gc.type == "type_identifier":
                                name = gc.text.decode()
                                break
                        if name:
                            info = {
                                "file": rel,
                                "line": child.start_point[0] + 1,
                                "kind": child.type,
                                "attrs": _item_attributes(child),
                                "fields": {},
                                "variants": {},
                            }
                            for gc in child.children:
                                if gc.type == "field_declaration_list":
                                    for field in gc.children:
                                        if field.type != "field_declaration":
                                            continue
                                        fname = None
                                        for fc in field.children:
                                            if fc.type == "field_identifier":
                                                fname = fc.text.decode()
                                                break
                                        if fname:
                                            info["fields"][fname] = {
                                                "attrs": _item_attributes(field),
                                                "doc": _doc_comments_before(
                                                    field, lines
                                                ),
                                                "line": field.start_point[0] + 1,
                                                "type": None,
                                            }
                                            for fc in field.children:
                                                if fc.type in (
                                                    "type_identifier",
                                                    "generic_type",
                                                    "scoped_type_identifier",
                                                    "primitive_type",
                                                    "reference_type",
                                                    "tuple_type",
                                                    "unit_type",
                                                ):
                                                    info["fields"][fname]["type"] = (
                                                        fc.text.decode(
                                                            "utf-8", errors="replace"
                                                        )
                                                    )
                                                    break
                            for gc in child.children:
                                if gc.type == "enum_variant_list":
                                    for variant in gc.children:
                                        if variant.type != "enum_variant":
                                            continue
                                        vname = None
                                        payloads = []
                                        for vc in variant.children:
                                            if (
                                                vc.type == "identifier"
                                                and vname is None
                                            ):
                                                vname = vc.text.decode(
                                                    "utf-8", errors="replace"
                                                )
                                            else:
                                                payloads.extend(
                                                    sorted(_type_names_in_node(vc))
                                                )
                                        if vname:
                                            info["variants"][vname] = {
                                                "attrs": _item_attributes(variant),
                                                "line": variant.start_point[0] + 1,
                                                "payloads": [
                                                    p
                                                    for p in payloads
                                                    if p
                                                    not in ("Option", "Box", "String")
                                                ],
                                            }
                            items[name] = info
                visit(child)

        visit(tree.root_node)
    return items


def _serde_attr_has(attrs, needle):
    return needle in " ".join(attrs)


def _serde_rename(attrs):
    for attr in attrs:
        m = re.search(r'rename\s*=\s*"([^"]+)"', attr)
        if m:
            return m.group(1)
    return None


def _serde_rename_all(attrs):
    for attr in attrs:
        m = re.search(r'rename_all\s*=\s*"([^"]+)"', attr)
        if m:
            return m.group(1)
    return None


def _serde_skip_if_none(attrs):
    return 'skip_serializing_if = "Option::is_none"' in " ".join(attrs)


def _snake_case(name):
    return re.sub(r"(?<!^)(?=[A-Z])", "_", name).lower()


def _kebab_case(name):
    return _snake_case(name).replace("_", "-")


def _serde_case(name, rule):
    if rule == "snake_case":
        return _snake_case(name)
    if rule == "lowercase":
        return name.lower()
    if rule == "kebab-case":
        return _kebab_case(name)
    return name


def _rust_variant_wire_name(variant_name, variant, enum_item):
    renamed = _serde_rename(variant.get("attrs", []))
    if renamed:
        return renamed
    return _serde_case(variant_name, _serde_rename_all(enum_item.get("attrs", [])))


def _rust_field_wire_name(name, field):
    renamed = _serde_rename(field.get("attrs", []))
    if renamed:
        return renamed
    return name.removeprefix("r#").removesuffix("_")


def _field_by_wire_name(item, wire_name):
    for rust_name, field in item.get("fields", {}).items():
        if _rust_field_wire_name(rust_name, field) == wire_name:
            return rust_name, field
    return None, None


def _rust_type_is_option(type_text):
    if not type_text:
        return False
    return re.search(r"(^|::)\bOption\s*<", type_text) is not None


def rust_tagged_variant_literals():
    """Map Rust payload types to literals supplied by tagged serde enums."""
    refs = {}
    for f in sorted(PROTO_DIR.rglob("*.rs")):
        buf = f.read_bytes()
        tree = RS.parse(buf)

        def visit(node):
            for child in node.children:
                if child.type == "enum_item":
                    enum_attrs = _item_attributes(child)
                    if not _serde_attr_has(enum_attrs, 'serde(tag = "type")'):
                        visit(child)
                        continue
                    for gc in child.children:
                        if gc.type != "enum_variant_list":
                            continue
                        for variant in gc.children:
                            if variant.type != "enum_variant":
                                continue
                            variant_name = None
                            payload_type = None
                            for vc in variant.children:
                                if vc.type == "identifier" and variant_name is None:
                                    variant_name = vc.text.decode(
                                        "utf-8", errors="replace"
                                    )
                                else:
                                    payload_names = [
                                        name
                                        for name in _type_names_in_node(vc)
                                        if name not in ("Option", "Vec", "Box")
                                    ]
                                    if payload_names:
                                        payload_type = sorted(payload_names)[0]
                            if not variant_name or not payload_type:
                                continue
                            literal = _serde_rename(
                                _item_attributes(variant)
                            ) or _snake_case(variant_name)
                            refs.setdefault(payload_type, set()).add(literal)
                visit(child)

        visit(tree.root_node)
    return refs


def rust_doc_items():
    """Return doc comment blocks attached to public Rust type items."""
    items = []
    for f in sorted(PROTO_DIR.rglob("*.rs")):
        rel = str(f.relative_to(PROTO_DIR))
        buf = f.read_bytes()
        tree = RS.parse(buf)
        lines = buf.decode("utf-8", errors="replace").splitlines()

        def visit(node):
            for child in node.children:
                if child.type in ("struct_item", "enum_item", "type_item"):
                    has_pub = any(
                        gc.type == "visibility_modifier" and gc.text.decode() == "pub"
                        for gc in child.children
                    )
                    if has_pub:
                        name = None
                        for gc in child.children:
                            if gc.type == "type_identifier":
                                name = gc.text.decode()
                                break
                        doc = []
                        cursor = child.prev_named_sibling
                        while cursor and cursor.type in (
                            "line_comment",
                            "attribute_item",
                        ):
                            if cursor.type == "line_comment" and cursor.text.decode(
                                "utf-8", errors="replace"
                            ).lstrip().startswith("///"):
                                line_no = cursor.start_point[0] + 1
                                text = re.sub(
                                    r"^\s*/// ?", "", lines[line_no - 1]
                                ).strip()
                                doc.append({"line": line_no, "text": text})
                            cursor = cursor.prev_named_sibling
                        if doc:
                            items.append(
                                {
                                    "file": rel,
                                    "line": child.start_point[0] + 1,
                                    "name": name,
                                    "kind": child.type,
                                    "doc": list(reversed(doc)),
                                }
                            )
                visit(child)

        visit(tree.root_node)
    return items


def rust_field_shape_comments():
    """Extract `/// Type.field: `TS type`.` snippets from Rust doc comments."""
    refs = []
    single_line = re.compile(r"^([A-Za-z_]\w*)\.([A-Za-z_]\w*):\s*`([^`]+)`\.?$")
    multiline = re.compile(r"^([A-Za-z_]\w*)\.([A-Za-z_]\w*):\s*$")
    for item in rust_doc_items():
        rel = item["file"]
        doc_lines = [(line["line"], line["text"]) for line in item["doc"]]
        i = 0
        while i < len(doc_lines):
            line_no, stripped = doc_lines[i]
            m = single_line.match(stripped)
            if m:
                refs.append(
                    {
                        "owner": m.group(1),
                        "field": m.group(2),
                        "type": _normalize_ts_type(m.group(3)),
                        "file": rel,
                        "line": line_no,
                    }
                )
                i += 1
                continue
            m = multiline.match(stripped)
            if m and i + 1 < len(doc_lines):
                _, next_line = doc_lines[i + 1]
                if next_line.startswith("`") and next_line.endswith("`."):
                    refs.append(
                        {
                            "owner": m.group(1),
                            "field": m.group(2),
                            "type": _normalize_ts_type(next_line[1:-2]),
                            "file": rel,
                            "line": line_no,
                        }
                    )
                    i += 2
                    continue
                if next_line.startswith("`") and next_line.endswith("`"):
                    refs.append(
                        {
                            "owner": m.group(1),
                            "field": m.group(2),
                            "type": _normalize_ts_type(next_line[1:-1]),
                            "file": rel,
                            "line": line_no,
                        }
                    )
                    i += 2
                    continue
            i += 1
    return refs


def uncontrolled_ts_comment_fragments():
    """Find TS-looking doc comments that are not tied to an SDK export/field."""
    allowed_prefixes = (
        "export ",
        "🎯 @use",
        "Used by:",
        "@sdk(",
        "Payload:",
        "Note:",
        "Flow:",
        "Read from ",
        "Constructed via ",
    )
    fragments = []
    tsish = re.compile(r"^([A-Za-z_]\w*)\??:\s*.+(;|$)|^\| .+;?$|^\{?$|^\}?$")
    owner_field = re.compile(r"^[A-Za-z_]\w*\.[A-Za-z_]\w*:\s*(`[^`]+`\.?)?$")
    for item in rust_doc_items():
        rel = item["file"]
        in_export_shape = False
        export_brace_depth = 0
        in_export_type = False
        in_namespace = False
        namespace_brace_depth = 0
        for line in item["doc"]:
            line_no = line["line"]
            text = line["text"]
            if re.search(r"\bexport\s+namespace\s+\w+", text):
                in_namespace = True
                namespace_brace_depth = max(1, text.count("{") - text.count("}"))
                continue
            if re.search(r"\bexport\s+interface\s+\w+", text):
                in_export_shape = True
                in_export_type = False
                export_brace_depth = text.count("{") - text.count("}")
                if export_brace_depth <= 0:
                    export_brace_depth = 1
                continue
            if re.search(r"\bexport\s+type\s+\w+", text):
                in_export_shape = False
                in_export_type = True
                if text.endswith(";"):
                    in_export_type = False
                continue
            if in_export_shape:
                export_brace_depth += text.count("{") - text.count("}")
                if text == "}" or export_brace_depth <= 0:
                    in_export_shape = False
                    export_brace_depth = 0
                    if in_namespace:
                        namespace_brace_depth -= 1
                continue
            if in_export_type:
                if text.endswith(";"):
                    in_export_type = False
                continue
            if in_namespace:
                namespace_brace_depth += text.count("{") - text.count("}")
                if text == "}" or namespace_brace_depth <= 0:
                    in_namespace = False
                    namespace_brace_depth = 0
                continue
            if not text:
                continue
            if owner_field.match(text):
                continue
            if text.startswith(allowed_prefixes):
                continue
            if tsish.match(text):
                fragments.append((rel, line_no, text))
    return fragments


def use_annotation_diffs():
    """Validate structured `@use` / `Used by` doc comment pairs."""
    diffs = []
    use_re = re.compile(r"^🎯 @use:\s+\S.+$")
    old_use_re = re.compile(r"^🎯 @use\s+")
    used_by_re = re.compile(r"^Used by:\s+([A-Za-z0-9_/]+)(,\s*[A-Za-z0-9_/]+)*$")
    for item in rust_doc_items():
        doc = item["doc"]
        use_lines = [line for line in doc if "@use" in line["text"]]
        used_by_lines = [line for line in doc if line["text"].startswith("Used by:")]
        locn = f"{item['file']}:{item['line']}"
        if use_lines and not used_by_lines:
            diffs.append((item["name"], locn, ["@use must be paired with `Used by:`"]))
        if used_by_lines and not use_lines:
            diffs.append((item["name"], locn, ["`Used by:` requires `🎯 @use:`"]))
        for line in use_lines:
            if old_use_re.match(line["text"]) and not use_re.match(line["text"]):
                diffs.append(
                    (
                        item["name"],
                        f"{item['file']}:{line['line']}",
                        ["use structured format: `🎯 @use: ...`"],
                    )
                )
            elif not use_re.match(line["text"]):
                diffs.append(
                    (
                        item["name"],
                        f"{item['file']}:{line['line']}",
                        ["invalid @use format"],
                    )
                )
        for line in used_by_lines:
            if not used_by_re.match(line["text"]):
                diffs.append(
                    (
                        item["name"],
                        f"{item['file']}:{line['line']}",
                        ["invalid Used by format"],
                    )
                )
        if len(use_lines) > 1:
            diffs.append((item["name"], locn, ["multiple @use lines on one item"]))
        if len(used_by_lines) > 1:
            diffs.append((item["name"], locn, ["multiple Used by lines on one item"]))
    return diffs


def _module_for_file(rel):
    path = Path(rel)
    parts = list(path.with_suffix("").parts)
    if parts and parts[-1] == "mod":
        return parts[-2] if len(parts) > 1 else "wire"
    if parts and parts[0] == "tools":
        return parts[-1] if len(parts) > 1 else "tools"
    return parts[-1] if parts else ""


def _type_names_in_node(node):
    names = set()
    if node.type in ("type_identifier", "scoped_type_identifier"):
        names.add(node.text.decode("utf-8", errors="replace").split("::")[-1])
    for child in node.children:
        names |= _type_names_in_node(child)
    return names


def _type_names_from_text(text):
    if not text:
        return set()
    return set(re.findall(r"\b[A-Z][A-Za-z0-9_]*\b", text))


def rust_type_references():
    """Extract wire-internal type references from Rust type positions."""
    definitions = {}
    for item in rust_doc_items():
        definitions[item["name"]] = item
    defined_names = set(definitions)
    refs = {name: set() for name in defined_names}

    for f in sorted(PROTO_DIR.rglob("*.rs")):
        rel = str(f.relative_to(PROTO_DIR))
        module = _module_for_file(rel)
        buf = f.read_bytes()
        tree = RS.parse(buf)

        def visit(node, owner=None):
            current_owner = owner
            if node.type in ("struct_item", "enum_item", "type_item"):
                for child in node.children:
                    if child.type == "type_identifier":
                        current_owner = child.text.decode()
                        break

            if node.type in ("field_declaration", "enum_variant", "type_item"):
                for name in _type_names_in_node(node):
                    if name in defined_names and name != current_owner:
                        refs[name].add(
                            "self" if definitions[name]["file"] == rel else module
                        )

            for child in node.children:
                visit(child, current_owner)

        visit(tree.root_node)
    return refs


def use_reference_diffs():
    """Compare structured `Used by:` annotations with actual wire type references."""
    refs = rust_type_references()
    diffs = []
    for item in rust_doc_items():
        use_lines = [
            line for line in item["doc"] if line["text"].startswith("🎯 @use:")
        ]
        if not use_lines:
            continue
        used_by_lines = [
            line for line in item["doc"] if line["text"].startswith("Used by:")
        ]
        if not used_by_lines:
            continue
        annotated = {
            part.strip()
            for part in used_by_lines[0]["text"].removeprefix("Used by:").split(",")
            if part.strip()
        }
        actual = refs.get(item["name"], set())
        if annotated != actual:
            diffs.append(
                (
                    item["name"],
                    f"{item['file']}:{used_by_lines[0]['line']}",
                    [
                        f"Used by differs: actual `{', '.join(sorted(actual)) or '(none)'}` vs comment `{', '.join(sorted(annotated))}`"
                    ],
                )
            )
    return diffs


# ═══════════════════════════════════════════════════════════════
#  Rust type coverage extraction
# ═══════════════════════════════════════════════════════════════


def px_types():
    """Extract all pub types with fields, variants, deprecated, convert info."""
    types = {}
    for f in sorted(PROTO_DIR.rglob("*.rs")):
        rel = str(f.relative_to(PROTO_DIR.parent.parent.parent))
        buf = f.read_bytes()
        tree = RS.parse(buf)
        _collect_px(tree.root_node, buf, types, rel)
    return types


def _collect_px(node, buf, types, rel):
    for child in node.children:
        has_pub = any(
            gc.type == "visibility_modifier" and gc.text.decode() == "pub"
            for gc in child.children
        )
        if not has_pub or child.type not in ("struct_item", "enum_item", "type_item"):
            _collect_px(child, buf, types, rel)
            continue

        # Type name
        name = None
        for gc in child.children:
            if gc.type == "type_identifier":
                name = gc.text.decode()
                break
        if not name:
            _collect_px(child, buf, types, rel)
            continue

        kinds = {"struct_item": "struct", "enum_item": "enum", "type_item": "type"}
        info = dict(
            kind=kinds.get(child.type, "?"), line=child.start_point[0] + 1, file=rel
        )

        # #[deprecated] on preceding attributes
        deprecated = False
        cursor = child.prev_named_sibling
        while cursor and cursor.type == "attribute_item":
            if cursor.text.decode().startswith("#[deprecated"):
                deprecated = True
                break
            cursor = cursor.prev_named_sibling
        info["deprecated"] = deprecated

        # Struct fields
        if child.type == "struct_item":
            fields = []
            deprecated_fields = set()
            for gc in child.children:
                if gc.type == "field_declaration_list":
                    for field in gc.children:
                        if field.type == "field_declaration":
                            fname = None
                            is_dep = False
                            # Check #[deprecated] on field
                            fc = field.prev_named_sibling
                            while fc and fc.type == "attribute_item":
                                if fc.text.decode().startswith("#[deprecated"):
                                    is_dep = True
                                    break
                                fc = fc.prev_named_sibling
                            for fc2 in field.children:
                                if fc2.type == "field_identifier":
                                    fname = fc2.text.decode()
                                    break
                            if fname:
                                fields.append(fname)
                                if is_dep:
                                    deprecated_fields.add(fname)
            info["fields"] = fields
            info["deprecated_fields"] = deprecated_fields

        # Enum variants
        if child.type == "enum_item":
            variants = []
            for gc in child.children:
                if gc.type == "enum_variant_list":
                    for variant in gc.children:
                        if variant.type == "enum_variant":
                            for fc in variant.children:
                                if fc.type == "identifier":
                                    variants.append(fc.text.decode())
                                    break
            info["variants"] = variants

        # #[convert(from(...))] annotation
        convert_target = None
        cursor = child.prev_named_sibling
        while cursor and cursor.type == "attribute_item":
            text = cursor.text.decode()
            m = re.search(r"convert\(from\((\w+(?:::\w+)*)\)\)", text)
            if m:
                convert_target = m.group(1)
                break
            cursor = cursor.prev_named_sibling
        if convert_target:
            info["convert_from"] = convert_target

        types[name] = info
        _collect_px(child, buf, types, rel)


def px_enum_variants(file, enum):
    """Extract variant names from a Rust enum file."""
    buf = (PROTO_DIR / file).read_bytes()
    tree = RS.parse(buf)

    def find_enum(node):
        if node.type == "enum_item":
            for c in node.children:
                if c.type == "type_identifier" and c.text.decode() == enum:
                    return node
            return None
        for c in node.children:
            r = find_enum(c)
            if r:
                return r
        return None

    enum_node = find_enum(tree.root_node)
    if not enum_node:
        return []
    for child in enum_node.children:
        if child.type == "enum_variant_list":
            names = []
            for v in child.children:
                if v.type == "enum_variant":
                    for c in v.children:
                        if c.type == "identifier":
                            names.append(c.text.decode())
                            break
            return names
    return []


# ═══════════════════════════════════════════════════════════════
#  Normalization & output
# ═══════════════════════════════════════════════════════════════
