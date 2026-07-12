"""Rust wire-model AST indexing and type inspection."""

import re

from .common import RS

def _is_serde_tag_type(rust_type):
    return (
        isinstance(rust_type, tuple)
        and len(rust_type) == 2
        and rust_type[0] == "__serde_tag__"
    )

def _serde_tag_values(rust_type):
    if not _is_serde_tag_type(rust_type):
        return None
    return set(rust_type[1])

def _rust_array_item_identity(rust_type, enums=None):
    item_type = _rust_array_item_type(rust_type)
    if item_type is None:
        return None
    identity = _rust_type_identity(item_type)
    if enums is None or identity not in enums:
        return identity

    enum = enums[identity]
    payloads = set(enum["payloads"])
    if len(payloads) == 1:
        return next(iter(payloads))
    if not payloads:
        return "string"
    return identity

def _rust_array_item_base_type(rust_type):
    item_type = _rust_array_item_type(rust_type)
    return _rust_base_type(item_type) if item_type is not None else None

def _rust_array_item_type(rust_type):
    compact = _unwrap_rust_carriers(rust_type)
    match = re.fullmatch(r"Vec<(.+)>", compact)
    return match.group(1) if match is not None else None

def _rust_type_identity(rust_type):
    compact = _unwrap_rust_carriers(rust_type)
    base = compact.rsplit("::", 1)[-1]
    if base == "String" or compact in {"&str", "str"}:
        return "string"
    if base == "bool":
        return "boolean"
    if re.fullmatch(r"[ui](?:8|16|32|64|128|size)", base):
        return "integer"
    if base in {"f32", "f64"}:
        return "number"
    if base == "Value" or "<" in compact or ">" in compact:
        return None
    return base

def _unwrap_rust_carriers(rust_type):
    compact = re.sub(r"\s+", "", rust_type)
    while True:
        match = re.fullmatch(
            r"(?:[A-Za-z_][A-Za-z0-9_]*::)*(?:Option|Box|Nullable|RequiredNullable|OptionalNullable)<(.+)>",
            compact,
        )
        if match is None:
            return compact
        compact = match.group(1)

def _rust_builtin_shape(rust_type):
    compact = _unwrap_rust_carriers(rust_type)

    base = compact.rsplit("::", 1)[-1]
    if base == "String" or compact in {"&str", "str"}:
        return "string"
    if base == "bool":
        return "boolean"
    if re.fullmatch(r"[ui](?:8|16|32|64|128|size)", base):
        return "integer"
    if base in {"f32", "f64"}:
        return "number"
    if re.fullmatch(r"Vec<.+>", compact):
        return "array"
    if re.fullmatch(r"(?:[\w:]+::)?(?:HashMap|BTreeMap)<.+>", compact):
        return "object"
    return None

def _rust_base_type(rust_type):
    compact = re.sub(r"\s+", "", rust_type)
    while True:
        match = re.fullmatch(
            r"(?:[A-Za-z_][A-Za-z0-9_]*::)*(?:Option|Box|Nullable|RequiredNullable|OptionalNullable)<(.+)>",
            compact,
        )
        if match is None:
            break
        compact = match.group(1)
    return compact.rsplit("::", 1)[-1]

def _local_structs(directories):
    structs = {}
    field_attributes = {}
    provenance = {}
    enums = {}
    parsed_files = []
    for directory in directories:
        for path in sorted(directory.rglob("*.rs")):
            source = path.read_bytes()
            parsed_files.append((RS.parse(source), source))

    # A tagged enum and its payload struct commonly live in different modules.
    # Index all carriers first so traversal order cannot hide serde-generated
    # discriminator fields. Untagged carrier enums need a second indirection:
    # their selected tuple payload receives the outer tag too.
    for tree, source in parsed_files:
        _index_structs(
            tree.root_node,
            source,
            structs,
            provenance,
            field_attributes,
        )
        _index_enums(tree.root_node, source, enums)
    flattens = {
        type_name: [
            rust_type
            for field_name, rust_type in fields.items()
            if field_name.startswith("__serde_flatten__:")
        ]
        for type_name, fields in structs.items()
    }
    raw_structs = {name: dict(fields) for name, fields in structs.items()}
    _expand_flattened_struct_fields(structs)
    _expand_flattened_field_attributes(field_attributes, raw_structs)
    for tree, source in parsed_files:
        _index_tagged_enum_payloads(tree.root_node, source, structs, enums)
    return structs, provenance, flattens, enums, field_attributes

def _index_structs(node, source, structs, provenance, field_attributes=None):
    if node.type == "struct_item" and _is_public(node):
        name = _type_name(node)
        if name:
            structs[name] = _struct_fields(node)
            if field_attributes is not None:
                field_attributes[name] = _struct_field_attributes(node)
            pointer = _openapi_schema_pointer(node, source)
            if pointer:
                provenance[name] = pointer
    for child in node.children:
        _index_structs(child, source, structs, provenance, field_attributes)

