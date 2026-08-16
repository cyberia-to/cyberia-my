//! Shared Gesing land data — plots, places, area, soft ratings.

use serde::Deserialize;

pub const MAP_JSON: &str = include_str!("cyberia_map.json");
pub const FLAG_SVG: &str = include_str!("../assets/cyberia-flag.svg");

#[derive(Clone, Debug, Deserialize)]
pub struct MapData {
    pub site: String,
    #[serde(default)]
    pub stats: MapStats,
    pub phase0: Vec<LandFlat>,
    #[serde(default)]
    pub districts: Vec<LandFlat>,
    pub places: Vec<LandFlat>,
    /// 21 cybics knowledge domains as citadel shill points (no core/bridge).
    #[serde(default)]
    pub domains: Vec<DomainPoint>,
}

/// Cybics domain marker on the volcano citadel.
#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct DomainPoint {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub kind: String,
    #[serde(default)]
    pub triad: String,
    #[serde(default)]
    pub question: String,
    #[serde(default)]
    pub zone: String,
    /// Display district (rockets := etherland on map).
    #[serde(default)]
    pub district: String,
    #[serde(default)]
    pub plot: String,
    #[serde(default)]
    pub phase: u32,
    #[serde(default)]
    pub geom: String,
    pub coords: Vec<[f64; 2]>,
    #[serde(default)]
    pub shill: String,
    #[serde(default)]
    pub href: String,
}

#[derive(Clone, Debug, Default, Deserialize)]
pub struct MapStats {
    #[serde(default)]
    pub plot_count: u32,
    #[serde(default)]
    pub plot_ha: f64,
    #[serde(default)]
    pub district_ha: f64,
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct LandFlat {
    pub id: String,
    pub name: String,
    pub kind: String,
    pub phase: u32,
    pub geom: String,
    pub coords: Vec<[f64; 2]>,
    #[serde(default)]
    pub zone: String,
}

pub fn load_map() -> MapData {
    serde_json::from_str(MAP_JSON).expect("cyberia_map.json")
}

fn open_ring(coords: &[[f64; 2]]) -> Vec<[f64; 2]> {
    let mut c = coords.to_vec();
    if c.len() > 1 && c.first() == c.last() {
        c.pop();
    }
    c
}

/// Equirectangular geodesic-approx polygon area in m².
pub fn area_m2(coords: &[[f64; 2]]) -> f64 {
    let ring = open_ring(coords);
    if ring.len() < 3 {
        return 0.0;
    }
    let lat0 = ring.iter().map(|c| c[1]).sum::<f64>() / ring.len() as f64;
    let lon0 = ring.iter().map(|c| c[0]).sum::<f64>() / ring.len() as f64;
    const R: f64 = 6_378_137.0;
    let cos_lat = lat0.to_radians().cos();
    let to_xy = |lon: f64, lat: f64| -> (f64, f64) {
        let x = (lon - lon0).to_radians() * R * cos_lat;
        let y = (lat - lat0).to_radians() * R;
        (x, y)
    };
    let mut a = 0.0;
    for i in 0..ring.len() {
        let (x1, y1) = to_xy(ring[i][0], ring[i][1]);
        let j = (i + 1) % ring.len();
        let (x2, y2) = to_xy(ring[j][0], ring[j][1]);
        a += x1 * y2 - x2 * y1;
    }
    a.abs() * 0.5
}

pub fn fmt_area_m2(m2: f64) -> String {
    if m2 >= 10_000.0 {
        format!("{:.2} ha", m2 / 10_000.0)
    } else if m2 >= 100.0 {
        format!("{:.0} m²", m2)
    } else {
        format!("{:.1} m²", m2)
    }
}

pub fn centroid(coords: &[[f64; 2]]) -> Option<(f64, f64)> {
    let ring = open_ring(coords);
    if ring.is_empty() {
        return None;
    }
    let n = ring.len() as f64;
    let lon = ring.iter().map(|c| c[0]).sum::<f64>() / n;
    let lat = ring.iter().map(|c| c[1]).sum::<f64>() / n;
    Some((lon, lat))
}

/// Zone premium for soft rating (0..1).
pub fn zone_weight(zone: &str) -> f64 {
    let z = zone.to_lowercase();
    match z.as_str() {
        "core" | "avatar" => 1.0,
        "avalon" | "edem" => 0.92,
        "sinwood" => 0.85,
        "asgard" | "etherland" => 0.78,
        "front" | "canyon" => 0.70,
        "bridge" => 0.62,
        "road" => 0.45,
        _ => 0.55,
    }
}

fn hash01(s: &str) -> f64 {
    let mut h: u64 = 0xcbf29ce484222325;
    for b in s.bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    // stable 0..1
    ((h >> 11) as f64) / ((1u64 << 53) as f64)
}

/// Soft3 local ratings — no closed index yet. Stable from id/zone/area.
#[derive(Clone, Debug, PartialEq)]
pub struct PlotRating {
    /// Composite 0–100
    pub score: f64,
    /// Size / area component 0–100
    pub size: f64,
    /// Zone premium 0–100
    pub zone: f64,
    /// Named / special hold boost 0–100
    pub depth: f64,
}

pub fn plot_rating(id: &str, name: &str, zone: &str, m2: f64) -> PlotRating {
    // size: log scale, ~50 m² → low, ~5000 m² → high
    let size = ((m2.max(1.0).ln() - 3.0) / 5.5 * 100.0).clamp(8.0, 98.0);
    let zone_s = (zone_weight(zone) * 100.0).clamp(20.0, 100.0);
    let named = name.contains(':') || name.contains('@') || name.len() > 14;
    let depth_base = if named { 72.0 } else { 38.0 };
    let depth = (depth_base + hash01(id) * 28.0).clamp(20.0, 96.0);
    let score = (size * 0.40 + zone_s * 0.35 + depth * 0.25).clamp(1.0, 99.5);
    PlotRating {
        score,
        size,
        zone: zone_s,
        depth,
    }
}

pub fn rating_tier(score: f64) -> &'static str {
    if score >= 80.0 {
        "A"
    } else if score >= 65.0 {
        "B"
    } else if score >= 50.0 {
        "C"
    } else if score >= 35.0 {
        "D"
    } else {
        "E"
    }
}

