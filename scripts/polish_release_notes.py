import argparse
import json
import os
import shutil
import sys
import time
import urllib.error
import urllib.request

DEFAULT_MODEL = "claude-sonnet-4-5-20250929"


def env_value(name: str) -> str:
    value = os.getenv(name, "").strip()
    if value:
        return value

    try:
        with open(".env", "r", encoding="utf-8-sig") as file:
            for line in file:
                key, separator, raw_value = line.partition("=")
                if separator and key.strip() == name:
                    return raw_value.strip().strip('"').strip("'")
    except FileNotFoundError:
        pass

    return ""


def anthropic_url(base_url: str) -> str:
    base = base_url.rstrip("/")
    return f"{base}/messages" if base.endswith("/v1") else f"{base}/v1/messages"


def polish(raw_notes: str) -> str | None:
    base_url = env_value("ANTHROPIC_BASE_URL")
    api_key = env_value("ANTHROPIC_API_KEY")
    model = env_value("ANTHROPIC_MODEL") or DEFAULT_MODEL
    package_name = env_value("RELEASE_PACKAGE_NAME") or "proxai-<tag>-windows-x86_64"

    if not base_url or not api_key:
        print(
            "warning: Anthropic release notes env is not configured; using raw notes",
            file=sys.stderr,
        )
        return None

    prompt = f"""
Rewrite these structured release notes into concise GitHub Release notes for proxai.

Keep the facts exactly the same. Do not invent changes.
Keep Markdown.
Use these sections:
- ## Highlights
- ## Changes
- ## Download

Requirements:
- Mention `{package_name}.exe` in Download.
- Make the wording useful to users and developers.
- Do not include secrets, environment values, or private URLs.
- Keep it short.

Raw release notes:
{raw_notes}
""".strip()

    payload = {
        "model": model,
        "max_tokens": 1200,
        "temperature": 0.2,
        "messages": [{"role": "user", "content": prompt}],
    }
    request = urllib.request.Request(
        anthropic_url(base_url),
        data=json.dumps(payload).encode("utf-8"),
        headers={
            "content-type": "application/json",
            "x-api-key": api_key,
            "authorization": f"Bearer {api_key}",
            "anthropic-version": "2023-06-01",
            "user-agent": "proxai-release-notes",
        },
        method="POST",
    )

    last_error: Exception | None = None
    for attempt in range(1, 4):
        try:
            with urllib.request.urlopen(request, timeout=60) as response:
                data = json.loads(response.read().decode("utf-8"))
            break
        except Exception as error:
            last_error = error
            print(
                f"warning: release notes AI polish attempt {attempt} failed: {error}",
                file=sys.stderr,
            )
            time.sleep(attempt * 2)
    else:
        print(f"warning: release notes AI polish failed: {last_error}", file=sys.stderr)
        return None

    parts = [
        part.get("text", "")
        for part in data.get("content", [])
        if part.get("type") == "text" and part.get("text")
    ]
    polished = "\n".join(parts).strip()
    return polished or None


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--input", required=True)
    parser.add_argument("--output", required=True)
    args = parser.parse_args()

    with open(args.input, "r", encoding="utf-8") as file:
        raw_notes = file.read()

    polished = polish(raw_notes)
    if polished is None:
        if env_value("RELEASE_NOTES_REQUIRE_AI").lower() == "true":
            print(
                "error: release notes AI polish is required but failed", file=sys.stderr
            )
            return 1
        shutil.copyfile(args.input, args.output)
        return 0

    with open(args.output, "w", encoding="utf-8", newline="\n") as file:
        file.write(polished.rstrip() + "\n")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
