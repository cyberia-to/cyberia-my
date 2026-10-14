#!/usr/bin/env python3
"""Give every flat its own page for crawlers — `dist/plot/<id>/index.html`.

The app is client-side; one index.html served for every route means every
plot URL carries the same title. This writes, after `trunk build`, one
HTML per flat with its own title, description, canonical, og:* and JSON-LD,
a visible summary (the app removes it on mount), the flat's `plan.svg`,
and its ERC-721 `metadata.json`. Also `sitemap.xml` and `robots.txt`.

Facts only, every one from the map or the lease book: district, land use,
size (surveyed · sheet · in plan as drawn on the Google map), price,
neighbours, coordinates.

    python3 scripts/render-plot-pages.py            # after trunk build
    python3 scripts/render-plot-pages.py --check    # print one page's head
"""
import html
import json
import math
import re
import sys
from datetime import date
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
DIST = ROOT / "dist"
MAP = ROOT / "src" / "cyberia_map.json"
TERMS = ROOT / "src" / "plot_terms.json"
SITE = "https://cyberia.my"
CITY = "cyber valley"

# land use → (label, colour, one line), the map's legend in prysm hues
LAND_USE = {
    "lease": ("residence", "#00fe00", "a home on a 25-year leasehold"),
    "dual": ("dual use", "#fcf000", "business and housing on one plot"),
    "venture": ("business", "#ff5b00", "ground for a business"),
    "hgb": ("for sale", "#00acff", "the whole district sold as one HGB title"),
    "commons": ("city land", "#d500f9", "belongs to the city, shared by everyone"),
    "special": ("special place", "#ff0000", "high-energy ground held by the valley"),
}

# district → what the graph says about it (districts/*.md)
DISTRICT = {
    "sinwood": "Sinwood, the bioluminescent forest district of the citadel: 4 ha of glowing understory under sengon and trema, the valley's night landscape",
    "bridge": "Bridge, the Gesing edge of the valley: about 5 ha of fertile west-facing hillside gardens with ocean and mountain views, reached by a private access corridor",
    "core": "Core, the densest zone of the valley, closest to the village services and the approach roads",
    "etherland": "Etherland, the lowest west edge of the citadel: open grass and the soft amenity strip",
    "avalon": "Avalon, the water hold of the citadel: a pond estate kept for one special project as a joint venture",
    "asgard": "Asgard, the residential hold of Rockets, designed for a small household cell",
    "edem": "Edem, the experimental magic forest of Rockets with 240+ genera and 300+ species",
    "canyon": "the canyon of the citadel",
    "avatar": "Avatar, the citadel's highest-value ground",
    "front": "the front of the citadel",
    "road": "the road of the citadel",
}

PLACE = "Cyber Valley, Gesing, Buleleng, Bali"


def bare_name(name: str) -> str:
    head = name.split(":@")[0]
    joined = " ".join(head.split())
    return joined.replace(" -", "-").replace("- ", "-").replace(":", " · ").upper()


def open_ring(coords):
    c = list(coords)
    if len(c) > 1 and c[0] == c[-1]:
        c.pop()
    return c


def plan_m2(coords) -> float:
    """Area in plan, the same projection as land.rs `area_m2`."""
    ring = open_ring(coords)
    if len(ring) < 3:
        return 0.0
    lat0 = sum(c[1] for c in ring) / len(ring)
    lon0 = sum(c[0] for c in ring) / len(ring)
    r = 6_378_137.0
    cos_lat = math.cos(math.radians(lat0))
    xy = [(math.radians(lon - lon0) * r * cos_lat, math.radians(lat - lat0) * r) for lon, lat in ring]
    a = 0.0
    for i, (x1, y1) in enumerate(xy):
        x2, y2 = xy[(i + 1) % len(xy)]
        a += x1 * y2 - x2 * y1
    return abs(a) * 0.5


def centroid(coords):
    ring = open_ring(coords)
    n = len(ring)
    return sum(c[0] for c in ring) / n, sum(c[1] for c in ring) / n


