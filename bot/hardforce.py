#!/usr/bin/env python3
"""The hard force bot — orders from cyberia.my become tasks in the crew's
Telegram group; what the crew sends back lands on the flat's page.

    ORDER on the site → t.me/<bot>?start=survey_<plot>  (or media_<plot>)
    bot → posts TASK to the hard force group, answers the orderer
    crew → replies to the TASK message: photos, a video, a survey sheet,
           a line `m2 612.4` for the surveyed area
    bot → pins every file to IPFS (Pinata), writes
          <docroot>/plot-data/<plot>.json, tells the group and the orderer

One process, long polling, the standard library only. State on disk next
to it: tasks.json (task message id → order), events.jsonl (everything).

Environment:
    BOT_TOKEN           from BotFather
    HARDFORCE_CHAT_ID   the group's id (add the bot, send /id there)
    PINATA_JWT          Pinata API key (JWT) for pinFileToIPFS
    DOCROOT             /var/www/html/cyberia.my
    SITE                https://cyberia.my
    GATEWAY             https://gateway.pinata.cloud/ipfs/
"""
import json
import mimetypes
import os
import re
import sys
import time
import urllib.error
import urllib.parse
import urllib.request
import uuid
from datetime import datetime, timezone
from pathlib import Path

TOKEN = os.environ.get("BOT_TOKEN", "")
CHAT = os.environ.get("HARDFORCE_CHAT_ID", "")
PINATA = os.environ.get("PINATA_JWT", "")
DOCROOT = Path(os.environ.get("DOCROOT", "/var/www/html/cyberia.my"))
SITE = os.environ.get("SITE", "https://cyberia.my").rstrip("/")
GATEWAY = os.environ.get("GATEWAY", "https://gateway.pinata.cloud/ipfs/")
STATE = Path(__file__).resolve().parent / "state"
API = f"https://api.telegram.org/bot{TOKEN}"
FILES = f"https://api.telegram.org/file/bot{TOKEN}"

KINDS = {
    "survey": "SURVEY · triangulation on the slope",
    "media": "MEDIA · photos and a drone video",
}


def now() -> str:
    return datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")


def log(event: dict):
    STATE.mkdir(parents=True, exist_ok=True)
    event["at"] = now()
    with (STATE / "events.jsonl").open("a") as f:
        f.write(json.dumps(event, ensure_ascii=False) + "\n")
    print(json.dumps(event, ensure_ascii=False), flush=True)


def tg(method: str, **params):
    data = json.dumps(params).encode()
    req = urllib.request.Request(f"{API}/{method}", data=data, headers={"Content-Type": "application/json"})
    try:
        with urllib.request.urlopen(req, timeout=70) as r:
            return json.load(r)
    except urllib.error.HTTPError as e:
        body = e.read().decode(errors="replace")
        log({"event": "tg-error", "method": method, "status": e.code, "body": body[:300]})
        return {"ok": False, "description": body}
    except (urllib.error.URLError, TimeoutError) as e:
        log({"event": "tg-error", "method": method, "error": str(e)})
        return {"ok": False}


def tasks() -> dict:
    p = STATE / "tasks.json"
    return json.load(p.open()) if p.exists() else {}


def save_tasks(t: dict):
    STATE.mkdir(parents=True, exist_ok=True)
    (STATE / "tasks.json").write_text(json.dumps(t, indent=1, ensure_ascii=False))


# ── orders ───────────────────────────────────────────────────────────────

def handle_start(msg: dict, payload: str):
    kind, _, plot = payload.partition("_")
    user = msg["from"]
    who = f"@{user['username']}" if user.get("username") else user.get("first_name", "someone")
    if kind not in KINDS or not re.fullmatch(r"[a-z0-9-]+", plot):
        tg("sendMessage", chat_id=msg["chat"]["id"], text="Order a survey or media from a flat's page on cyberia.my — the button brings you here with the order filled in.")
        return
    page = f"{SITE}/plot/{plot}/"
    task_text = (
        f"TASK · {KINDS[kind]}\n"
        f"flat: {plot}\n{page}\n"
        f"ordered by {who}\n\n"
        f"Reply to this message with what the ground gives: photos, the drone video, the survey sheet; "
        f"for a survey add a line `m2 <figure>`. Every reply goes to the page."
    )
    posted = tg("sendMessage", chat_id=CHAT, text=task_text, disable_web_page_preview=True)
    if not posted.get("ok"):
        tg("sendMessage", chat_id=msg["chat"]["id"], text="The crew's group is out of reach right now — write to support, the order is logged.")
        log({"event": "order-failed", "kind": kind, "plot": plot, "user": user.get("id")})
        return
    mid = posted["result"]["message_id"]
    t = tasks()
    t[str(mid)] = {"kind": kind, "plot": plot, "user_id": user["id"], "who": who, "opened": now()}
    save_tasks(t)
    tg(
        "sendMessage",
        chat_id=msg["chat"]["id"],
        text=f"Order taken: {KINDS[kind]} for {plot}.\nThe hard force has the task; the result appears on {page} and I will tell you here.",
        disable_web_page_preview=True,
    )
    log({"event": "order", "kind": kind, "plot": plot, "task": mid, "user": user.get("id")})


# ── results ──────────────────────────────────────────────────────────────

def download(file_id: str) -> tuple[bytes, str]:
    info = tg("getFile", file_id=file_id)
    path = info["result"]["file_path"]
    with urllib.request.urlopen(f"{FILES}/{path}", timeout=300) as r:
        return r.read(), Path(path).name


