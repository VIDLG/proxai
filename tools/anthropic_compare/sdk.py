import re

from .common import TS

# ═══════════════════════════════════════════════════════════════
#  SDK (TypeScript) extraction
# ═══════════════════════════════════════════════════════════════

def sdk_types(text):
    """Extract all exported types with fields, variants, deprecated info."""
    tree = TS.parse(bytes(text, 'utf-8'))
    types = {}
    _walk(tree.root_node, types)
    return types


def _walk(node, types, ns=''):
    if node.type == 'export_statement':
        _record(node, types, ns)
        _walk_ns_body(node, types, ns)
    else:
        for c in node.children:
            _walk(c, types, ns)


def _walk_ns_body(node, types, outer_ns):
    """Walk export namespace { ... } blocks."""
    block = None
    for c in node.children:
        if c.type in ('module', 'internal_module', 'statement_block'):
            block = c
            break
    if block is None:
        return
    name = None
    for c in block.children:
        if c.type == 'identifier':
            name = c.text.decode()
            break
    if name is None:
        return
    full_ns = f"{outer_ns}.{name}" if outer_ns else name
    def _inner(node):
        for c in node.children:
            if c.type == 'export_statement':
                _record(c, types, full_ns)
                _walk_ns_body(c, types, full_ns)
            _inner(c)
    _inner(block)


def _record(node, types, ns):
    """Record a single export statement: type/interface/class, its fields, deprecated."""
    src = node.text.decode('utf-8') if node.text else ''
    m = re.search(r'\b(type|interface|class)\s+(\w+)', src)
    if not m:
        return
    kind, name = m.group(1), m.group(2)
    tag = f"{ns}.{name}" if ns else name
    info = dict(kind=kind, line=node.start_point[0] + 1, ns=ns)

    # Detect simple `export type X = Y;` aliases — record the RHS target
    # so the classifier can skip aliases of already-matched types.
    alias_of = None
    if kind == 'type':
        alias_m = re.search(rf'\btype\s+{re.escape(name)}\s*=\s*(\w+)\s*;', src)
        if alias_m:
            alias_of = alias_m.group(1)
            info['alias_of'] = alias_of
            info['alias_name'] = name

    # Extract fields for interfaces
    if kind == 'interface':
        fields = _ts_interface_fields(node)
        info['fields'] = [f['name'] for f in fields]
        info['deprecated_fields'] = {f['name'] for f in fields if f['deprecated']}
    else:
        info['fields'] = []
        info['deprecated_fields'] = set()

    types[tag] = info


def _ts_interface_fields(interface_node):
    """Extract field names and deprecated status from a TS interface node."""
    fields = []
    # Walk through export_statement → interface_declaration → interface_body
    # (tree-sitter TS grammar: interface_body is the { ... } block).
    def _find_body(node):
        if node.type == 'interface_body':
            return node
        for c in node.children:
            r = _find_body(c)
            if r:
                return r
        return None
    body = _find_body(interface_node)
    if body is None:
        return fields
    for prop in body.children:
        if prop.type != 'property_signature':
            continue
        deprecated = False
        # Check for @deprecated in preceding comments
        cursor = prop.prev_named_sibling
        while cursor is not None:
            if cursor.type == 'comment':
                if '@deprecated' in cursor.text.decode():
                    deprecated = True
                    break
            elif cursor.type not in ('property_signature',):
                break
            cursor = cursor.prev_named_sibling
        # Also check directly preceding comment siblings
        if not deprecated:
            ps = prop.prev_sibling
            while ps is not None:
                if ps.type == 'comment' and '@deprecated' in (ps.text or b'').decode('utf-8', errors='replace'):
                    deprecated = True
                    break
                if ps.type not in ('comment',):
                    break
                ps = ps.prev_sibling
        # Get field name
        for gc in prop.children:
            if gc.type == 'property_identifier':
                fields.append({'name': gc.text.decode(), 'deprecated': deprecated})
                break
    return fields


def sdk_tool_union(text):
    m = re.search(r'export type ToolUnion =[\s\S]*?;', text)
    if m:
        return re.findall(r'(?<=\|\s)\w+|(?<=^type ToolUnion = )\w+', m.group(0))
    return []


# ═══════════════════════════════════════════════════════════════
#  TS shape comments
# ═══════════════════════════════════════════════════════════════

def _strip_ts_comments(text):
    text = re.sub(r'/\*[\s\S]*?\*/', '', text)
    return re.sub(r'//.*', '', text)


