#!/usr/bin/env python3
"""Build the globe's divisions: every administrative unit of the earth, by level.

Source: geoBoundaries gbOpen (https://www.geoboundaries.org), every level it has
for every country — ADM0 the state, ADM1, ADM2 … ADM5 — in its simplified
geometry. Licences are per boundary (CC BY, ODbL, public domain …) and are kept
in sources.json. States are named as cyberstates defines them (states.json).

The output is static files, read by the globe straight from our own disk:

  countries.json            level 0: [{id, c (ISO3), a (cyberstates code), n, b}]
  t/{level}/{x}/{y}.json    levels 1–5: the units whose box touches a grid cell,
                            flat [id, min_lon, min_lat, max_lon, max_lat, …];
                            a cell is 360/2^Z[level] degrees on both axes
  u/{id//10^6}/{(id%10^6)//1000}/{id}.json
                            one unit: {n, l, t (kind), c, b, p: [ring, …]} with a
                            ring as flat [lon, lat, …] (outer rings only)
  l/{level}/{x}/{y}.bin     levels 0–5: the boundary lines of a level in a grid cell,
                            int16 quads x1, y1, x2, y2 in 1/16384 of the cell from
                            its north-west corner (a segment lives in the cell of
                            its midpoint and may reach two cells out)
  s/{hex}.json              search: the units whose name starts with three letters
                            (hex of their utf-8), [name, level, ISO3, id, lon, lat,
                            span°], states first
  cells.json                which t/ and l/ cells exist, so nothing is asked in vain
  manifest.json             counts per level, the grid, the date
  sources.json              boundary, licence and source per country and level

Beyond gbOpen: Indonesia's kecamatan and desa (levels 3 and 4) come from
geoBoundaries gbHumanitarian (OCHA ROAP via HDX, CC BY 3.0 IGO), because gbOpen
stops at the regency there; the states cyberstates names and gbOpen lacks —
Abkhazia, South Ossetia, Transnistria, Northern Cyprus — come from Natural
Earth's breakaway areas (public domain), and Kurdistan is the four northern
governorates of Iraq from gbOpen.

A point is in one unit per level, so its chain of parents is found by asking
every level for the unit under the point: no parent links are stored.

    python3 build-divisions.py OUT --cache DIR --states states.json [--jobs 8]

Run it on the build host and copy OUT to the front. Never on the front itself:
it has 4 cores and 8 GiB, and this build takes tens of GiB with many jobs — on
2026-11-17 a 48-job run there got the front's container killed and re-created.
"""
import argparse, json, math, os, struct, sys, time, unicodedata, urllib.request
from concurrent.futures import ThreadPoolExecutor, ProcessPoolExecutor

API = "https://www.geoboundaries.org/api/current/gbOpen/ALL/ADM{level}/"
UA = "cyberia.my divisions build (one-time fetch; contact: cyberia.to)"
LEVELS = range(6)
GRID_Z = {1: 4, 2: 6, 3: 7, 4: 8, 5: 9}
LINE_Z = {0: 2, 1: 4, 2: 6, 3: 7, 4: 8, 5: 9}
QUANTA = 16384  # steps of a line cell
EXTRA_API = "https://www.geoboundaries.org/api/current/gbHumanitarian/{iso}/ADM{level}/"
EXTRA = [("IDN", 3), ("IDN", 4)]  # levels gbOpen lacks, taken from gbHumanitarian
NE_BREAKAWAY = "https://raw.githubusercontent.com/nvkelso/natural-earth-vector/master/geojson/ne_10m_admin_0_disputed_areas.geojson"
# Natural Earth's name → the code cyberstates gives the state
BREAKAWAY = {"Abkhazia": "ABK", "South Ossetia": "OST", "Transnistria": "PMR", "N. Cyprus": "NCY"}
KURDISTAN = ("erbil", "arbil", "dahuk", "dohuk", "duhok", "dihok", "sulaymaniyah", "sulaimaniyah", "halabja")
SHARD_CAP = 3000
# units a source draws inside another state that cyberstates names on its own
DROP = {("CHN", 1, "Taiwan Province")}


def get(url, path):
    if os.path.exists(path) and os.path.getsize(path) > 0:
        return True
    err = None
    for attempt in range(5):
        try:
            req = urllib.request.Request(url, headers={"User-Agent": UA})
            with urllib.request.urlopen(req, timeout=180) as r:
                data = r.read()
            with open(path + ".part", "wb") as f:
                f.write(data)
            os.replace(path + ".part", path)
            return True
        except Exception as e:  # noqa: BLE001 — retry any network error
            err = e
            time.sleep(2.0 * (attempt + 1))
    print(f"FAILED {url}: {err}", file=sys.stderr, flush=True)
    return False


