#!/usr/bin/env python3
"""Build src/cyberia_map.json's plots — the polygons as drawn on the map —
src/plot_terms.json — the lease book, one entry per map plot — and
src/cyberia_trails.json — the roads, paths and canyons between the plots — and
src/friend_sales.json — land our friends sell beside the valley.

Two sources, both owned by the valley:
- the land-use map (Google My Maps, plot fill colour = land use)
- the plot sheet (type · ares · owner · purpose · price)

    python3 scripts/sync-plot-terms.py
    # then rebuild / deploy

Land-use legend (the map's colours, aligned with the sheet 2026-09-28):
    green  → residence (sheet: private)     orange → business (sheet: community)
    violet → city land (sheet: commons)     yellow → dual use (business + housing)
    crimson → special place                 blue   → HGB sale of a whole district
                                                      (bridge, $3,500 / are)
Valley rulings layered on top:
- flats whose title is being re-registered are off sale, keep their colour
- sinwood-1…5 are the construction business — sinwood-1 the city's own
- inside a district sold whole, the map colour stays as the buyer's mark
  of a special-purpose place
- a business niche the sheet writes in the owner column (spa, chill) is a
  niche for the flat, not a holder
- an are price set by the valley where the sheet has none prices the flat
- a surveyed area outranks the sheet's ares and the drawn polygon, and
  reprices the flat at its are price (the sheet's figure kept as sheet_price_usd)
Rows the sheet has but the map lacks, and map plots the sheet lacks, are
reported, not dropped.
"""
import csv
import io
import json
import math
import re
import sys
import urllib.request
import xml.etree.ElementTree as ET
from collections import defaultdict
from pathlib import Path

SHEET = "1dJ_3rrDhc85y34wYbziVwjnRiFJLwzP4IW13oCfZzGo"
SHEET_URL = f"https://docs.google.com/spreadsheets/d/{SHEET}/export?format=csv"
MY_MAPS = "1txZioQKBBvOdmox1Had5aI-Zz4kUEJI"
KML_URL = f"https://www.google.com/maps/d/u/0/kml?forcekml=1&mid={MY_MAPS}"
ROOT = Path(__file__).resolve().parent.parent
MAP = ROOT / "src" / "cyberia_map.json"
OUT = ROOT / "src" / "plot_terms.json"
TRAILS_OUT = ROOT / "src" / "cyberia_trails.json"
FRIENDS_OUT = ROOT / "src" / "friend_sales.json"
NS = "{http://www.opengis.net/kml/2.2}"

# the sheet's spelling → one word each
KIND = {
    "private": "private",
    "community": "community",
    "commons": "commons",
    "common": "commons",
    "commers": "commerce",
    "commerce": "commerce",
}

# My Maps fill (#rrggbb) → land use; shades of one hue mean the same use
LAND_USE = {
    "#0f9d58": "lease",
    "#097138": "lease",
    "#f57c00": "venture",
    "#e65100": "venture",
    "#9c27b0": "commons",
    "#ffea00": "dual",
    "#fbc02d": "dual",
    "#c2185b": "special",
}

# are prices the valley set where the sheet is blank: flat id → $ / are
ARE_PRICE = {"sinwood-39": 5000.0}

# surveyed on the ground: flat id → (m², date)
MEASURED = {"sinwood-25-alex-dzin": (1138.24, "2026-09-28")}

# owner-column words that name a business niche, not a holder
NICHES = {"spa", "chill"}

# the valley's construction business
CONSTRUCTION = {"sinwood-1-laba", "sinwood-2", "sinwood-3", "sinwood-4", "sinwood-5"}

# titles in re-registration — off sale until the papers settle
REREGISTRATION = {"sinwood-25-alex-dzin"}

# whole districts sold as one HGB title
DISTRICT_SALE = {"bridge": {"land_use": "hgb", "are_price_usd": 3500.0}}


def num(s: str):
    s = s.strip().replace(",", "").replace("$", "")
    if not s:
        return None
    try:
        return float(s)
    except ValueError:
        return None


def fetch(url: str) -> str:
    return urllib.request.urlopen(url, timeout=30).read().decode("utf-8")


