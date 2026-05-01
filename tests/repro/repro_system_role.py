from __future__ import annotations

import argparse
import json
import os
import sys
import urllib.error
import urllib.request
from copy import deepcopy
from pathlib import Path
from typing import Any


PROJECT_ROOT = Path(__file__).resolve().parents[2]
DEFAULT_MODEL = "gpt-5.5"


def load_dotenv(path: Path) -> None:
    if not path.exists():
        return
    for line in path.read_text(encoding="utf-8").splitlines():
        line = line.strip()
        if not line or line.startswith("#") or "=" not in line:
            continue
        key, value = line.split("=", 1)
        os.environ.setdefault(key.strip(), value.strip().strip('"').strip("'"))


def normalize_payload(value: Any) -> Any:
    if isinstance(value, list):
        return [normalize_payload(item) for item in value]
    if not isinstance(value, dict):
        return value

    normalized = {key: normalize_payload(item) for key, item in value.items() if key != "input"}
    input_value = value.get("input")
    if not isinstance(input_value, list):
        if "input" in value:
            normalized["input"] = normalize_payload(input_value)
        return normalized

    remaining = []
    system_texts = []
    changed = False
    for item in input_value:
        normalized_item = normalize_payload(item)
        if isinstance(normalized_item, dict) and normalized_item.get("role") == "system":
            text = extract_text(normalized_item.get("content"))
            if text:
                system_texts.append(text)
            changed = True
        else:
            remaining.append(normalized_item)

    normalized["input"] = remaining
    if changed and system_texts:
        extracted = "\n\n".join(system_texts)
        existing = normalized.get("instructions")
        normalized["instructions"] = (
            f"{extracted}\n\n{existing}" if isinstance(existing, str) and existing.strip() else extracted
        )
    return normalized


def extract_text(content: Any) -> str | None:
    if isinstance(content, str):
        return content or None
    if not isinstance(content, list):
        return None

    parts = []
    for part in content:
        if not isinstance(part, dict):
            continue
        if part.get("type") not in {"input_text", "text"}:
            continue
        text = part.get("text")
        if isinstance(text, str):
            parts.append(text)
    return "".join(parts) or None


def post_json(url: str, api_key: str, payload: dict[str, Any]) -> tuple[int, str]:
    data = json.dumps(payload).encode("utf-8")
    request = urllib.request.Request(
        url,
        data=data,
        headers={
            "Authorization": f"Bearer {api_key}",
            "Content-Type": "application/json",
            "Accept": "*/*",
        },
        method="POST",
    )
    try:
        with urllib.request.urlopen(request, timeout=60) as response:
            return response.status, response.read().decode("utf-8", errors="replace")
    except urllib.error.HTTPError as error:
        return error.code, error.read().decode("utf-8", errors="replace")


def print_case(url: str, api_key: str, name: str, payload: dict[str, Any]) -> None:
    status, body = post_json(url, api_key, payload)
    roles = [item.get("role") for item in payload.get("input", []) if isinstance(item, dict)]
    print(f"\nCASE {name}")
    print("roles", roles)
    print("instructions", repr(payload.get("instructions")))
    print("status", status)
    print(body[:1200].replace("\n", "\\n"))


def main() -> int:
    load_dotenv(PROJECT_ROOT / ".env")

    parser = argparse.ArgumentParser(description="Reproduce upstream handling of system input messages.")
    parser.add_argument("--url", help="Responses endpoint URL. Defaults to OPENAI_SHIM_UPSTREAM + /v1/responses.")
    parser.add_argument("--model", default=DEFAULT_MODEL)
    args = parser.parse_args()

    url = args.url
    if not url and os.environ.get("OPENAI_SHIM_UPSTREAM"):
        url = os.environ["OPENAI_SHIM_UPSTREAM"].rstrip("/") + "/v1/responses"
    if not url:
        raise SystemExit("pass --url or set OPENAI_SHIM_UPSTREAM")

    api_key = os.environ.get("OPENAI_SHIM_API_KEY")
    if not api_key:
        raise SystemExit("OPENAI_SHIM_API_KEY is not set")

    payload = {
        "model": args.model,
        "input": [
            {
                "type": "message",
                "role": "system",
                "content": [{"type": "input_text", "text": "You are a terse test assistant."}],
            },
            {
                "type": "message",
                "role": "user",
                "content": [{"type": "input_text", "text": "Reply with ok."}],
            },
        ],
        "stream": True,
        "max_output_tokens": 64,
    }

    print_case(url, api_key, "minimal_system", payload)
    print_case(url, api_key, "minimal_system_to_instructions", normalize_payload(deepcopy(payload)))
    return 0


if __name__ == "__main__":
    sys.exit(main())