def meters(a, b) -> float:
    r = 6_378_137.0
    cos_lat = math.cos(math.radians((a[1] + b[1]) / 2))
    dx = math.radians(b[0] - a[0]) * r * cos_lat
    dy = math.radians(b[1] - a[1]) * r
    return math.hypot(dx, dy)


def neighbours(flat, flats, touch_m=2.0):
    """Flats that share a boundary: a vertex of each within `touch_m`."""
    out = []
    mine = open_ring(flat["coords"])
    for other in flats:
        if other["id"] == flat["id"]:
            continue
        theirs = open_ring(other["coords"])
        if any(meters(p, q) <= touch_m for p in mine for q in theirs):
            out.append(other)
    return out


def usd(v: float) -> str:
    return f"${int(round(v)):,}"


def ares(m2: float) -> str:
    s = f"{m2 / 100:.2f}".rstrip("0").rstrip(".")
    return f"{s} ares"


def status(t: dict, lu: str) -> str:
    """Mirrors deals.rs `flat_status` without the browser's own deals."""
    if not t:
        return "not on offer"
    owner = t.get("owner", "").strip()
    if owner and owner.lower() != CITY:
        return "owned"
    if t.get("hold") == "re-registration":
        return "not on offer"
    city = t.get("kind") == "commons" or owner.lower() == CITY
    priced = (t.get("plot_price_usd") or 0) > 0 and not city
    if lu == "venture" and t.get("address", "").startswith("avalon"):
        return "on request"
    if lu in ("lease", "dual", "venture") and priced:
        return "available"
    if lu in ("lease", "dual") and not city:
        return "available"
    if lu == "venture":
        return "on request"
    if lu == "hgb" and not city:
        return "on request"
    return "not on offer"


def facts(flat, t, flats):
    """Everything the page says, as data — one dict per flat."""
    lu = (t or {}).get("land_use", "")
    label, colour, line = LAND_USE.get(lu, ("land", "#4b4b4d", "land use not set on the map yet"))
    zone = flat.get("zone") or "gesing"
    drawn = plan_m2(flat["coords"])
    measured = (t or {}).get("measured_m2")
    sheet_ares = (t or {}).get("ares")
    size_m2 = measured if measured else (sheet_ares * 100 if sheet_ares else drawn)
    size = []
    if measured:
        size.append(f"{measured:,.2f} m² surveyed by triangulation on the slope on {t.get('measured_on')}")
    elif sheet_ares:
        size.append(f"{sheet_ares * 100:,.0f} m² on the plot sheet")
    size.append(f"about {drawn:,.0f} m² in plan as drawn on the Google map")
    st = status(t, lu)
    price = None
    if t:
        p, a = t.get("plot_price_usd"), t.get("are_price_usd")
        if lu == "hgb" and a:
            price = f"{usd(a)} per are, the whole district as one HGB title"
        elif st in ("available", "owned") and p:
            price = f"{usd(p)}" + (f" ({usd(a)} per are)" if a else "")
        elif lu == "venture" and zone == "avalon":
            price = "joint-venture terms"
        elif st == "on request":
            price = "price on request"
    nb = [bare_name(n["name"]) for n in neighbours(flat, flats)]
    lon, lat = centroid(flat["coords"])
    return {
        "id": flat["id"],
        "name": bare_name(flat["name"]),
        "zone": zone,
        "district": DISTRICT.get(zone, f"{zone} in the citadel"),
        "land_use": lu,
        "label": label,
        "colour": colour,
        "line": line,
        "status": st,
        "size_m2": size_m2,
        "size": size,
        "drawn_m2": drawn,
        "measured_m2": measured,
        "measured_on": (t or {}).get("measured_on"),
        "price": price,
        "neighbours": nb,
        "lat": lat,
        "lon": lon,
        "coords": flat["coords"],
        "note": (t or {}).get("note", ""),
        "niche": (t or {}).get("niche", ""),
        "hold": (t or {}).get("hold", ""),
    }


