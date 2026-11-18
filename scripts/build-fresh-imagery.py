#!/usr/bin/env python3
"""A fresh, sharp picture of a region from the Sentinel-2 scenes themselves.

The earth's mosaic we serve (Sentinel-2 cloudless 2016) is a median of a year:
old, and soft. Around a city we want this year's ground at the full ten metres.
Source: Copernicus Sentinel-2 L2A scenes as cloud-optimised GeoTIFFs on AWS
open data (Element 84 Earth Search); free for any use with the notice
"contains modified Copernicus Sentinel data <year>".

  select   per MGRS square of the region, the recent scenes with the most
           clear ground (scene classification), the city's surroundings first;
  compose  the first clear pixel in that order — no averaging, so the picture
           keeps a single scene's sharpness;
  tiles    warped to Web Mercator per zoom-14 tile, sharpened, toned, laid over
           the earth's mosaic with a feathered edge and wherever no clear pixel
           was found; written for zooms 12..--zoom-max.

    python3 build-fresh-imagery.py WORK TILES --region bali --city 115.0898,-8.2999
        [--squares 50LKR,50LLR,50LKQ,50LLQ] [--zoom-max 16] [--jobs 16]

WORK holds the downloads and WORK/out the tiles to copy over TILES (which is
only read: its zoom-14 tiles are the backdrop, its manifest names the region's
tile range; --range and --backdrop stand in for a TILES without them). Build host only: needs GDAL with Python bindings and numpy.
"""
import argparse, json, math, os, sys, urllib.request
from multiprocessing import Pool

import numpy as np
from osgeo import gdal, osr

gdal.UseExceptions()
STAC = "https://earth-search.aws.element84.com/v1/search"
UA = "cyberia.my imagery build (contact: cyberia.to)"
R = 6378137.0
CLEAR = (4, 5, 6)  # vegetation, bare ground, water
A = None  # arguments, shared with the workers


def http(url, data=None):
    req = urllib.request.Request(url, data=data, headers={"User-Agent": UA, "Content-Type": "application/json"})
    return urllib.request.urlopen(req, timeout=120)


def fetch(url, path):
    if not os.path.exists(path):
        os.makedirs(os.path.dirname(path), exist_ok=True)
        with http(url) as r, open(path + ".part", "wb") as f:
            while chunk := r.read(1 << 20):
                f.write(chunk)
        os.replace(path + ".part", path)
    return path


def lonlat_of(z, x, y):
    n = 1 << z
    return x / n * 360.0 - 180.0, math.degrees(math.atan(math.sinh(math.pi * (1 - 2 * y / n))))


def merc(z, x, y):
    """Web Mercator metres of a tile's corner."""
    n = 1 << z
    return (x / n * 2 - 1) * math.pi * R, (1 - y / n * 2) * math.pi * R


def to_pixel(ds, lon, lat):
    src = osr.SpatialReference()
    src.ImportFromEPSG(4326)
    src.SetAxisMappingStrategy(osr.OAMS_TRADITIONAL_GIS_ORDER)
    dst = osr.SpatialReference(wkt=ds.GetProjection())
    x, y, _ = osr.CoordinateTransformation(src, dst).TransformPoint(lon, lat)
    g = ds.GetGeoTransform()
    return (x - g[0]) / g[1], (y - g[3]) / g[5]


def grow(mask, n):
    """Dilate a boolean mask by n pixels."""
    for _ in range(n):
        m = mask.copy()
        m[1:] |= mask[:-1]
        m[:-1] |= mask[1:]
        m[:, 1:] |= mask[:, :-1]
        m[:, :-1] |= mask[:, 1:]
        mask = m
    return mask


