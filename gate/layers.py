#!/usr/bin/env python3
"""the map of cyberia.my drawn by its people: layers, features, verification — as signed acts and files (specs/map.md).

the shape is soft3's, on git and wallet signatures for now. a feature is data in a file; a file is named by the hash of
its bytes; an act is a short text a wallet signs that names files by hash — one signer, one step, a bundle of changes
(a signal). the map is the fold of the acts in the order they were committed. nothing else is stored.

    python3 gate/layers.py verify <map-dir>    every act's signature, right and file, in order; exit 1 on a failure

the act, as the wallet shows it:

    cyberia.my map
    kind: change
    id: 6f1c…-uuid
    at: 2026-11-21T10:00:00Z
    layer: 0b7e…-uuid
    file: sha256:9c1e…          the changeset: {"layer", "put": [feature…], "remove": [id…]}
    put: 3
    remove: 0
    note: the east boundary of sinwood

kinds and who may sign them:

    change   the layer's owner or an editor; anyone on    layer · file · put · remove · note · name (when the layer is new:
             an open layer, for features of their own      the act founds it and the signer owns it)
    layer    the layer's owner                             layer · name · about · public yes|no · open yes|no · file (its
                                                           definition: fields, style rules, a program — specs/map.md)
    grant    the layer's owner                             layer · address · role editor|none
    attest   anyone                                        layer · feature · version · statement confirm|dispute ·
                                                           role holder|neighbour|witness|surveyor · method phone|rtk|manual ·
                                                           lat_e7 · lon_e7 · accuracy_cm · samples · spread_cm · vertex · note

a feature, in a changeset — integers only, so the same feature is the same bytes everywhere:

    {"id": "<uuid>", "kind": "point|line|plot|building", "geom": [[lon_e7, lat_e7], …], "props": {…}, "prev": "<version>"}

geom is one pair for a point, the path for a line, the ring without its closing pair for a plot and a building.
coordinates are degrees × 10^7 (about a centimetre). a building's props carry `height_cm` and `base_cm`. `prev` is the
version being replaced — the sha256 of the feature's canonical json — and empty for a new feature; a put whose prev is
not the feature's head is refused, so nobody's edit is lost silently.
"""
import hashlib, json, math, re, sys
from datetime import datetime
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import sign  # noqa: E402

HEAD = "cyberia.my map"
KINDS = ("change", "layer", "grant", "attest")
FEATURES = ("point", "line", "plot", "building")
UUID = re.compile(r"^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$")
ADDR = re.compile(r"^0x[0-9a-fA-F]{40}$")
HASH = re.compile(r"^sha256:[0-9a-f]{64}$")
MAX_LINE, MAX_FILE, MAX_PUT, MAX_RING, MAX_PROPS = 700, 2 * 1024 * 1024, 5000, 5000, 60
E7 = 10_000_000
EARTH_CM = 637_100_880


def canon(obj):
    """the bytes of a value: json with sorted keys, no spaces, no floats."""
    return json.dumps(obj, sort_keys=True, separators=(",", ":"), ensure_ascii=False).encode()


def name_of(data): return "sha256:" + hashlib.sha256(data).hexdigest()


def version(feature):
    """a feature's version: the name of its bytes without `prev` — the same geometry and props are the same version."""
    return name_of(canon({k: feature[k] for k in ("id", "kind", "geom", "props")}))


def fields(message):
    lines = message.split("\n")
    if not lines or lines[0] != HEAD: raise ValueError("not an act of cyberia.my's map")
    out = {}
    for l in lines[1:]:
        if not l.strip(): continue
        if ": " not in l and not l.endswith(":"): raise ValueError(f"not a field: {l[:60]}")
        k, _, v = l.partition(":")
        if len(l) > MAX_LINE: raise ValueError(f"{k} is too long")
        if k in out: raise ValueError(f"{k} is written twice")
        out[k] = v.strip()
    if out.get("kind") not in KINDS: raise ValueError(f"unknown kind {out.get('kind')}")
    if not UUID.match(out.get("id", "")): raise ValueError("the id is not a uuid")
    datetime.strptime(out.get("at", ""), "%Y-%m-%dT%H:%M:%SZ")
    return out


def render(f):
    head = [f"{k}: {f[k]}" for k in ("kind", "id", "at")]
    rest = [f"{k}: {str(v).replace(chr(10), ' ').strip()}" for k, v in f.items() if k not in ("kind", "id", "at") and str(v).strip() != ""]
    return "\n".join([HEAD] + head + rest)


def _int(v, what, lo, hi):
    if isinstance(v, bool) or not isinstance(v, int) or not lo <= v <= hi: raise ValueError(f"{what} is a whole number from {lo} to {hi}")
    return v


