import http.server
import socketserver
import json
import urllib.parse

PORT = 8080

class KamishMockHandler(http.server.BaseHTTPRequestHandler):
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

        if path == '/api/v1/auth/session':
            self._send_json({
                "authenticated": True,
                "user": {
                    "username": "admin",
                    "role": "admin",
                    "must_change_password": False
                }
            })
        elif path == '/api/v1/status':
            self._send_json({
                "status": "ready",
                "version": "1.0.0",
                "app_uptime_secs": 3600,
                "device": {
                    "hostname": "kamish",
                    "local_url": "http://localhost:8080",
                    "ip_address": "127.0.0.1",
                    "memory_used_bytes": 150000000,
                    "memory_total_bytes": 4000000000,
                    "cpu_count": 4,
                    "arch": "aarch64",
                    "system_uptime_secs": 86400,
                    "os": "Armbian Linux"
                },
                "database": {"connected": True, "applied_migrations": 1},
                "subsystems": {
                    "audio": "ok",
                    "bluetooth": "idle",
                    "network": "connected",
                    "vpn": "ready",
                    "speech": "ready",
                    "ai": "ready",
                    "scheduler": "running",
                    "music": "ready"
                },
                "scheduler": {"active_jobs": 0},
                "hardware": {
                    "cpu_temperature_celsius": 48.5,
                    "cpu_usage_percent": 12.0,
                    "memory_used_bytes": 150000000,
                    "memory_total_bytes": 4000000000,
                    "disk_used_bytes": 1000000000,
                    "disk_total_bytes": 16000000000
                },
                "readiness": {
                    "is_ready": True,
                    "setup_complete": True,
                    "issues": [],
                    "ai": {
                        "ready": True,
                        "mode": "cli",
                        "cli_path": "agy",
                        "cli_version": "1.2.14",
                        "error_reason": None
                    }
                }
            })
        elif path == '/api/v1/setup/status':
            self._send_json({
                "is_completed": True,
                "completed_steps": ["auth", "network", "audio", "voice", "ai"],
                "current_step": "completed",
                "degraded_components": []
            })
        elif path == '/api/v1/alarms':
            self._send_json([])
        elif path == '/api/v1/reminders':
            self._send_json([])
        elif path == '/api/v1/music/current':
            self._send_json({
                "status": "stopped",
                "current_track": None,
                "volume": 60,
                "position_secs": 0,
                "duration_secs": None
            })
        elif path == '/api/v1/music/favorites':
            self._send_json({"count": 0, "favorites": []})
        elif path == '/api/v1/music/recommendations':
            self._send_json({"count": 0, "recommendations": []})
        elif path == '/api/v1/music/radio':
            self._send_json({
                "count": 2,
                "stations": [
                    {"id": "lofi", "name": "Lofi Hip Hop", "genre": "Chill", "stream_uri": "https://stream.example.com/lofi"},
                    {"id": "rock", "name": "Classic Rock", "genre": "Rock", "stream_uri": "https://stream.example.com/rock"}
                ]
            })
        elif path == '/api/v1/music/history':
            self._send_json({"count": 0, "history": []})
        elif path == '/api/v1/ai/status':
            self._send_json({
                "ready": True,
                "mode": "cli",
                "cli_path": "agy",
                "cli_version": "1.2.14",
                "error_reason": None
            })
        elif path == '/api/v1/speech/status':
            self._send_json({
                "ready": True,
                "tts_engine": "rhvoice",
                "stt_engine": "vosk",
                "vad_active": True
            })
        elif path == '/api/v1/network/status':
            self._send_json({
                "connected": True,
                "interface": "wlan0",
                "ip": "192.168.1.105",
                "ssid": "Home-WiFi"
            })
        elif path == '/api/v1/system/hardware':
            self._send_json({
                "cpu_temp": 48.5,
                "cpu_usage": 12.0,
                "memory_used_mb": 145,
                "memory_total_mb": 4096
            })
        else:
            self._send_json({"status": "ok"})

    def do_POST(self):
        parsed = urllib.parse.urlparse(self.path)
        path = parsed.path
        length = int(self.headers.get('content-length', 0))
        body = self.rfile.read(length) if length > 0 else b'{}'
        try:
            req_data = json.loads(body.decode('utf-8'))
        except:
            req_data = {}

        if path == '/api/v1/setup/complete':
            self._send_json({
                "success": True,
                "message": "Первичная настройка колонки успешно завершена!"
            })
        elif path == '/api/v1/music/play':
            self._send_json({
                "status": "playing",
                "track": {
                    "id": req_data.get("track_id", "yt:sample"),
                    "title": req_data.get("title", "YouTube Music Stream"),
                    "artist": req_data.get("artist", "YouTube"),
                    "source": "ytmusic"
                }
            })
        elif path == '/api/v1/music/pause':
            self._send_json({"status": "paused"})
        elif path == '/api/v1/music/resume':
            self._send_json({"status": "playing"})
        elif path == '/api/v1/music/stop':
            self._send_json({"status": "stopped"})
        elif path == '/api/v1/music/like':
            self._send_json({"track_id": req_data.get("track_id", ""), "liked": req_data.get("liked", True)})
        else:
            self._send_json({"success": True})

with socketserver.TCPServer(("", PORT), KamishMockHandler) as httpd:
    httpd.serve_forever()
