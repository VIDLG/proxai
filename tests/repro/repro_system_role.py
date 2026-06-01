from __future__ import annotations

import argparse
import json
import sys
import urllib.error
import urllib.request
from copy import deepcopy
from pathlib import Path
from typing import Any

import tomllib

PROJECT_ROOT = Path(__file__).resolve().parents[2]
DEFAULT_MODEL = "gpt-5.5"


def default_config_path() -> Path:
    return Path.home() / ".proxai" / "config.toml"


def load_config(path: Path) -> dict[str, Any]:
    try:
        with path.open("rb") as file:
            return tomllib.load(file)
    except FileNotFoundError:
        return {}


def default_openai_provider_name(config: dict[str, Any]) -> str:
    routing = config.get("routing")
    if isinstance(routing, dict):
        default_provider_names = routing.get("default_provider_names")
        if isinstance(default_provider_names, dict):
            value = default_provider_names.get("openai_responses")
            if isinstance(value, str):
                return value.strip()
    return ""


def provider_value(config: dict[str, Any], provider_name: str, key: str) -> str:
    providers = config.get("providers")
    if isinstance(providers, dict):
        provider = providers.get(provider_name)
        if isinstance(provider, dict):
            value = provider.get(key)
            if isinstance(value, str):
                return value.strip()
    return ""


def default_openai_provider_value(config: dict[str, Any], key: str) -> str:
    provider_name = default_openai_provider_name(config)
    if not provider_name:
        return ""
    return provider_value(config, provider_name, key)


def normalize_payload(value: Any) -> Any:
    if isinstance(value, list):
        return [normalize_payload(item) for item in value]
    if not isinstance(value, dict):
        return value

    normalized = {
        key: normalize_payload(item) for key, item in value.items() if key != "input"
    }
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
        if (
            isinstance(normalized_item, dict)
            and normalized_item.get("role") == "system"
        ):
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
            f"{extracted}\n\n{existing}"
            if isinstance(existing, str) and existing.strip()
            else extracted
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
    roles = [
        item.get("role") for item in payload.get("input", []) if isinstance(item, dict)
    ]
    print(f"\nCASE {name}")
    print("roles", roles)
    print("instructions", repr(payload.get("instructions")))
    print("status", status)
    print(body[:1200].replace("\n", "\\n"))


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Reproduce upstream handling of system input messages."
    )
    parser.add_argument("--config", help="Path to config.toml")
    parser.add_argument(
        "--url",
        help="Responses endpoint URL. Defaults to the default openai_responses provider base_url + /v1/responses.",
    )
    parser.add_argument(
        "--api-key",
        help="Bearer API key. Defaults to the default openai_responses provider api_key.",
    )
    parser.add_argument("--model", default=DEFAULT_MODEL)
    args = parser.parse_args()

    config_path = Path(args.config) if args.config else default_config_path()
    config = load_config(config_path)

    url = args.url
    if not url:
        base_url = default_openai_provider_value(config, "base_url")
        if base_url:
            url = base_url.rstrip("/") + "/v1/responses"
    if not url:
        raise SystemExit(
            "pass --url or configure the default openai_responses provider base_url in config.toml"
        )

    api_key = (args.api_key or default_openai_provider_value(config, "api_key")).strip()
    if not api_key:
        raise SystemExit(
            "set --api-key or configure the default openai_responses provider api_key in config.toml"
        )

    payload = {
        "model": args.model,
        "input": [
            {
                "type": "message",
                "role": "system",
                "content": [
                    {"type": "input_text", "text": "You are a terse test assistant."}
                ],
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
    print_case(
        url,
        api_key,
        "minimal_system_to_instructions",
        normalize_payload(deepcopy(payload)),
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