def title(f) -> str:
    return f"{f['name']} · {ares(f['size_m2'])} {f['label']} in {f['zone'].title()}, Cyber Valley, Bali"


def description(f) -> str:
    s = f"{f['name']}: {ares(f['size_m2'])} of {f['label']} ground — {f['line']} — in {f['district']}, {PLACE}. "
    s += "Size: " + "; ".join(f["size"]) + ". "
    if f["price"]:
        s += f"Price: {f['price']}. "
    if f["hold"] == "re-registration":
        s += "The title is being re-registered; off sale until it settles. "
    if f["note"]:
        s += f"{f['note'][0].upper()}{f['note'][1:]}. "
    if f["niche"]:
        s += f"Kept for a {f['niche']} business. "
    if f["neighbours"]:
        s += "Neighbours: " + ", ".join(f["neighbours"]) + ". "
    s += f"Status: {f['status']}. Centre {f['lat']:.5f}, {f['lon']:.5f}."
    return s


def json_ld(f) -> str:
    place = {
        "@context": "https://schema.org",
        "@type": "Place",
        "@id": f"{SITE}/plot/{f['id']}/",
        "name": f["name"],
        "description": description(f),
        "url": f"{SITE}/plot/{f['id']}/",
        "geo": {"@type": "GeoCoordinates", "latitude": round(f["lat"], 6), "longitude": round(f["lon"], 6)},
        "address": {"@type": "PostalAddress", "addressLocality": "Gesing", "addressRegion": "Bali", "addressCountry": "ID"},
        "containedInPlace": {"@type": "Place", "name": "Cyber Valley"},
        "additionalProperty": [
            {"@type": "PropertyValue", "name": "district", "value": f["zone"]},
            {"@type": "PropertyValue", "name": "land use", "value": f["label"]},
            {"@type": "PropertyValue", "name": "area", "value": round(f["size_m2"], 2), "unitCode": "MTK"},
            {"@type": "PropertyValue", "name": "area in plan on the map", "value": round(f["drawn_m2"]), "unitCode": "MTK"},
            {"@type": "PropertyValue", "name": "status", "value": f["status"]},
        ],
    }
    if f["measured_m2"]:
        place["additionalProperty"].append(
            {"@type": "PropertyValue", "name": "surveyed area", "value": f["measured_m2"], "unitCode": "MTK", "valueReference": f["measured_on"]}
        )
    if f["price"] and f["status"] == "available" and f["price"].startswith("$"):
        amount = int(re.sub(r"[^\d]", "", f["price"].split(" ")[0]))
        place["offers"] = {
            "@type": "Offer",
            "name": "25-year leasehold (hak sewa)",
            "price": amount,
            "priceCurrency": "USD",
            "availability": "https://schema.org/InStock",
            "url": f"{SITE}/plot/{f['id']}/",
            "seller": {"@type": "Organization", "name": "PT Cyber Valley Estate"},
        }
    return json.dumps(place, ensure_ascii=False, separators=(",", ":"))


def plan_svg(f) -> str:
    """The flat's outline in its land-use colour on black, north up."""
    ring = open_ring(f["coords"])
    lat0 = sum(c[1] for c in ring) / len(ring)
    lon0 = sum(c[0] for c in ring) / len(ring)
    r = 6_378_137.0
    cos_lat = math.cos(math.radians(lat0))
    pts = [(math.radians(lon - lon0) * r * cos_lat, -math.radians(lat - lat0) * r) for lon, lat in ring]
    xs, ys = [p[0] for p in pts], [p[1] for p in pts]
    w, h = max(xs) - min(xs), max(ys) - min(ys)
    scale = 560 / max(w, h, 1)
    pad = 40
    poly = " ".join(f"{(x - min(xs)) * scale + pad:.1f},{(y - min(ys)) * scale + pad:.1f}" for x, y in pts)
    cw, ch = w * scale + 2 * pad, h * scale + 2 * pad
    return (
        f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {cw:.0f} {ch + 48:.0f}" width="{cw:.0f}" height="{ch + 48:.0f}">'
        f'<rect width="100%" height="100%" fill="#000"/>'
        f'<polygon points="{poly}" fill="{f["colour"]}" fill-opacity="0.18" stroke="{f["colour"]}" stroke-width="3"/>'
        f'<text x="{pad}" y="{ch + 30:.0f}" font-family="Play, sans-serif" font-size="22" fill="#fff">{html.escape(f["name"])} · {html.escape(ares(f["size_m2"]))} · {html.escape(f["zone"])}</text>'
        "</svg>\n"
    )


