#!/usr/bin/env python3
"""Loads dist/ in headless Firefox, waits for the page's ?shot=<ms> hook to POST the canvas and the
console log back, and saves them.  Usage: shot.py [out.png] [delay_ms] [width] [height] [ops-json] [devicePixelRatio]  (ops: see index.html)
Everything runs locally; the page only talks to this script's server."""
import http.server, subprocess, sys, tempfile, threading, time, functools
from pathlib import Path
HERE = Path(__file__).resolve().parent.parent
out = Path(sys.argv[1] if len(sys.argv) > 1 else '/tmp/pika-web.png')
delay = int(sys.argv[2]) if len(sys.argv) > 2 else 6000
width = int(sys.argv[3]) if len(sys.argv) > 3 else 1088
height = int(sys.argv[4]) if len(sys.argv) > 4 else 644
ops = sys.argv[5] if len(sys.argv) > 5 else ''
dpr = sys.argv[6] if len(sys.argv) > 6 else '1'
got = {}
class Handler(http.server.SimpleHTTPRequestHandler):
    def do_POST(self):
        body = self.rfile.read(int(self.headers.get('Content-Length', 0)))
        if self.path == '/shot': out.write_bytes(body); got['shot'] = True
        elif self.path == '/log': Path(str(out) + '.log').write_text(body.decode('utf-8', 'replace')); got['log'] = True
        self.send_response(204); self.end_headers()
    def log_message(self, *a): pass
server = http.server.ThreadingHTTPServer(('127.0.0.1', 0), functools.partial(Handler, directory=str(HERE / 'dist')))
port = server.server_address[1]
threading.Thread(target=server.serve_forever, daemon=True).start()
profile = tempfile.mkdtemp(prefix='pika-ff-')
(Path(profile) / 'user.js').write_text('user_pref("webgl.force-enabled", true);\nuser_pref("layers.acceleration.disabled", true);\nuser_pref("media.autoplay.default", 0);\n' + f'user_pref("layout.css.devPixelsPerPx", "{dpr}");\n')
import urllib.parse
url = f'http://127.0.0.1:{port}/index.html?shot={delay}' + (('&ops=' + urllib.parse.quote(ops)) if ops else '')
proc = subprocess.Popen(['firefox', '--headless', '--no-remote', '--profile', profile, '--window-size', f'{width},{height}', url],
                        stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
deadline = time.time() + delay / 1000 + 60
while time.time() < deadline and not (got.get('shot') and got.get('log')): time.sleep(0.5)
proc.terminate()
try: proc.wait(10)
except subprocess.TimeoutExpired: proc.kill()
server.shutdown()
print('screenshot:', out if got.get('shot') else 'MISSING', '| log:', (str(out) + '.log') if got.get('log') else 'MISSING')
if got.get('log'): print(Path(str(out) + '.log').read_text()[:3000])