def kml_fills(kml: str):
    """plot placemark name → [#rrggbb, …] in document order (names repeat)."""
    root = ET.fromstring(kml)
    styles = {}
    for st in root.iter(NS + "Style"):
        poly = st.find(NS + "PolyStyle")
        c = poly.find(NS + "color").text if poly is not None and poly.find(NS + "color") is not None else None
        # kml aabbggrr → #rrggbb
        styles[st.get("id")] = f"#{c[6:8]}{c[4:6]}{c[2:4]}".lower() if c else None
    normal = {}
    for sm in root.iter(NS + "StyleMap"):
        for pair in sm.findall(NS + "Pair"):
            if pair.find(NS + "key").text == "normal":
                normal[sm.get("id")] = pair.find(NS + "styleUrl").text.lstrip("#")
    plots = next(f for f in root.iter(NS + "Folder") if f.find(NS + "name").text == "plots")
    out = defaultdict(list)
    for pm in plots.findall(NS + "Placemark"):
        sid = pm.find(NS + "styleUrl").text.lstrip("#")
        out[pm.find(NS + "name").text.strip()].append(styles.get(normal.get(sid, sid)))
    return out


def plot_id(name: str) -> str:
    """'sinwood-25:@alex_dzin' → 'sinwood-25-alex-dzin', 'sinwood - 20' → 'sinwood-20'."""
    return re.sub(r"[^a-z0-9]+", "-", name.lower()).strip("-")


def plan_m2(coords) -> float:
    """Area in plan, the same projection as land.rs `area_m2`."""
    ring = coords[:-1] if len(coords) > 1 and coords[0] == coords[-1] else coords
    lat0 = sum(c[1] for c in ring) / len(ring)
    lon0 = sum(c[0] for c in ring) / len(ring)
    r, k = 6_378_137.0, math.cos(math.radians(lat0))
    xy = [(math.radians(c[0] - lon0) * r * k, math.radians(c[1] - lat0) * r) for c in ring]
    return abs(sum(xy[i - 1][0] * xy[i][1] - xy[i][0] * xy[i - 1][1] for i in range(len(xy)))) / 2


def kml_plots(kml: str):
    """The map's `plots` folder → phase-0 flats, in map order. A name drawn
    twice gets `-2` on its id; the district is the name's first word."""
    root = ET.fromstring(kml)
    folder = next(f for f in root.iter(NS + "Folder") if f.find(NS + "name").text == "plots")
    out, seen = [], defaultdict(int)
    for pm in folder.findall(NS + "Placemark"):
        ring = pm.find(".//" + NS + "Polygon//" + NS + "coordinates")
        if ring is None:
            continue
        name = pm.find(NS + "name").text.strip()
        base = plot_id(name)
        seen[base] += 1
        pid = base if seen[base] == 1 else f"{base}-{seen[base]}"
        coords = [[round(float(v), 7) for v in t.split(",")[:2]] for t in ring.text.split()]
        out.append({
            "id": pid,
            "name": name,
            "kind": "plot",
            "zone": base.split("-")[0],
            "phase": 0,
            "geom": "polygon",
            "coords": coords,
        })
    return out


