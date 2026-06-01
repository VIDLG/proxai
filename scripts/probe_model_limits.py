import argparse
import json
import urllib.error
import urllib.request
from pathlib import Path
from typing import Any

import tomllib

DEFAULT_MODELS = ["gpt-5.5", "gpt-5.4", "gpt-5.3-codex"]
DEFAULT_STEPS = [
    32000,
    64000,
    96000,
    128000,
    160000,
    192000,
    224000,
    256000,
    288000,
    320000,
]
DEFAULT_MAX_OUTPUT_TOKENS = 128000
DEFAULT_TIMEOUT_SECONDS = 180
DEFAULT_PROMPT_TEXT = "x"
DEFAULT_REPEAT_FACTOR = 4


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


def upstream_base_url(cli_upstream: str | None, config: dict[str, Any]) -> str:
    value = (cli_upstream or default_openai_provider_value(config, "base_url")).strip()
    if not value:
        raise SystemExit(
            "error: provide --upstream or configure the default openai_responses provider base_url in config.toml"
        )
    return value.rstrip("/")


def request_url(base_url: str) -> str:
    return f"{base_url}/v1/responses"


def build_text(token_count: int, repeat_factor: int) -> str:
    # This is only an approximation of token count. The goal is to probe practical limits,
    # not to guarantee an exact tokenizer-aligned size. `repeat_factor` lets us build a
    # larger prompt without enormous step counts when testing very large windows.
    chunk = (DEFAULT_PROMPT_TEXT + " ") * repeat_factor
    return chunk * token_count


def build_payload(
    model: str, input_tokens: int, max_output_tokens: int, repeat_factor: int
) -> dict[str, Any]:
    return {
        "model": model,
        "stream": False,
        "max_output_tokens": max_output_tokens,
        "input": [
            {
                "type": "message",
                "role": "user",
                "content": [
                    {
                        "type": "input_text",
                        "text": build_text(input_tokens, repeat_factor),
                    }
                ],
            }
        ],
    }


def send_request(
    url: str, api_key: str | None, payload: dict[str, Any], timeout_seconds: int
) -> tuple[int, str]:
    headers = {
        "content-type": "application/json",
        "user-agent": "proxai-limit-probe",
    }
    if api_key:
        headers["authorization"] = f"Bearer {api_key}"

    request = urllib.request.Request(
        url,
        data=json.dumps(payload).encode("utf-8"),
        headers=headers,
        method="POST",
    )

    try:
        with urllib.request.urlopen(request, timeout=timeout_seconds) as response:
            body = response.read().decode("utf-8", errors="replace")
            return response.status, body
    except urllib.error.HTTPError as error:
        body = error.read().decode("utf-8", errors="replace")
        return error.code, body
    except TimeoutError:
        return 599, "request timed out"
    except OSError as error:
        return 598, str(error)


def summarize_body(body: str, limit: int = 240) -> str:
    compact = " ".join(body.split())
    return compact[:limit] + ("..." if len(compact) > limit else "")


def parse_models(value: str | None) -> list[str]:
    if not value:
        return DEFAULT_MODELS.copy()
    return [item.strip() for item in value.split(",") if item.strip()]


def parse_steps(value: str | None) -> list[int]:
    if not value:
        return DEFAULT_STEPS.copy()
    steps = [int(item.strip()) for item in value.split(",") if item.strip()]
    steps.sort()
    return steps


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Probe practical Responses API input limits for configured models."
    )
    parser.add_argument("--config", help="Path to config.toml")
    parser.add_argument(
        "--upstream",
        help="OpenAI-compatible upstream base URL. Defaults to the default openai_responses provider base_url.",
    )
    parser.add_argument("--api-key", help="Bearer API key for upstream requests")
    parser.add_argument("--models", help="Comma-separated model list")
    parser.add_argument("--steps", help="Comma-separated approximate input token steps")
    parser.add_argument(
        "--max-output-tokens",
        type=int,
        default=DEFAULT_MAX_OUTPUT_TOKENS,
        help="max_output_tokens to request during probing",
    )
    parser.add_argument(
        "--repeat-factor",
        type=int,
        default=DEFAULT_REPEAT_FACTOR,
        help="Multiply each approximate input step by this factor when building the prompt",
    )
    parser.add_argument(
        "--timeout-seconds",
        type=int,
        default=DEFAULT_TIMEOUT_SECONDS,
        help="Per-request timeout in seconds",
    )
    args = parser.parse_args()

    config_path = Path(args.config) if args.config else default_config_path()
    config = load_config(config_path)
    base_url = upstream_base_url(args.upstream, config)
    api_key = (
        args.api_key or default_openai_provider_value(config, "api_key")
    ).strip() or None
    models = parse_models(args.models)
    steps = parse_steps(args.steps)
    url = request_url(base_url)

    print(f"upstream: {base_url}")
    print(f"endpoint: {url}")
    print(f"models: {', '.join(models)}")
    print(f"steps: {', '.join(str(step) for step in steps)}")
    print(f"repeat_factor: {args.repeat_factor}")
    print(f"max_output_tokens: {args.max_output_tokens}")
    print()

    overall: list[dict[str, Any]] = []

    for model in models:
        print(f"== {model} ==")
        model_results: list[dict[str, Any]] = []
        for step in steps:
            payload = build_payload(
                model, step, args.max_output_tokens, args.repeat_factor
            )
            status, body = send_request(url, api_key, payload, args.timeout_seconds)
            ok = 200 <= status < 300
            summary = summarize_body(body)
            print(f"input≈{step:>6} -> {status} {'ok' if ok else 'fail'} :: {summary}")
            model_results.append(
                {
                    "approx_input_tokens": step,
                    "repeat_factor": args.repeat_factor,
                    "approx_effective_input_tokens": step * args.repeat_factor,
                    "status": status,
                    "ok": ok,
                    "body": body,
                }
            )
            if not ok:
                break
        print()
        overall.append({"model": model, "results": model_results})

    print("json_summary=")
    print(json.dumps(overall, ensure_ascii=False, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
