#!/usr/bin/env python3
"""Serve dist/ the way cyberproxy's nginx does: a file if it exists, a
directory by its index.html (redirecting to the slash form), everything
else from /index.html so the router takes over.

    python3 scripts/serve-dist.py 8099
"""
import http.server
import sys
from pathlib import Path

DIST = Path(__file__).resolve().parent.parent / "dist"


class Handler(http.server.SimpleHTTPRequestHandler):
    def __init__(self, *a, **kw):
        super().__init__(*a, directory=str(DIST), **kw)

    def send_head(self):
        path = self.path.split("?", 1)[0]
        target = DIST / path.lstrip("/")
        if target.is_file():
            return super().send_head()
        if target.is_dir():
            if not path.endswith("/"):
                self.send_response(301)
                self.send_header("Location", path + "/")
                self.end_headers()
                return None
            if (target / "index.html").is_file():
                return super().send_head()
        self.path = "/index.html"
        return super().send_head()

    def log_message(self, *_):
        pass


if __name__ == "__main__":
    port = int(sys.argv[1]) if len(sys.argv) > 1 else 8099
    http.server.ThreadingHTTPServer(("127.0.0.1", port), Handler).serve_forever()