def _openapi_schema_pointer(node, source):
    leading = source[: node.start_byte].decode()
    matches = list(
        re.finditer(
            r"^\s*/// OpenAPI schema:(?:\s*`([^`]+)`\s*$|\s*$\n\s*///\s*`([^`]+)`\s*$)",
            leading,
            re.MULTILINE,
        )
    )
    for match in reversed(matches):
        intervening = leading[match.end() :]
        if not re.search(r"\b(?:pub\s+)?(?:struct|enum)\b", intervening):
            return match.group(1) or match.group(2)
    return None

def _expand_flattened_struct_fields(structs):
    def expand(type_name, seen):
        if type_name in seen or type_name not in structs:
            return {}
        expanded = {}
        for name, rust_type in structs[type_name].items():
            if name.startswith("__serde_flatten__:"):
                flattened = expand(rust_type, seen | {type_name})
                if flattened:
                    expanded.update(flattened)
                else:
                    expanded[name.removeprefix("__serde_flatten__:")] = rust_type
            else:
                expanded[name] = rust_type
        return expanded

    for type_name in list(structs):
        structs[type_name] = expand(type_name, set())

def _expand_flattened_field_attributes(field_attributes, raw_structs):
    def expand(type_name, seen):
        if type_name in seen or type_name not in field_attributes:
            return {}
        expanded = {}
        for name, attributes in field_attributes[type_name].items():
            if name.startswith("__serde_flatten__:"):
                flattened_type = raw_structs.get(type_name, {}).get(name)
                if flattened_type is not None:
                    expanded.update(expand(flattened_type, seen | {type_name}))
            else:
                expanded[name] = attributes
        return expanded

    for type_name in list(field_attributes):
        field_attributes[type_name] = expand(type_name, set())

def _index_enums(node, source, enums):
    if node.type == "enum_item":
        name = _type_name(node)
        if name:
            payloads = []
            variant_payloads = {}
            wire_values = set()
            open_string = False
            rename_all = _serde_rename_all(node)
            tag = _serde_tag(node)
            for child in node.children:
                if child.type != "enum_variant_list":
                    continue
                for variant in child.children:
                    if variant.type != "enum_variant":
                        continue
                    variant_name = _variant_name(variant)
                    payload = _tuple_variant_payload_type(variant)
                    if variant_name and (payload is None or tag is not None):
                        wire_value = _serde_variant_name(
                            variant, variant_name, rename_all
                        )
                        wire_values.add(wire_value)
                        if payload is not None:
                            variant_payloads[wire_value] = payload
                    if payload:
                        payloads.append(payload)
                        if payload == "String" and _has_serde_flag(variant, "untagged"):
                            open_string = True
                        continue
            enums[name] = {
                "tag": tag,
                "untagged": _is_serde_untagged(node),
                "payloads": payloads,
                "variant_payloads": variant_payloads,
                "wire_values": wire_values,
                "open_string": open_string,
            }
    for child in node.children:
        _index_enums(child, source, enums)

def _index_tagged_enum_payloads(node, source, structs, enums):
    """Record serde tags carried by direct and untagged tuple payloads."""
    if node.type == "enum_item":
        tag = _serde_tag(node)
        if tag:
            rename_all = _serde_rename_all(node)
            for child in node.children:
                if child.type != "enum_variant_list":
                    continue
                for variant in child.children:
                    if variant.type == "enum_variant":
                        variant_name = _variant_name(variant)
                        if variant_name is None:
                            continue
                        wire_value = _serde_variant_name(
                            variant, variant_name, rename_all
                        )
                        _mark_serde_tag(
                            _tuple_variant_payload_type(variant),
                            tag,
                            wire_value,
                            structs,
                            enums,
                            set(),
                        )
    for child in node.children:
        _index_tagged_enum_payloads(child, source, structs, enums)

def _mark_serde_tag(payload, tag, wire_value, structs, enums, seen):
    if payload in structs:
        existing = structs[payload].get(tag)
        values = _serde_tag_values(existing) or set()
        values.add(wire_value)
        structs[payload][tag] = ("__serde_tag__", frozenset(values))
        return
    if payload in seen:
        return

    enum = enums.get(payload)
    if enum is None:
        return
    untagged = enum["untagged"]
    nested_payloads = enum["payloads"]
    if not untagged:
        return
    for nested_payload in nested_payloads:
        _mark_serde_tag(
            nested_payload,
            tag,
            wire_value,
            structs,
            enums,
            seen | {payload},
        )

