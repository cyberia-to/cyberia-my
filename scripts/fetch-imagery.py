#!/usr/bin/env python3
"""Fetch the globe's imagery tiles once, to be served from our own machines.

Source: Sentinel-2 cloudless 2016 by EOX IT Services GmbH (https://s2maps.eu),
CC BY 4.0, Web Mercator tiles. Three parts, each resumable (an existing tile
is skipped):

  world   the whole earth to --world-zoom;
  deep    from there to --deep-zoom, only where there is something to see:
          to --full-zoom every child of a tile that has a picture, beyond it
          the children of a "rich" tile — one whose JPEG is larger than
          --rich-bytes, which open sea is not. The source answers with a
          blank white tile (under --blank-bytes) where it has nothing — deep
          sea, the ice sheets; those are not kept;
  region  each --region box to --region-zoom, everything.

Writes manifest.json (the x/y ranges per zoom, and the deep span) and
have/{z}.rle for each deep zoom — which tiles exist: the tiles of the zoom
row by row (index y * 2^z + x) as run lengths, absent run first, then
present, alternating, each a LEB128 varint.

    python3 fetch-imagery.py OUT [--world-zoom 7] [--full-zoom 9] [--deep-zoom 11]
        [--region-zoom 14] [--region name:min_lon,min_lat,max_lon,max_lat ...]

Without --region the region ranges of an existing manifest are kept.
Light work — network and disk only — and memory stays small: jobs go to the
pool in batches.
"""
import argparse, http.client, json, math, os, sys, threading, time
from concurrent.futures import ThreadPoolExecutor

HOST = "tiles.maps.eox.at"
PATH = "/wmts/1.0.0/s2cloudless_3857/default/g/{z}/{y}/{x}.jpg"
LOCAL = threading.local()  # one kept connection per thread
UA = "cyberia.my imagery mirror (one-time fetch; contact: cyberia.to)"
BATCH = 20000
BLANK = 200  # set from --blank-bytes
WORLD = 7  # set from --world-zoom: to here every tile is kept, blank or not


def runs(present, n):
    """Run lengths over the n*n tiles of a zoom, absent first, as varints."""
    out, at = bytearray(), 0

    def put(v):
        while v >= 0x80:
            out.append(v & 0x7F | 0x80)
            v >>= 7
        out.append(v)

    idx = sorted(y * n + x for x, y in present)
    i = 0
    while i < len(idx):
        j = i
        while j + 1 < len(idx) and idx[j + 1] == idx[j] + 1:
            j += 1
        put(idx[i] - at)
        put(j - i + 1)
        at = idx[j] + 1
        i = j + 1
    return bytes(out)


def tile_xy(lon, lat, z):
    n = 1 << z
    x = int((lon + 180.0) / 360.0 * n)
    r = math.radians(lat)
    y = int((1.0 - math.asinh(math.tan(r)) / math.pi) / 2.0 * n)
    return max(0, min(n - 1, x)), max(0, min(n - 1, y))


def fetch(job):
    """Size of the tile on disk after the call; -1 when it could not be had."""
    out, z, x, y = job
    path = os.path.join(out, str(z), str(x), f"{y}.jpg")
    try:
        size = os.path.getsize(path)
        if size > 0:
            return size
    except OSError:
        pass
    os.makedirs(os.path.dirname(path), exist_ok=True)
    err = None
    for attempt in range(5):
        try:
            conn = getattr(LOCAL, "conn", None)
            if conn is None:
                conn = LOCAL.conn = http.client.HTTPSConnection(HOST, timeout=30)
            conn.request("GET", PATH.format(z=z, x=x, y=y), headers={"User-Agent": UA})
            r = conn.getresponse()
            data = r.read()
            if r.status != 200:
                raise OSError(f"http {r.status}")
            if len(data) <= BLANK and z > WORLD:
                return len(data)
            tmp = path + ".part"
            with open(tmp, "wb") as f:
                f.write(data)
            os.replace(tmp, path)
            return len(data)
        except Exception as e:  # noqa: BLE001 — retry any network error, on a new connection
            if getattr(LOCAL, "conn", None) is not None:
                LOCAL.conn.close()
                LOCAL.conn = None
            time.sleep(1.5 * (attempt + 1))
            err = e
    print(f"FAILED {z}/{x}/{y}: {err}", file=sys.stderr)
    return -1