def _compact_type(text):
    return re.sub(r'\s+', ' ', text).strip().rstrip(';')


def _split_top_level_union(text):
    """Split `A | Array<B | C>` on top-level union bars."""
    parts = []
    cur = []
    angle = paren = brace = bracket = 0
    for ch in text:
        if ch == '<':
            angle += 1
        elif ch == '>' and angle:
            angle -= 1
        elif ch == '(':
            paren += 1
        elif ch == ')' and paren:
            paren -= 1
        elif ch == '{':
            brace += 1
        elif ch == '}' and brace:
            brace -= 1
        elif ch == '[':
            bracket += 1
        elif ch == ']' and bracket:
            bracket -= 1
        if ch == '|' and not any((angle, paren, brace, bracket)):
            part = _compact_type(''.join(cur))
            if part:
                parts.append(part)
            cur = []
        else:
            cur.append(ch)
    part = _compact_type(''.join(cur))
    if part:
        parts.append(part)
    return parts


def _normalize_ts_type(text):
    text = _compact_type(text)
    text = re.sub(r'\bMessagesAPI\.', '', text)
    text = text.replace('ReadonlyArray<', 'Array<')
    text = re.sub(r'\bunknown\[\]', 'Array<unknown>', text)
    text = re.sub(r'\b([A-Za-z_][\w.]*)\[\]', r'Array<\1>', text)
    parts = _split_top_level_union(text)
    if len(parts) > 1:
        return ' | '.join(parts)
    return text


def _parse_interface_shape(src):
    src = _strip_ts_comments(src)
    m = re.search(r'export\s+interface\s+(\w+)(?:\s+extends\s+([^{]+))?\s*\{([\s\S]*)\}\s*$', src.strip())
    if not m:
        return None
    name = m.group(1)
    extends = _compact_type(m.group(2) or '')
    body = m.group(3)
    fields = []
    cur = []
    depth = 0
    for ch in body:
        if ch in '<({[':
            depth += 1
        elif ch in '>)}]' and depth:
            depth -= 1
        cur.append(ch)
        if ch == ';' and depth == 0:
            prop = _compact_type(''.join(cur))
            cur = []
            pm = re.match(r'([A-Za-z_$][\w$]*)(\?)?\s*:\s*([\s\S]+);?$', prop)
            if pm:
                fields.append({
                    'name': pm.group(1),
                    'optional': bool(pm.group(2)),
                    'type': _normalize_ts_type(pm.group(3)),
                })
    return {'kind': 'interface', 'name': name, 'extends': extends, 'fields': fields}


def _parse_type_shape(src):
    src = _strip_ts_comments(src)
    m = re.search(r'export\s+type\s+(\w+)\s*=\s*([\s\S]*?)\s*;\s*$', src.strip())
    if not m:
        return None
    rhs = _normalize_ts_type(m.group(2))
    return {'kind': 'type', 'name': m.group(1), 'rhs': rhs}


def _parse_ts_shape(src):
    src = src.strip()
    if re.search(r'export\s+interface\s+\w+', src):
        return _parse_interface_shape(src)
    if re.search(r'export\s+type\s+\w+', src):
        return _parse_type_shape(src)
    return None


def sdk_comment_shapes(text):
    """Extract comparable SDK export shapes without JSDoc/comment trivia."""
    shapes = {}
    clean = _strip_ts_comments(text)
    for m in re.finditer(r'export\s+interface\s+\w+', clean):
        start = m.start()
        brace = clean.find('{', m.end())
        if brace < 0:
            continue
        depth = 0
        end = None
        for i in range(brace, len(clean)):
            if clean[i] == '{':
                depth += 1
            elif clean[i] == '}':
                depth -= 1
                if depth == 0:
                    end = i + 1
                    break
        if end is None:
            continue
        shape = _parse_interface_shape(clean[start:end])
        if shape:
            shapes[shape['name']] = shape
    for m in re.finditer(r'export\s+type\s+\w+\s*=', clean):
        start = m.start()
        depth = 0
        end = None
        for i in range(m.end(), len(clean)):
            ch = clean[i]
            if ch in '<({[':
                depth += 1
            elif ch in '>)}]' and depth:
                depth -= 1
            elif ch == ';' and depth == 0:
                end = i + 1
                break
        if end is None:
            continue
        shape = _parse_type_shape(clean[start:end])
        if shape:
            shapes.setdefault(shape['name'], shape)
    return shapes
