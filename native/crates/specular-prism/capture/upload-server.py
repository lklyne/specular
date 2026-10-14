#!/usr/bin/env python3
"""Tiny CORS upload sink: POST /<relative path> writes the body under ROOT."""
import os, sys
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
ROOT = os.path.abspath(sys.argv[1])
PORT = int(sys.argv[2]) if len(sys.argv) > 2 else 8765
class H(BaseHTTPRequestHandler):
    def _cors(self):
        self.send_header('Access-Control-Allow-Origin', '*')
        self.send_header('Access-Control-Allow-Methods', 'POST, OPTIONS')
        self.send_header('Access-Control-Allow-Headers', '*')
    def do_OPTIONS(self):
        self.send_response(204); self._cors(); self.end_headers()
    def do_POST(self):
        rel = self.path.lstrip('/').split('?')[0]
        dest = os.path.abspath(os.path.join(ROOT, rel))
        if not dest.startswith(ROOT + os.sep):
            self.send_response(400); self._cors(); self.end_headers(); return
        n = int(self.headers.get('Content-Length', 0))
        data = self.rfile.read(n)
        os.makedirs(os.path.dirname(dest), exist_ok=True)
        with open(dest, 'wb') as f: f.write(data)
        self.send_response(200); self._cors(); self.end_headers(); self.wfile.write(b'ok')
    def log_message(self, *a): pass
ThreadingHTTPServer(('127.0.0.1', PORT), H).serve_forever()