def fetch(job):
    entry, cache = job
    if entry.get("local"):
        return entry["local"]
    path = os.path.join(cache, f"{entry['boundaryISO']}-ADM{entry['level']}{'-' + entry['extra'] if entry.get('extra') else ''}.geojson")
    ok = get(entry.get("simplifiedGeometryGeoJSON") or entry["gjDownloadURL"], path) or get(entry["gjDownloadURL"], path)
    return path if ok else None


def outer_rings(geom):
    if not geom:
        return []
    t, c = geom["type"], geom["coordinates"]
    polys = [c] if t == "Polygon" else c if t == "MultiPolygon" else []
    out = []
    for poly in polys:
        if not poly or len(poly[0]) < 4:
            continue
        ring = []
        for pt in poly[0]:
            ring += [round(pt[0], 5), round(pt[1], 5)]
        out.append(ring)
    return out


def fold(text):
    """Lower case without accents: Île → ile."""
    return "".join(c for c in unicodedata.normalize("NFKD", text.lower()) if not unicodedata.combining(c))


def line_cells(rings, level, cells):
    """Add the rings' segments to the line cells of a level (bytearrays of int16 quads)."""
    size = 360.0 / (1 << LINE_Z[level])
    q = size / QUANTA
    keep = 2.0 * q  # vertices closer than two steps to the last kept one say nothing
    step = size * 0.5
    for ring in rings:
        n = len(ring) // 2
        pts = []
        for i in range(n):
            x, y = ring[2 * i], ring[2 * i + 1]
            if pts and abs(x - pts[-1][0]) < keep and abs(y - pts[-1][1]) < keep and i != n - 1:
                continue
            pts.append((x, y))
        if len(pts) < 2:
            continue
        if pts[0] != pts[-1]:
            pts.append(pts[0])
        for (x1, y1), (x2, y2) in zip(pts, pts[1:]):
            # not the seams a source cuts along the date line and at the pole
            if (abs(x1) > 179.99 and abs(x2) > 179.99) or (y1 < -89.9 and y2 < -89.9) or abs(x2 - x1) > 180.0:
                continue
            parts = max(1, int(math.ceil(max(abs(x2 - x1), abs(y2 - y1)) / step)))
            for k in range(parts):
                ax, ay = x1 + (x2 - x1) * k / parts, y1 + (y2 - y1) * k / parts
                bx, by = x1 + (x2 - x1) * (k + 1) / parts, y1 + (y2 - y1) * (k + 1) / parts
                cx = min(int(((ax + bx) * 0.5 + 180.0) / size), (1 << LINE_Z[level]) - 1)
                cy = min(int((90.0 - (ay + by) * 0.5) / size), (1 << LINE_Z[level]) // 2 - 1)
                ox, oy = cx * size - 180.0, 90.0 - cy * size
                quad = (round((ax - ox) / q), round((oy - ay) / q), round((bx - ox) / q), round((oy - by) / q))
                if quad[0] == quad[2] and quad[1] == quad[3]:
                    continue
                if all(-32768 <= v <= 32767 for v in quad):
                    cells.setdefault((cx, cy), bytearray()).extend(struct.pack("<4h", *quad))


def process(job):
    """One boundary file → unit files on disk; returns the index rows and the line cells."""
    index, entry, path, out, states = job
    level, iso = entry["level"], entry["boundaryISO"]
    try:
        with open(path, encoding="utf-8") as f:
            gj = json.load(f)
    except Exception as e:  # noqa: BLE001
        print(f"UNREADABLE {path}: {e}", file=sys.stderr, flush=True)
        return [], {}
    state = states.get(iso)
    rows = []
    cells = {}
    for i, feat in enumerate(gj.get("features", [])):
        rings = outer_rings(feat.get("geometry"))
        if not rings:
            continue
        p = feat.get("properties") or {}
        if (iso, level, p.get("shapeName")) in DROP:
            continue
        uid = index * 1_000_000 + i
        lons = [v for r in rings for v in r[0::2]]
        lats = [v for r in rings for v in r[1::2]]
        box = [min(lons), min(lats), max(lons), max(lats)]
        name = (state["name"] if level == 0 and state else p.get("shapeName")) or entry["boundaryName"]
        kind = entry.get("boundaryCanonical") or ""
        if kind in ("Unknown", "ADM%d" % level):
            kind = ""
        unit = {"n": name, "l": level, "t": kind, "c": iso, "b": box, "p": rings}
        d = os.path.join(out, "u", str(uid // 1_000_000), str((uid % 1_000_000) // 1000))
        os.makedirs(d, exist_ok=True)
        with open(os.path.join(d, f"{uid}.json"), "w", encoding="utf-8") as f:
            json.dump(unit, f, ensure_ascii=False, separators=(",", ":"))
        rows.append((level, uid, box, iso, name))
        line_cells(rings, level, cells)
    return rows, {k: bytes(v) for k, v in cells.items()}


def extra_entries(cache):
    """The boundaries gbOpen lacks, as entries of the same shape (files written into the cache)."""
    out = []
    for iso, level in EXTRA:
        path = os.path.join(cache, f"api-h-{iso}-ADM{level}.json")
        if get(EXTRA_API.format(iso=iso, level=level), path):
            e = json.load(open(path, encoding="utf-8"))
            e["level"] = level
            e["extra"] = "h"
            out.append(e)
    ne = os.path.join(cache, "ne-breakaway.geojson")
    if get(NE_BREAKAWAY, ne):
        feats = {}
        for f in json.load(open(ne, encoding="utf-8"))["features"]:
            code = BREAKAWAY.get(f["properties"].get("BRK_NAME"))
            if code:
                feats.setdefault(code, []).append(f)
        for code, fs in feats.items():
            path = os.path.join(cache, f"{code}-ADM0.geojson")
            with open(path, "w", encoding="utf-8") as f:
                json.dump({"type": "FeatureCollection", "features": [{"type": "Feature", "properties": {"shapeName": code}, "geometry": x["geometry"]} for x in fs[:1]]}, f)
            out.append({"boundaryID": f"{code}-ADM0-naturalearth", "boundaryName": code, "boundaryISO": code, "level": 0, "local": path,
                        "boundarySource": "Natural Earth, breakaway and disputed areas", "boundaryLicense": "Public Domain"})
    irq = os.path.join(cache, "IRQ-ADM1.geojson")
    if os.path.exists(irq):
        polys = []
        for f in json.load(open(irq, encoding="utf-8"))["features"]:
            name = fold((f.get("properties") or {}).get("shapeName") or "")
            g = f.get("geometry") or {}
            if any(k in name for k in KURDISTAN):
                polys += [g["coordinates"]] if g.get("type") == "Polygon" else g.get("coordinates", [])
        if polys:
            path = os.path.join(cache, "KRD-ADM0.geojson")
            with open(path, "w", encoding="utf-8") as f:
                json.dump({"type": "FeatureCollection", "features": [{"type": "Feature", "properties": {"shapeName": "KRD"}, "geometry": {"type": "MultiPolygon", "coordinates": polys}}]}, f)
            out.append({"boundaryID": "KRD-ADM0-gbOpen-IRQ-ADM1", "boundaryName": "KRD", "boundaryISO": "KRD", "level": 0, "local": path,
                        "boundarySource": "geoBoundaries gbOpen, the northern governorates of Iraq", "boundaryLicense": "as IRQ ADM1"})
    return out


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("out")
    ap.add_argument("--cache", required=True)
    ap.add_argument("--states", required=True)
    ap.add_argument("--jobs", type=int, default=8)
    a = ap.parse_args()
    os.makedirs(a.cache, exist_ok=True)
    os.makedirs(a.out, exist_ok=True)
    with open(a.states, encoding="utf-8") as f:
        states = json.load(f)

    entries = []
    for level in LEVELS:
        path = os.path.join(a.cache, f"api-ADM{level}.json")
        if not get(API.format(level=level), path):
            sys.exit(f"no listing for ADM{level}")
        for e in json.load(open(path, encoding="utf-8")):
            e["level"] = level
            entries.append(e)
    with ThreadPoolExecutor(16) as pool:
        list(pool.map(fetch, [(e, a.cache) for e in entries if e["boundaryISO"] == "IRQ" and e["level"] == 1]))
    entries += extra_entries(a.cache)
    entries.sort(key=lambda e: (e["level"], e["boundaryISO"], e.get("extra", "")))
    print(f"{len(entries)} boundary files", flush=True)

    with ThreadPoolExecutor(16) as pool:
        paths = list(pool.map(fetch, [(e, a.cache) for e in entries]))
    missing = [e["boundaryID"] for e, p in zip(entries, paths) if not p]
    print(f"fetched; missing {len(missing)}: {' '.join(missing[:20])}", flush=True)

    jobs = [(i, e, p, a.out, states) for i, (e, p) in enumerate(zip(entries, paths)) if p]
    counts = {l: 0 for l in LEVELS}
    countries = []
    tiles = {l: {} for l in GRID_Z}
    lines = {l: {} for l in LINE_Z}
    shards = {}
    with ProcessPoolExecutor(a.jobs) as pool:
        for (rows, cells), job in zip(pool.map(process, jobs, chunksize=1), jobs):
            for key, data in cells.items():
                lines[job[1]["level"]].setdefault(key, bytearray()).extend(data)
            for level, uid, box, iso, name in rows:
                counts[level] += 1
                row = [name, level, iso, uid, round((box[0] + box[2]) / 2, 4), round((box[1] + box[3]) / 2, 4), round(max(box[2] - box[0], box[3] - box[1]), 4)]
                low = name.lower()
                for key in {(low + "__")[:3], (fold(name) + "__")[:3]}:
                    shards.setdefault(key, []).append(row)
                if level == 0:
                    countries.append({"id": uid, "c": iso, "a": (states.get(iso) or {}).get("code", ""), "n": name, "b": box})
                    continue
                cell = 360.0 / (1 << GRID_Z[level])
                x0, x1 = int((box[0] + 180.0) / cell), int((box[2] + 180.0) / cell)
                y0, y1 = int((90.0 - box[3]) / cell), int((90.0 - box[1]) / cell)
                for x in range(x0, x1 + 1):
                    for y in range(y0, y1 + 1):
                        tiles[level].setdefault((x, y), []).extend([uid] + box)
    n_tiles = 0
    for level, cells in tiles.items():
        for (x, y), flat in cells.items():
            d = os.path.join(a.out, "t", str(level), str(x))
            os.makedirs(d, exist_ok=True)
            with open(os.path.join(d, f"{y}.json"), "w") as f:
                json.dump(flat, f, separators=(",", ":"))
            n_tiles += 1
    n_lines = 0
    for level, cells in lines.items():
        for (x, y), data in cells.items():
            d = os.path.join(a.out, "l", str(level), str(x))
            os.makedirs(d, exist_ok=True)
            with open(os.path.join(d, f"{y}.bin"), "wb") as f:
                f.write(data)
            n_lines += 1
    os.makedirs(os.path.join(a.out, "s"), exist_ok=True)
    for key, rows in shards.items():
        rows.sort(key=lambda r: (r[1], r[0]))
        with open(os.path.join(a.out, "s", key.encode("utf-8").hex() + ".json"), "w", encoding="utf-8") as f:
            json.dump(rows[:SHARD_CAP], f, ensure_ascii=False, separators=(",", ":"))
    with open(os.path.join(a.out, "cells.json"), "w") as f:
        json.dump({"t": {str(l): [f"{x}/{y}" for (x, y) in c] for l, c in tiles.items()},
                   "l": {str(l): [f"{x}/{y}" for (x, y) in c] for l, c in lines.items()}}, f, separators=(",", ":"))
    countries.sort(key=lambda c: c["n"])
    with open(os.path.join(a.out, "countries.json"), "w", encoding="utf-8") as f:
        json.dump(countries, f, ensure_ascii=False, separators=(",", ":"))
    with open(os.path.join(a.out, "sources.json"), "w", encoding="utf-8") as f:
        json.dump([{k: e.get(k) for k in ("boundaryID", "boundaryName", "boundaryISO", "level", "boundaryCanonical", "boundarySource", "boundaryLicense", "licenseSource", "sourceDataUpdateDate")} for e in entries], f, ensure_ascii=False, separators=(",", ":"))
    with open(os.path.join(a.out, "manifest.json"), "w") as f:
        json.dump({"source": "geoBoundaries gbOpen, simplified geometry", "built": time.strftime("%Y-%m-%d"), "units": counts, "total": sum(counts.values()), "grid_zoom": GRID_Z, "line_zoom": LINE_Z, "tiles": n_tiles, "line_cells": n_lines, "search_shards": len(shards), "missing": missing}, f)
    print(f"done: {sum(counts.values())} units {counts}, {n_tiles} cells, {n_lines} line cells, {len(shards)} search shards, {len(missing)} files missing", flush=True)


if __name__ == "__main__":
    main()
