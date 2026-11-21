#!/usr/bin/env python3
"""The titles the flats sit on — `src/cyberia_titles.json`.

The valley's map layer (the same Google My Maps the plots come from) keeps a
folder `certificates`: one polygon per parcel of land the estate holds or is
acquiring, with its number, the kind of document, the kind of right (HGB),
the declared size in ares and the day the right runs out. A flat is a piece
of one of those parcels. This script reads that folder and says, for every
flat of `src/cyberia_map.json`, which parcel its centre lies in.

    python3 scripts/sync-titles.py            fetch the layer, write the file
    python3 scripts/sync-titles.py <kml>      the same from a saved kml

Stdlib only. The file is data for the desk and the flat's page; the papers of
a title live in the documents library, by content hash.
"""
import json, re, sys, urllib.request, xml.etree.ElementTree as ET
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
NS = "{http://www.opengis.net/kml/2.2}"


def my_maps_url():
    """the layer's address, from the one place it is written: the plot sync."""
    src = (ROOT / "scripts" / "sync-plot-terms.py").read_text()
    mid = re.search(r'MY_MAPS\s*=\s*"([^"]+)"', src).group(1)
    return f"https://www.google.com/maps/d/u/0/kml?forcekml=1&mid={mid}"


def inside(lon, lat, ring):
    """is the point inside the ring (even-odd)."""
    hit = False
    j = len(ring) - 1
    for i in range(len(ring)):
        (xi, yi), (xj, yj) = ring[i], ring[j]
        if (yi > lat) != (yj > lat) and lon < (xj - xi) * (lat - yi) / (yj - yi) + xi:
            hit = not hit
        j = i
    return hit


def centre(ring):
    pts = ring[:-1] if len(ring) > 1 and ring[0] == ring[-1] else ring
    return sum(p[0] for p in pts) / len(pts), sum(p[1] for p in pts) / len(pts)


def titles(kml):
    root = ET.fromstring(kml)
    folder = next(f for f in root.iter(NS + "Folder") if f.find(NS + "name").text == "certificates")
    out = []
    for m in folder.findall(NS + "Placemark"):
        poly = m.find(".//" + NS + "Polygon")
        if poly is None:
            continue
        coords = poly.find(".//" + NS + "coordinates").text.split()
        ring = [[round(float(c.split(",")[0]), 7), round(float(c.split(",")[1]), 7)] for c in coords]
        data = {d.get("name"): (d.find(NS + "value").text or "").strip() for d in m.iter(NS + "Data")}
        size = data.get("size", "")
        own = data.get("type of ownership", "")
        out.append({
            "id": data.get("id", ""),
            "name": (m.find(NS + "name").text or "").strip(),
            "note": data.get("description", "").split("\n")[0][:120],
            # the layer's words, kept as written: certificate · agreement · pengikatan jual beli · wait
            "doc": data.get("type of doc", ""),
            "right": own if own not in ("", "0") else "",
            "declared_ares": float(size) if re.fullmatch(r"\d+(\.\d+)?", size) else None,
            "expires": data.get("expiration", ""),
            "coords": ring,
            "plots": [],
        })
    return out


def main():
    kml = Path(sys.argv[1]).read_text() if len(sys.argv) > 1 else urllib.request.urlopen(my_maps_url(), timeout=30).read().decode("utf-8")
    ts = titles(kml)
    flats = json.loads((ROOT / "src" / "cyberia_map.json").read_text())["phase0"]
    loose = []
    for f in flats:
        lon, lat = centre(f["coords"])
        home = next((t for t in ts if inside(lon, lat, t["coords"])), None)
        (home["plots"] if home else loose).append(f["id"])
    ts.sort(key=lambda t: (-len(t["plots"]), t["name"]))
    (ROOT / "src" / "cyberia_titles.json").write_text(json.dumps({"titles": ts, "outside": loose}, ensure_ascii=False, indent=1) + "\n")
    for t in ts:
        print(f"{t['name']:18} id {t['id']:10} {t['right'] or '—':4} {t['doc']:22} {len(t['plots']):3} flats")
    print(f"outside every mapped title: {len(loose)} flats {loose}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
