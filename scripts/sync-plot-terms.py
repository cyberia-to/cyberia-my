#!/usr/bin/env python3
"""Build src/plot_terms.json — the lease book, one entry per map plot.

Two sources, both owned by the valley:
- the land-use map (Google My Maps, plot fill colour = land use)
- the plot sheet (type · ares · owner · purpose · price)

    python3 scripts/sync-plot-terms.py
    # then rebuild / deploy

Land-use legend (the map's colours):
    green  → lease           orange → joint venture / business
    violet → city commons    yellow → dual use (business + housing)
    crimson → special place  blue   → HGB sale of a whole district
                                      (bridge, $3,500 / are)
Valley rulings layered on top (2026-09-25):
- the sheet's community / commons wins over a lease or dual-use colour
- flats whose title is being re-registered are off sale, keep their colour
- orange ground in sinwood is the construction business — not private
- inside a district sold whole, the map colour stays as the buyer's mark
  of a special-purpose place
Rows the sheet has but the map lacks, and map plots the sheet lacks, are
reported, not dropped.
"""
import csv
import io
import json
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

# orange ground in these districts is the valley's construction business
CONSTRUCTION_DISTRICTS = {"sinwood"}

# titles in re-registration — off sale until the papers settle
REREGISTRATION = {"sinwood-25-alex-dzin", "sinwood-32"}

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


def main() -> int:
    plots = json.load(open(MAP))["phase0"]
    ids = [p["id"] for p in plots]

    # ── land use from the map ──
    fills = kml_fills(fetch(KML_URL))
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
        if pid in REREGISTRATION:
            t["hold"] = "re-registration"
            continue
        city = t.get("kind") in ("community", "commons")
        if city and t["land_use"] in ("lease", "dual"):
            t["map_use"] = t["land_use"]
            t["land_use"] = "commons"
        zone = next(p.get("zone", "") for p in plots if p["id"] == pid)
        if t["land_use"] == "venture" and zone in CONSTRUCTION_DISTRICTS:
            t["note"] = "construction business"
            if t.get("kind") == "private":
                t["kind"] = "business"

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
