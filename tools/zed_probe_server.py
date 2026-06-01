#!/usr/bin/env python3
"""Local server for verifying and capturing whether Zed reaches localhost.

This intentionally does not proxy upstream. It captures inbound request
metadata/body and the synthetic response, then returns a tiny
OpenAI-compatible response.
"""

from __future__ import annotations

import argparse
import json
import re
import threading
import time
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path
from typing import Any

SENSITIVE_HEADERS = {
    "authorization",
    "proxy-authorization",
    "x-api-key",
    "api-key",
    "anthropic-api-key",
    "cookie",
    "set-cookie",
}


class ProbeHandler(BaseHTTPRequestHandler):
    server_version = "proxai-zed-probe/0.1"

    def do_GET(self) -> None:
        capture = self._capture_request(None)
        self._send_json({"ok": True, "path": self.path}, capture=capture)

    def do_OPTIONS(self) -> None:
        capture = self._capture_request(None)
        self._send_empty(status=204, capture=capture)

    def do_POST(self) -> None:
        raw_body = self.rfile.read(self._content_length())
        payload = self._decode_json(raw_body)
        capture = self._capture_request(raw_body)

        if self.path.endswith("/chat/completions"):
            if payload.get("stream") is True:
                self._send_sse(
                    [
                        {
                            "id": "zed_probe_chat",
                            "object": "chat.completion.chunk",
                            "created": int(time.time()),
                            "model": payload.get("model", "zed-probe"),
                            "choices": [
                                {
                                    "index": 0,
                                    "delta": {"content": "zed probe ok"},
                                    "finish_reason": None,
                                }
                            ],
                        },
                        {
                            "id": "zed_probe_chat",
                            "object": "chat.completion.chunk",
                            "created": int(time.time()),
                            "model": payload.get("model", "zed-probe"),
                            "choices": [
                                {
                                    "index": 0,
                                    "delta": {},
                                    "finish_reason": "stop",
                                }
                            ],
                        },
                    ],
                    capture=capture,
                )
            else:
                self._send_json(
                    {
                        "id": "zed_probe_chat",
                        "object": "chat.completion",
                        "created": int(time.time()),
                        "model": payload.get("model", "zed-probe"),
                        "choices": [
                            {
                                "index": 0,
                                "message": {
                                    "role": "assistant",
                                    "content": "zed probe ok",
                                },
                                "finish_reason": "stop",
                            }
                        ],
                    },
                    capture=capture,
                )
            return

        if self.path.endswith("/responses"):
            if payload.get("stream") is True:
                self._send_sse(
                    [
                        {
                            "type": "response.created",
                            "response": self._response_payload(
                                payload, status="in_progress", text=None
                            ),
                        },
                        {
                            "type": "response.output_text.delta",
                            "item_id": "msg_zed_probe",
                            "output_index": 0,
                            "content_index": 0,
                            "delta": "zed probe ok",
                        },
                        {
                            "type": "response.completed",
                            "response": self._response_payload(payload),
                        },
                    ],
                    capture=capture,
                )
            else:
                self._send_json(self._response_payload(payload), capture=capture)
            return

        self._send_json(
            {
                "error": {
                    "message": f"unsupported probe path: {self.path}",
                    "type": "invalid_request_error",
                }
            },
            status=404,
            capture=capture,
        )

    def log_message(self, fmt: str, *args: Any) -> None:
        return

    def _content_length(self) -> int:
        value = self.headers.get("Content-Length")
        if not value:
            return 0
        try:
            return int(value)
        except ValueError:
            return 0

    def _decode_json(self, body: bytes) -> dict[str, Any]:
        try:
            payload = json.loads(body.decode("utf-8")) if body else {}
        except Exception:
            return {}
        return payload if isinstance(payload, dict) else {}

    def _capture_request(self, raw_body: bytes | None) -> dict[str, Any]:
        now = time.strftime("%Y-%m-%dT%H:%M:%S%z")
        request_id = self.server.next_request_id()
        capture_dir = Path(getattr(self.server, "probe_capture_dir"))
        request_dir = capture_dir / request_id
        request_dir.mkdir(parents=True, exist_ok=True)

        inbound_meta_path = request_dir / "inbound_request.json"
        inbound_body_path = request_dir / "inbound_request.body"
        inbound_body_json_path = request_dir / "inbound_request.body.json"

        if raw_body is not None:
            inbound_body_path.write_bytes(raw_body)
            parsed = self._decode_json(raw_body)
            if parsed:
                inbound_body_json_path.write_text(
                    json.dumps(parsed, ensure_ascii=False, indent=2) + "\n",
                    encoding="utf-8",
                )

        body_preview = ""
        if raw_body is not None:
            body_preview = raw_body[: self.server.probe_body_preview_bytes].decode(
                "utf-8",
                errors="replace",
            )

        headers = self._sanitized_headers()
        inbound_meta = {
            "request_id": request_id,
            "time": now,
            "client": self.client_address[0],
            "method": self.command,
            "path": self.path,
            "headers": headers,
            "content_length": self.headers.get("Content-Length"),
            "authorization_present": "Authorization" in self.headers,
            "body_path": str(inbound_body_path) if raw_body is not None else None,
            "body_json_path": str(inbound_body_json_path)
            if inbound_body_json_path.exists()
            else None,
        }
        inbound_meta_path.write_text(
            json.dumps(inbound_meta, ensure_ascii=False, indent=2) + "\n",
            encoding="utf-8",
        )

        record = {
            "request_id": request_id,
            "time": now,
            "client": self.client_address[0],
            "method": self.command,
            "path": self.path,
            "content_length": self.headers.get("Content-Length"),
            "authorization_present": "Authorization" in self.headers,
            "content_type": self.headers.get("Content-Type"),
            "accept": self.headers.get("Accept"),
            "user_agent": self.headers.get("User-Agent"),
            "capture_dir": str(request_dir),
            "inbound_request": str(inbound_meta_path),
            "body_preview": body_preview,
        }
        line = json.dumps(record, ensure_ascii=False)
        print(line, flush=True)
        log_path = getattr(self.server, "probe_log_path", None)
        if log_path:
            with Path(log_path).open("a", encoding="utf-8") as file:
                file.write(line + "\n")
        return {"request_id": request_id, "request_dir": request_dir}

    def _send_json(
        self,
        payload: dict[str, Any],
        status: int = 200,
        capture: dict[str, Any] | None = None,
    ) -> None:
        body = json.dumps(payload, ensure_ascii=False).encode("utf-8")
        headers = {
            "Content-Type": "application/json",
            "Content-Length": str(len(body)),
        }
        self._send_headers(status, headers)
        self._capture_response(capture, status, headers, body)
        self.wfile.write(body)

    def _send_sse(
        self,
        events: list[dict[str, Any]],
        capture: dict[str, Any] | None = None,
    ) -> None:
        body = b"".join(
            b"data: "
            + json.dumps(event, ensure_ascii=False).encode("utf-8")
            + b"\n\n"
            for event in events
        )
        body += b"data: [DONE]\n\n"
        headers = {
            "Content-Type": "text/event-stream",
            "Cache-Control": "no-cache",
            "Content-Length": str(len(body)),
        }
        self._send_headers(200, headers)
        self._capture_response(capture, 200, headers, body)
        self.wfile.write(body)

    def _send_empty(
        self,
        status: int,
        capture: dict[str, Any] | None = None,
    ) -> None:
        headers = {"Content-Length": "0"}
        self._send_headers(status, headers)
        self._capture_response(capture, status, headers, b"")

    def _send_headers(self, status: int, headers: dict[str, str]) -> None:
        self.send_response(status)
        for key, value in headers.items():
            self.send_header(key, value)
        self.end_headers()

    def _capture_response(
        self,
        capture: dict[str, Any] | None,
        status: int,
        headers: dict[str, str],
        body: bytes,
    ) -> None:
        if capture is None:
            return
        request_dir = Path(capture["request_dir"])
        response_meta_path = request_dir / "outbound_response.json"
        response_body_path = request_dir / "outbound_response.body"
        response_body_path.write_bytes(body)
        response_meta = {
            "request_id": capture["request_id"],
            "status": status,
            "headers": headers,
            "body_path": str(response_body_path),
        }
        response_meta_path.write_text(
            json.dumps(response_meta, ensure_ascii=False, indent=2) + "\n",
            encoding="utf-8",
        )

    def _sanitized_headers(self) -> dict[str, str]:
        sanitized = {}
        for key, value in self.headers.items():
            if key.lower() in SENSITIVE_HEADERS:
                sanitized[key] = "[redacted]"
            else:
                sanitized[key] = value
        return sanitized

    def _response_payload(
        self,
        request: dict[str, Any],
        *,
        status: str = "completed",
        text: str | None = "zed probe ok",
    ) -> dict[str, Any]:
        content = []
        if text is not None:
            content.append({"type": "output_text", "text": text, "annotations": []})
        return {
            "id": "resp_zed_probe",
            "object": "response",
            "created_at": int(time.time()),
            "status": status,
            "model": request.get("model", "zed-probe"),
            "output": [
                {
                    "id": "msg_zed_probe",
                    "type": "message",
                    "status": status,
                    "role": "assistant",
                    "content": content,
                }
            ],
            "parallel_tool_calls": True,
            "tool_choice": "auto",
            "tools": [],
        }


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--host", default="127.0.0.1")
    parser.add_argument("--port", type=int, default=18080)
    parser.add_argument("--log", default="D:/tmp/zed-probe-requests.ndjson")
    parser.add_argument("--capture-dir", default="D:/tmp/zed-probe-captures")
    parser.add_argument("--body-preview-bytes", type=int, default=4000)
    args = parser.parse_args()

    server = ThreadingHTTPServer((args.host, args.port), ProbeHandler)
    server.probe_log_path = args.log
    server.probe_capture_dir = args.capture_dir
    server.probe_body_preview_bytes = args.body_preview_bytes
    server.probe_counter = 0
    server.probe_lock = threading.Lock()

    def next_request_id() -> str:
        with server.probe_lock:
            server.probe_counter += 1
            counter = server.probe_counter
        clean_path = re.sub(r"[^A-Za-z0-9]+", "-", str(counter)).strip("-")
        return f"{time.strftime('%Y%m%d-%H%M%S')}-{clean_path}"

    server.next_request_id = next_request_id
    Path(args.capture_dir).mkdir(parents=True, exist_ok=True)
    print(
        (
            f"zed probe listening on http://{args.host}:{args.port}; "
            f"log={args.log}; capture_dir={args.capture_dir}"
        ),
        flush=True,
    )
    server.serve_forever()


if __name__ == "__main__":
    main()