def check_feature(f):
    """a feature of a changeset, or ValueError: its shape, its limits, integers only."""
    if not isinstance(f, dict) or set(f) - {"id", "kind", "geom", "props", "prev"}: raise ValueError("a feature is id, kind, geom, props, prev")
    if not UUID.match(str(f.get("id", ""))): raise ValueError("a feature's id is a uuid")
    if f.get("kind") not in FEATURES: raise ValueError("a feature is a point, a line, a plot or a building")
    g = f.get("geom")
    need = {"point": (1, 1), "line": (2, MAX_RING)}.get(f["kind"], (3, MAX_RING))
    if not isinstance(g, list) or not need[0] <= len(g) <= need[1]: raise ValueError(f"a {f['kind']} has {need[0]} to {need[1]} points")
    for p in g:
        if not isinstance(p, list) or len(p) != 2: raise ValueError("a point is [lon_e7, lat_e7]")
        _int(p[0], "a longitude", -180 * E7, 180 * E7); _int(p[1], "a latitude", -90 * E7, 90 * E7)
    props = f.get("props", {})
    if not isinstance(props, dict) or len(props) > MAX_PROPS: raise ValueError(f"props are at most {MAX_PROPS} keys")
    for k, v in props.items():
        if not re.fullmatch(r"[a-z][a-z0-9_]{0,39}", k): raise ValueError(f"a prop's key is a short lowercase word: {k[:40]}")
        if isinstance(v, str):
            if len(v) > 4000: raise ValueError(f"{k} is too long")
        else: _int(v, k, -10**15, 10**15)
    if not isinstance(f.get("prev", ""), str): raise ValueError("prev is a version or empty")
    return f


def dist_cm(a, b):
    """the ground between two [lon_e7, lat_e7] points, in centimetres (equirectangular: exact enough under a kilometre)."""
    la = math.radians((a[1] + b[1]) / 2 / E7)
    dx = math.radians((a[0] - b[0]) / E7) * math.cos(la); dy = math.radians((a[1] - b[1]) / E7)
    return round(math.hypot(dx, dy) * EARTH_CM)


def empty(): return {"layers": {}, "ids": [], "log": []}


def _layer(state, lid):
    l = state["layers"].get(lid)
    if not l: raise ValueError("no such layer")
    return l


def may_edit(l, addr): return addr.lower() == l["owner"].lower() or l["editors"].get(addr.lower()) == "editor"


def may_see(l, addr): return l["public"] or (addr is not None and may_edit(l, addr))


