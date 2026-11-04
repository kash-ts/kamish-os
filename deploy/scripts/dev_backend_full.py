import http.server
import socketserver
import json
import urllib.parse
import uuid
import time
import subprocess
import os

PORT = 8080

# In-memory / persistent device state
STATE = {
    "volume": 55,
    "volume_ceiling": 85,
    "audio_route": "gt112",
    "alarms": [
        {"id": "alm-1", "time": "07:30", "recurrence": "weekdays", "label": "Подъем", "enabled": True},
        {"id": "alm-2", "time": "09:00", "recurrence": "weekends", "label": "Выходные", "enabled": False}
    ],
    "reminders": [
        {"id": "rem-1", "text": "Проверить обновление прошивки", "time": "18:00", "status": "pending"}
    ],
    "wifi_connected": True,
    "wifi_ssid": "Kamish-WiFi-5G",
    "wifi_ip": "192.168.1.105",
    "ap_active": False,
    "vpn_active": False,
    "ssh_active": True,
    "current_track": {
        "id": "yt:fJ9rUzIMcZQ",
        "title": "Bohemian Rhapsody",
        "artist": "Queen",
        "duration_secs": 354,
        "source": "ytmusic",
        "uri": "https://music.youtube.com/watch?v=fJ9rUzIMcZQ"
    },
    "playback_status": "paused",
    "position_secs": 45,
    "favorites": [],
    "history": [],
    "speech_voice": "aleksandr",
    "speech_rate": 1.0,
    "speech_pitch": 1.0,
    "ai_mode": "cli"
}

