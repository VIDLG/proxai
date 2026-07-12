"""Human-readable rendering for OpenAI protocol comparison results."""

from .common import SCHEMA_PATH


def render_report(result):
    label = "Responses API" if result.protocol == "responses" else "Chat Completions"
    check_kind = "structural check" if result.structural else "required-field check"
    print(f"OpenAI schema {check_kind}: {label}")
    print(f"  Schema: {SCHEMA_PATH}")
    print(f"  Object schemas checked: {result.checked}")
    if not result.gaps:
        if result.structural:
            print("  OK: local wire structure matches all enforced official schema contracts")
        else:
            print("  OK: required, non-nullable schema fields are required locally")
        return

    print(f"  Gaps: {len(result.gaps)}")
    for type_name, field_name, reason in result.gaps:
        print(f"  - {type_name}.{field_name}: {reason}")