def kml_trails(kml: str):
    """The map's `trails` folder → roads, paths and canyons with a width in metres.

    concrete / gravel roads and anything named road · street → road (5 m,
    3.5 m for small vehicles); the canyon colours (teal) → canyon; the rest are
    walking paths (2 m)."""
    root = ET.fromstring(kml)
    styles = {}
    for st in root.iter(NS + "Style"):
        ls = st.find(NS + "LineStyle")
        c = ls.find(NS + "color").text if ls is not None and ls.find(NS + "color") is not None else None
        styles[st.get("id")] = f"#{c[6:8]}{c[4:6]}{c[2:4]}".lower() if c else None
    normal = {}
    for sm in root.iter(NS + "StyleMap"):
        for pair in sm.findall(NS + "Pair"):
            if pair.find(NS + "key").text == "normal":
                normal[sm.get("id")] = pair.find(NS + "styleUrl").text.lstrip("#")
    folder = next(f for f in root.iter(NS + "Folder") if f.find(NS + "name").text == "trails")
    out = []
    for pm in folder.findall(NS + "Placemark"):
        # a trail drawn in several strokes is one placemark with a line per stroke
        strokes = [
            [[round(float(v), 7) for v in t.split(",")[:2]] for t in line.text.split()]
            for line in pm.findall(".//" + NS + "LineString/" + NS + "coordinates")
        ]
        strokes = [c for c in strokes if len(c) >= 2]
        if not strokes:
            continue
        name = pm.find(NS + "name").text.strip()
        d = pm.find(NS + "description")
        desc = re.sub(r"<br>.*", "", (d.text or "") if d is not None else "", flags=re.S).strip()
        sid = pm.find(NS + "styleUrl").text.lstrip("#")
        colour = styles.get(normal.get(sid, sid))
        low = f"{name} {desc}".lower()
        if colour in ("#0097a7", "#006064"):
            kind, width = "canyon", 3.0
        elif colour in ("#bdbdbd", "#757575") or re.search(r"\broad\b|street|\bstr\b|\bst\b|concrete|gravel", low):
            kind = "road"
            width = 3.5 if "small vehicles" in low else 5.0
        else:
            kind, width = "path", 2.0
        for coords in strokes:
            out.append({"name": name, "kind": kind, "width_m": width, "color": colour or "", "note": desc, "coords": coords})
    return out


def kml_friend_sales(kml: str):
    """The map's `friend sales` folder — land our friends sell beside the valley.

    Fields stay as the friend wrote them, in order (the listing has two `price`
    rows); empty ones drop, `9460.0` reads `9460`."""
    root = ET.fromstring(kml)
    folder = next((f for f in root.iter(NS + "Folder") if f.find(NS + "name").text == "friend sales"), None)
    out = []
    for pm in folder.findall(NS + "Placemark") if folder is not None else []:
        ring = pm.find(".//" + NS + "Polygon//" + NS + "coordinates")
        if ring is None:
            continue
        coords = [[round(float(v), 7) for v in t.split(",")[:2]] for t in ring.text.split()]
        description, fields = "", []
        ext = pm.find(NS + "ExtendedData")
        for d in ext.findall(NS + "Data") if ext is not None else []:
            v = d.find(NS + "value")
            value = (v.text or "").strip() if v is not None else ""
            if not value:
                continue
            key = d.get("name")
            if key == "description":
                description = value
                continue
            if re.fullmatch(r"\d+\.0", value):
                value = value[:-2]
            fields.append([key, value])
        out.append({
            "name": pm.find(NS + "name").text.strip(),
            "description": description,
            "fields": fields,
            "coords": coords,
        })
    return out