def apply(state, f, signer, files):
    """fold one act; `files` gives a named file's bytes (a callable). ValueError when the signer may not do it."""
    kind, at, me = f["kind"], f["at"], signer.lower()
    if f["id"] in state["ids"]: raise ValueError("the id is used")
    lid = f.get("layer", "")
    if not UUID.match(lid): raise ValueError("an act names its layer")
    detail = ""
    if kind == "change":
        if not HASH.match(f.get("file", "")): raise ValueError("a change names its file")
        data = files(f["file"])
        if data is None or name_of(data) != f["file"]: raise ValueError("the file is not the one the act names")
        cs = json.loads(data)
        if canon(cs) != data: raise ValueError("the file is not in the canonical form")
        if cs.get("layer") != lid or set(cs) - {"layer", "put", "remove"}: raise ValueError("the changeset is of another layer")
        put, remove = cs.get("put", []), cs.get("remove", [])
        if not isinstance(put, list) or not isinstance(remove, list) or len(put) + len(remove) > MAX_PUT: raise ValueError(f"a change is at most {MAX_PUT} features")
        if not put and not remove: raise ValueError("a change changes something")
        if str(len(put)) != f.get("put", "0") or str(len(remove)) != f.get("remove", "0"): raise ValueError("the act's counts are not the file's")
        l = state["layers"].get(lid)
        founded = l is None
        if founded:   # the first change founds the layer; its signer owns it
            if not f.get("name"): raise ValueError("a new layer has a name")
            l = {"id": lid, "name": f["name"][:80], "about": "", "public": True, "open": False, "owner": signer, "editors": {}, "def": "",
                 "features": {}, "gone": {}, "changes": 0, "at": at, "updated": at}
        full = may_edit(l, signer)
        if not full and not l["open"]: raise ValueError("only the layer's owner and editors change it")
        feats = dict(l["features"]); gone = dict(l["gone"])
        seen = set()
        for x in put:
            check_feature(x)
            if x["id"] in seen: raise ValueError("a feature is written twice in one change")
            seen.add(x["id"])
            was = feats.get(x["id"])
            if was is None and x["id"] in gone: raise ValueError("the feature was removed")
            if was is not None:
                if not full and was["by"].lower() != me: raise ValueError("on an open layer a guest changes only what they drew")
                if x.get("prev", "") != was["version"]: raise ValueError(f"the feature changed since you read it — reload: {x['props'].get('name', x['id'][:8])}")
            elif x.get("prev", ""): raise ValueError("a new feature has no prev")
            v = version(x)
            hist = (was["history"] if was else []) + [{"version": v, "by": signer, "at": at, "file": f["file"]}]
            feats[x["id"]] = {"id": x["id"], "kind": x["kind"], "geom": x["geom"], "props": x["props"], "version": v,
                              "by": was["by"] if was else signer, "edited_by": signer, "at": at, "history": hist[-50:],
                              # what was signed on the spot holds for the version it saw; a moved boundary is verified again
                              "attests": [a for a in (was["attests"] if was else []) if a["version"] == v]}
        for fid in remove:
            was = feats.get(fid)
            if was is None: raise ValueError("no such feature to remove")
            if not full and was["by"].lower() != me: raise ValueError("on an open layer a guest removes only what they drew")
            gone[fid] = {"by": signer, "at": at, "version": was["version"]}; del feats[fid]
        l["features"], l["gone"] = feats, gone
        l["changes"] += 1; l["updated"] = at
        if founded: state["layers"][lid] = l
        detail = f"put {len(put)} · remove {len(remove)}" + (f" · {f['note'][:80]}" if f.get("note") else "")
    elif kind == "layer":
        l = _layer(state, lid)
        if me != l["owner"].lower(): raise ValueError("only the layer's owner amends it")
        for k in ("public", "open"):
            if f.get(k) and f[k] not in ("yes", "no"): raise ValueError(f"{k} is yes or no")
        if f.get("file"):
            if not HASH.match(f["file"]): raise ValueError("a definition is a named file")
            data = files(f["file"])
            if data is None or name_of(data) != f["file"]: raise ValueError("the file is not the one the act names")
            if canon(json.loads(data)) != data: raise ValueError("the file is not in the canonical form")
            l["def"] = f["file"]
        if f.get("name"): l["name"] = f["name"][:80]
        if "about" in f: l["about"] = f["about"][:400]
        if f.get("public"): l["public"] = f["public"] == "yes"
        if f.get("open"): l["open"] = f["open"] == "yes"
        l["updated"] = at
        detail = " · ".join(f"{k} {f[k]}" for k in ("name", "public", "open") if f.get(k))
    elif kind == "grant":
        l = _layer(state, lid)
        if me != l["owner"].lower(): raise ValueError("only the layer's owner names its editors")
        if not ADDR.match(f.get("address", "")) or f["address"].lower() == me: raise ValueError("an editor is another address")
        if f.get("role") not in ("editor", "none"): raise ValueError("a role is editor or none")
        if f["role"] == "editor": l["editors"][f["address"].lower()] = "editor"
        else: l["editors"].pop(f["address"].lower(), None)
        detail = f"{f['role']} {f['address'][:10]}"
    else:
        l = _layer(state, lid)
        if not may_see(l, signer): raise ValueError("no such layer")
        x = l["features"].get(f.get("feature", ""))
        if not x: raise ValueError("no such feature")
        if f.get("version") != x["version"]: raise ValueError("the feature changed since you read it — reload")
        if f.get("statement") not in ("confirm", "dispute"): raise ValueError("a statement is confirm or dispute")
        if f.get("role", "witness") not in ("holder", "neighbour", "witness", "surveyor"): raise ValueError("a role is holder, neighbour, witness or surveyor")
        if f.get("method", "phone") not in ("phone", "rtk", "manual"): raise ValueError("a method is phone, rtk or manual")
        num = {}
        for k, lo, hi in (("lat_e7", -90 * E7, 90 * E7), ("lon_e7", -180 * E7, 180 * E7), ("accuracy_cm", 0, 10**8), ("samples", 1, 10**4), ("spread_cm", 0, 10**8), ("vertex", 0, MAX_RING)):
            if f.get(k, "") != "":
                if not re.fullmatch(r"-?\d{1,11}", f[k]): raise ValueError(f"{k} is a whole number")
                num[k] = _int(int(f[k]), k, lo, hi)
        here = None
        if "lat_e7" in num and "lon_e7" in num:
            here = [num["lon_e7"], num["lat_e7"]]
            if "accuracy_cm" not in num: raise ValueError("a position says its accuracy")
        elif f.get("method", "phone") != "manual": raise ValueError("a statement made on the spot carries its position")
        if "vertex" in num and num["vertex"] >= len(x["geom"]): raise ValueError("no such corner")
        # how far the signer stood from what they speak of: the named corner, or the nearest point of the feature
        to = None
        if here: to = dist_cm(here, x["geom"][num["vertex"]]) if "vertex" in num else min(dist_cm(here, p) for p in x["geom"])
        x["attests"] = [a for a in x["attests"] if not (a["by"].lower() == me and a.get("vertex") == num.get("vertex"))]   # one word per signer and corner
        x["attests"].append({"by": signer, "at": at, "version": x["version"], "statement": f["statement"], "role": f.get("role", "witness"),
                             "method": f.get("method", "phone"), "lat_e7": num.get("lat_e7"), "lon_e7": num.get("lon_e7"), "accuracy_cm": num.get("accuracy_cm"),
                             "samples": num.get("samples", 1), "spread_cm": num.get("spread_cm"), "vertex": num.get("vertex"), "distance_cm": to, "note": f.get("note", "")[:300]})
        detail = f"{f['statement']} {x['props'].get('name', x['id'][:8])}" + (f" · {to / 100:.1f} m off" if to is not None else "")
    state["ids"].append(f["id"])
    state["log"].append({"n": len(state["ids"]), "kind": kind, "by": signer, "at": at, "layer": lid, "detail": detail[:160]})
    return state