class Run:
    def __init__(self, out, threads):
        self.out, self.pool = out, ThreadPoolExecutor(threads)
        self.done = self.failed = self.size = 0

    def fetch(self, jobs, label):
        """Fetch (z, x, y) jobs in batches; yields (job, size)."""
        batch = []

        def flush():
            for job, n in zip(batch, self.pool.map(fetch, [(self.out, *j) for j in batch])):
                self.done += 1
                if n < 0:
                    self.failed += 1
                else:
                    self.size += n
                yield job, n
            print(f"{label}: {self.done} tiles, {self.size >> 20} MB, failed {self.failed}", flush=True)
            batch.clear()

        for j in jobs:
            batch.append(j)
            if len(batch) >= BATCH:
                yield from flush()
        if batch:
            yield from flush()


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("out")
    ap.add_argument("--world-zoom", type=int, default=7)
    ap.add_argument("--deep-zoom", type=int, default=0)
    ap.add_argument("--full-zoom", type=int, default=0)
    ap.add_argument("--rich-bytes", type=int, default=2600)
    ap.add_argument("--blank-bytes", type=int, default=200)
    ap.add_argument("--region-zoom", type=int, default=14)
    ap.add_argument("--region", action="append", default=[])
    ap.add_argument("--threads", type=int, default=8)
    a = ap.parse_args()

    global BLANK, WORLD
    BLANK, WORLD = a.blank_bytes, a.world_zoom
    run = Run(a.out, a.threads)
    ranges = []
    sizes = {}  # (x, y) -> bytes, of the zoom just fetched
    for z in range(a.world_zoom + 1):
        n = 1 << z
        ranges.append({"z": z, "x0": 0, "y0": 0, "x1": n - 1, "y1": n - 1})
        jobs = ((z, x, y) for x in range(n) for y in range(n))
        got = [(j, s) for j, s in run.fetch(jobs, f"world z{z}")]
        if z == a.world_zoom:
            sizes = {(j[1], j[2]): s for j, s in got}

    deep = None
    if a.deep_zoom > a.world_zoom:
        os.makedirs(os.path.join(a.out, "have"), exist_ok=True)
        for z in range(a.world_zoom + 1, a.deep_zoom + 1):
            floor = a.blank_bytes if z <= a.full_zoom else a.rich_bytes
            parents = sorted(k for k, s in sizes.items() if s > floor)
            print(f"z{z}: {4 * len(parents)} tiles to ask", flush=True)
            jobs = ((z, 2 * x + i, 2 * y + j) for x, y in parents for i in (0, 1) for j in (0, 1))
            sizes = {(j[1], j[2]): s for j, s in run.fetch(jobs, f"deep z{z}") if s > a.blank_bytes}
            with open(os.path.join(a.out, "have", f"{z}.rle"), "wb") as f:
                f.write(runs(sizes.keys(), 1 << z))
            print(f"z{z}: {len(sizes)} tiles kept", flush=True)
        deep = {"from": a.world_zoom + 1, "to": a.deep_zoom}

    manifest_path = os.path.join(a.out, "manifest.json")
    if a.region:
        for spec in a.region:
            name, box = spec.split(":")
            lon0, lat0, lon1, lat1 = (float(v) for v in box.split(","))
            for z in range(a.world_zoom + 1, a.region_zoom + 1):
                x0, y1 = tile_xy(lon0, lat0, z)
                x1, y0 = tile_xy(lon1, lat1, z)
                ranges.append({"z": z, "x0": x0, "y0": y0, "x1": x1, "y1": y1, "region": name})
                jobs = ((z, x, y) for x in range(x0, x1 + 1) for y in range(y0, y1 + 1))
                for _ in run.fetch(jobs, f"{name} z{z}"):
                    pass
    elif os.path.exists(manifest_path):
        with open(manifest_path) as f:
            ranges += [r for r in json.load(f)["ranges"] if "region" in r]

    manifest = {
        "source": "Sentinel-2 cloudless 2016 by EOX IT Services GmbH (https://s2maps.eu), CC BY 4.0",
        "format": "jpg", "tile": 256, "ranges": ranges,
    }
    if deep:
        manifest["deep"] = deep
    with open(manifest_path + ".part", "w") as f:
        json.dump(manifest, f)
    os.replace(manifest_path + ".part", manifest_path)
    print(f"done: {run.done} tiles, {run.failed} failed, {run.size >> 20} MB on disk", flush=True)
    sys.exit(1 if run.failed else 0)


if __name__ == "__main__":
    main()
