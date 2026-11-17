#!/usr/bin/env python3
"""The list of states the globe names its countries by: cyberstates.

Reads ../cyberstates/states/*.toml (code, name, slug) and writes
assets/globe/states.json keyed by the three-letter code the boundary source
uses, so a boundary of Taiwan is Taiwan, as cyberstates defines it.
The two- to three-letter mapping comes from Natural Earth's attribute table
(cached by build-lines.py / build-regions.py).

    python3 scripts/build-states.py CYBERSTATES_DIR
"""
import json, os, sys, tomllib, urllib.request

NE = "https://raw.githubusercontent.com/nvkelso/natural-earth-vector/master/geojson/ne_50m_admin_0_countries.geojson"
CACHE = os.path.expanduser("~/.cache/cyber/naturalearth")
ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
# three-letter codes Natural Earth's table misses or spells its own way
EXTRA = {"GI": "GIB", "XK": "XKX", "TW": "TWN", "PS": "PSE", "HK": "HKG", "MO": "MAC", "GF": "GUF", "GP": "GLP", "MQ": "MTQ", "RE": "REU", "YT": "MYT", "FR": "FRA", "NO": "NOR"}


def main():
    states_dir = os.path.join(sys.argv[1], "states")
    src = os.path.join(CACHE, os.path.basename(NE))
    if not os.path.exists(src):
        os.makedirs(CACHE, exist_ok=True)
        urllib.request.urlretrieve(NE, src)
    a2_a3 = dict(EXTRA)
    for f in json.load(open(src, encoding="utf-8"))["features"]:
        p = f["properties"]
        a2, a3 = p.get("ISO_A2_EH"), p.get("ISO_A3_EH")
        if a2 and a3 and "-" not in a2 and "-" not in a3:
            a2_a3.setdefault(a2, a3)
    out, unmapped = {}, []
    for name in sorted(os.listdir(states_dir)):
        with open(os.path.join(states_dir, name), "rb") as f:
            s = tomllib.load(f)
        code = s["code"]
        # states the boundary sources know by no ISO code keep their cyberstates code
        a3 = a2_a3.get(code) or (code if code in ("ABK", "KRD", "NCY", "OST", "PMR") else None)
        if a3:
            out[a3] = {"code": code, "name": s["name"], "slug": s.get("slug", "")}
        else:
            unmapped.append(code)
    path = os.path.join(ROOT, "assets", "globe", "states.json")
    with open(path, "w", encoding="utf-8") as f:
        json.dump(out, f, ensure_ascii=False, separators=(",", ":"), sort_keys=True)
    print(f"{len(out)} states mapped; without a three-letter code: {' '.join(unmapped)}")


if __name__ == "__main__":
    main()
