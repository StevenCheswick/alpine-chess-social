"""SPA dev server — serves index.html for all routes (clean URL support)."""
import http.server, os, sys

PORT = int(sys.argv[1]) if len(sys.argv) > 1 else 5174
DIR = os.path.dirname(os.path.abspath(__file__))

class SPAHandler(http.server.SimpleHTTPRequestHandler):
    def __init__(self, *a, **kw):
        super().__init__(*a, directory=DIR, **kw)

    def do_GET(self):
        # Serve actual files (JS, CSS, images) normally
        path = os.path.join(DIR, self.path.lstrip('/'))
        if os.path.isfile(path):
            return super().do_GET()
        # Everything else → index.html (SPA fallback)
        self.path = '/index.html'
        return super().do_GET()

print(f"SPA server on http://localhost:{PORT}")
http.server.HTTPServer(('', PORT), SPAHandler).serve_forever()
