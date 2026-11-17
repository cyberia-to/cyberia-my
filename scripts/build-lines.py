#!/usr/bin/env python3
"""Build the globe's world line layers from Natural Earth (public domain).

Each output is raw little-endian float32: lon1, lat1, lon2, lat2 per segment,
drawn as GL lines. Sources are cached in ~/.cache/cyber/naturalearth.

    python3 scripts/build-lines.py            # writes assets/globe/*.bin
"""
import json, os, struct, sys, urllib.request

BASE = "https://raw.githubusercontent.com/nvkelso/natural-earth-vector/master/geojson/"
LAYERS = {
    "coast": "ne_50m_coastline.geojson",
    "borders": "ne_50m_admin_0_boundary_lines_land.geojson",
    "admin1": "ne_10m_admin_1_states_provinces_lines.geojson",
}
CACHE = os.path.expanduser("~/.cache/cyber/naturalearth")
ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))


def lines_of(geom):
    t, c = geom["type"], geom["coordinates"]
    if t == "LineString":
        yield c
    elif t == "MultiLineString":
        yield from c
    elif t == "Polygon":
        yield from c
    elif t == "MultiPolygon":
        for poly in c:
            yield from poly


def main():
    os.makedirs(CACHE, exist_ok=True)
    out_dir = os.path.join(ROOT, "assets", "globe")
    os.makedirs(out_dir, exist_ok=True)
    for name, file in LAYERS.items():
        src = os.path.join(CACHE, file)
        if not os.path.exists(src):
            print("fetch", file)
            urllib.request.urlretrieve(BASE + file, src)
        with open(src, encoding="utf-8") as f:
            gj = json.load(f)
        buf = bytearray()
        n = 0
        for feat in gj["features"]:
            if not feat.get("geometry"):
                continue
            for line in lines_of(feat["geometry"]):
                for a, b in zip(line, line[1:]):
                    buf += struct.pack("<4f", a[0], a[1], b[0], b[1])
                    n += 1
        with open(os.path.join(out_dir, name + ".bin"), "wb") as f:
            f.write(buf)
        print(f"{name}: {n} segments, {len(buf) >> 10} KB")


if __name__ == "__main__":
    sys.exit(main())
