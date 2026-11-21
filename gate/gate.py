#!/usr/bin/env python3
"""the gate of cyberia.my's land market (projects/listings.md): signed acts in, commits out.

the same way the apparatus takes commands: a wallet signs a text, the gate recovers the signer, checks the right, and
commits the act as one file to the acts repository — the only thing it can write. the market's state is the fold of
those files (gate/acts.py); after every commit the gate writes the public listings beside the site for the map to read.
it keeps no database and no secret but the one that signs its sessions.

    python3 gate/gate.py --repo <acts clone> --root <file with the root address> --secret <file> --publish <listings.json>
                         [--port 8770] [--no-push] [--static <dist>] [--dev-keys 11,22,…]

    GET  /api/acts/nonce                      a nonce for the sign-in
    POST /api/acts/signin  {message,signature} → {token, view}    the wallet proves its address; the token lasts 30 days
    GET  /api/acts/me      Authorization: Bearer <token>          what this address sees of the market
    POST /api/acts/act     {message,signature} → {ok, id, view}   one act, verified, committed, published
    GET  /api/acts/public                                         the live listings

--static and --dev-keys are for the bench only: the first serves a built site with the api beside it, the second
answers POST /api/acts/dev/sign with signatures of throwaway keys so a headless browser can walk the funnel.
"""
import argparse, copy, hashlib, hmac, json, mimetypes, secrets, subprocess, sys, threading, time
from datetime import datetime, timezone
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import acts, sign  # noqa: E402

SESSION_S = 30 * 86400
NONCE_S = 300
SKEW_S = 600
MAX_BODY = 16 * 1024
RATE = (60, 3600)
SIGN_IN = "cyberia.my sign in"


class Gate:
    def __init__(self, repo, root, secret, publish, push=True):
        self.repo = Path(repo); self.root = root; self.key = secret; self.publish = Path(publish) if publish else None; self.push = push
        self.lock = threading.Lock(); self.nonces = {}; self.sent = {}
        (self.repo / "acts").mkdir(parents=True, exist_ok=True)
        self.state, self.errors = acts.fold(self.repo / "acts", self.root)
        self.write_public()

    def git(self, *args):
        r = subprocess.run(["git", "-C", str(self.repo), *args], capture_output=True, text=True, timeout=60)
        if r.returncode: raise RuntimeError(f"git {args[0]}: {(r.stderr or r.stdout).strip()[-300:]}")
        return r.stdout

    def write_public(self):
        if not self.publish: return
        self.publish.parent.mkdir(parents=True, exist_ok=True)
        tmp = self.publish.with_suffix(".tmp"); tmp.write_text(json.dumps(acts.public(self.state))); tmp.replace(self.publish)

    # sessions
    def nonce(self):
        now = time.time(); n = secrets.token_hex(16)
        with self.lock:
            self.nonces = {k: t for k, t in self.nonces.items() if now - t < NONCE_S}
            if len(self.nonces) > 10_000: raise ValueError("too many sign-ins at once")
            self.nonces[n] = now
        return n

    def sign_in(self, message, signature):
        lines = message.split("\n")
        if lines[0] != SIGN_IN: raise ValueError("not a sign-in for cyberia.my")
        f = dict(l.split(": ", 1) for l in lines[1:] if ": " in l)
        with self.lock:
            if self.nonces.pop(f.get("nonce", ""), None) is None: raise ValueError("the nonce is unknown or used")
        who = sign.recover(message, signature)
        if who.lower() != f.get("address", "").lower(): raise ValueError("the signature is not the address's")
        exp = int(time.time()) + SESSION_S
        val = f"{who}.{exp}"
        return f"{val}.{hmac.new(self.key, val.encode(), hashlib.sha256).hexdigest()}", who

    def who(self, header):
        token = (header or "").removeprefix("Bearer ").strip()
        bits = token.split(".")
        if len(bits) != 3: return None
        addr, exp, mac = bits
        good = hmac.new(self.key, f"{addr}.{exp}".encode(), hashlib.sha256).hexdigest()
        return addr if hmac.compare_digest(good, mac) and exp.isdigit() and int(exp) > time.time() else None

    def view(self, addr):
        with self.lock: return acts.view(self.state, addr, self.root)

    # acts
    def limited(self, addr):
        now = time.time()
        q = [t for t in self.sent.get(addr, []) if now - t < RATE[1]]
        if len(q) >= RATE[0]: raise ValueError(f"{RATE[0]} acts an hour is the limit")
        q.append(now); self.sent[addr] = q

    def act(self, message, signature):
        f = acts.fields(message)
        at = datetime.strptime(f["at"], "%Y-%m-%dT%H:%M:%SZ").replace(tzinfo=timezone.utc)
        if abs(time.time() - at.timestamp()) > SKEW_S: raise ValueError("the act's time is more than ten minutes off")
        who = sign.recover(message, signature)
        with self.lock:
            self.limited(who.lower())
            trial = copy.deepcopy(self.state)
            acts.apply(trial, f, who, self.root)   # raises when this address may not do it
            seq = len(list((self.repo / "acts").rglob("*.md")))   # the commit order is the fold order (acts.fold)
            rel = Path("acts") / f"{seq:08d}-{f['kind']}-{f['id'][:8]}.md"
            path = self.repo / rel
            if path.exists(): raise ValueError("the id is used")
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text(acts.file_text(message, signature, who))
            try:
                self.git("add", str(rel))
                self.git("-c", "user.name=cyberia.my gate", "-c", "user.email=gate@cyberia.my", "commit", "-q", "-m", f"{f['kind']}: {who[:10]} {f.get('plot') or f.get('listing') or f.get('target') or f.get('address') or ''}".strip())
            except Exception:
                path.unlink(missing_ok=True); raise
            if self.push:
                for _ in range(3):
                    try: self.git("push", "-q", "origin", "HEAD:main"); break
                    except RuntimeError: self.git("pull", "-q", "--rebase", "origin", "main")
            # the state is always the fold of the files, never the trial: what CI recomputes is what is served
            self.state, self.errors = acts.fold(self.repo / "acts", self.root)
            self.write_public()
        return {"ok": True, "id": f["id"], "path": str(rel), "view": acts.view(self.state, who, self.root)}


