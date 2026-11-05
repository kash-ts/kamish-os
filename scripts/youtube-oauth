#!/usr/bin/env python3
"""youtube-oauth — OAuth device-flow helper for the Kamish speaker.

Этот помощник общается с Google по OAuth 2.0 Device Flow и используется
сервисом `kamish-core` (`crates/kamish-core/src/music/youtube_auth.rs`) через
три команды:

    youtube-oauth begin <client_id> <client_secret>   -> JSON {verification_url, user_code, ...}
    youtube-oauth finish <client_id> <client_secret> <device_code>  -> JSON с токенами
    youtube-oauth refresh <client_id> <client_secret> <refresh_token> -> JSON с access_token

`ytmusicapi` тут не используется намеренно: он тянет сетевой стек и требует
установленного пакета, а device-flow — это два обычных POST-запроса. Меньше
зависимостей у сервиса, меньше шансов сломаться при обновлении.

Все секреты печатаются в stdout только по запросу вызывающей стороны и никогда
не логируются. Ошибки уходят в stderr с ненулевым кодом возврата.
"""
from __future__ import annotations

import json
import sys
import urllib.error
import urllib.parse
import urllib.request

OAUTH_CODE_URL = "https://oauth2.googleapis.com/device/code"
OAUTH_TOKEN_URL = "https://oauth2.googleapis.com/token"
# Минимальный scope: только чтение YouTube (для YouTube Music этого достаточно).
OAUTH_SCOPE = "https://www.googleapis.com/auth/youtube"
TIMEOUT = 30


def _post(url: str, data: dict) -> dict:
    body = urllib.parse.urlencode(data).encode()
    req = urllib.request.Request(
        url,
        data=body,
        headers={
            "Content-Type": "application/x-www-form-urlencoded",
            "User-Agent": "kamish-speaker/0.1",
        },
        method="POST",
    )

    try:
        with urllib.request.urlopen(req, timeout=TIMEOUT) as resp:
            return json.loads(resp.read().decode())
    except urllib.error.HTTPError as e:
        # Google возвращает тело с описанием ошибки даже при 4xx.
        raw = e.read().decode(errors="replace")
        try:
            payload = json.loads(raw)
        except json.JSONDecodeError:
            payload = {"error": f"http_{e.code}", "error_description": raw[:500]}
        payload.setdefault("http_status", e.code)
        return payload


def cmd_begin(client_id: str, client_secret: str) -> int:
    resp = _post(OAUTH_CODE_URL, {"client_id": client_id, "scope": OAUTH_SCOPE})
    if "device_code" not in resp:
        print(json.dumps(resp))
        return 1

    print(
        json.dumps(
            {
                "device_code": resp["device_code"],
                "user_code": resp.get("user_code"),
                "verification_url": resp.get("verification_url"),
                "expires_in": resp.get("expires_in"),
                "interval": resp.get("interval", 5),
            }
        )
    )
    return 0


def cmd_finish(client_id: str, client_secret: str, device_code: str) -> int:
    resp = _post(
        OAUTH_TOKEN_URL,
        {
            "client_id": client_id,
            "client_secret": client_secret,
            "grant_type": "http://oauth.net/grant_type/device/1.0",
            "code": device_code,
        },
    )

    if "refresh_token" not in resp:
        # Пользователь ещё не подтвердил код, или код истёк.
        print(json.dumps(resp))
        return 1

    print(
        json.dumps(
            {
                "access_token": resp.get("access_token"),
                "refresh_token": resp.get("refresh_token"),
                "expires_in": resp.get("expires_in"),
                "scope": resp.get("scope"),
                "token_type": resp.get("token_type"),
            }
        )
    )
    return 0


def cmd_refresh(client_id: str, client_secret: str, refresh_token: str) -> int:
    resp = _post(
        OAUTH_TOKEN_URL,
        {
            "client_id": client_id,
            "client_secret": client_secret,
            "grant_type": "refresh_token",
            "refresh_token": refresh_token,
        },
    )

    if "access_token" not in resp:
        print(json.dumps(resp))
        return 1

    print(
        json.dumps(
            {
                "access_token": resp.get("access_token"),
                "expires_in": resp.get("expires_in"),
                "scope": resp.get("scope"),
                "token_type": resp.get("token_type"),
            }
        )
    )
    return 0


def main() -> int:
    if len(sys.argv) < 2:
        print(__doc__, file=sys.stderr)
        return 2

    action = sys.argv[1]
    args = sys.argv[2:]

    try:
        if action == "begin" and len(args) == 2:
            return cmd_begin(args[0], args[1])
        if action == "finish" and len(args) == 3:
            return cmd_finish(args[0], args[1], args[2])
        if action == "refresh" and len(args) == 3:
            return cmd_refresh(args[0], args[1], args[2])
    except Exception as e:  # noqa: BLE001 - наверх отдаём текст, не трассировку
        print(json.dumps({"error": type(e).__name__, "error_description": str(e)[:500]}))
        return 1

    print(
        json.dumps(
            {"error": "bad_arguments", "error_description": f"unknown action: {action}"}
        ),
        file=sys.stderr,
    )
    return 2


if __name__ == "__main__":
    sys.exit(main())
