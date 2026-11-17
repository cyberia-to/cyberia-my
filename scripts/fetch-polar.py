#!/usr/bin/env python3
"""Fetch the globe's polar imagery once, to be served from our own machines.

Web Mercator ends at 85.05°, so the poles get their own small pyramid of
equirectangular tiles (EPSG:4326, 2^(z+1) × 2^z tiles of 256 px per zoom):

  north  rows above 84.375°N   Sentinel-2 cloudless 2016 by EOX (CC BY 4.0)
  south  rows below 60°S       Blue Marble by NASA (public domain), via EOX —
                               the Sentinel mosaic paints Antarctica flat white

Resumable: an existing tile is skipped. Layout: OUT/{z}/{x}/{y}.jpg.

    python3 fetch-polar.py OUT [--north-zoom 6] [--south-zoom 7]
"""
import argparse, os, sys, time, urllib.request
from concurrent.futures import ThreadPoolExecutor

URL = "https://tiles.maps.eox.at/wmts/1.0.0/{layer}/default/WGS84/{z}/{y}/{x}.jpg"
UA = "cyberia.my imagery mirror (one-time fetch; contact: cyberia.to)"
MIN_ZOOM = 3
NORTH_FROM = 84.375  # degrees north: the first row of zoom 5
SOUTH_FROM = -60.0


def rows(z):
    """(north rows, south rows) of a zoom — the same rule the engine uses."""
    h = 180.0 / (1 << z)
    n = 1 << z
    north = [y for y in range(n) if y * h < 90.0 - NORTH_FROM]
    south = [y for y in range(n) if 90.0 - (y + 1) * h < SOUTH_FROM]
    return north, south


def fetch(job):
    out, layer, z, x, y = job
    path = os.path.join(out, str(z), str(x), f"{y}.jpg")
    if os.path.exists(path) and os.path.getsize(path) > 0:
        return 0
    os.makedirs(os.path.dirname(path), exist_ok=True)
    url = URL.format(layer=layer, z=z, x=x, y=y)
    err = None
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
    ap.add_argument("--north-zoom", type=int, default=6)
    ap.add_argument("--south-zoom", type=int, default=7)
    ap.add_argument("--threads", type=int, default=8)
    a = ap.parse_args()

    jobs = []
    for z in range(MIN_ZOOM, max(a.north_zoom, a.south_zoom) + 1):
        north, south = rows(z)
        cols = range(2 << z)
        if z <= a.north_zoom:
            jobs += [(a.out, "s2cloudless", z, x, y) for y in north for x in cols]
        if z <= a.south_zoom:
            jobs += [(a.out, "bluemarble", z, x, y) for y in south for x in cols]

    print(f"{len(jobs)} tiles", flush=True)
    done = failed = size = 0
    with ThreadPoolExecutor(a.threads) as pool:
        for n in pool.map(fetch, jobs):
            done += 1
            if n < 0:
                failed += 1
            else:
                size += n
            if done % 1000 == 0:
                print(f"{done}/{len(jobs)} fetched {size >> 20} MB failed {failed}", flush=True)
    print(f"done: {done} tiles, {failed} failed, {size >> 20} MB new", flush=True)
    sys.exit(1 if failed else 0)


if __name__ == "__main__":
    main()
