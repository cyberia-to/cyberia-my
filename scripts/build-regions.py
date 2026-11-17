#!/usr/bin/env python3
"""Build the globe's selectable regions from Natural Earth (public domain).

Today a region is a country (admin 0, 1:50m): its name, its three-letter code,
its box and the outer rings of its polygons, as compact JSON the engine uses to
pick a region under a tap and to draw its outline. Source cached in
~/.cache/cyber/naturalearth.

    python3 scripts/build-regions.py          # writes assets/globe/regions.json
"""
import json, os, sys, urllib.request

URL = "https://raw.githubusercontent.com/nvkelso/natural-earth-vector/master/geojson/ne_50m_admin_0_countries.geojson"
CACHE = os.path.expanduser("~/.cache/cyber/naturalearth")
ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))


def main():
    os.makedirs(CACHE, exist_ok=True)
    src = os.path.join(CACHE, os.path.basename(URL))
    if not os.path.exists(src):
        print("fetch", os.path.basename(URL))
        urllib.request.urlretrieve(URL, src)
    with open(src, encoding="utf-8") as f:
        gj = json.load(f)
    out, points = [], 0
    for feat in gj["features"]:
        g, p = feat.get("geometry"), feat["properties"]
        if not g:
            continue
        polys = [g["coordinates"]] if g["type"] == "Polygon" else g["coordinates"]
        rings = []
        for poly in polys:
            ring = [v for c in poly[0] for v in (round(c[0], 3), round(c[1], 3))]
            if len(ring) >= 8:
                rings.append(ring)
                points += len(ring) // 2
        if not rings:
            continue
        lons = [v for r in rings for v in r[0::2]]
        lats = [v for r in rings for v in r[1::2]]
        out.append({"n": p["NAME"], "i": p["ADM0_A3"], "b": [min(lons), min(lats), max(lons), max(lats)], "p": rings})
    out.sort(key=lambda r: r["n"])
    path = os.path.join(ROOT, "assets", "globe", "regions.json")
    with open(path, "w", encoding="utf-8") as f:
        json.dump(out, f, ensure_ascii=False, separators=(",", ":"))
    print(f"{len(out)} regions, {points} points, {os.path.getsize(path) >> 10} KB")


if __name__ == "__main__":
    sys.exit(main())