def _serde_rename_all(node):
    attribute = node.prev_named_sibling
    while attribute is not None and attribute.type == "attribute_item":
        match = re.search(
            r'\bserde\s*\([^]]*\brename_all\s*=\s*"([^"]+)"',
            attribute.text.decode(),
        )
        if match:
            return match.group(1)
        attribute = attribute.prev_named_sibling
    return None

def _variant_name(variant):
    return next(
        (
            child.text.decode()
            for child in variant.children
            if child.type == "identifier"
        ),
        None,
    )

def _serde_variant_name(variant, variant_name, rename_all):
    attribute = variant.prev_named_sibling
    while attribute is not None and attribute.type == "attribute_item":
        match = re.search(
            r'\bserde\s*\([^]]*\brename\s*=\s*"([^"]+)"',
            attribute.text.decode(),
        )
        if match:
            return match.group(1)
        attribute = attribute.prev_named_sibling
    return _apply_rename_all(variant_name, rename_all)

def _apply_rename_all(name, rename_all):
    if rename_all is None:
        return name
    words = re.findall(
        r"[A-Z]+\d+(?=[A-Z]|$)|[A-Z]+(?=[A-Z][a-z]|$)|[A-Z]?[a-z]+\d*|\d+[a-zA-Z]*",
        name,
    )
    words = [word.lower() for word in words]
    if rename_all == "lowercase":
        return "".join(words)
    if rename_all == "snake_case":
        return "_".join(words)
    if rename_all == "kebab-case":
        return "-".join(words)
    if rename_all == "SCREAMING_SNAKE_CASE":
        return "_".join(words).upper()
    return name

def _has_serde_flag(node, flag):
    attribute = node.prev_named_sibling
    while attribute is not None and attribute.type == "attribute_item":
        if re.search(
            rf"\bserde\s*\([^]]*\b{re.escape(flag)}\b", attribute.text.decode()
        ):
            return True
        attribute = attribute.prev_named_sibling
    return False

def _serde_tag(node):
    attribute = node.prev_named_sibling
    while attribute is not None and attribute.type == "attribute_item":
        match = re.search(r'\bserde\s*\([^]]*\btag\s*=\s*"([^"]+)"', attribute.text.decode())
        if match:
            return match.group(1)
        attribute = attribute.prev_named_sibling
    return None

def _is_serde_untagged(node):
    attribute = node.prev_named_sibling
    while attribute is not None and attribute.type == "attribute_item":
        if re.search(r"\bserde\s*\([^]]*\buntagged\b", attribute.text.decode()):
            return True
        attribute = attribute.prev_named_sibling
    return False

def _tuple_variant_payload_type(variant):
    match = re.search(r"\(\s*([A-Za-z_][A-Za-z0-9_]*)\s*\)", variant.text.decode())
    return match.group(1) if match else None

def _is_public(node):
    return any(
        child.type == "visibility_modifier" and child.text.decode() == "pub"
        for child in node.children
    )

def _type_name(node):
    return next(
        (child.text.decode() for child in node.children if child.type == "type_identifier"),
        None,
    )

def _struct_fields(node):
    fields = {}
    for child in node.children:
        if child.type != "field_declaration_list":
            continue
        attributes = []
        for field in child.children:
            if field.type == "attribute_item":
                attributes.append(field.text.decode())
                continue
            if field.type != "field_declaration":
                continue
            name = next(
                (
                    part.text.decode()
                    for part in field.children
                    if part.type == "field_identifier"
                ),
                None,
            )
            if name:
                wire_name = _wire_field_name(name, attributes)
                if any(re.search(r"\bserde\s*\([^]]*\bflatten\b", attribute) for attribute in attributes):
                    wire_name = f"__serde_flatten__:{wire_name}"
                fields[wire_name] = field.named_children[-1].text.decode()
            attributes.clear()
    return fields

def _struct_field_attributes(node):
    field_attributes = {}
    for child in node.children:
        if child.type != "field_declaration_list":
            continue
        attributes = []
        for field in child.children:
            if field.type == "attribute_item":
                attributes.append(field.text.decode())
                continue
            if field.type != "field_declaration":
                continue
            name = next(
                (
                    part.text.decode()
                    for part in field.children
                    if part.type == "field_identifier"
                ),
                None,
            )
            if name:
                wire_name = _wire_field_name(name, attributes)
                if any(
                    re.search(r"\bserde\s*\([^]]*\bflatten\b", attribute)
                    for attribute in attributes
                ):
                    wire_name = f"__serde_flatten__:{wire_name}"
                field_attributes[wire_name] = list(attributes)
            attributes.clear()
    return field_attributes

def _wire_field_name(name, attributes):
    name = name.removeprefix("r#")
    for attribute in attributes:
        match = re.search(r'\bserde\s*\([^]]*\brename\s*=\s*"([^"]+)"', attribute)
        if match:
            return match.group(1)
    return name