def metadata(f) -> str:
    """ERC-721 metadata: the plot as a token, the same facts."""
    attrs = [
        {"trait_type": "district", "value": f["zone"]},
        {"trait_type": "land use", "value": f["label"]},
        {"trait_type": "area m2", "value": round(f["size_m2"], 2)},
        {"trait_type": "area in plan m2", "value": round(f["drawn_m2"])},
        {"trait_type": "status", "value": f["status"]},
        {"trait_type": "vertices", "value": len(open_ring(f["coords"]))},
    ]
    if f["measured_m2"]:
        attrs.append({"trait_type": "surveyed m2", "value": f["measured_m2"]})
        attrs.append({"trait_type": "surveyed on", "value": f["measured_on"]})
    if f["price"]:
        attrs.append({"trait_type": "price", "value": f["price"]})
    doc = {
        "name": f"{f['name']} · Cyber Valley",
        "description": description(f),
        "image": f"{SITE}/plot/{f['id']}/plan.svg",
        "external_url": f"{SITE}/plot/{f['id']}/",
        "attributes": attrs,
        "properties": {
            "centre": {"lat": round(f["lat"], 6), "lon": round(f["lon"], 6)},
            "boundary_lonlat": f["coords"],
            "boundary_kml": f"{SITE}/plot/{f['id']}/boundary.kml",
            "right": "hak sewa, 25 years, PT Cyber Valley Estate",
            "terms": "https://cyberia.blog/cyber-valley/products/leasehold/",
        },
    }
    return json.dumps(doc, ensure_ascii=False, indent=1) + "\n"


def boundary_kml(f) -> str:
    ring = " ".join(f"{c[0]},{c[1]},0" for c in f["coords"])
    return (
        '<?xml version="1.0" encoding="UTF-8"?>\n<kml xmlns="http://www.opengis.net/kml/2.2"><Document>'
        f"<name>{html.escape(f['name'])}</name><Placemark><name>{html.escape(f['name'])}</name>"
        f"<description>{html.escape(f['zone'])} · cyberia.my</description>"
        f"<Polygon><outerBoundaryIs><LinearRing><coordinates>{ring}</coordinates></LinearRing></outerBoundaryIs></Polygon>"
        "</Placemark></Document></kml>\n"
    )


def summary_html(f) -> str:
    rows = [
        ("district", f["zone"]),
        ("land use", f"{f['label']} — {f['line']}"),
        ("size", "; ".join(f["size"])),
        ("price", f["price"] or "—"),
        ("status", f["status"]),
        ("neighbours", ", ".join(f["neighbours"]) or "—"),
        ("centre", f"{f['lat']:.5f}, {f['lon']:.5f}"),
    ]
    table = "".join(f"<tr><th>{html.escape(k)}</th><td>{html.escape(v)}</td></tr>" for k, v in rows)
    return (
        '<article id="plot-seo" style="font-family:Play,sans-serif;color:#fff;background:#000;padding:24px;max-width:720px;">'
        f"<h1>{html.escape(f['name'])}</h1>"
        f"<p>{html.escape(ares(f['size_m2']))} of {html.escape(f['label'])} ground in {html.escape(f['district'])}, {PLACE}.</p>"
        f"<table>{table}</table>"
        f'<p><a href="/plot/{f["id"]}/plan.svg">plan</a> · <a href="/plot/{f["id"]}/boundary.kml">boundary KML</a> · '
        f'<a href="/plot/{f["id"]}/metadata.json">token metadata</a> · <a href="/map?plot={f["id"]}">on the map</a> · '
        '<a href="https://cyberia.blog/cyber-valley/products/leasehold/">leasehold terms</a></p>'
        "</article>"
    )


