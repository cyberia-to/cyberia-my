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
  manifest.json             counts per level, the grid, the date
  sources.json              boundary, licence and source per country and level

A point is in one unit per level, so its chain of parents is found by asking
every level for the unit under the point: no parent links are stored.

    python3 build-divisions.py OUT --cache DIR --states states.json [--jobs 48]
"""
import argparse, json, os, sys, time, urllib.request
from concurrent.futures import ThreadPoolExecutor, ProcessPoolExecutor

API = "https://www.geoboundaries.org/api/current/gbOpen/ALL/ADM{level}/"
UA = "cyberia.my divisions build (one-time fetch; contact: cyberia.to)"
LEVELS = range(6)
GRID_Z = {1: 4, 2: 6, 3: 7, 4: 8, 5: 9}


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
    path = os.path.join(cache, f"{entry['boundaryISO']}-ADM{entry['level']}.geojson")
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


def process(job):
    """One boundary file → unit files on disk; returns the index rows."""
    index, entry, path, out, states = job
    level, iso = entry["level"], entry["boundaryISO"]
    try:
        with open(path, encoding="utf-8") as f:
            gj = json.load(f)
    except Exception as e:  # noqa: BLE001
        print(f"UNREADABLE {path}: {e}", file=sys.stderr, flush=True)
        return []
    state = states.get(iso)
    rows = []
    for i, feat in enumerate(gj.get("features", [])):
        rings = outer_rings(feat.get("geometry"))
        if not rings:
            continue
        p = feat.get("properties") or {}
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
    return rows


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("out")
    ap.add_argument("--cache", required=True)
    ap.add_argument("--states", required=True)
    ap.add_argument("--jobs", type=int, default=48)
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
    entries.sort(key=lambda e: (e["level"], e["boundaryISO"]))
    print(f"{len(entries)} boundary files", flush=True)

    with ThreadPoolExecutor(16) as pool:
        paths = list(pool.map(fetch, [(e, a.cache) for e in entries]))
    missing = [e["boundaryID"] for e, p in zip(entries, paths) if not p]
    print(f"fetched; missing {len(missing)}: {' '.join(missing[:20])}", flush=True)

    jobs = [(i, e, p, a.out, states) for i, (e, p) in enumerate(zip(entries, paths)) if p]
    counts = {l: 0 for l in LEVELS}
    countries = []
    tiles = {l: {} for l in GRID_Z}
    with ProcessPoolExecutor(a.jobs) as pool:
        for rows in pool.map(process, jobs, chunksize=1):
            for level, uid, box, iso, name in rows:
                counts[level] += 1
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
    countries.sort(key=lambda c: c["n"])
    with open(os.path.join(a.out, "countries.json"), "w", encoding="utf-8") as f:
        json.dump(countries, f, ensure_ascii=False, separators=(",", ":"))
    with open(os.path.join(a.out, "sources.json"), "w", encoding="utf-8") as f:
        json.dump([{k: e.get(k) for k in ("boundaryID", "boundaryName", "boundaryISO", "level", "boundaryCanonical", "boundarySource", "boundaryLicense", "licenseSource", "sourceDataUpdateDate")} for e in entries], f, ensure_ascii=False, separators=(",", ":"))
    with open(os.path.join(a.out, "manifest.json"), "w") as f:
        json.dump({"source": "geoBoundaries gbOpen, simplified geometry", "built": time.strftime("%Y-%m-%d"), "units": counts, "total": sum(counts.values()), "grid_zoom": GRID_Z, "tiles": n_tiles, "missing": missing}, f)
    print(f"done: {sum(counts.values())} units {counts}, {n_tiles} cells, {len(missing)} files missing", flush=True)


if __name__ == "__main__":
    main()