def handler(gate, static=None, dev_keys=None):
    dev = {sign.address_of(k).lower(): k for k in (dev_keys or [])}

    class H(BaseHTTPRequestHandler):
        protocol_version = "HTTP/1.1"

        def log_message(self, fmt, *args): sys.stderr.write("%s %s\n" % (self.address_string(), fmt % args))

        def reply(self, code, obj):
            data = json.dumps(obj).encode()
            self.send_response(code); self.send_header("Content-Type", "application/json"); self.send_header("Cache-Control", "no-store")
            self.send_header("Content-Length", str(len(data))); self.end_headers(); self.wfile.write(data)

        def body(self):
            n = int(self.headers.get("Content-Length", "0") or 0)
            if n > MAX_BODY: raise ValueError("too large")
            return json.loads(self.rfile.read(n) or b"{}")

        def do_GET(self):
            p = self.path.split("?")[0]
            try:
                if p == "/api/acts/nonce": return self.reply(200, {"nonce": gate.nonce()})
                if p == "/api/acts/public": return self.reply(200, acts.public(gate.state))
                if p == "/api/acts/me":
                    who = gate.who(self.headers.get("Authorization"))
                    return self.reply(200, gate.view(who)) if who else self.reply(401, {"error": "sign in"})
                if p == "/api/acts/dev" and dev: return self.reply(200, {"addresses": [sign.checksum(bytes.fromhex(a[2:])) for a in dev]})
                if static and not p.startswith("/api/"): return self.file(p)
                self.reply(404, {"error": "no such door"})
            except Exception as e: self.reply(400, {"error": str(e)[:300]})

        def do_POST(self):
            p = self.path.split("?")[0]
            try:
                b = self.body()
                if p == "/api/acts/signin":
                    token, who = gate.sign_in(str(b.get("message", "")), str(b.get("signature", "")))
                    return self.reply(200, {"token": token, "view": gate.view(who)})
                if p == "/api/acts/act": return self.reply(200, gate.act(str(b.get("message", "")), str(b.get("signature", ""))))
                if p == "/api/acts/dev/sign" and dev:
                    k = dev.get(str(b.get("address", "")).lower())
                    if not k: raise ValueError("not a bench key")
                    return self.reply(200, {"signature": sign.sign(str(b.get("message", "")), k)})
                self.reply(404, {"error": "no such door"})
            except Exception as e: self.reply(400, {"error": str(e)[:300]})

        def file(self, p):
            root = Path(static).resolve()
            f = (root / p.lstrip("/")).resolve()
            if not str(f).startswith(str(root)) or not f.is_file():
                f = (root / p.lstrip("/") / "index.html") if (root / p.lstrip("/") / "index.html").is_file() else root / "index.html"
            data = f.read_bytes()
            self.send_response(200); self.send_header("Content-Type", mimetypes.guess_type(str(f))[0] or "application/octet-stream")
            self.send_header("Content-Length", str(len(data))); self.end_headers(); self.wfile.write(data)

    return H


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    ap.add_argument("--repo", required=True); ap.add_argument("--root", required=True); ap.add_argument("--secret", required=True)
    ap.add_argument("--publish"); ap.add_argument("--port", type=int, default=8770); ap.add_argument("--bind", default="127.0.0.1")
    ap.add_argument("--no-push", action="store_true"); ap.add_argument("--static"); ap.add_argument("--dev-keys", default="")
    ap.add_argument("--flats", help="the site's map json: only its flats can be claimed and bound")
    a = ap.parse_args()
    root = Path(a.root).read_text().strip()
    if a.flats: acts.load_flats(a.flats)
    gate = Gate(a.repo, root, Path(a.secret).read_bytes().strip(), a.publish, push=not a.no_push)
    keys = [int(k) for k in a.dev_keys.split(",") if k.strip()]
    for e in gate.errors: sys.stderr.write(f"act refused on fold: {e}\n")
    sys.stderr.write(f"gate: {len(gate.state['ids'])} acts · root {root} · port {a.port}{' · BENCH KEYS' if keys else ''}\n")
    ThreadingHTTPServer((a.bind, a.port), handler(gate, a.static, keys)).serve_forever()


if __name__ == "__main__":
    main()