class KamishFullBackendHandler(http.server.BaseHTTPRequestHandler):
    def _send_json(self, data, status=200):
        self.send_response(status)
        self.send_header('Content-Type', 'application/json')
        self.send_header('Access-Control-Allow-Origin', '*')
        self.send_header('Access-Control-Allow-Methods', 'GET, POST, PUT, DELETE, OPTIONS')
        self.send_header('Access-Control-Allow-Headers', '*')
        self.end_headers()
        self.wfile.write(json.dumps(data).encode('utf-8'))

    def do_OPTIONS(self):
        self._send_json({"status": "ok"})

    def do_GET(self):
        parsed = urllib.parse.urlparse(self.path)
        path = parsed.path
        qs = urllib.parse.parse_qs(parsed.query)

        # 1. Auth & Session
        if path == '/api/v1/auth/session':
            self._send_json({
                "authenticated": True,
                "user": {"username": "admin", "role": "admin", "must_change_password": False}
            })

        # 2. Status & Hardware
        elif path == '/api/v1/status':
            self._send_json({
                "status": "ready",
                "version": "1.0.0",
                "app_uptime_secs": int(time.time() % 86400),
                "device": {
                    "hostname": "kamish",
                    "local_url": "http://localhost:8080",
                    "ip_address": STATE["wifi_ip"],
                    "memory_used_bytes": 185000000,
                    "memory_total_bytes": 4294967296,
                    "cpu_count": 8,
                    "arch": "aarch64",
                    "system_uptime_secs": 142800,
                    "os": "Kamish Linux (Buildroot ARM64)"
                },
                "database": {"connected": True, "applied_migrations": 1},
                "subsystems": {
                    "audio": "ok",
                    "bluetooth": "idle",
                    "network": "connected",
                    "vpn": "connected" if STATE["vpn_active"] else "disconnected",
                    "speech": "ready",
                    "ai": "ready",
                    "scheduler": "running",
                    "music": "ready"
                },
                "scheduler": {"active_jobs": len([a for a in STATE["alarms"] if a["enabled"]])},
                "hardware": {
                    "cpu_temperature_celsius": 46.2,
                    "cpu_usage_percent": 8.5,
                    "memory_used_bytes": 185000000,
                    "memory_total_bytes": 4294967296,
                    "disk_used_bytes": 2100000000,
                    "disk_total_bytes": 32000000000
                },
                "readiness": {
                    "is_ready": True,
                    "setup_complete": True,
                    "issues": [],
                    "ai": {
                        "ready": True,
                        "mode": STATE["ai_mode"],
                        "cli_path": "agy",
                        "cli_version": "1.2.14",
                        "error_reason": None
                    }
                }
            })

        elif path == '/api/v1/system/hardware':
            self._send_json({
                "cpu_temp": 46.2,
                "cpu_usage": 8.5,
                "memory_used_mb": 185,
                "memory_total_mb": 4096,
                "storage_used_gb": 2.1,
                "storage_total_gb": 32.0,
                "npu_status": "ready (3 TOPS)"
            })

        elif path == '/api/v1/setup/status':
            self._send_json({
                "is_completed": True,
                "completed_steps": ["auth", "network", "audio", "voice", "ai"],
                "current_step": "completed",
                "degraded_components": []
            })

        # 3. Alarms & Reminders
        elif path == '/api/v1/alarms':
            self._send_json(STATE["alarms"])

        elif path == '/api/v1/reminders':
            self._send_json(STATE["reminders"])

        # 4. Music Management
        elif path == '/api/v1/music/current':
            self._send_json({
                "status": STATE["playback_status"],
                "current_track": STATE["current_track"],
                "volume": STATE["volume"],
                "position_secs": STATE["position_secs"],
                "duration_secs": STATE["current_track"]["duration_secs"] if STATE["current_track"] else None
            })

        elif path == '/api/v1/music/search':
            query = qs.get("q", [""])[0] or qs.get("query", [""])[0]
            source = qs.get("source", ["all"])[0]
            limit = int(qs.get("limit", [10])[0])

            # Call real ytmusic_service if query given
            results = []
            if query:
                try:
                    out = subprocess.check_output(
                        ["python", "deploy/scripts/ytmusic_service.py", "search", query, str(limit)],
                        stderr=subprocess.DEVNULL, timeout=6
                    )
                    parsed_res = json.loads(out.decode("utf-8"))
                    if parsed_res.get("success"):
                        for t in parsed_res.get("tracks", []):
                            results.append({
                                "id": f"yt:{t['id']}",
                                "title": t["title"],
                                "artist": t["artist"],
                                "duration_secs": t.get("duration_seconds") or 210,
                                "source": "ytmusic",
                                "uri": t["uri"],
                                "thumbnail_url": t.get("thumbnail")
                            })
                except Exception:
                    # Fallback default catalog results
                    results.append({
                        "id": f"yt:{uuid.uuid4().hex[:8]}",
                        "title": f"{query} (Studio Version)",
                        "artist": "YouTube Music Artist",
                        "duration_secs": 224,
                        "source": "ytmusic",
                        "uri": f"https://music.youtube.com/watch?v=mock_{query[:4]}"
                    })

            self._send_json({
                "query": query,
                "source": source,
                "count": len(results),
                "results": results
            })

        elif path == '/api/v1/music/favorites':
            self._send_json({"count": len(STATE["favorites"]), "favorites": STATE["favorites"]})

        elif path == '/api/v1/music/recommendations':
            self._send_json({"count": 0, "recommendations": []})

        elif path == '/api/v1/music/radio':
            self._send_json({
                "count": 4,
                "stations": [
                    {"id": "lofi", "name": "Lofi Hip Hop Radio", "genre": "Lofi / Study", "stream_uri": "https://stream.example.com/lofi"},
                    {"id": "jazz", "name": "Smooth Jazz 24/7", "genre": "Jazz", "stream_uri": "https://stream.example.com/jazz"},
                    {"id": "rock", "name": "Classic Rock FM", "genre": "Rock", "stream_uri": "https://stream.example.com/rock"},
                    {"id": "synth", "name": "Synthwave Space", "genre": "Electronic", "stream_uri": "https://stream.example.com/synth"}
                ]
            })

        elif path == '/api/v1/music/history':
            self._send_json({"count": len(STATE["history"]), "history": STATE["history"]})

        # 5. AI Assistant & Tools
        elif path == '/api/v1/ai/status':
            self._send_json({
                "ready": True,
                "mode": STATE["ai_mode"],
                "cli_path": "agy",
                "cli_version": "1.2.14",
                "error_reason": None
            })

        elif path == '/api/v1/ai/tools':
            self._send_json({
                "count": 14,
                "tools": [
                    {"name": "create_alarm", "description": "Установить будильник"},
                    {"name": "delete_alarm", "description": "Удалить будильник"},
                    {"name": "list_alarms", "description": "Список будильников"},
                    {"name": "create_reminder", "description": "Создать напоминание"},
                    {"name": "list_reminders", "description": "Список напоминаний"},
                    {"name": "set_volume", "description": "Установить громкость"},
                    {"name": "play_music", "description": "Запустить музыку или радио"},
                    {"name": "pause_music", "description": "Поставить музыку на паузу"},
                    {"name": "resume_music", "description": "Возобновить музыку"},
                    {"name": "stop_music", "description": "Остановить воспроизведение"},
                    {"name": "search_music", "description": "Поиск по каталогу"},
                    {"name": "get_current_track", "description": "Текущий трек"},
                    {"name": "get_device_status", "description": "Статус устройства и сенсоров"},
                    {"name": "snooze_alarm", "description": "Отложить будильник"}
                ]
            })

        # 6. Speech & Voice System
        elif path == '/api/v1/speech/status':
            self._send_json({
                "ready": True,
                "tts_engine": "RHVoice",
                "stt_engine": "Vosk Small RU",
                "vad_active": True,
                "sample_rate": 16000,
                "channels": 4,
                "current_voice": STATE["speech_voice"]
            })

        elif path == '/api/v1/speech/voices':
            self._send_json({
                "voices": [
                    {"id": "aleksandr", "name": "Александр (Русский)", "gender": "male", "lang": "ru-RU"},
                    {"id": "elena", "name": "Елена (Русский)", "gender": "female", "lang": "ru-RU"},
                    {"id": "irina", "name": "Ирина (Русский)", "gender": "female", "lang": "ru-RU"},
                    {"id": "anna", "name": "Анна (Русский)", "gender": "female", "lang": "ru-RU"}
                ]
            })

        # 7. Network & Bluetooth & VPN
        elif path == '/api/v1/network/status':
            self._send_json({
                "connected": STATE["wifi_connected"],
                "interface": "wlan0",
                "ip": STATE["wifi_ip"],
                "ssid": STATE["wifi_ssid"],
                "signal_level": -52,
                "ap_mode": STATE["ap_active"]
            })

        elif path == '/api/v1/network/wifi/scan':
            self._send_json({
                "networks": [
                    {"ssid": STATE["wifi_ssid"], "signal": 92, "security": "WPA2-PSK", "active": True},
                    {"ssid": "Neighbor-Guest", "signal": 58, "security": "WPA2-PSK", "active": False},
                    {"ssid": "Beeline_Home", "signal": 40, "security": "WPA3-SAE", "active": False}
                ]
            })

        elif path == '/api/v1/bluetooth/status':
            self._send_json({
                "powered": True,
                "discoverable": False,
                "connected_device": None,
                "a2dp_sink_active": True
            })

        elif path == '/api/v1/vpn/status':
            self._send_json({
                "connected": STATE["vpn_active"],
                "protocol": "VLESS Reality (XTLS-Vision)",
                "server": "nl-amsterdam.kamish.node:443",
                "ping_ms": 38 if STATE["vpn_active"] else None,
                "bytes_rx": 14200000 if STATE["vpn_active"] else 0,
                "bytes_tx": 3100000 if STATE["vpn_active"] else 0
            })

        elif path == '/api/v1/system/ssh/status':
            self._send_json({"enabled": STATE["ssh_active"], "port": 22})

        elif path == '/api/v1/system/update/status':
            self._send_json({
                "current_slot": "slot_a",
                "version": "1.0.0",
                "system_type": "Buildroot Linux",
                "status": "idle",
                "last_checked": "2026-11-02 15:00:00"
            })

        else:
            self._send_json({"status": "ok"})

    def do_POST(self):
        parsed = urllib.parse.urlparse(self.path)
        path = parsed.path
        length = int(self.headers.get('content-length', 0))
        body = self.rfile.read(length) if length > 0 else b'{}'
        try:
            req = json.loads(body.decode('utf-8'))
        except:
            req = {}

        # AI prompt test
        if path == '/api/v1/ai/prompt':
            user_prompt = req.get("prompt", "")
            # Invoke REAL Antigravity CLI
            try:
                out = subprocess.check_output(
                    [r"C:\Users\ipoma\AppData\Local\agy\bin\agy.exe", "-p", user_prompt, "--output-format", "json"],
                    stderr=subprocess.DEVNULL, timeout=12
                )
                parsed_out = json.loads(out.decode("utf-8"))
                response_text = parsed_out.get("response", "Команда выполнена.")
            except Exception as e:
                response_text = f"Голосовой ассистент обработал команду: «{user_prompt}». Действие подтверждено."

            self._send_json({
                "response": response_text.strip(),
                "tool_calls": [],
                "session_id": req.get("session_id", uuid.uuid4().hex)
            })

        # Alarms
        elif path == '/api/v1/alarms':
            new_alarm = {
                "id": f"alm-{uuid.uuid4().hex[:6]}",
                "time": req.get("time", "08:00"),
                "recurrence": req.get("recurrence", "once"),
                "label": req.get("label", "Будильник"),
                "enabled": True
            }
            STATE["alarms"].append(new_alarm)
            self._send_json(new_alarm)

        elif path.startswith('/api/v1/alarms/') and path.endswith('/toggle'):
            parts = path.split('/')
            aid = parts[4]
            for a in STATE["alarms"]:
                if a["id"] == aid:
                    a["enabled"] = not a["enabled"]
                    self._send_json(a)
                    return
            self._send_json({"error": "Alarm not found"}, status=404)

        # Reminders
        elif path == '/api/v1/reminders':
            new_rem = {
                "id": f"rem-{uuid.uuid4().hex[:6]}",
                "text": req.get("text", "Напоминание"),
                "time": req.get("time", "12:00"),
                "status": "pending"
            }
            STATE["reminders"].append(new_rem)
            self._send_json(new_rem)

        # Music Controls
        elif path == '/api/v1/music/play':
            STATE["playback_status"] = "playing"
            if req.get("title"):
                STATE["current_track"] = {
                    "id": req.get("track_id", f"track-{uuid.uuid4().hex[:6]}"),
                    "title": req.get("title"),
                    "artist": req.get("artist", "YouTube Music"),
                    "duration_secs": 210,
                    "source": "ytmusic",
                    "uri": req.get("uri")
                }
            self._send_json({"status": "playing", "track": STATE["current_track"]})

        elif path == '/api/v1/music/pause':
            STATE["playback_status"] = "paused"
            self._send_json({"status": "paused"})

        elif path == '/api/v1/music/resume':
            STATE["playback_status"] = "playing"
            self._send_json({"status": "playing"})

        elif path == '/api/v1/music/stop':
            STATE["playback_status"] = "stopped"
            STATE["position_secs"] = 0
            self._send_json({"status": "stopped"})

        elif path == '/api/v1/music/skip':
            STATE["position_secs"] = 0
            self._send_json({"status": "skipped", "track_id": STATE["current_track"]["id"]})

        elif path == '/api/v1/music/like':
            track_id = req.get("track_id", STATE["current_track"]["id"])
            liked = req.get("liked", True)
            if liked:
                if not any(f["id"] == track_id for f in STATE["favorites"]):
                    STATE["favorites"].append(STATE["current_track"])
            else:
                STATE["favorites"] = [f for f in STATE["favorites"] if f["id"] != track_id]
            self._send_json({"track_id": track_id, "liked": liked})

        # System volume
        elif path == '/api/v1/system/audio/volume':
            new_vol = min(STATE["volume_ceiling"], max(0, int(req.get("volume", 50))))
            STATE["volume"] = new_vol
            self._send_json({"volume": new_vol, "ceiling": STATE["volume_ceiling"]})

        elif path == '/api/v1/system/audio/ceiling':
            new_ceil = min(85, max(10, int(req.get("ceiling", 85))))
            STATE["volume_ceiling"] = new_ceil
            STATE["volume"] = min(STATE["volume"], new_ceil)
            self._send_json({"ceiling": new_ceil})

        # Speech test
        elif path == '/api/v1/speech/synthesize':
            text = req.get("text", "Тестовая фраза синтезатора речи")
            self._send_json({"success": True, "text": text, "status": "synthesized"})

        # VPN Toggle
        elif path == '/api/v1/vpn/connect':
            STATE["vpn_active"] = True
            self._send_json({"status": "connected", "protocol": "VLESS Reality"})

        elif path == '/api/v1/vpn/disconnect':
            STATE["vpn_active"] = False
            self._send_json({"status": "disconnected"})

        # SSH Toggle
        elif path == '/api/v1/system/ssh/toggle':
            STATE["ssh_active"] = not STATE["ssh_active"]
            self._send_json({"enabled": STATE["ssh_active"], "port": 22})

        elif path == '/api/v1/setup/complete':
            self._send_json({"success": True, "message": "Первичная настройка завершена"})

        else:
            self._send_json({"success": True})

    def do_DELETE(self):
        parsed = urllib.parse.urlparse(self.path)
        path = parsed.path

        if path.startswith('/api/v1/alarms/'):
            aid = path.split('/')[-1]
            STATE["alarms"] = [a for a in STATE["alarms"] if a["id"] != aid]
            self._send_json({"status": "deleted", "id": aid})

        elif path.startswith('/api/v1/reminders/'):
            rid = path.split('/')[-1]
            STATE["reminders"] = [r for r in STATE["reminders"] if r["id"] != rid]
            self._send_json({"status": "deleted", "id": rid})

        else:
            self._send_json({"status": "deleted"})

socketserver.TCPServer.allow_reuse_address = True
with socketserver.TCPServer(("", PORT), KamishFullBackendHandler) as httpd:
    httpd.serve_forever()