def pin(data: bytes, name: str, plot: str) -> str:
    """pinFileToIPFS — returns the CID."""
    boundary = uuid.uuid4().hex
    mime = mimetypes.guess_type(name)[0] or "application/octet-stream"
    meta = json.dumps({"name": f"{plot}/{name}", "keyvalues": {"plot": plot, "site": "cyberia.my"}})
    body = b"".join(
        [
            f"--{boundary}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"{name}\"\r\nContent-Type: {mime}\r\n\r\n".encode(),
            data,
            f"\r\n--{boundary}\r\nContent-Disposition: form-data; name=\"pinataMetadata\"\r\n\r\n{meta}\r\n".encode(),
            f"--{boundary}--\r\n".encode(),
        ]
    )
    req = urllib.request.Request(
        "https://api.pinata.cloud/pinning/pinFileToIPFS",
        data=body,
        headers={"Authorization": f"Bearer {PINATA}", "Content-Type": f"multipart/form-data; boundary={boundary}"},
    )
    with urllib.request.urlopen(req, timeout=600) as r:
        return json.load(r)["IpfsHash"]


def plot_data(plot: str) -> dict:
    p = DOCROOT / "plot-data" / f"{plot}.json"
    return json.load(p.open()) if p.exists() else {"survey": None, "media": [], "updated": ""}


def save_plot_data(plot: str, d: dict):
    d["updated"] = now()
    out = DOCROOT / "plot-data"
    out.mkdir(parents=True, exist_ok=True)
    tmp = out / f".{plot}.json.tmp"
    tmp.write_text(json.dumps(d, ensure_ascii=False, indent=1))
    tmp.replace(out / f"{plot}.json")


def handle_result(msg: dict, task: dict):
    plot, kind = task["plot"], task["kind"]
    d = plot_data(plot)
    text = msg.get("text") or msg.get("caption") or ""
    today = datetime.now(timezone.utc).strftime("%Y-%m-%d")
    added = []

    files = []
    if msg.get("photo"):
        files.append(("photo", msg["photo"][-1]["file_id"], "jpg"))
    if msg.get("video"):
        files.append(("video", msg["video"]["file_id"], None))
    if msg.get("document"):
        doc = msg["document"]
        kind_guess = "video" if (doc.get("mime_type") or "").startswith("video/") else "photo" if (doc.get("mime_type") or "").startswith("image/") else "file"
        files.append((kind_guess, doc["file_id"], None))

    m2 = re.search(r"\bm2\s*[:=]?\s*([\d][\d\s,.]*)", text, re.I)
    for fkind, file_id, ext in files:
        data, name = download(file_id)
        if ext and not name.lower().endswith(ext):
            name = f"{name}.{ext}"
        cid = pin(data, name, plot)
        url = f"{GATEWAY}{cid}"
        if fkind == "file" or (kind == "survey" and m2):
            d["survey"] = d.get("survey") or {}
            d["survey"]["file"] = url
            added.append(f"survey sheet {name}")
        else:
            d["media"].append({"kind": fkind, "url": url, "cid": cid, "taken": today, "note": text[:120] if fkind != "file" else ""})
            added.append(f"{fkind} {name}")
    if m2:
        figure = float(m2.group(1).replace(" ", "").replace(",", "."))
        d["survey"] = {**(d.get("survey") or {}), "m2": figure, "on": today, "method": "triangulation", "by": "hard force"}
        added.append(f"survey {figure} m²")

    if not added:
        return
    save_plot_data(plot, d)
    page = f"{SITE}/plot/{plot}/"
    tg("sendMessage", chat_id=CHAT, reply_to_message_id=msg["message_id"], text=f"On the page: {', '.join(added)}\n{page}", disable_web_page_preview=True)
    tg("sendMessage", chat_id=task["user_id"], text=f"{plot}: {', '.join(added)} — now on {page}", disable_web_page_preview=True)
    log({"event": "result", "plot": plot, "kind": kind, "added": added})


# ── loop ─────────────────────────────────────────────────────────────────

def handle(update: dict):
    msg = update.get("message")
    if not msg:
        return
    text = msg.get("text") or ""
    chat_id = str(msg["chat"]["id"])
    if text.startswith("/start"):
        handle_start(msg, text.partition(" ")[2].strip())
        return
    if text.strip() == "/id":
        tg("sendMessage", chat_id=msg["chat"]["id"], text=f"chat id: {chat_id}")
        return
    if chat_id == CHAT and msg.get("reply_to_message"):
        task = tasks().get(str(msg["reply_to_message"]["message_id"]))
        if task:
            try:
                handle_result(msg, task)
            except Exception as e:  # one bad reply must not stop the crew
                log({"event": "result-error", "error": repr(e)})
                tg("sendMessage", chat_id=CHAT, reply_to_message_id=msg["message_id"], text=f"could not publish this: {e}")


def main() -> int:
    if not TOKEN or not CHAT:
        print("BOT_TOKEN and HARDFORCE_CHAT_ID are required", file=sys.stderr)
        return 2
    offset = 0
    log({"event": "start", "chat": CHAT, "docroot": str(DOCROOT)})
    while True:
        r = tg("getUpdates", offset=offset, timeout=60, allowed_updates=["message"])
        if not r.get("ok"):
            time.sleep(5)
            continue
        for u in r["result"]:
            offset = u["update_id"] + 1
            try:
                handle(u)
            except Exception as e:
                log({"event": "update-error", "error": repr(e)})


if __name__ == "__main__":
    sys.exit(main())