def select(square, box, city):
    """The scenes of a square, the clearest first."""
    body = {"collections": ["sentinel-2-l2a"], "datetime": A.dates, "limit": 60,
            "query": {"grid:code": {"eq": f"MGRS-{square}"}, "eo:cloud_cover": {"lt": 30}}}
    items = json.load(http(STAC, json.dumps(body).encode()))["features"]
    items.sort(key=lambda f: f["properties"]["eo:cloud_cover"] + 0.3 * f["properties"].get("s2:nodata_pixel_percentage", 0))
    scored = []
    for f in items[: A.candidates]:
        scl = gdal.Open(fetch(f["assets"]["scl"]["href"], os.path.join(A.work, "scenes", f["id"], "SCL.tif")))
        a = scl.GetRasterBand(1).ReadAsArray()
        clear = np.isin(a, CLEAR)
        (x0, y0), (x1, y1) = to_pixel(scl, box[0], box[3]), to_pixel(scl, box[2], box[1])
        h, w = a.shape
        sl = (slice(max(0, int(y0)), min(h, int(y1))), slice(max(0, int(x0)), min(w, int(x1))))
        score = float(clear[sl].mean()) if clear[sl].size else 0.0
        cx, cy = to_pixel(scl, *city)
        near = clear[max(0, int(cy) - 400): int(cy) + 400, max(0, int(cx) - 400): int(cx) + 400] if 0 <= cx < w and 0 <= cy < h else None
        if near is not None and near.size:
            score += 2.0 * float(near.mean())  # eight kilometres around the city weigh twice the rest
        scored.append((score, f))
        print(f"  {f['id']} {f['properties']['datetime'][:10]} clear {score:.2f}", flush=True)
    scored.sort(key=lambda s: -s[0])
    return [f for _, f in scored[: A.scenes]]


def compose(square, scenes):
    """The first clear pixel in the order given; band 4 says where one was found."""
    path = os.path.join(A.work, f"{square}.tif")
    if os.path.exists(path):
        return path
    out = alpha = ref = None
    for step, classes in enumerate((CLEAR, CLEAR + (7, 2))):  # then what is merely unclassified or dark
        for f in scenes:
            d = os.path.join(A.work, "scenes", f["id"])
            tci = gdal.Open(fetch(f["assets"]["visual"]["href"], os.path.join(d, "TCI.tif")))
            scl_ds = gdal.Open(os.path.join(d, "SCL.tif"))  # held: a band dies with its dataset
            scl = scl_ds.GetRasterBand(1).ReadAsArray()
            ok = ~grow(~np.isin(scl, classes), 4 if step == 0 else 1)
            ok = np.repeat(np.repeat(ok, 2, axis=0), 2, axis=1)
            rgb = tci.ReadAsArray()
            ok &= rgb.max(axis=0) > 0
            if out is None:
                out, alpha, ref = np.zeros_like(rgb), np.zeros(ok.shape, bool), tci
            take = ok & ~alpha
            out[:, take] = rgb[:, take]
            alpha |= take
            print(f"  {square} pass {step} {f['id']}: {alpha.mean():.3f} filled", flush=True)
    ds = gdal.GetDriverByName("GTiff").Create(path + ".part.tif", out.shape[2], out.shape[1], 4, gdal.GDT_Byte, ["TILED=YES"])
    ds.SetGeoTransform(ref.GetGeoTransform())
    ds.SetProjection(ref.GetProjection())
    for i in range(3):
        ds.GetRasterBand(i + 1).WriteArray(out[i])
    ds.GetRasterBand(4).WriteArray(alpha.astype(np.uint8) * 255)
    ds.GetRasterBand(4).SetColorInterpretation(gdal.GCI_AlphaBand)
    ds = None
    os.replace(path + ".part.tif", path)
    return path


def blur(a, r):
    """Box blur of radius r over the last two axes, twice (close to a gaussian)."""
    for _ in range(2):
        for ax in (-2, -1):
            c = np.cumsum(np.pad(a, [(0, 0)] * (a.ndim - 2) + ([(r + 1, r), (0, 0)] if ax == -2 else [(0, 0), (r + 1, r)]), mode="edge"), axis=ax, dtype=np.float32)
            n = a.shape[ax]
            a = (np.take(c, range(2 * r + 1, 2 * r + 1 + n), axis=ax) - np.take(c, range(0, n), axis=ax)) / (2 * r + 1)
    return a


def backdrop(name):
    """The path of a tile of the earth's mosaic: in TILES, or fetched once from where it is served."""
    path = os.path.join(A.tiles, name)
    if os.path.exists(path) or not A.backdrop:
        return path
    path = os.path.join(A.work, "back", name)
    try:
        return fetch(f"{A.backdrop}/{name}", path)
    except OSError:
        return path