/// Place category for estates / buildings / land features.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PlaceKind {
    Estate,
    Amenity,
    Nature,
    Trail,
    Landmark,
}

impl PlaceKind {
    pub fn label(self) -> &'static str {
        match self {
            PlaceKind::Estate => "ESTATE",
            PlaceKind::Amenity => "AMENITY",
            PlaceKind::Nature => "NATURE",
            PlaceKind::Trail => "TRAIL",
            PlaceKind::Landmark => "LANDMARK",
        }
    }
}

pub fn place_kind(name: &str, id: &str) -> PlaceKind {
    let s = format!("{} {}", name, id).to_lowercase();
    if s.contains("parking") || s.contains("wc") || s.contains("helipad") || s.contains("stairs") {
        return PlaceKind::Amenity;
    }
    if s.contains("path")
        || s.contains("trail")
        || s.contains("bridge")
        || s.contains("borders")
        || s.contains("way")
    {
        return PlaceKind::Trail;
    }
    if s.contains("canyon")
        || s.contains("pond")
        || s.contains("hill")
        || s.contains("stone")
        || s.contains("spring")
        || s.contains("tree")
        || s.contains("root")
        || s.contains("peak")
        || s.contains("view")
        || s.contains("rest")
        || s.contains("wall")
        || s.contains("stage")
    {
        return PlaceKind::Nature;
    }
    // named holds / buildings
    const ESTATES: &[&str] = &[
        "andara",
        "nik",
        "andrey",
        "vitalik",
        "soft",
        "elona",
        "edem",
        "obsidian",
        "firefly",
        "camping",
        "jurassic",
        "monastery",
        "laba",
        "banya",
        "organiq",
        "baikal",
        "satoshi",
        "gavin",
        "titikaka",
        "emerald",
        "yudkowsky",
        "citrin",
        "olymp",
        "avalon",
        "carrot",
        "quartz",
        "toba",
        "chickens",
        "lolok",
        "hulk",
        "chunk",
        "crimson",
        "guardian",
        "unicorn",
        "jointwood",
        "2074",
    ];
    let base = id.to_lowercase();
    if ESTATES.iter().any(|e| base.contains(e) || s.contains(e)) {
        return PlaceKind::Estate;
    }
    PlaceKind::Landmark
}

/// Soft place score for sort (estates higher, amenities mid).
pub fn place_score(name: &str, id: &str, kind: PlaceKind) -> f64 {
    let base = match kind {
        PlaceKind::Estate => 78.0,
        PlaceKind::Landmark => 62.0,
        PlaceKind::Nature => 55.0,
        PlaceKind::Trail => 48.0,
        PlaceKind::Amenity => 40.0,
    };
    base + hash01(&format!("{id}:{name}")) * 20.0
}

/// Which plot (if any) contains a lon/lat point — simple point-in-polygon.
pub fn plot_containing(plots: &[LandFlat], lon: f64, lat: f64) -> Option<&LandFlat> {
    plots.iter().find(|p| point_in_poly(lon, lat, &p.coords))
}

fn point_in_poly(lon: f64, lat: f64, coords: &[[f64; 2]]) -> bool {
    let ring = open_ring(coords);
    if ring.len() < 3 {
        return false;
    }
    let mut inside = false;
    let mut j = ring.len() - 1;
    for i in 0..ring.len() {
        let (xi, yi) = (ring[i][0], ring[i][1]);
        let (xj, yj) = (ring[j][0], ring[j][1]);
        let intersect =
            ((yi > lat) != (yj > lat)) && (lon < (xj - xi) * (lat - yi) / ((yj - yi) + 1e-18) + xi);
        if intersect {
            inside = !inside;
        }
        j = i;
    }
    inside
}