def main() -> int:
    # ── the polygons, as drawn ──
    kml = fetch(KML_URL)
    world = json.load(open(MAP))
    before = {p["id"]: p["coords"] for p in world["phase0"]}
    plots = kml_plots(kml)
    world["phase0"] = plots
    world["stats"]["plot_count"] = len(plots)
    world["stats"]["plot_ha"] = round(sum(plan_m2(p["coords"]) for p in plots) / 10_000, 2)
    MAP.write_text(json.dumps(world, ensure_ascii=False, indent=2) + "\n")
    now = {p["id"]: p["coords"] for p in plots}
    added = [i for i in now if i not in before]
    gone = [i for i in before if i not in now]
    redrawn = [i for i in now if i in before and now[i] != before[i]]
    print(f"{len(plots)} plots → {MAP.relative_to(ROOT)}"
          + (f" · new {', '.join(added)}" if added else "")
          + (f" · gone {', '.join(gone)}" if gone else "")
          + (f" · redrawn {', '.join(redrawn)}" if redrawn else ""))
    ids = [p["id"] for p in plots]

    # ── land use from the map ──
    fills = kml_fills(kml)
    trails = kml_trails(kml)
    friends = kml_friend_sales(kml)
    FRIENDS_OUT.write_text(json.dumps(friends, ensure_ascii=False, indent=1) + "\n")
    print(f"{len(friends)} friend sales → {FRIENDS_OUT.relative_to(ROOT)}: " + ", ".join(f["name"] for f in friends))
    TRAILS_OUT.write_text(json.dumps(trails, ensure_ascii=False, separators=(",", ":")) + "\n")
    kinds = defaultdict(int)
    for t in trails:
        kinds[t["kind"]] += 1
    print(f"{len(trails)} trails → {TRAILS_OUT.relative_to(ROOT)} · " + ", ".join(f"{k} {v}" for k, v in sorted(kinds.items())))
    terms = {}
    unknown_fill = []
    for p in plots:
        queue = fills.get(p["name"], [])
        fill = queue.pop(0) if queue else None
        zone = p.get("zone", "")
        entry = {"fill": fill}
        if zone in DISTRICT_SALE:
            entry.update(DISTRICT_SALE[zone])
            if fill in LAND_USE:
                entry["buyer_mark"] = LAND_USE[fill]
        elif fill in LAND_USE:
            entry["land_use"] = LAND_USE[fill]
        else:
            entry["land_use"] = "unclassified"
            unknown_fill.append(f"{p['id']} {fill}")
        terms[p["id"]] = entry

    # ── holder, type and price from the sheet ──
    rows = list(csv.DictReader(io.StringIO(fetch(SHEET_URL))))

    def match(address: str):
        # "core-5 avatar" → try "core-5", then "avatar"
        for tok in address.strip().lower().split():
            tok = re.sub(r"[^a-z0-9-]", "", tok)
            if tok in ids:
                return tok
            hit = [i for i in ids if i.startswith(tok + "-") and not re.match(rf"^{tok}-\d", i)]
            if len(hit) == 1:
                return hit[0]
        return None

    unmatched = []
    for r in rows:
        addr = (r.get("address") or "").strip()
        if not addr:
            continue
        pid = match(addr)
        if pid is None:
            unmatched.append(addr)
            continue
        sheet = {
            "address": addr,
            "kind": KIND.get((r.get("type") or "").strip().lower(), ""),
            "ares": num(r.get("size") or ""),
            "owner": (r.get("owner") or "").strip(),
            "purpose": (r.get("purpose") or "").strip(),
            "are_price_usd": num(r.get("are price") or ""),
            "plot_price_usd": num(r.get("plot price") or ""),
        }
        for k, v in sheet.items():
            # a district sale price outranks a per-plot one
            if v in (None, "") or (k == "are_price_usd" and "are_price_usd" in terms[pid]):
                continue
            terms[pid][k] = v

    # ── rulings ──
    for pid, t in terms.items():
        if pid in ARE_PRICE and t.get("ares"):
            t["are_price_usd"] = ARE_PRICE[pid]
            t["plot_price_usd"] = round(t["ares"] * ARE_PRICE[pid], 2)
        if pid in MEASURED:
            t["measured_m2"], t["measured_on"] = MEASURED[pid]
            if t.get("are_price_usd"):
                t["sheet_price_usd"] = t.get("plot_price_usd")
                t["plot_price_usd"] = round(t["measured_m2"] / 100 * t["are_price_usd"], 2)
        if pid in REREGISTRATION:
            t["hold"] = "re-registration"
            continue
        owner = t.get("owner", "").strip()
        if owner.lower() in NICHES:
            t["niche"] = owner.lower()
            del t["owner"]
        if pid in CONSTRUCTION:
            t["note"] = "construction business"

    OUT.write_text(json.dumps(terms, indent=1, ensure_ascii=False) + "\n")
    uses = defaultdict(int)
    for t in terms.values():
        uses[t["land_use"]] += 1
    print(f"{len(terms)} plots → {OUT.relative_to(ROOT)} · " + ", ".join(f"{k} {v}" for k, v in sorted(uses.items())))
    if unknown_fill:
        print("fill with no land use:", ", ".join(unknown_fill))
    if unmatched:
        print("sheet rows with no plot on the map:", ", ".join(unmatched))
    missing = [i for i in ids if "address" not in terms[i]]
    print(f"{len(missing)} map plots without a sheet row: {', '.join(missing)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
