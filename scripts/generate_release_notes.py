import argparse
import json
import os
import sys
import urllib.error
import urllib.request


DEFAULT_MODEL = "claude-sonnet-4-5-20250929"
ARTIFACT = "zed-openai-shim-windows-x86_64.zip"


def fallback_notes(tag: str, previous_tag: str, commits: str) -> str:
    title = f"# {tag}\n\n"
    if previous_tag:
        title += f"Changes since `{previous_tag}`.\n\n"
    return (
        title
        + "## Changes\n\n"
        + (commits.strip() or "- Initial release")
        + "\n\n## Download\n\n"
        + f"- Windows x86_64: `{ARTIFACT}`\n"
    )


def anthropic_url(base_url: str) -> str:
    base = base_url.rstrip("/")
    return f"{base}/messages" if base.endswith("/v1") else f"{base}/v1/messages"


def generate_notes(tag: str, previous_tag: str, commits: str) -> str | None:
    base_url = os.getenv("ANTHROPIC_BASE_URL", "").strip()
    api_key = os.getenv("ANTHROPIC_API_KEY", "").strip()
    model = os.getenv("ANTHROPIC_MODEL", "").strip() or DEFAULT_MODEL

    if not base_url or not api_key:
        return None

    prompt = f"""
Write concise GitHub Release notes in Markdown for zed-openai-shim.

Release tag: {tag}
Previous tag: {previous_tag or "none"}
Release artifact: {ARTIFACT}

Commit list:
{commits.strip() or "- Initial release"}

Rules:
- Write for users and developers, not as an internal commit dump.
- Use these sections exactly: "## Highlights", "## Changes", "## Download".
- Mention the Windows zip artifact in Download.
- Keep it concise.
- Do not include secrets, environment values, or raw URLs except artifact names.
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
            "anthropic-version": "2023-06-01",
        },
        method="POST",
    )

    try:
        with urllib.request.urlopen(request, timeout=60) as response:
            data = json.loads(response.read().decode("utf-8"))
    except (urllib.error.URLError, urllib.error.HTTPError, TimeoutError, json.JSONDecodeError) as error:
        print(f"warning: Anthropic release notes generation failed: {error}", file=sys.stderr)
        return None

    text_parts = [
        part.get("text", "")
        for part in data.get("content", [])
        if part.get("type") == "text" and part.get("text")
    ]
    notes = "\n".join(text_parts).strip()
    return notes or None


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--tag", required=True)
    parser.add_argument("--previous-tag", default="")
    parser.add_argument("--commits", required=True)
    parser.add_argument("--output", required=True)
    args = parser.parse_args()

    with open(args.commits, "r", encoding="utf-8") as file:
        commits = file.read()

    notes = generate_notes(args.tag, args.previous_tag, commits)
    if notes is None:
        notes = fallback_notes(args.tag, args.previous_tag, commits)

    with open(args.output, "w", encoding="utf-8", newline="\n") as file:
        file.write(notes.rstrip() + "\n")

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
