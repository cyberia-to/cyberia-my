#!/usr/bin/env python3
"""Fetch the globe's imagery tiles once, to be served from our own machines.

Source: Sentinel-2 cloudless 2016 by EOX IT Services GmbH (https://s2maps.eu),
CC BY 4.0, Web Mercator tiles. The whole earth to --world-zoom, and each
--region box to --region-zoom. Resumable: an existing tile is skipped.
Writes manifest.json (the x/y ranges present per zoom) beside the tiles.

    python3 fetch-imagery.py OUT [--world-zoom 7] [--region-zoom 14]
        [--region name:min_lon,min_lat,max_lon,max_lat ...]
"""
import argparse, json, math, os, sys, time, urllib.request
from concurrent.futures import ThreadPoolExecutor

URL = "https://tiles.maps.eox.at/wmts/1.0.0/s2cloudless_3857/default/g/{z}/{y}/{x}.jpg"
UA = "cyberia.my imagery mirror (one-time fetch; contact: cyberia.to)"


def tile_xy(lon, lat, z):
    n = 1 << z
    x = int((lon + 180.0) / 360.0 * n)
    r = math.radians(lat)
    y = int((1.0 - math.asinh(math.tan(r)) / math.pi) / 2.0 * n)
    return max(0, min(n - 1, x)), max(0, min(n - 1, y))


def fetch(job):
    out, z, x, y = job
    path = os.path.join(out, str(z), str(x), f"{y}.jpg")
    if os.path.exists(path) and os.path.getsize(path) > 0:
        return 0
    os.makedirs(os.path.dirname(path), exist_ok=True)
    url = URL.format(z=z, x=x, y=y)
    for attempt in range(5):
        try:
            req = urllib.request.Request(url, headers={"User-Agent": UA})
            with urllib.request.urlopen(req, timeout=30) as r:
                data = r.read()
            tmp = path + ".part"
            with open(tmp, "wb") as f:
                f.write(data)
            os.replace(tmp, path)
            return len(data)
        except Exception as e:  # noqa: BLE001 — retry any network error
            time.sleep(1.5 * (attempt + 1))
            err = e
    print(f"FAILED {z}/{x}/{y}: {err}", file=sys.stderr)
    return -1


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("out")
    ap.add_argument("--world-zoom", type=int, default=7)
    ap.add_argument("--region-zoom", type=int, default=14)
    ap.add_argument("--region", action="append", default=[])
    ap.add_argument("--threads", type=int, default=8)
    a = ap.parse_args()

    jobs, ranges = [], []
    for z in range(a.world_zoom + 1):
        n = 1 << z
        ranges.append({"z": z, "x0": 0, "y0": 0, "x1": n - 1, "y1": n - 1})
        jobs += [(a.out, z, x, y) for x in range(n) for y in range(n)]
    for spec in a.region:
        name, box = spec.split(":")
        lon0, lat0, lon1, lat1 = (float(v) for v in box.split(","))
        for z in range(a.world_zoom + 1, a.region_zoom + 1):
            x0, y1 = tile_xy(lon0, lat0, z)
            x1, y0 = tile_xy(lon1, lat1, z)
            ranges.append({"z": z, "x0": x0, "y0": y0, "x1": x1, "y1": y1, "region": name})
            jobs += [(a.out, z, x, y) for x in range(x0, x1 + 1) for y in range(y0, y1 + 1)]

    print(f"{len(jobs)} tiles", flush=True)
    done = failed = size = 0
    with ThreadPoolExecutor(a.threads) as pool:
        for n in pool.map(fetch, jobs):
            done += 1
            if n < 0:
                failed += 1
            else:
                size += n
            if done % 2000 == 0:
                print(f"{done}/{len(jobs)} fetched {size >> 20} MB failed {failed}", flush=True)
    manifest = {
        "source": "Sentinel-2 cloudless 2016 by EOX IT Services GmbH (https://s2maps.eu), CC BY 4.0",
        "format": "jpg", "tile": 256, "ranges": ranges,
    }
    with open(os.path.join(a.out, "manifest.json"), "w") as f:
        json.dump(manifest, f)
    print(f"done: {done} tiles, {failed} failed, {size >> 20} MB new", flush=True)
    sys.exit(1 if failed else 0)


if __name__ == "__main__":
    main()
