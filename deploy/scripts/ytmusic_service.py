#!/usr/bin/env python3
"""
YouTube Music Microservice for Kamish Smart Speaker.
Provides JSON-RPC / CLI interface for:
- Search songs, albums, artists
- Rate track (LIKE / DISLIKE / INDIFFERENT)
- Get liked songs
- Get song radio / recommendations
- OAuth / Headers auth management
"""

import sys
import json
import os
from typing import Optional, Dict, Any

try:
    from ytmusicapi import YTMusic
except ImportError:
    print(json.dumps({"error": "ytmusicapi package not installed"}), file=sys.stderr)
    sys.exit(1)


class YtMusicService:
    def __init__(self, auth_file: Optional[str] = None):
        self.auth_file = auth_file or os.environ.get("YTMUSIC_AUTH", "oauth.json")
        self.client = self._init_client()

    def _init_client(self) -> YTMusic:
        if os.path.exists(self.auth_file):
            try:
                return YTMusic(self.auth_file)
            except Exception as e:
                # Fallback to unauthenticated client if auth file is invalid
                pass
        return YTMusic()

    def search_songs(self, query: str, limit: int = 5) -> Dict[str, Any]:
        """Search songs on YouTube Music"""
        try:
            results = self.client.search(query, filter="songs", limit=limit)
            tracks = []
            for item in results:
                artists = [a.get("name", "") for a in item.get("artists", [])]
                tracks.append({
                    "id": item.get("videoId"),
                    "title": item.get("title"),
                    "artist": ", ".join(artists),
                    "duration_seconds": item.get("duration_seconds"),
                    "thumbnail": item.get("thumbnails", [{}])[-1].get("url") if item.get("thumbnails") else None,
                    "uri": f"https://music.youtube.com/watch?v={item.get('videoId')}"
                })
            return {"success": True, "tracks": tracks}
        except Exception as e:
            return {"success": False, "error": str(e)}

    def rate_song(self, video_id: str, rating: str = "LIKE") -> Dict[str, Any]:
        """Rate song: 'LIKE', 'DISLIKE', or 'INDIFFERENT'"""
        rating_upper = rating.upper()
        if rating_upper not in ("LIKE", "DISLIKE", "INDIFFERENT"):
            return {"success": False, "error": "Rating must be LIKE, DISLIKE, or INDIFFERENT"}
        try:
            res = self.client.rate_song(video_id, rating_upper)
            return {"success": True, "rating": rating_upper, "result": res}
        except Exception as e:
            return {"success": False, "error": str(e)}

    def get_liked_songs(self, limit: int = 25) -> Dict[str, Any]:
        """Get user's liked songs playlist"""
        try:
            res = self.client.get_liked_songs(limit=limit)
            tracks = []
            for item in res.get("tracks", []):
                artists = [a.get("name", "") for a in item.get("artists", [])]
                tracks.append({
                    "id": item.get("videoId"),
                    "title": item.get("title"),
                    "artist": ", ".join(artists),
                    "uri": f"https://music.youtube.com/watch?v={item.get('videoId')}"
                })
            return {"success": True, "tracks": tracks}
        except Exception as e:
            return {"success": False, "error": str(e)}

    def get_watch_playlist(self, video_id: str, limit: int = 20) -> Dict[str, Any]:
        """Get radio / auto-mix playlist for a song"""
        try:
            res = self.client.get_watch_playlist(videoId=video_id, limit=limit)
            tracks = []
            for item in res.get("tracks", []):
                artists = [a.get("name", "") for a in item.get("artists", [])]
                tracks.append({
                    "id": item.get("videoId"),
                    "title": item.get("title"),
                    "artist": ", ".join(artists),
                    "uri": f"https://music.youtube.com/watch?v={item.get('videoId')}"
                })
            return {"success": True, "tracks": tracks}
        except Exception as e:
            return {"success": False, "error": str(e)}


def main():
    if len(sys.argv) < 2:
        print(json.dumps({"error": "Usage: ytmusic_service.py <action> [args...]"}), file=sys.stderr)
        sys.exit(1)

    action = sys.argv[1]
    svc = YtMusicService()

    if action == "search":
        query = sys.argv[2] if len(sys.argv) > 2 else ""
        limit = int(sys.argv[3]) if len(sys.argv) > 3 else 5
        print(json.dumps(svc.search_songs(query, limit)))

    elif action == "rate":
        video_id = sys.argv[2] if len(sys.argv) > 2 else ""
        rating = sys.argv[3] if len(sys.argv) > 3 else "LIKE"
        print(json.dumps(svc.rate_song(video_id, rating)))

    elif action == "liked":
        limit = int(sys.argv[2]) if len(sys.argv) > 2 else 25
        print(json.dumps(svc.get_liked_songs(limit)))

    elif action == "radio":
        video_id = sys.argv[2] if len(sys.argv) > 2 else ""
        limit = int(sys.argv[3]) if len(sys.argv) > 3 else 20
        print(json.dumps(svc.get_watch_playlist(video_id, limit)))

    else:
        print(json.dumps({"error": f"Unknown action: {action}"}), file=sys.stderr)
        sys.exit(1)


if __name__ == "__main__":
    main()
