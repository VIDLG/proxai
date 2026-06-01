from __future__ import annotations

import argparse
import os
import signal
import socket
import subprocess
import sys
import time
import urllib.error
import urllib.request
from pathlib import Path
from typing import TypedDict

PROJECT_ROOT = Path(__file__).resolve().parents[2]
DEFAULT_PAYLOAD = (
    PROJECT_ROOT / "tests/repro/fixtures/proxai_tool_delta_timeout.local.json"
)


class ReplayResult(TypedDict):
    attempt: int
    status: int | None
    body_bytes: int
    has_delta: bool
    has_done: bool
    has_completed: bool
    incomplete_tool_delta: bool
    body: Path
    log: Path


def load_dotenv(path: Path) -> None:
    if not path.exists():
        return
    for line in path.read_text(encoding="utf-8").splitlines():
        line = line.strip()
        if not line or line.startswith("#") or "=" not in line:
            continue
        key, value = line.split("=", 1)
        os.environ.setdefault(key.strip(), value.strip().strip('"').strip("'"))


def wait_for_server(host: str, port: int, timeout: float) -> None:
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        try:
            socket.create_connection((host, port), timeout=1).close()
            return
        except OSError:
            time.sleep(0.2)
    raise TimeoutError(f"shim did not start within {timeout:.0f}s")


def post_payload(
    url: str, payload: Path, output: Path, timeout: int
) -> tuple[int | None, bytes]:
    request = urllib.request.Request(
        url,
        data=payload.read_bytes(),
        headers={"content-type": "application/json"},
        method="POST",
    )
    status = None
    chunks: list[bytes] = []
    deadline = time.monotonic() + timeout
    try:
        with urllib.request.urlopen(request, timeout=min(timeout, 5)) as response:
            status = response.status
            while time.monotonic() < deadline:
                chunk = response.read(65536)
                if not chunk:
                    break
                chunks.append(chunk)
    except (TimeoutError, socket.timeout):
        pass
    except urllib.error.HTTPError as error:
        status = error.code
        chunks.append(error.read())

    body = b"".join(chunks)
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_bytes(body)
    return status, body


def has(body: bytes, needle: str) -> bool:
    return needle.encode("utf-8") in body


def run_once(args: argparse.Namespace, attempt: int) -> ReplayResult:
    exe = PROJECT_ROOT / "target/debug/proxai.exe"
    if not exe.exists():
        raise SystemExit(f"{exe} does not exist; run `pixi run -- cargo build` first")

    logs_dir = PROJECT_ROOT / "logs"
    logs_dir.mkdir(exist_ok=True)
    replay_dir = PROJECT_ROOT / "target/replay"
    replay_dir.mkdir(parents=True, exist_ok=True)
    suffix = f".attempt-{attempt}" if args.attempts > 1 else ""
    stdout_path = logs_dir / f"replay-capture{suffix}.log"
    stderr_path = logs_dir / f"replay-capture{suffix}.err.log"
    output_path = replay_dir / f"replay-capture{suffix}.sse"

    with stdout_path.open("wb") as stdout, stderr_path.open("wb") as stderr:
        process = subprocess.Popen(
            [str(exe), "--port", str(args.port)],
            cwd=PROJECT_ROOT,
            stdout=stdout,
            stderr=stderr,
            creationflags=subprocess.CREATE_NO_WINDOW if sys.platform == "win32" else 0,
        )
        try:
            wait_for_server("127.0.0.1", args.port, 5)
            status, body = post_payload(
                f"http://127.0.0.1:{args.port}/v1/responses",
                args.payload,
                output_path,
                args.timeout,
            )
        finally:
            if process.poll() is None:
                process.send_signal(signal.SIGTERM)
                try:
                    process.wait(timeout=5)
                except subprocess.TimeoutExpired:
                    process.kill()

    incomplete_tool_delta = (
        has(body, "response.function_call_arguments.delta")
        and not has(body, "response.function_call_arguments.done")
        and not has(body, "response.completed")
    )
    result: ReplayResult = {
        "attempt": attempt,
        "status": status,
        "body_bytes": len(body),
        "has_delta": has(body, "response.function_call_arguments.delta"),
        "has_done": has(body, "response.function_call_arguments.done"),
        "has_completed": has(body, "response.completed"),
        "incomplete_tool_delta": incomplete_tool_delta,
        "body": output_path,
        "log": stdout_path,
    }
    return result


def main() -> int:
    load_dotenv(PROJECT_ROOT / ".env")

    parser = argparse.ArgumentParser(
        description="Replay a captured Responses request through the shim."
    )
    parser.add_argument("--payload", type=Path, default=DEFAULT_PAYLOAD)
    parser.add_argument("--port", type=int, default=18081)
    parser.add_argument("--timeout", type=int, default=120)
    parser.add_argument("--attempts", type=int, default=1)
    args = parser.parse_args()

    print(f"payload={args.payload}")
    incomplete_count = 0
    for attempt in range(1, args.attempts + 1):
        result = run_once(args, attempt)
        incomplete_count += int(result["incomplete_tool_delta"])
        print(
            " ".join(
                [
                    f"attempt={result['attempt']}",
                    f"status={result['status']}",
                    f"body_bytes={result['body_bytes']}",
                    f"has_delta={result['has_delta']}",
                    f"has_done={result['has_done']}",
                    f"has_completed={result['has_completed']}",
                    f"incomplete_tool_delta={result['incomplete_tool_delta']}",
                    f"body={result['body']}",
                    f"log={result['log']}",
                ]
            )
        )
    print(f"incomplete_attempts={incomplete_count}/{args.attempts}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