def read_tile(path, size):
    """A tile of the pyramid as float RGB at `size` pixels, or None."""
    if not os.path.exists(path):
        return None
    ds = gdal.Open(path)
    a = ds.ReadAsArray(buf_xsize=size, buf_ysize=size, resample_alg=gdal.GRIORA_Cubic).astype(np.float32)
    return a[:3] if a.ndim == 3 else np.stack([a] * 3)


def write_tile(rgb, z, x, y):
    path = os.path.join(A.work, "out", str(z), str(x), f"{y}.jpg")
    os.makedirs(os.path.dirname(path), exist_ok=True)
    mem = gdal.GetDriverByName("MEM").Create("", rgb.shape[2], rgb.shape[1], 3, gdal.GDT_Byte)
    for i in range(3):
        mem.GetRasterBand(i + 1).WriteArray(np.clip(rgb[i] + 0.5, 0, 255).astype(np.uint8))
    gdal.GetDriverByName("JPEG").CreateCopy(path, mem, options=[f"QUALITY={A.quality}"])


def half(a):
    return (a[:, 0::2, 0::2] + a[:, 1::2, 0::2] + a[:, 0::2, 1::2] + a[:, 1::2, 1::2]) / 4


def block(job):
    """One zoom-14 tile of the region: its picture at the deepest zoom, then down to 14."""
    x, y = job
    up = A.zoom_max - 14
    n, pad = 256 << up, 16
    (w, top), (e, bot) = merc(14, x, y), merc(14, x + 1, y + 1)
    res = (e - w) / n
    ds = gdal.Warp("", A.vrt, format="MEM", dstSRS="EPSG:3857", resampleAlg="cubic", dstAlpha=True,
                   outputBounds=(w - pad * res, bot - pad * res, e + pad * res, top + pad * res), width=n + 2 * pad, height=n + 2 * pad)
    a = ds.ReadAsArray().astype(np.float32)
    rgb, alpha = a[:3], a[3] / 255.0
    # sharpen at the scale of a source pixel, then lift the tone: the scenes come flat and dark
    rgb = rgb + A.sharpen * (rgb - blur(rgb, max(1, (1 << up) // 2 + 1)))
    rgb = 255.0 * np.clip(rgb / 255.0, 0, 1) ** A.gamma
    grey = rgb.mean(axis=0, keepdims=True)
    rgb = grey + A.saturation * (rgb - grey)
    rgb, alpha = rgb[:, pad:-pad, pad:-pad], alpha[pad:-pad, pad:-pad]
    # towards the region's edge the picture gives way to the earth's mosaic
    x0, y0, x1, y1 = A.range
    gx = (x - x0) * n + np.arange(n) + 0.5
    gy = (y - y0) * n + np.arange(n) + 0.5
    edge = np.minimum(np.minimum(gx, (x1 - x0 + 1) * n - gx)[None, :], np.minimum(gy, (y1 - y0 + 1) * n - gy)[:, None])
    alpha = alpha * np.clip(edge / (96 << up), 0, 1)
    back = read_tile(backdrop(os.path.join("14", str(x), f"{y}.jpg")), n)
    if back is None:
        back = np.broadcast_to(np.array([20, 30, 60], np.float32)[:, None, None], rgb.shape)
    rgb = rgb * alpha + back * (1 - alpha)
    for z in range(A.zoom_max, 13, -1):
        k = 1 << (z - 14)
        for j in range(k):
            for i in range(k):
                write_tile(rgb[:, j * 256:(j + 1) * 256, i * 256:(i + 1) * 256], z, x * k + i, y * k + j)
        if z > 14:
            rgb = half(rgb)
    return 1


def parent(job):
    """A tile of zoom 13 or 12 from its four children, ours where we have them."""
    z, x, y = job
    full = np.zeros((3, 512, 512), np.float32)
    for j in (0, 1):
        for i in (0, 1):
            name = os.path.join(str(z + 1), str(2 * x + i), f"{2 * y + j}.jpg")
            t = read_tile(os.path.join(A.work, "out", name), 256)
            if t is None:
                t = read_tile(backdrop(name), 256)
            if t is None:
                return 0  # a child the pyramid does not have: leave the parent as it is
            full[:, j * 256:(j + 1) * 256, i * 256:(i + 1) * 256] = t
    write_tile(half(full), z, x, y)
    return 1


def main():
    global A
    ap = argparse.ArgumentParser()
    ap.add_argument("work")
    ap.add_argument("tiles")
    ap.add_argument("--region", default="bali")
    ap.add_argument("--city", default="115.0897935,-8.2998539")
    ap.add_argument("--squares", default="50LKR,50LLR,50LKQ,50LLQ")
    ap.add_argument("--dates", default="2026-05-01T00:00:00Z/2026-11-18T00:00:00Z")
    ap.add_argument("--candidates", type=int, default=12)
    ap.add_argument("--scenes", type=int, default=6)
    ap.add_argument("--zoom-max", type=int, default=16)
    ap.add_argument("--sharpen", type=float, default=0.8)
    ap.add_argument("--gamma", type=float, default=0.82)
    ap.add_argument("--saturation", type=float, default=1.12)
    ap.add_argument("--quality", type=int, default=88)
    ap.add_argument("--jobs", type=int, default=16)
    ap.add_argument("--range", default="", help="x0,y0,x1,y1 at zoom 14, when TILES has no manifest entry for the region")
    ap.add_argument("--backdrop", default="https://cyberia.my/tiles/s2", help="where a tile missing from TILES is fetched from")
    ap.add_argument("--only", default="", help="x,y of one zoom-14 tile: a proof before the whole region")
    A = ap.parse_args()
    if A.range:
        r = dict(zip(("x0", "y0", "x1", "y1"), (int(v) for v in A.range.split(","))))
    else:
        with open(os.path.join(A.tiles, "manifest.json")) as f:
            r = next(r for r in json.load(f)["ranges"] if r.get("region") == A.region and r["z"] == 14)
    A.range = (r["x0"], r["y0"], r["x1"], r["y1"])
    w, n = lonlat_of(14, r["x0"], r["y0"])
    e, s = lonlat_of(14, r["x1"] + 1, r["y1"] + 1)
    city = tuple(float(v) for v in A.city.split(","))
    print(f"{A.region}: tiles {A.range} at zoom 14, {w:.3f},{s:.3f} .. {e:.3f},{n:.3f}", flush=True)

    parts = []
    for square in A.squares.split(","):
        path = os.path.join(A.work, f"{square}.tif")
        if not os.path.exists(path):
            print(f"{square}: choosing scenes", flush=True)
            scenes = select(square, (w, s, e, n), city)
            with open(os.path.join(A.work, f"{square}.json"), "w") as f:
                json.dump([{"id": i["id"], "datetime": i["properties"]["datetime"]} for i in scenes], f)
            compose(square, scenes)
        parts.append(path)
    A.vrt = os.path.join(A.work, "fresh.vrt")
    gdal.BuildVRT(A.vrt, parts)

    jobs = [(x, y) for x in range(r["x0"], r["x1"] + 1) for y in range(r["y0"], r["y1"] + 1)]
    if A.only:
        jobs = [tuple(int(v) for v in A.only.split(","))]
    with Pool(A.jobs) as pool:
        done = sum(pool.imap_unordered(block, jobs, chunksize=4))
        print(f"zoom 14..{A.zoom_max}: {done} blocks", flush=True)
        if not A.only:
            for z in (13, 12):
                k = 14 - z
                up = sorted({(z, x >> k, y >> k) for x, y in jobs})
                print(f"zoom {z}: {sum(pool.imap_unordered(parent, up))} of {len(up)} tiles", flush=True)
    ranges = [{"z": z, "x0": r["x0"] << (z - 14), "y0": r["y0"] << (z - 14), "x1": ((r["x1"] + 1) << (z - 14)) - 1,
               "y1": ((r["y1"] + 1) << (z - 14)) - 1, "region": A.region} for z in range(15, A.zoom_max + 1)]
    with open(os.path.join(A.work, "ranges.json"), "w") as f:
        json.dump(ranges, f)
    print("done", flush=True)


if __name__ == "__main__":
    main()