def page(shell: str, f) -> str:
    t = html.escape(title(f), quote=True)
    d = html.escape(description(f), quote=True)
    # nginx answers /plot/<id> with a redirect to /plot/<id>/ — the slash form is canonical
    url = f"{SITE}/plot/{f['id']}/"
    out = shell
    out = re.sub(r"<title>.*?</title>", f"<title>{t}</title>", out, count=1, flags=re.S)
    out = re.sub(r'<meta\s+name="description"\s+content=".*?"\s*/>', f'<meta name="description" content="{d}" />', out, count=1, flags=re.S)
    out = re.sub(r'<link rel="canonical" href=".*?" />', f'<link rel="canonical" href="{url}" />', out, count=1)
    out = re.sub(r'<meta property="og:url" content=".*?" />', f'<meta property="og:url" content="{url}" />', out, count=1)
    out = re.sub(r'<meta property="og:title" content=".*?" />', f'<meta property="og:title" content="{t}" />', out, count=1)
    out = re.sub(r'<meta\s+property="og:description"\s+content=".*?"\s*/>', f'<meta property="og:description" content="{d}" />', out, count=1, flags=re.S)
    out = re.sub(r'<meta property="og:type" content=".*?" />', '<meta property="og:type" content="place" />', out, count=1)
    extra = (
        f'<meta property="place:location:latitude" content="{f["lat"]:.6f}" />'
        f'<meta property="place:location:longitude" content="{f["lon"]:.6f}" />'
        f'<meta name="geo.position" content="{f["lat"]:.6f};{f["lon"]:.6f}" />'
        '<meta name="geo.placename" content="Gesing, Buleleng, Bali" />'
        '<meta name="geo.region" content="ID-BA" />'
        f'<link rel="alternate" type="application/json" href="{url}/metadata.json" />'
        f'<script type="application/ld+json">{json_ld(f)}</script>'
    )
    out = out.replace("</head>", extra + "</head>", 1)
    out = out.replace("<body>", "<body>" + summary_html(f), 1)
    return out


def main() -> int:
    shell_path = DIST / "index.html"
    if not shell_path.exists():
        print("dist/index.html missing — run trunk build first", file=sys.stderr)
        return 1
    shell = shell_path.read_text()
    flats = json.load(MAP.open())["phase0"]
    terms = json.load(TERMS.open())
    today = date.today().isoformat()
    urls = [f"{SITE}/", f"{SITE}/map", f"{SITE}/cities"]
    check = "--check" in sys.argv
    for flat in flats:
        f = facts(flat, terms.get(flat["id"]), flats)
        d = DIST / "plot" / f["id"]
        d.mkdir(parents=True, exist_ok=True)
        (d / "index.html").write_text(page(shell, f))
        (d / "plan.svg").write_text(plan_svg(f))
        (d / "metadata.json").write_text(metadata(f))
        (d / "boundary.kml").write_text(boundary_kml(f))
        urls.append(f"{SITE}/plot/{f['id']}/")
        if check and f["id"] == "sinwood-7":
            print(title(f))
            print(description(f))
            print(json_ld(f))
    sitemap = ['<?xml version="1.0" encoding="UTF-8"?>', '<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">']
    sitemap += [f"<url><loc>{u}</loc><lastmod>{today}</lastmod></url>" for u in urls]
    sitemap.append("</urlset>")
    (DIST / "sitemap.xml").write_text("\n".join(sitemap) + "\n")
    (DIST / "robots.txt").write_text(f"User-agent: *\nAllow: /\nSitemap: {SITE}/sitemap.xml\n")
    print(f"✓ {len(flats)} plot pages, sitemap with {len(urls)} urls → dist/")
    return 0


if __name__ == "__main__":
    sys.exit(main())