def file_text(message, signature, signer): return f"---\nfrom: {signer}\nscheme: eip191\nsignature: {signature}\n---\n{message}\n"


def parse_file(text):
    m = re.match(r"^---\nfrom: (0x[0-9a-fA-F]{40})\nscheme: eip191\nsignature: (0x[0-9a-f]{130})\n---\n(.*)\n$", text, re.S)
    if not m: raise ValueError("not an act file")
    return m.group(3), m.group(2), m.group(1)


def file_path(directory, name): return Path(directory) / "files" / name[7:9] / f"{name[7:]}.json"


def reader(directory):
    def read(name):
        p = file_path(directory, name)
        return p.read_bytes() if HASH.match(name) and p.is_file() else None
    return read


def fold(directory):
    """(state, errors) from `<dir>/acts/*.md` in the order of their names and `<dir>/files/` beside them."""
    state, errors = empty(), []
    read = reader(directory)
    for p in sorted((Path(directory) / "acts").glob("*.md"), key=lambda p: p.name):
        try:
            message, signature, signer = parse_file(p.read_text())
            f = fields(message)
            who = sign.recover(message, signature)
            if who.lower() != signer.lower(): raise ValueError(f"signed by {who}, filed as {signer}")
            apply(state, f, who, read)
        except Exception as e: errors.append(f"{p}: {e}")
    return state, errors


def verified(x):
    """a feature's standing from what was signed about its present version."""
    ok = [a for a in x["attests"] if a["statement"] == "confirm"]
    return {"confirm": len(ok), "dispute": len(x["attests"]) - len(ok),
            "on_spot": sum(1 for a in ok if a["method"] != "manual" and a["distance_cm"] is not None and a["distance_cm"] <= max(a["accuracy_cm"] or 0, 300) * 2)}


def card(l, addr=None):
    """a layer in a list: what it is, how much is on it, what this address may do."""
    return {"id": l["id"], "name": l["name"], "about": l["about"], "public": l["public"], "open": l["open"], "owner": l["owner"], "def": l["def"],
            "features": len(l["features"]), "changes": l["changes"], "updated": l["updated"],
            "mine": addr is not None and addr.lower() == l["owner"].lower(), "edit": addr is not None and may_edit(l, addr)}


def layers(state, addr=None): return [card(l, addr) for l in state["layers"].values() if may_see(l, addr)]


def layer(state, lid, addr=None, read=None):
    """a whole layer as the page draws it, or None when this address may not see it."""
    l = state["layers"].get(lid)
    if not l or not may_see(l, addr): return None
    out = card(l, addr)
    out["editors"] = sorted(l["editors"]) if addr is not None and may_edit(l, addr) else []
    out["definition"] = json.loads(read(l["def"])) if l["def"] and read and read(l["def"]) else None
    out["items"] = [{**{k: x[k] for k in ("id", "kind", "geom", "props", "version", "by", "edited_by", "at", "attests")}, "versions": len(x["history"]), "standing": verified(x)}
                    for x in l["features"].values()]
    return out


if __name__ == "__main__":
    if len(sys.argv) != 3 or sys.argv[1] != "verify":
        print(__doc__); sys.exit(2)
    st, errs = fold(sys.argv[2])
    for e in errs: print("FAIL", e)
    print(f"{len(st['ids'])} map acts verified, {len(errs)} failed")
    sys.exit(1 if errs else 0)
