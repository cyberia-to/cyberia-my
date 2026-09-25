//! Experimental cyberia console: fleets & flats.
//!
//! soft3 compliance (experimental surface):
//! - intent is first-class (assign fleet × action × flat)
//! - open map data (public Gesing KML), no closed gatekeeper
//! - client-local queue only — no proprietary backend API
//! - vocabulary: fleet (work), flat (hold/space), intent (write)

use crate::deals::{flat_status, fmt_ts, load_dd, load_deals, load_jv, request_dd, request_jv, FlatStatus};
use crate::terms::{
    bare_name, district_sale, fmt_usd, land_use, price_line, size_line, terms, LandUse, AVALON_TRACKS,
};
use crate::land::FLAG_SVG;
use crate::nav::CyberiaNav;
use crate::robots::load_owned;
use crate::wallet::{load_intents, load_leases, push_intent};
use leptos::prelude::*;
use leptos_router::hooks::use_query_map;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use wasm_bindgen::closure::Closure;
use wasm_bindgen::JsCast;

const MAP_JSON: &str = include_str!("cyberia_map.json");

#[derive(Clone, Debug, Deserialize)]
struct MapData {
    site: String,
    center: [f64; 2],
    bbox: BBox,
    #[serde(default)]
    stats: MapStats,
    /// All interactive land plots (from KML `plots` folder).
    phase0: Vec<Flat>,
    #[serde(default)]
    districts: Vec<Flat>,
    places: Vec<Flat>,
    /// 21 cybics domains as citadel shill points (no core/bridge).
    #[serde(default)]
    domains: Vec<DomainMark>,
    source: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
struct DomainMark {
    id: String,
    name: String,
    #[serde(default)]
    triad: String,
    #[serde(default)]
    question: String,
    #[serde(default)]
    zone: String,
    #[serde(default)]
    district: String,
    #[serde(default)]
    shill: String,
    #[serde(default)]
    href: String,
    coords: Vec<[f64; 2]>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum LeftBoard {
    Fleets,
    Domains,
}

const DOMAIN_POS_KEY: &str = "cyberia.domain_positions.v1";

fn triad_color(triad: &str) -> &'static str {
    match triad {
        "form" => "#00b4ff",  // cyan — rules
        "mass" => "#ff6501",  // orange — matter
        "space" => "#1700fe", // blue — where
        "life" => "#00ff01",  // green — alive
        "word" => "#6b00fe",  // violet — meaning
        "work" => "#ffc501",  // yellow — making
        "play" => "#fe0000",  // red — coordinate
        _ => "#00ff01",
    }
}

/// Canonical cybics triad order for domain board grouping.
const TRIAD_ORDER: &[&str] = &["form", "mass", "space", "life", "word", "work", "play"];

fn triad_question(triad: &str) -> &'static str {
    match triad {
        "form" => "what are the rules?",
        "mass" => "what is it made of?",
        "space" => "where does it happen?",
        "life" => "who is alive?",
        "word" => "what does it mean?",
        "work" => "how is it made?",
        "play" => "how do we coordinate?",
        _ => "",
    }
}

#[derive(Clone, Debug, Default, Deserialize)]
struct MapStats {
    #[serde(default)]
    plot_count: u32,
    #[serde(default)]
    plot_ha: f64,
    #[serde(default)]
    district_ha: f64,
}

#[derive(Clone, Debug, Deserialize)]
struct BBox {
    min_lon: f64,
    max_lon: f64,
    min_lat: f64,
    max_lat: f64,
}

fn load_domain_positions() -> HashMap<String, [f64; 2]> {
    web_sys::window()
        .and_then(|w| w.local_storage().ok().flatten())
        .and_then(|ls| ls.get_item(DOMAIN_POS_KEY).ok().flatten())
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

fn save_domain_positions(domains: &[DomainMark]) {
    let mut m = HashMap::new();
    for d in domains {
        if let Some(c) = d.coords.first() {
            m.insert(d.id.clone(), [c[0], c[1]]);
            m.insert(d.name.clone(), [c[0], c[1]]);
        }
    }
    if let Some(ls) = web_sys::window().and_then(|w| w.local_storage().ok().flatten()) {
        if let Ok(s) = serde_json::to_string(&m) {
            let _ = ls.set_item(DOMAIN_POS_KEY, &s);
        }
    }
}

fn clear_domain_positions() {
    if let Some(ls) = web_sys::window().and_then(|w| w.local_storage().ok().flatten()) {
        let _ = ls.remove_item(DOMAIN_POS_KEY);
    }
}

fn apply_domain_positions(mut domains: Vec<DomainMark>) -> Vec<DomainMark> {
    let pos = load_domain_positions();
    for d in &mut domains {
        if let Some(c) = pos.get(&d.id).or_else(|| pos.get(&d.name)) {
            d.coords = vec![[c[0], c[1]]];
        }
    }
    domains
}

/// flat id → [lon, lat] of the worksite the plot's owner set on the ground.
const WORK_AT_KEY: &str = "cyberia.work_at.v1";

fn load_work_at() -> HashMap<String, [f64; 2]> {
    web_sys::window()
        .and_then(|w| w.local_storage().ok().flatten())
        .and_then(|ls| ls.get_item(WORK_AT_KEY).ok().flatten())
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

fn save_work_at(m: &HashMap<String, [f64; 2]>) {
    if let Some(ls) = web_sys::window().and_then(|w| w.local_storage().ok().flatten()) {
        if let Ok(s) = serde_json::to_string(m) {
            let _ = ls.set_item(WORK_AT_KEY, &s);
        }
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
struct Flat {
    id: String,
    name: String,
    kind: String,
    phase: u32,
    geom: String,
    coords: Vec<[f64; 2]>,
    #[serde(default)]
    zone: String,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct FleetUnit {
    id: &'static str,
    name: &'static str,
    kind: &'static str, // worker | machine
    role: &'static str,
    status: &'static str, // idle | busy | offline
    phase: u32,
}

/// Gesing hard-force workers (ops roster) — six on the map.
/// Source: cve/ops/hard force.md crews (repair · cube · base · pruning).
const WORKERS: &[FleetUnit] = &[
    FleetUnit {
        id: "w-sutar",
        name: "SUTAR",
        kind: "worker",
        role: "repair lead · energy/water",
        status: "idle",
        phase: 0,
    },
    FleetUnit {
        id: "w-sudi",
        name: "SUDI",
        kind: "worker",
        role: "cube lead · build",
        status: "idle",
        phase: 0,
    },
    FleetUnit {
        id: "w-budi",
        name: "BUDI",
        kind: "worker",
        role: "cube · build",
        status: "idle",
        phase: 0,
    },
    FleetUnit {
        id: "w-angga",
        name: "ANGGA",
        kind: "worker",
        role: "base lead · road/trail",
        status: "idle",
        phase: 0,
    },
    FleetUnit {
        id: "w-arima",
        name: "ARIMA",
        kind: "worker",
        role: "pruning lead · land",
        status: "idle",
        phase: 0,
    },
    FleetUnit {
        id: "w-suardita",
        name: "SUARDITA",
        kind: "worker",
        role: "pruning · compost",
        status: "idle",
        phase: 0,
    },
];

fn all_stock_fleets() -> impl Iterator<Item = &'static FleetUnit> {
    WORKERS.iter()
}

fn stock_fleet(id: &str) -> Option<&'static FleetUnit> {
    all_stock_fleets().find(|f| f.id == id)
}

/// Open ring (no duplicate close point).
fn open_ring(coords: &[[f64; 2]]) -> Vec<[f64; 2]> {
    let mut c = coords.to_vec();
    if c.len() > 1 && c.first() == c.last() {
        c.pop();
    }
    c
}

fn poly_bbox(coords: &[[f64; 2]]) -> (f64, f64, f64, f64) {
    let ring = open_ring(coords);
    let mut min_lon = f64::INFINITY;
    let mut max_lon = f64::NEG_INFINITY;
    let mut min_lat = f64::INFINITY;
    let mut max_lat = f64::NEG_INFINITY;
    for p in &ring {
        min_lon = min_lon.min(p[0]);
        max_lon = max_lon.max(p[0]);
        min_lat = min_lat.min(p[1]);
        max_lat = max_lat.max(p[1]);
    }
    (min_lon, max_lon, min_lat, max_lat)
}

/// Planar area in m² via equirectangular projection around centroid (WGS84).
fn area_m2(coords: &[[f64; 2]]) -> f64 {
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
    (a.abs()) * 0.5
}

fn fmt_area_m2(m2: f64) -> String {
    if m2 >= 10_000.0 {
        format!("{:.2} ha ({:.0} m²)", m2 / 10_000.0, m2)
    } else if m2 >= 100.0 {
        format!("{:.0} m²", m2)
    } else {
        format!("{:.1} m²", m2)
    }
}

fn zone_key(flat: &Flat) -> &str {
    if !flat.zone.is_empty() {
        return flat.zone.as_str();
    }
    // fallback: prefix of id/name
    let s = if flat.id.is_empty() {
        flat.name.as_str()
    } else {
        flat.id.as_str()
    };
    s
}

/// District colour — it lives on the outline only.
fn zone_color(zone_or_id: &str) -> &'static str {
    let z = zone_or_id.to_lowercase();
    if z.contains("avalon") {
        "#ff6600"
    } else if z.contains("sinwood") {
        "#00ff41"
    } else if z.contains("bridge") {
        "#00e5ff"
    } else if z.contains("core") {
        "#ffd700"
    } else if z.contains("ether") {
        "#9945ff"
    } else if z.contains("asgard") {
        "#ff0040"
    } else if z.contains("edem") || z.contains("canyon") {
        "#00ffd0"
    } else {
        "#c8c8d2"
    }
}

/// Land-use colour is the fill (prysm: acid, not pastel); how free the flat
/// is sets the strength, hover / select lift it.
fn land_fill(lu: LandUse, status: FlatStatus, selected: bool, hover: bool) -> String {
    let hex = lu.color();
    let r = u8::from_str_radix(&hex[1..3], 16).unwrap_or(255);
    let g = u8::from_str_radix(&hex[3..5], 16).unwrap_or(255);
    let b = u8::from_str_radix(&hex[5..7], 16).unwrap_or(255);
    let lift = if selected {
        0.32
    } else if hover {
        0.2
    } else {
        0.0
    };
    let a = (status.alpha() + lift).min(0.95);
    format!("rgba({r},{g},{b},{a:.2})")
}

fn render_fleet_card(
    f: &'static FleetUnit,
    selected_fleet: RwSignal<Option<String>>,
) -> impl IntoView {
    let id = f.id.to_string();
    let id2 = id.clone();
    let offline = f.status == "offline" || f.phase > 0;
    let kind_worker = f.kind == "worker";
    let name = f.name;
    let role = f.role;
    let kind = f.kind;
    let status = f.status;
    let phase = f.phase;
    view! {
        <button
            class=move || {
                let sel = selected_fleet.get().as_deref() == Some(id.as_str());
                format!(
                    "fleet-card{}{}{}",
                    if sel { " sel" } else { "" },
                    if offline { " offline" } else { "" },
                    if kind_worker { " worker" } else { " machine" },
                )
            }
            disabled=offline
            on:click=move |_| {
                if !offline {
                    selected_fleet.set(Some(id2.clone()));
                }
            }
        >
            <div class="fleet-top">
                <span class="fleet-name">{name}</span>
                <span class="fleet-status" style:color=status_color(status)>
                    {status.to_uppercase()}
                </span>
            </div>
            <div class="fleet-role">{role}</div>
            <div class="fleet-meta">
                <span>{kind.to_uppercase()}</span>
                <span>{format!("P{phase}")}</span>
            </div>
        </button>
    }
}


fn load_map() -> MapData {
    serde_json::from_str(MAP_JSON).expect("cyberia_map.json")
}

/// Project lon/lat into a viewBox-local SVG plane (y flipped).
fn project(lon: f64, lat: f64, bbox: &BBox, w: f64, h: f64, pad: f64) -> (f64, f64) {
    let dx = (bbox.max_lon - bbox.min_lon).max(1e-9);
    let dy = (bbox.max_lat - bbox.min_lat).max(1e-9);
    // square-ish fit with padding
    let x = pad + (lon - bbox.min_lon) / dx * (w - 2.0 * pad);
    let y = pad + (1.0 - (lat - bbox.min_lat) / dy) * (h - 2.0 * pad);
    (x, y)
}

/// Inverse of `project` — SVG/world → lon/lat (for domain drag).
fn unproject(x: f64, y: f64, bbox: &BBox, w: f64, h: f64, pad: f64) -> (f64, f64) {
    let dx = (bbox.max_lon - bbox.min_lon).max(1e-9);
    let dy = (bbox.max_lat - bbox.min_lat).max(1e-9);
    let uw = (w - 2.0 * pad).max(1e-9);
    let uh = (h - 2.0 * pad).max(1e-9);
    let lon = bbox.min_lon + (x - pad) / uw * dx;
    let lat = bbox.min_lat + (1.0 - (y - pad) / uh) * dy;
    (lon, lat)
}

fn poly_path(coords: &[[f64; 2]], bbox: &BBox, w: f64, h: f64, pad: f64) -> String {
    if coords.is_empty() {
        return String::new();
    }
    let mut s = String::new();
    for (i, c) in coords.iter().enumerate() {
        let (x, y) = project(c[0], c[1], bbox, w, h, pad);
        if i == 0 {
            s.push_str(&format!("M{x:.2},{y:.2}"));
        } else {
            s.push_str(&format!(" L{x:.2},{y:.2}"));
        }
    }
    s.push('Z');
    s
}

/// Game camera zoom range.
/// Max ~1000 → ~1.5 m view width on Gesing site → 10 cm tile ≈ 60–70 screen px
/// (enough to layout tiny hex/tiles as a planning game).
const MAP_ZOOM_MIN: f64 = 0.5;
const MAP_ZOOM_MAX: f64 = 1000.0;

/// Keep HUD-sized marks (labels, dots) constant on screen while viewBox zooms.
/// Anchor at (x,y) in SVG/world units.
fn screen_stable_tf(x: f64, y: f64, zoom: f64) -> String {
    let s = 1.0 / zoom.max(0.25);
    format!("translate({x:.2},{y:.2}) scale({s:.4})")
}

/// SVG camera: viewBox centered at `center` with size (W/z)×(H/z).
/// Pure vector zoom — no CSS scale raster blur.
fn map_view_box(center: (f64, f64), zoom: f64, w: f64, h: f64) -> String {
    let z = zoom.max(0.25);
    let vw = w / z;
    let vh = h / z;
    format!(
        "{:.4} {:.4} {:.4} {:.4}",
        center.0 - vw * 0.5,
        center.1 - vh * 0.5,
        vw,
        vh
    )
}

/// Site envelope size in meters from lon/lat bbox (approx WGS84).
fn site_span_m(bbox: &BBox) -> (f64, f64) {
    let lat0 = (bbox.min_lat + bbox.max_lat) * 0.5;
    let m_lat = 111_320.0;
    let m_lon = 111_320.0 * lat0.to_radians().cos();
    let ew = (bbox.max_lon - bbox.min_lon) * m_lon;
    let ns = (bbox.max_lat - bbox.min_lat) * m_lat;
    (ew, ns)
}

/// Visible world width (meters) at current zoom — for HUD scale readout.
fn view_width_m(zoom: f64, map_w: f64, pad: f64, site_ew_m: f64) -> f64 {
    let z = zoom.max(0.25);
    let usable = (map_w - 2.0 * pad).max(1.0);
    let m_per_svg = site_ew_m / usable;
    (map_w / z) * m_per_svg
}

fn format_zoom_hud(zoom: f64, view_m: f64) -> String {
    let z = zoom;
    let z_s = if z >= 100.0 {
        format!("×{z:.0}")
    } else if z >= 10.0 {
        format!("×{z:.1}")
    } else {
        format!("×{z:.2}")
    };
    let scale = if view_m >= 100.0 {
        format!("~{view_m:.0} m")
    } else if view_m >= 10.0 {
        format!("~{view_m:.1} m")
    } else if view_m >= 1.0 {
        format!("~{view_m:.2} m")
    } else {
        format!("~{:.0} cm", view_m * 100.0)
    };
    format!("{z_s} · {scale}")
}

/// Map client point (relative to wrap) → world/SVG coords under current camera.
pub(crate) fn client_to_world(
    cx: f64,
    cy: f64,
    wrap_w: f64,
    wrap_h: f64,
    center: (f64, f64),
    zoom: f64,
    w: f64,
    h: f64,
) -> (f64, f64) {
    let z = zoom.max(0.25);
    let vw = w / z;
    let vh = h / z;
    let left = center.0 - vw * 0.5;
    let top = center.1 - vh * 0.5;
    // meet: uniform scale of viewBox into wrap
    let fit = (wrap_w / vw).min(wrap_h / vh);
    let ox = (wrap_w - vw * fit) * 0.5;
    let oy = (wrap_h - vh * fit) * 0.5;
    let wx = left + (cx - ox) / fit;
    let wy = top + (cy - oy) / fit;
    (wx, wy)
}

fn centroid(coords: &[[f64; 2]]) -> (f64, f64) {
    if coords.is_empty() {
        return (0.0, 0.0);
    }
    let n = coords.len() as f64;
    let lon = coords.iter().map(|c| c[0]).sum::<f64>() / n;
    let lat = coords.iter().map(|c| c[1]).sum::<f64>() / n;
    (lon, lat)
}

fn status_color(s: &str) -> &'static str {
    match s {
        "idle" => "var(--cyber-green)",
        "busy" => "var(--cyber-yellow)",
        _ => "#555",
    }
}

/// Ops map — fleets rail + flats.
#[component]
pub fn ValleyConsole() -> impl IntoView {
    view! { <MapConsole domains_board=false /> }
}

/// Domains board — same camera, left rail = 21 cybics (draggable shill points).
#[component]
pub fn DomainsBoard() -> impl IntoView {
    view! { <MapConsole domains_board=true /> }
}

#[component]
fn MapConsole(domains_board: bool) -> impl IntoView {
    let map = load_map();
    let map = std::sync::Arc::new(map);
    let map_bbox = map.bbox.clone();
    let base_domains = map.domains.clone();

    let selected_flat = RwSignal::new({
        let from_q = web_sys::window()
            .and_then(|w| w.location().search().ok())
            .and_then(|s| {
                let s = s.trim_start_matches('?');
                s.split('&').find_map(|pair| {
                    let mut it = pair.splitn(2, '=');
                    let k = it.next()?;
                    let v = it.next().unwrap_or("");
                    if k == "plot" && !v.is_empty() {
                        // plot ids are ascii (core-0, sinwood-1-laba, …)
                        Some(v.replace('+', " "))
                    } else {
                        None
                    }
                })
            });
        from_q
            .filter(|id| map.phase0.iter().any(|f| f.id == *id))
            .or_else(|| map.phase0.first().map(|f| f.id.clone()))
            .or_else(|| Some("sinwood".into()))
    });
    // the `?plot=` initializer above races leptos_router's own URL update on
    // client-side nav (e.g. a `/plots` card linking to `/map?plot=X`) — the
    // window.location read above can fire before the router commits the new
    // query, silently falling back to the first plot. Re-sync reactively off
    // the router's own query memo so every `plot=` change actually lands,
    // not just the one present at first mount.
    {
        let map = map.clone();
        Effect::new(move |_| {
            let Some(pid) = use_query_map().get().get("plot") else {
                return;
            };
            if !map.phase0.iter().any(|f| f.id == pid) {
                return;
            }
            if selected_flat.get_untracked().as_deref() != Some(pid.as_str()) {
                selected_flat.set(Some(pid));
            }
        });
    }
    let selected_fleet = RwSignal::new(Some("w-sutar".to_string()));
    let intents = RwSignal::new(load_intents());
    let dd = RwSignal::new(load_dd());
    let jv = RwSignal::new(load_jv());
    let owned = RwSignal::new(load_owned());
    let leased = RwSignal::new(
        load_leases()
            .into_iter()
            .map(|l| l.flat_id)
            .collect::<Vec<String>>(),
    );
    let flats = RwSignal::new(map.phase0.clone()); // live land geometry (split/merge)
    // per-flat worksite: where the assigned robot stands, set by the plot's
    // owner (device GPS on site, or a manual tap in placing mode as fallback)
    let work_at = RwSignal::new(load_work_at());
    let placing_work = RwSignal::new(false);
    let gps_status = RwSignal::new(String::new());
    // (flat name, why it can't be leased) — the dock's LEASE press on a held flat
    let taken_popup = RwSignal::new(None::<(String, String, String, FlatStatus)>);
    let deals = RwSignal::new(load_deals());

    // left rail: fleets (ops) or domains (shill points)
    let left_board = RwSignal::new(if domains_board {
        LeftBoard::Domains
    } else {
        LeftBoard::Fleets
    });
    let domains = RwSignal::new(apply_domain_positions(base_domains.clone()));
    let selected_domain = RwSignal::new(
        domains
            .get_untracked()
            .first()
            .map(|d| d.id.clone())
            .or_else(|| Some("domain-ai".into())),
    );
    // live domain drag (id) — positions persist to localStorage
    let domain_drag = RwSignal::new(None::<String>);
    let domain_did_drag = RwSignal::new(false);

    // game map camera — SVG viewBox zoom/pan (vector-sharp; never CSS scale)
    // max zoom high enough for ~10 cm tile/hex planning on the ground
    const MAP_W: f64 = 960.0;
    const MAP_H: f64 = 720.0;
    const MAP_PAD: f64 = 20.0;
    let site_ew_m = site_span_m(&map.bbox).0;
    let map_zoom = RwSignal::new(1.0_f64);
    let map_center = RwSignal::new((MAP_W * 0.5, MAP_H * 0.5)); // world/SVG units
    let map_hover = RwSignal::new(None::<(String, String, f64)>); // id, label, m2
    let map_dragging = RwSignal::new(false);
    // true once pointer moved past threshold — suppress plot-click select after pan
    let map_did_drag = RwSignal::new(false);
    let map_drag_last = RwSignal::new((0.0_f64, 0.0_f64));
    let map_wrap_ref = NodeRef::<leptos::html::Div>::new();

    // focus camera on a domain id (used from list cards)

    let zoom_by = move |factor: f64| {
        map_zoom.update(|z| *z = (*z * factor).clamp(MAP_ZOOM_MIN, MAP_ZOOM_MAX));
    };
    let zoom_at = move |factor: f64, cx: f64, cy: f64| {
        // keep world point under cursor stable while viewBox zooms
        let z0 = map_zoom.get_untracked();
        let z1 = (z0 * factor).clamp(MAP_ZOOM_MIN, MAP_ZOOM_MAX);
        if (z1 - z0).abs() < 1e-9 {
            return;
        }
        let c0 = map_center.get_untracked();
        let (ww, wh) = if let Some(el) = map_wrap_ref.get_untracked() {
            let r = el.get_bounding_client_rect();
            (r.width(), r.height())
        } else {
            (MAP_W, MAP_H)
        };
        let (wx, wy) = client_to_world(cx, cy, ww, wh, c0, z0, MAP_W, MAP_H);
        let vw1 = MAP_W / z1;
        let vh1 = MAP_H / z1;
        let fit1 = (ww / vw1).min(wh / vh1);
        let ox1 = (ww - vw1 * fit1) * 0.5;
        let oy1 = (wh - vh1 * fit1) * 0.5;
        let left1 = wx - (cx - ox1) / fit1;
        let top1 = wy - (cy - oy1) / fit1;
        let c1 = (left1 + vw1 * 0.5, top1 + vh1 * 0.5);
        map_zoom.set(z1);
        map_center.set(c1);
    };
    let pan_by = move |dx_screen: f64, dy_screen: f64| {
        let z = map_zoom.get_untracked().max(0.25);
        let (ww, wh) = if let Some(el) = map_wrap_ref.get_untracked() {
            let r = el.get_bounding_client_rect();
            (r.width(), r.height())
        } else {
            (MAP_W, MAP_H)
        };
        let vw = MAP_W / z;
        let vh = MAP_H / z;
        let fit = (ww / vw).min(wh / vh).max(1e-9);
        map_center.update(|(cx, cy)| {
            *cx -= dx_screen / fit;
            *cy -= dy_screen / fit;
        });
    };
    let reset_cam = move || {
        map_zoom.set(1.0);
        map_center.set((MAP_W * 0.5, MAP_H * 0.5));
    };

    // secondary path: an owner who happens to be standing on the plot taps
    // "USE MY GPS" and device geolocation sets the exact worksite. Most
    // owners manage land remotely, so tapping the rendered polygon (below)
    // is the default way to place the point.
    let request_gps = move || {
        let Some(fid) = selected_flat.get_untracked() else {
            return;
        };
        let Some(geo) = web_sys::window().and_then(|w| w.navigator().geolocation().ok()) else {
            gps_status.set("geolocation unavailable".into());
            return;
        };
        gps_status.set("locating…".into());
        let opts = web_sys::PositionOptions::new();
        opts.set_enable_high_accuracy(true);
        opts.set_timeout(15_000);
        let ok = Closure::once(move |pos: web_sys::Position| {
            let c = pos.coords();
            let (lon, lat) = (c.longitude(), c.latitude());
            work_at.update(|m| {
                m.insert(fid.clone(), [lon, lat]);
                save_work_at(m);
            });
            gps_status.set(format!("set · {lat:.5}, {lon:.5} · ±{:.0}m", c.accuracy()));
        });
        let err = Closure::once(move |e: web_sys::PositionError| {
            gps_status.set(format!("gps error: {}", e.message()));
        });
        let _ = geo.get_current_position_with_error_callback_and_options(
            ok.as_ref().unchecked_ref(),
            Some(err.as_ref().unchecked_ref()),
            &opts,
        );
        ok.forget();
        err.forget();
    };

    Effect::new(move |_| {
        let title = if left_board.get() == LeftBoard::Domains {
            "Cyberia — domains · Gesing, Bali"
        } else {
            "Cyberia — map · Gesing, Bali"
        };
        document().set_title(title);
    });

    let map_for_svg = map.clone();
    let nav_active = if domains_board { "domains" } else { "map" };
    // split clones so left-rail closure and map markers don't fight over ownership
    let base_domains_rail = base_domains.clone();
    let base_domains_map = base_domains.clone();
    let map_bbox_rail = map_bbox.clone();
    let map_bbox_drag = map_bbox.clone();

    view! {
        <div class="page-shell cyberia-shell">
            <div class="site-chrome cyberia-chrome">
            <div class="chrome-inner">
            <div class="header-row1">
                <div class="logo-zone">
                    <h1 class="logo">
                        <a href="/cities" class="brand-flag" title="home" inner_html=FLAG_SVG></a>
                        <span style="color: var(--cyber-green);">"cyber"</span>
                        <span style="color: var(--cyber-green); margin: 0 1px;">"•"</span>
                        <span style="color: #fff;">"ia"</span>
                    </h1>
                </div>
                <div class="cyberia-phase-pill">
                    <span class="phase-dot"></span>
                    {move || {
                        if left_board.get() == LeftBoard::Domains {
                            let n = domains.get().len();
                            format!("21 CYBICS · {n} DOMAINS · DRAG")
                        } else {
                            let n = flats.get().len();
                            format!("PHASE 0 · {n} PLOTS · GESING")
                        }
                    }}
                </div>
                <CyberiaNav active=nav_active />
            </div>
            </div>
            </div>

            <div class=if domains_board { "cyberia-stage" } else { "cyberia-stage no-rail" }>
                // LEFT — DOMAINS board only; the fleets rail left the main screen
                {domains_board.then(|| view! {
                <section class="cyberia-panel cyberia-fleets">
                    <div class="cyberia-panel-h">
                        <div class="board-tabs">
                            <a
                                class=move || if left_board.get() == LeftBoard::Fleets { "board-tab on" } else { "board-tab" }
                                href="/map"
                                on:click=move |ev| {
                                    if !domains_board {
                                        ev.prevent_default();
                                        left_board.set(LeftBoard::Fleets);
                                    }
                                }
                            >"FLEETS"</a>
                            <a
                                class=move || if left_board.get() == LeftBoard::Domains { "board-tab on" } else { "board-tab" }
                                href="/domains"
                                on:click=move |ev| {
                                    if domains_board {
                                        ev.prevent_default();
                                        left_board.set(LeftBoard::Domains);
                                    }
                                }
                            >"DOMAINS"</a>
                        </div>
                        <span class="panel-sub">
                            {move || match left_board.get() {
                                LeftBoard::Fleets => format!("{} workers", WORKERS.len()),
                                LeftBoard::Domains => {
                                    let n = domains.get().len();
                                    format!("{n} cybics · drag on map")
                                }
                            }}
                        </span>
                    </div>
                    <div class="fleet-list">
                    {move || match left_board.get() {
                        LeftBoard::Domains => {
                            let list = domains.get();
                            let baked_reset = base_domains_rail.clone();
                            let bbox_list = map_bbox_rail.clone();
                            // group by triad — question once per group, not on every card
                            let mut by_triad: HashMap<String, Vec<DomainMark>> = HashMap::new();
                            for d in list {
                                by_triad.entry(d.triad.clone()).or_default().push(d);
                            }
                            // stable order inside group: name alpha
                            for v in by_triad.values_mut() {
                                v.sort_by(|a, b| a.name.cmp(&b.name));
                            }
                            let mut groups: Vec<(String, Vec<DomainMark>)> = TRIAD_ORDER
                                .iter()
                                .filter_map(|t| {
                                    by_triad
                                        .remove(*t)
                                        .map(|v| ((*t).to_string(), v))
                                })
                                .collect();
                            // any unknown triad tail
                            let mut rest: Vec<_> = by_triad.into_iter().collect();
                            rest.sort_by(|a, b| a.0.cmp(&b.0));
                            groups.extend(rest);

                            view! {
                                <div class="fleet-section">"CYBICS · BY TRIAD"</div>
                                <div class="domain-board-actions">
                                    <button
                                        class="act-pill"
                                        type="button"
                                        title="reset domain positions to baked map"
                                        on:click=move |_| {
                                            clear_domain_positions();
                                            domains.set(baked_reset.clone());
                                        }
                                    >"RESET"</button>
                                    <button
                                        class="act-pill"
                                        type="button"
                                        title="log lon/lat JSON overrides to browser console"
                                        on:click=move |_| {
                                            let list = domains.get_untracked();
                                            let mut pos = HashMap::new();
                                            for d in &list {
                                                if let Some(c) = d.coords.first() {
                                                    pos.insert(d.name.clone(), [c[0], c[1]]);
                                                }
                                            }
                                            let s = serde_json::to_string_pretty(&pos).unwrap_or_else(|_| "{}".into());
                                            web_sys::console::log_1(&s.into());
                                            if let Some(w) = web_sys::window() {
                                                let _ = w.alert_with_message(
                                                    "Domain positions logged to console (F12).\nAlso saved in localStorage.",
                                                );
                                            }
                                        }
                                    >"EXPORT"</button>
                                </div>
                                {groups.into_iter().map(|(triad, doms)| {
                                    let color = triad_color(&triad);
                                    let q = triad_question(&triad);
                                    let triad_up = triad.to_uppercase();
                                    let n = doms.len();
                                    let bbox_group = bbox_list.clone();
                                    view! {
                                        <div class="domain-triad-group" style=format!("--dom-color: {color}")>
                                            <div class="domain-triad-h">
                                                <span class="domain-triad-name" style:color=color>{triad_up}</span>
                                                <span class="domain-triad-q">{q}</span>
                                                <span class="domain-triad-n">{format!("{n}")}</span>
                                            </div>
                                            <div class="domain-triad-cards">
                                                {doms.into_iter().map(|dom| {
                                                    let id = dom.id.clone();
                                                    let id2 = id.clone();
                                                    let name = dom.name.clone();
                                                    let district = if dom.district.is_empty() {
                                                        dom.zone.clone()
                                                    } else {
                                                        dom.district.clone()
                                                    };
                                                    let color = triad_color(&dom.triad);
                                                    let bbox_focus = bbox_group.clone();
                                                    view! {
                                                        <button
                                                            class=move || {
                                                                let sel = selected_domain.get().as_deref() == Some(id.as_str());
                                                                format!("fleet-card domain-card domain-card-compact{}", if sel { " sel" } else { "" })
                                                            }
                                                            style=format!("--dom-color: {color}")
                                                            on:click=move |_| {
                                                                selected_domain.set(Some(id2.clone()));
                                                                if let Some(dom) = domains
                                                                    .get_untracked()
                                                                    .into_iter()
                                                                    .find(|d| d.id == id2)
                                                                {
                                                                    if let Some(c) = dom.coords.first() {
                                                                        let (x, y) = project(
                                                                            c[0],
                                                                            c[1],
                                                                            &bbox_focus,
                                                                            MAP_W,
                                                                            MAP_H,
                                                                            MAP_PAD,
                                                                        );
                                                                        map_center.set((x, y));
                                                                        if map_zoom.get_untracked() < 3.0 {
                                                                            map_zoom.set(4.0);
                                                                        }
                                                                    }
                                                                }
                                                            }
                                                        >
                                                            <div class="fleet-top">
                                                                <span class="fleet-name" style:color=color>{name.to_uppercase()}</span>
                                                                <span class="fleet-status">{district.to_uppercase()}</span>
                                                            </div>
                                                        </button>
                                                    }
                                                }).collect_view()}
                                            </div>
                                        </div>
                                    }
                                }).collect_view()}
                            }.into_any()
                        }
                        LeftBoard::Fleets => view! {
                        <div class="fleet-section">"WORKERS · HARD FORCE"</div>
                        {WORKERS.iter().map(|f| render_fleet_card(f, selected_fleet)).collect_view()}
                        // robots you bought this session
                        {move || owned.get().into_iter().map(|r| {
                            let id = r.id.clone();
                            let id2 = id.clone();
                            let kind_worker = r.kind == "worker";
                            let name = r.name.clone();
                            let role = r.role.clone();
                            let kind = r.kind.clone();
                            view! {
                                <button
                                    class=move || {
                                        let sel = selected_fleet.get().as_deref() == Some(id.as_str());
                                        format!(
                                            "fleet-card owned{}{}",
                                            if sel { " sel" } else { "" },
                                            if kind_worker { " worker" } else { " machine" },
                                        )
                                    }
                                    on:click=move |_| selected_fleet.set(Some(id2.clone()))
                                >
                                    <div class="fleet-top">
                                        <span class="fleet-name">{name}</span>
                                        <span class="fleet-status" style:color=status_color("idle")>"OWNED"</span>
                                    </div>
                                    <div class="fleet-role">{role}</div>
                                    <div class="fleet-meta">
                                        <span>{kind.to_uppercase()}</span>
                                        <span>"BOUGHT"</span>
                                    </div>
                                </button>
                            }
                        }).collect_view()}
                        }.into_any()
                    }}
                    </div>
                    <div class="cyberia-hint">
                        {move || match left_board.get() {
                            LeftBoard::Domains => "Select a domain · drag its glow on the map · positions save in this browser",
                            LeftBoard::Fleets => "Pick a fleet unit, a flat on the map, an action — then commit intent.",
                        }}
                    </div>
                </section>
                })}

                // CENTER — FLATS MAP (game camera: static viewport, wheel/keys zoom, drag pan)
                <section class="cyberia-panel cyberia-flats">
                    <div class="cyberia-panel-h">
                        <span class="panel-kicker">"FLATS"</span>
                        <span class="panel-sub">
                            {format!(
                                "{} plots · {:.1} ha plots · {:.0} ha site · scroll zoom",
                                map.stats.plot_count.max(map.phase0.len() as u32),
                                map.stats.plot_ha,
                                map.stats.district_ha,
                            )}
                        </span>
                        <span class="map-zoom-readout" title="zoom · visible width (game scale)">
                            {move || {
                                let z = map_zoom.get();
                                let m = view_width_m(z, MAP_W, MAP_PAD, site_ew_m);
                                format_zoom_hud(z, m)
                            }}
                        </span>
                    </div>
                    <div
                        class="flat-map-wrap game-map"
                        node_ref=map_wrap_ref
                        tabindex="0"
                        on:wheel=move |ev| {
                            ev.prevent_default();
                            ev.stop_propagation();
                            let dy = ev.delta_y();
                            if dy == 0.0 { return; }
                            // slightly faster step so deep game zoom (×1000) is reachable
                            let factor = if dy < 0.0 { 1.18 } else { 1.0 / 1.18 };
                            if let Some(el) = map_wrap_ref.get_untracked() {
                                let rect = el.get_bounding_client_rect();
                                let cx = ev.client_x() as f64 - rect.left();
                                let cy = ev.client_y() as f64 - rect.top();
                                zoom_at(factor, cx, cy);
                            } else {
                                zoom_by(factor);
                            }
                        }
                        on:keydown=move |ev| {
                            let key = ev.key();
                            let step = 36.0;
                            match key.as_str() {
                                "ArrowLeft" | "a" | "A" => { ev.prevent_default(); pan_by(step, 0.0); }
                                "ArrowRight" | "d" | "D" => { ev.prevent_default(); pan_by(-step, 0.0); }
                                "ArrowUp" | "w" | "W" => { ev.prevent_default(); pan_by(0.0, step); }
                                "ArrowDown" | "s" | "S" => { ev.prevent_default(); pan_by(0.0, -step); }
                                "+" | "=" => { ev.prevent_default(); zoom_by(1.25); }
                                "-" | "_" => { ev.prevent_default(); zoom_by(1.0 / 1.25); }
                                "0" => { ev.prevent_default(); reset_cam(); }
                                _ => {}
                            }
                        }
                        on:pointerdown=move |ev| {
                            // pan from anywhere on the map (plots, empty, places) —
                            // filled flats used to block drag; click still selects if no pan
                            // domain-dot starts its own drag via stop_propagation
                            if ev.button() != 0 { return; }
                            if domain_drag.get_untracked().is_some() { return; }
                            let target = ev.target().and_then(|t| t.dyn_into::<web_sys::Element>().ok());
                            let on_ui = target
                                .as_ref()
                                .and_then(|el| el.closest(".map-zoom-rail, .map-cam-btn, .domain-dot").ok().flatten())
                                .is_some();
                            if on_ui {
                                return;
                            }
                            map_dragging.set(true);
                            map_did_drag.set(false);
                            map_drag_last.set((ev.client_x() as f64, ev.client_y() as f64));
                            // pointer capture is deliberately NOT taken here: once a mouse
                            // is captured, Chrome retargets the click that follows pointerup
                            // to the capturing element (this wrapper) instead of the plot
                            // actually tapped, so a bare tap could never select anything.
                            // Capture only once a real drag is confirmed, below.
                        }
                        on:pointermove=move |ev| {
                            // —— domain drag (shill points) ——
                            if let Some(dom_id) = domain_drag.get_untracked() {
                                let (ww, wh) = if let Some(el) = map_wrap_ref.get_untracked() {
                                    let r = el.get_bounding_client_rect();
                                    (r.width(), r.height())
                                } else {
                                    (MAP_W, MAP_H)
                                };
                                let rect = map_wrap_ref
                                    .get_untracked()
                                    .map(|el| el.get_bounding_client_rect());
                                let (cx, cy) = if let Some(r) = rect {
                                    (
                                        ev.client_x() as f64 - r.left(),
                                        ev.client_y() as f64 - r.top(),
                                    )
                                } else {
                                    (ev.client_x() as f64, ev.client_y() as f64)
                                };
                                let (lx, ly) = map_drag_last.get_untracked();
                                let dx = (ev.client_x() as f64) - lx;
                                let dy = (ev.client_y() as f64) - ly;
                                if !domain_did_drag.get_untracked() {
                                    if dx * dx + dy * dy < 16.0 {
                                        return;
                                    }
                                    domain_did_drag.set(true);
                                }
                                let (wx, wy) = client_to_world(
                                    cx,
                                    cy,
                                    ww,
                                    wh,
                                    map_center.get_untracked(),
                                    map_zoom.get_untracked(),
                                    MAP_W,
                                    MAP_H,
                                );
                                let (lon, lat) = unproject(wx, wy, &map_bbox_drag, MAP_W, MAP_H, MAP_PAD);
                                domains.update(|list| {
                                    if let Some(d) = list.iter_mut().find(|d| d.id == dom_id) {
                                        d.coords = vec![[lon, lat]];
                                    }
                                });
                                map_drag_last.set((ev.client_x() as f64, ev.client_y() as f64));
                                return;
                            }

                            if !map_dragging.get_untracked() { return; }
                            let (lx, ly) = map_drag_last.get_untracked();
                            let cx = ev.client_x() as f64;
                            let cy = ev.client_y() as f64;
                            let dx = cx - lx;
                            let dy = cy - ly;
                            // ~5px threshold so a tap still counts as plot select
                            if !map_did_drag.get_untracked() {
                                if dx * dx + dy * dy < 25.0 {
                                    return;
                                }
                                map_did_drag.set(true);
                                // now it's a real drag, not a tap — capture so panning
                                // stays smooth even if the cursor leaves the map
                                if let Some(el) = map_wrap_ref.get_untracked() {
                                    let _ = el.set_pointer_capture(ev.pointer_id());
                                }
                            }
                            pan_by(dx, dy);
                            map_drag_last.set((cx, cy));
                        }
                        on:pointerup=move |ev| {
                            if domain_drag.get_untracked().is_some() {
                                if domain_did_drag.get_untracked() {
                                    save_domain_positions(&domains.get_untracked());
                                }
                                domain_drag.set(None);
                                domain_did_drag.set(false);
                            }
                            map_dragging.set(false);
                            // release now, before the browser synthesizes the click that
                            // follows — otherwise the capturing wrapper (not the plot the
                            // user actually tapped) stays the click's target and the plot
                            // never sees it, so a tap can never select anything
                            if let Some(el) = map_wrap_ref.get_untracked() {
                                let _ = el.release_pointer_capture(ev.pointer_id());
                            }
                        }
                        on:pointercancel=move |_| {
                            domain_drag.set(None);
                            domain_did_drag.set(false);
                            map_dragging.set(false);
                            map_did_drag.set(false);
                        }
                        on:pointerleave=move |_| {
                            if !map_dragging.get_untracked() {
                                map_hover.set(None);
                            }
                        }
                    >
                        // left zoom rail (game-style) — deep zoom for cm-scale tile planning
                        <div class="map-zoom-rail" aria-label="map zoom">
                            <button class="map-cam-btn" type="button" title="zoom in (to ~10cm tile scale)"
                                on:click=move |ev| { ev.stop_propagation(); zoom_by(1.35); }
                            >"+"</button>
                            <button class="map-cam-btn" type="button" title="zoom out"
                                on:click=move |ev| { ev.stop_propagation(); zoom_by(1.0 / 1.35); }
                            >"−"</button>
                            <button class="map-cam-btn" type="button" title="reset view"
                                on:click=move |ev| { ev.stop_propagation(); reset_cam(); }
                            >"⌂"</button>
                            <div class="map-cam-sep"></div>
                            <button class="map-cam-btn" type="button" title="pan left"
                                on:click=move |ev| { ev.stop_propagation(); pan_by(48.0, 0.0); }
                            >"←"</button>
                            <button class="map-cam-btn" type="button" title="pan right"
                                on:click=move |ev| { ev.stop_propagation(); pan_by(-48.0, 0.0); }
                            >"→"</button>
                            <button class="map-cam-btn" type="button" title="pan up"
                                on:click=move |ev| { ev.stop_propagation(); pan_by(0.0, 48.0); }
                            >"↑"</button>
                            <button class="map-cam-btn" type="button" title="pan down"
                                on:click=move |ev| { ev.stop_propagation(); pan_by(0.0, -48.0); }
                            >"↓"</button>
                        </div>

                        {
                            let m = map_for_svg.clone();
                            let m_places = map_for_svg.clone();
                            let m_cap = map_for_svg.clone();
                            let m_districts = map_for_svg.clone();
                            const W: f64 = 960.0;
                            const H: f64 = 720.0;
                            const PAD: f64 = 20.0;
                            view! {
                                <div class="map-world">
                                <svg
                                    class="flat-map"
                                    viewBox=move || map_view_box(map_center.get(), map_zoom.get(), W, H)
                                    preserveAspectRatio="xMidYMid meet"
                                >
                                    // full world backdrop (fixed world bounds)
                                    <rect x="0" y="0" width=W height=H fill="#070707" />
                                    <defs>
                                        <pattern id="hatch-owned" width="4" height="4" patternUnits="userSpaceOnUse" patternTransform="rotate(45)">
                                            <line x1="0" y1="0" x2="0" y2="4" stroke="rgba(255,255,255,0.75)" stroke-width="1.1" />
                                        </pattern>
                                    </defs>
                                    {(0..12).map(|i| {
                                        let x = PAD + i as f64 * (W - 2.0*PAD) / 11.0;
                                        let y2 = H - PAD;
                                        view! {
                                            <line x1=x y1=PAD x2=x y2=y2 stroke="#121212" stroke-width="1" />
                                        }
                                    }).collect_view()}
                                    {(0..9).map(|i| {
                                        let y = PAD + i as f64 * (H - 2.0*PAD) / 8.0;
                                        let x2 = W - PAD;
                                        view! {
                                            <line x1=PAD y1=y x2=x2 y2=y stroke="#121212" stroke-width="1" />
                                        }
                                    }).collect_view()}

                                    // district outlines — full site ~37 ha envelope
                                    {m_districts.districts.iter().map(|d| {
                                        let path = poly_path(&d.coords, &m_districts.bbox, W, H, PAD);
                                        let (clon, clat) = centroid(&d.coords);
                                        let (lx, ly) = project(clon, clat, &m_districts.bbox, W, H, PAD);
                                        let label = d.name.to_uppercase();
                                        view! {
                                            <g class="district-poly" pointer-events="none">
                                                <path
                                                    d=path
                                                    fill="rgba(255,255,255,0.015)"
                                                    stroke="rgba(255,255,255,0.12)"
                                                    stroke-width="1.25"
                                                    stroke-dasharray="4 3"
                                                    vector-effect="non-scaling-stroke"
                                                />
                                                <text
                                                    class="district-label"
                                                    text-anchor="middle"
                                                    dominant-baseline="middle"
                                                    transform=move || screen_stable_tf(lx, ly, map_zoom.get())
                                                >{label.clone()}</text>
                                            </g>
                                        }
                                    }).collect_view()}

                                    // ALL plots (interactive)
                                    {move || {
                                        let list = flats.get();
                                        list.into_iter().map(|flat| {
                                            let id_click = flat.id.clone();
                                            let id_fill = flat.id.clone();
                                            let id_sw = flat.id.clone();
                                            let id_sw2 = flat.id.clone();
                                            let id_cls = flat.id.clone();
                                            let id_lab = flat.id.clone();
                                            let id_hov = flat.id.clone();
                                            let id_hov2 = flat.id.clone();
                                            let zkey = zone_key(&flat).to_string();
                                            let z_stroke = zkey.clone();
                                            let label_hov = bare_name(&flat.name);
                                            let area = area_m2(&flat.coords);
                                            let d = poly_path(&flat.coords, &m.bbox, W, H, PAD);
                                            let (clon, clat) = centroid(&flat.coords);
                                            let (lx, ly) = project(clon, clat, &m.bbox, W, H, PAD);
                                            // labels only on selected plot (click) — hover uses HUD tooltip
                                            let base_label = bare_name(&flat.name);
                                            let d_under = d.clone();
                                            let d_mark = d.clone();
                                            let id_mark = flat.id.clone();
                                            view! {
                                                <g class="flat-poly"
                                                    on:click=move |ev| {
                                                        // after a pan, browser still fires click — ignore it
                                                        if map_did_drag.get_untracked() {
                                                            map_did_drag.set(false);
                                                            ev.stop_propagation();
                                                            return;
                                                        }
                                                        ev.stop_propagation();
                                                        selected_flat.set(Some(id_click.clone()));
                                                    }
                                                    on:pointerenter=move |_| {
                                                        map_hover.set(Some((id_hov.clone(), label_hov.clone(), area)));
                                                    }
                                                    on:pointerleave=move |_| {
                                                        map_hover.update(|h| {
                                                            if h.as_ref().map(|t| t.0.as_str()) == Some(id_hov2.as_str()) {
                                                                *h = None;
                                                            }
                                                        });
                                                    }
                                                >
                                                    // dark under-stroke: thin seam idle, wide halo on hover/select
                                                    <path
                                                        d=d_under
                                                        fill="none"
                                                        stroke="#000000"
                                                        stroke-width=move || {
                                                            let sel = selected_flat.get().as_deref() == Some(id_sw2.as_str());
                                                            let hov = map_hover.get().as_ref().map(|t| t.0.as_str()) == Some(id_sw2.as_str());
                                                            // non-scaling-stroke: screen px, constant under viewBox zoom
                                                            if sel { "4.0" } else if hov { "3.4" } else { "1.35" }
                                                        }
                                                        stroke-linejoin="round"
                                                        stroke-linecap="round"
                                                        vector-effect="non-scaling-stroke"
                                                        class="flat-path-under"
                                                        pointer-events="none"
                                                    />
                                                    <path
                                                        d=d
                                                        fill=move || {
                                                            let sel = selected_flat.get().as_deref() == Some(id_fill.as_str());
                                                            let hov = map_hover.get().as_ref().map(|t| t.0.as_str()) == Some(id_fill.as_str());
                                                            let st = flat_status(&id_fill, &deals.get());
                                                            land_fill(land_use(&id_fill), st, sel, hov)
                                                        }
                                                        stroke=zone_color(&z_stroke)
                                                        stroke-width=move || {
                                                            let sel = selected_flat.get().as_deref() == Some(id_sw.as_str());
                                                            let hov = map_hover.get().as_ref().map(|t| t.0.as_str()) == Some(id_sw.as_str());
                                                            // the district seam: thin idle, bold when hot
                                                            if sel { "2.8" } else if hov { "2.2" } else { "1.2" }
                                                        }
                                                        stroke-linejoin="round"
                                                        stroke-linecap="round"
                                                        paint-order="stroke fill"
                                                        vector-effect="non-scaling-stroke"
                                                        class=move || {
                                                            let sel = selected_flat.get().as_deref() == Some(id_cls.as_str());
                                                            let hov = map_hover.get().as_ref().map(|t| t.0.as_str()) == Some(id_cls.as_str());
                                                            format!(
                                                                "flat-path{}{}{}",
                                                                if sel { " is-sel" } else { "" },
                                                                if hov { " is-hov" } else { "" },
                                                                if land_use(&id_cls) == LandUse::Special { " is-special" } else { "" },
                                                            )
                                                        }
                                                    />
                                                    // owned → hatch, in transfer → white dash; the hue stays the land use
                                                    {move || match flat_status(&id_mark, &deals.get()) {
                                                        FlatStatus::Owned => Some(view! {
                                                            <path d=d_mark.clone() fill="url(#hatch-owned)" pointer-events="none" />
                                                        }.into_any()),
                                                        FlatStatus::Transfer => Some(view! {
                                                            <path
                                                                d=d_mark.clone()
                                                                fill="none"
                                                                stroke="#ffffff"
                                                                stroke-width="1.6"
                                                                stroke-dasharray="4 3"
                                                                vector-effect="non-scaling-stroke"
                                                                class="flat-transfer"
                                                                pointer-events="none"
                                                            />
                                                        }.into_any()),
                                                        _ => None,
                                                    }}
                                                    {move || {
                                                        let sel = selected_flat.get().as_deref() == Some(id_lab.as_str());
                                                        if !sel {
                                                            return view! { <g></g> }.into_any();
                                                        }
                                                        let text = if leased.get().iter().any(|x| x == &id_lab) {
                                                            format!("{base_label} · LEASED")
                                                        } else {
                                                            base_label.clone()
                                                        };
                                                        view! {
                                                            <text
                                                                class="flat-label"
                                                                text-anchor="middle"
                                                                dominant-baseline="middle"
                                                                transform=move || screen_stable_tf(lx, ly, map_zoom.get())
                                                            >{text}</text>
                                                        }.into_any()
                                                    }}
                                                </g>
                                            }
                                        }).collect_view()
                                    }}

                                    {m_places.places.iter().map(|p| {
                                        if p.coords.is_empty() { return view! { <g></g> }.into_any(); }
                                        let (x, y) = project(p.coords[0][0], p.coords[0][1], &m_places.bbox, W, H, PAD);
                                        let name = p.name.clone();
                                        view! {
                                            // dots + names stay screen-stable under camera zoom
                                            <g
                                                class="place-dot"
                                                transform=move || screen_stable_tf(x, y, map_zoom.get())
                                            >
                                                <circle cx="0" cy="0" r="3" fill="#3a3a3a" stroke="#777" stroke-width="0.9" />
                                                <text x="0" y="-7" text-anchor="middle" class="place-label">{name}</text>
                                            </g>
                                        }.into_any()
                                    }).collect_view()}

                                    // 21 cybics domain shill points — draggable; node graph is stable (pos from signal)
                                    {
                                        let bbox = m_places.bbox.clone();
                                        // meta from baked list (ids/names fixed); coords live in `domains` signal
                                        let domain_meta = base_domains_map.clone();
                                        domain_meta.iter().map(|dom| {
                                        let id = dom.id.clone();
                                        let id_cls = id.clone();
                                        let id_tf = id.clone();
                                        let id_down = id.clone();
                                        let id_r = id.clone();
                                        let id_sw = id.clone();
                                        let name = dom.name.clone();
                                        let triad = dom.triad.clone();
                                        let q = dom.question.clone();
                                        let district = if dom.district.is_empty() {
                                            dom.zone.clone()
                                        } else {
                                            dom.district.clone()
                                        };
                                        let color = triad_color(&dom.triad);
                                        let title = format!(
                                            "{} · {} · {}\n{} · drag to move",
                                            triad.to_uppercase(),
                                            name,
                                            district.to_uppercase(),
                                            q
                                        );
                                        let bbox2 = bbox.clone();
                                        view! {
                                            <g
                                                class=move || {
                                                    let sel = selected_domain.get().as_deref() == Some(id_cls.as_str());
                                                    let dragging = domain_drag.get().as_deref() == Some(id_cls.as_str());
                                                    format!(
                                                        "domain-dot{}{}",
                                                        if sel { " is-sel" } else { "" },
                                                        if dragging { " is-drag" } else { "" },
                                                    )
                                                }
                                                transform=move || {
                                                    let (x, y) = domains
                                                        .get()
                                                        .into_iter()
                                                        .find(|d| d.id == id_tf)
                                                        .and_then(|d| d.coords.first().copied())
                                                        .map(|c| project(c[0], c[1], &bbox2, W, H, PAD))
                                                        .unwrap_or((0.0, 0.0));
                                                    screen_stable_tf(x, y, map_zoom.get())
                                                }
                                                on:pointerdown=move |ev| {
                                                    ev.stop_propagation();
                                                    ev.prevent_default();
                                                    if ev.button() != 0 { return; }
                                                    selected_domain.set(Some(id_down.clone()));
                                                    domain_drag.set(Some(id_down.clone()));
                                                    domain_did_drag.set(false);
                                                    map_dragging.set(false);
                                                    map_drag_last.set((ev.client_x() as f64, ev.client_y() as f64));
                                                    if let Some(el) = map_wrap_ref.get_untracked() {
                                                        let _ = el.set_pointer_capture(ev.pointer_id());
                                                    }
                                                }
                                            >
                                                <title>{title.clone()}</title>
                                                <circle
                                                    cx="0"
                                                    cy="0"
                                                    r="10"
                                                    fill="transparent"
                                                    class="domain-hit"
                                                />
                                                <circle
                                                    cx="0"
                                                    cy="0"
                                                    r=move || {
                                                        let sel = selected_domain.get().as_deref() == Some(id_r.as_str());
                                                        if sel { "7.5" } else { "5.5" }
                                                    }
                                                    fill=color
                                                    fill-opacity="0.22"
                                                    stroke=color
                                                    stroke-width=move || {
                                                        let sel = selected_domain.get().as_deref() == Some(id_sw.as_str());
                                                        if sel { "2.0" } else { "1.4" }
                                                    }
                                                    class="domain-ring"
                                                />
                                                <circle
                                                    cx="0"
                                                    cy="0"
                                                    r="2.4"
                                                    fill=color
                                                    class="domain-core"
                                                />
                                                <text
                                                    x="0"
                                                    y="-9"
                                                    text-anchor="middle"
                                                    class="domain-label"
                                                    fill=color
                                                >{name.clone()}</text>
                                            </g>
                                        }
                                        .into_any()
                                    }).collect_view()
                                    }

                                    <text
                                        class="map-caption"
                                        text-anchor="start"
                                        transform=move || screen_stable_tf(PAD, H - 10.0, map_zoom.get())
                                    >
                                        {format!(
                                            "N ↑  ·  {} plots  ·  {:.1} ha plots / {:.0} ha site  ·  21 domains",
                                            m_cap.stats.plot_count,
                                            m_cap.stats.plot_ha,
                                            m_cap.stats.district_ha,
                                        )}
                                    </text>
                                </svg>
                                </div>
                            }
                        }

                        // hover tooltip (game HUD)
                        {move || map_hover.get().map(|(id, label, m2)| {
                            let leased_tag = if leased.get().iter().any(|x| x == &id) { " · LEASED" } else { "" };
                            let sel = selected_flat.get().as_deref() == Some(id.as_str());
                            view! {
                                <div class="map-tooltip">
                                    <div class="mt-name">{format!("{label}{leased_tag}")}</div>
                                    <div class="mt-area">{size_line(terms(&id), m2)}</div>
                                    {
                                        let st = flat_status(&id, &deals.get());
                                        let lu = land_use(&id);
                                        let line = match (st, lu) {
                                            (FlatStatus::Available, _) => terms(&id).and_then(price_line).unwrap_or_default(),
                                            (FlatStatus::OnRequest, LandUse::Hgb) => terms(&id)
                                                .and_then(|t| t.are_price_usd)
                                                .map(|a| format!("{} / are · whole district", fmt_usd(a)))
                                                .unwrap_or_default(),
                                            (s, _) => s.label().to_string(),
                                        };
                                        view! {
                                            <div class="mt-use" style:color=lu.color()>{lu.label()}</div>
                                            <div class="mt-price">{line}</div>
                                        }
                                    }
                                    <div class="mt-hint">{if sel { "SELECTED" } else { "click to select" }}</div>
                                </div>
                            }
                        })}

                        <div class="map-hud-hint">
                            {move || if left_board.get() == LeftBoard::Domains {
                                "drag domain glows to place · pan empty space · scroll zoom · saved in browser"
                            } else {
                                "drag pan · scroll zoom · drag domain glows · /domains for domain board"
                            }}
                        </div>
                    </div>
                    <div class="flat-legend">
                        <span class="leg dim">"fill = land use"</span>
                        {move || {
                            let list = flats.get();
                            LandUse::ALL
                                .iter()
                                .filter(|u| list.iter().any(|f| land_use(&f.id) == **u))
                                .map(|u| view! {
                                    <span class="leg swatch" title=u.label() style:--sw=u.color()>{u.short()}</span>
                                })
                                .collect_view()
                        }}
                        <span class="leg hatch">"OWNED"</span>
                        <span class="leg dash">"IN TRANSFER"</span>
                        <span class="leg dim">"· outline = district"</span>
                    </div>
                </section>

                // RIGHT — 3D + INTENT
                <section class="cyberia-panel cyberia-right">
                    <div class="cyberia-panel-h">
                        <span class="panel-kicker">"RENDER"</span>
                        <span class="panel-sub">"flat · robot · 3d"</span>
                    </div>
                    <div class="render-3d">
                        {move || {
                            // —— flat ——
                            let fid = selected_flat.get().unwrap_or_else(|| "sinwood".into());
                            let list = flats.get();
                            let flat = list.iter().find(|f| f.id == fid).cloned();
                            let flat_name = flat
                                .as_ref()
                                .map(|f| bare_name(&f.name))
                                .unwrap_or_else(|| fid.to_uppercase());
                            let flat_kind = flat.as_ref().map(|f| f.kind.clone()).unwrap_or_default();
                            let n = flat.as_ref().map(|f| f.coords.len()).unwrap_or(0);
                            let land_hue = if fid.contains("avalon") {
                                "var(--cyber-orange)"
                            } else if fid.contains("sinwood") {
                                "var(--cyber-green)"
                            } else {
                                "var(--cyber-cyan)"
                            };
                            let leased_tag = if leased.get().iter().any(|x| x == &fid) {
                                " · LEASED"
                            } else {
                                ""
                            };

                            // real footprint of the selected plot, in its own
                            // local frame (12% margin so the outline doesn't
                            // touch the render edge)
                            const RW: f64 = 120.0;
                            const RH: f64 = 120.0;
                            const RPAD: f64 = 14.0;
                            let local_bbox = flat.as_ref().map(|f| {
                                let (min_lon, max_lon, min_lat, max_lat) = poly_bbox(&f.coords);
                                let mlon = ((max_lon - min_lon) * 0.12).max(1e-7);
                                let mlat = ((max_lat - min_lat) * 0.12).max(1e-7);
                                BBox {
                                    min_lon: min_lon - mlon,
                                    max_lon: max_lon + mlon,
                                    min_lat: min_lat - mlat,
                                    max_lat: max_lat + mlat,
                                }
                            });
                            let plot_path = flat
                                .as_ref()
                                .zip(local_bbox.as_ref())
                                .map(|(f, bb)| poly_path(&f.coords, bb, RW, RH, RPAD))
                                .unwrap_or_default();

                            // worksite: the GPS spot the owner set on the ground,
                            // or the plot's centroid until they set one
                            let work_map = work_at.get();
                            let has_explicit = flat
                                .as_ref()
                                .map(|f| work_map.contains_key(&f.id))
                                .unwrap_or(false);
                            let work_pt = flat.as_ref().map(|f| {
                                work_map
                                    .get(&f.id)
                                    .copied()
                                    .unwrap_or_else(|| {
                                        let (clon, clat) = centroid(&f.coords);
                                        [clon, clat]
                                    })
                            });
                            let (dot_x, dot_y) = work_pt
                                .zip(local_bbox.as_ref())
                                .map(|(p, bb)| project(p[0], p[1], bb, RW, RH, RPAD))
                                .unwrap_or((RW / 2.0, RH / 2.0));
                            let dot_cls = if has_explicit {
                                "plot-work-dot set"
                            } else {
                                "plot-work-dot default"
                            };
                            let fid_click = fid.clone();
                            let local_bbox_click = local_bbox.clone();

                            // —— robot / worker ——
                            let rid = selected_fleet.get();
                            let (bot_name, bot_kind, bot_role, bot_status, bot_owned) = rid
                                .as_ref()
                                .and_then(|id| {
                                    stock_fleet(id).map(|f| {
                                        (
                                            f.name.to_string(),
                                            f.kind.to_string(),
                                            f.role.to_string(),
                                            f.status.to_string(),
                                            false,
                                        )
                                    }).or_else(|| {
                                        owned.get().into_iter().find(|r| &r.id == id).map(|r| {
                                            (r.name, r.kind, r.role, "idle".into(), true)
                                        })
                                    })
                                })
                                .unwrap_or_else(|| {
                                    ("—".into(), "none".into(), "no unit selected".into(), "—".into(), false)
                                });
                            let is_worker = bot_kind == "worker";
                            let bot_hue = if bot_kind == "none" {
                                "#444"
                            } else if is_worker {
                                "var(--cyber-cyan)"
                            } else {
                                "var(--cyber-orange)"
                            };
                            let bot_cls = if bot_kind == "none" {
                                "bot-figure empty"
                            } else if is_worker {
                                "bot-figure worker"
                            } else {
                                "bot-figure machine"
                            };

                            view! {
                                // one scene: the plot's real footprint fills the render window,
                                // the assigned unit is marked where it actually works on it
                                <div class="render-stage">
                                    <div
                                        class=move || if placing_work.get() { "plot-slab placing" } else { "plot-slab" }
                                        style:--prism-color=land_hue
                                    >
                                        <svg
                                            class="plot-slab-svg"
                                            viewBox="0 0 120 120"
                                            on:click=move |ev| {
                                                if !placing_work.get_untracked() { return; }
                                                let Some(bb) = local_bbox_click.clone() else { return; };
                                                let Some(el) = ev.current_target().and_then(|t| t.dyn_into::<web_sys::Element>().ok()) else { return; };
                                                let rect = el.get_bounding_client_rect();
                                                let (wx, wy) = client_to_world(
                                                    ev.client_x() as f64 - rect.left(),
                                                    ev.client_y() as f64 - rect.top(),
                                                    rect.width(),
                                                    rect.height(),
                                                    (RW / 2.0, RH / 2.0),
                                                    1.0,
                                                    RW,
                                                    RH,
                                                );
                                                let (lon, lat) = unproject(wx, wy, &bb, RW, RH, RPAD);
                                                work_at.update(|m| {
                                                    m.insert(fid_click.clone(), [lon, lat]);
                                                    save_work_at(m);
                                                });
                                                gps_status.set("set · tap".into());
                                                placing_work.set(false);
                                            }
                                        >
                                            <path d=plot_path class="plot-slab-path" />
                                            <circle cx=dot_x cy=dot_y r="9" class="plot-work-pulse" style:--bot-color=bot_hue></circle>
                                            <circle cx=dot_x cy=dot_y r="5" class=dot_cls style:--bot-color=bot_hue></circle>
                                        </svg>
                                    </div>
                                    <div class="prism-meta">
                                        <div class="prism-name">{format!("{flat_name}{leased_tag}")}</div>
                                        <div class="prism-sub">{format!("{flat_kind} · {n} verts")}</div>
                                        <div class="prism-sub">
                                            {if has_explicit { "worksite · set by owner" } else { "worksite · centroid (default)" }}
                                        </div>
                                        <a class="prism-link" href=format!("/plot/{fid}")>"OPEN FLAT →"</a>
                                    </div>
                                    <div class="worksite-strip">
                                        <div class="bot-mini">
                                            <div class=bot_cls style:--bot-color=bot_hue>
                                                <div class="bot-head"></div>
                                                <div class="bot-body">
                                                    <div class="bot-chest"></div>
                                                    <div class="bot-arm bot-arm-l"></div>
                                                    <div class="bot-arm bot-arm-r"></div>
                                                </div>
                                                <div class="bot-legs">
                                                    <div class="bot-leg"></div>
                                                    <div class="bot-leg"></div>
                                                </div>
                                                <div class="bot-glow"></div>
                                            </div>
                                        </div>
                                        <div class="worksite-info">
                                            <div class="wi-name" style:color=bot_hue>{bot_name}</div>
                                            <div class="wi-role">{bot_role}</div>
                                            <div class="wi-status">
                                                {
                                                    let status_lbl = if bot_owned {
                                                        "OWNED".to_string()
                                                    } else {
                                                        bot_status.to_uppercase()
                                                    };
                                                    format!("{} · {}", bot_kind.to_uppercase(), status_lbl)
                                                }
                                            </div>
                                        </div>
                                        <div class="gps-controls">
                                            <button
                                                type="button"
                                                class="gps-btn primary"
                                                on:click=move |_| placing_work.update(|p| *p = !*p)
                                            >
                                                {move || if placing_work.get() { "TAP THE PLOT" } else { "SET ON MAP" }}
                                            </button>
                                            <button type="button" class="gps-btn" on:click=move |_| request_gps()>
                                                "USE MY GPS · on site"
                                            </button>
                                        </div>
                                    </div>
                                    <div class="gps-status">{move || gps_status.get()}</div>
                                </div>
                            }
                        }}
                    </div>

                    <div class="cyberia-panel-h" style="margin-top: 24px;">
                        <span class="panel-kicker">"FLAT"</span>
                        <span class="panel-sub">"pick on the map · lease or ask for its papers"</span>
                    </div>
                    <div class="intent-form">
                        {move || {
                            let fid = selected_flat.get().unwrap_or_default();
                            let list = flats.get();
                            let flat = list.iter().find(|f| f.id == fid).cloned();
                            let Some(flat) = flat else {
                                return view! { <div class="intent-empty">"tap a flat on the map"</div> }.into_any();
                            };
                            let all_deals = deals.get();
                            let status = flat_status(&flat.id, &all_deals);
                            let book = terms(&flat.id);
                            let lu = land_use(&flat.id);
                            let zone = zone_key(&flat).to_string();
                            // a deal in this browser means the flat is yours (or on its way)
                            let mine = all_deals.get(&flat.id).map(|d| !d.cancelled).unwrap_or(false);
                            let holder_line = if mine {
                                "you".to_string()
                            } else {
                                match book {
                                    Some(t) if t.holder().is_some() => t.holder().unwrap_or_default().to_string(),
                                    Some(t) if t.is_city_land() || lu == LandUse::Commons => "cyber valley".to_string(),
                                    _ => "—".to_string(),
                                }
                            };
                            let areas: Vec<(String, String, f64)> = list
                                .iter()
                                .map(|f| (f.id.clone(), zone_key(f).to_string(), area_m2(&f.coords)))
                                .collect();
                            let sale = (lu == LandUse::Hgb).then(|| district_sale(&zone, &areas)).flatten();
                            let price = match lu {
                                LandUse::Hgb => sale
                                    .as_ref()
                                    .map(|d| format!("{} / are · district {}", fmt_usd(d.are_price), fmt_usd(d.total())))
                                    .unwrap_or_else(|| "—".into()),
                                LandUse::Venture => "business / joint-venture terms".to_string(),
                                _ if status == FlatStatus::Closed => "—".to_string(),
                                _ => book.and_then(price_line).unwrap_or_else(|| "—".into()),
                            };
                            let req_key = if lu == LandUse::Hgb { format!("district:{zone}") } else { flat.id.clone() };
                            let req_ts = jv.get().get(&req_key).copied();
                            let req_intent = if lu == LandUse::Hgb { "hgb" } else { "jv" };
                            let dd_ts = dd.get().get(&flat.id).copied();
                            let size = size_line(book, area_m2(&flat.coords));
                            let kind = book.map(|t| t.kind_line()).unwrap_or_else(|| "not in the plot sheet".into());
                            let closed_line = match (lu, book) {
                                (_, Some(t)) if t.in_reregistration() => "the title is being re-registered · off sale until it settles".to_string(),
                                (LandUse::Commons | LandUse::Special | LandUse::Unclassified, _) => lu.line().to_string(),
                                (LandUse::Lease | LandUse::Dual, Some(t)) if !t.is_city_land() && t.price().is_none() => {
                                    "opens with the second wave".to_string()
                                }
                                (_, Some(t)) if t.is_city_land() => format!("{} — held by the valley for everyone", t.kind_line()),
                                (_, Some(_)) => "the plot sheet has no price for it yet".to_string(),
                                (_, None) => "this flat is not in the plot sheet yet".to_string(),
                            };
                            let page = format!("/plot/{}", flat.id);
                            let lease_href = format!("/plot/{}/lease", flat.id);
                            let fid_dd = flat.id.clone();
                            let district = zone.to_uppercase();
                            view! {
                                <div class="intent-row">
                                    <span class="intent-k">"flat"</span>
                                    <span class="intent-v">{bare_name(&flat.name)}</span>
                                </div>
                                <div class="intent-row">
                                    <span class="intent-k">"land use"</span>
                                    <span class="intent-v" style:color=lu.color()>{lu.label()}</span>
                                </div>
                                <div class="intent-row">
                                    <span class="intent-k">"district · size"</span>
                                    <span class="intent-v dim">{format!("{district} · {size}")}</span>
                                </div>
                                <div class="intent-row">
                                    <span class="intent-k">"sheet"</span>
                                    <span class="intent-v dim">{kind}</span>
                                </div>
                                <div class="intent-row">
                                    <span class="intent-k">"price"</span>
                                    <span class=if status == FlatStatus::Available { "intent-v price" } else { "intent-v dim" }>{price}</span>
                                </div>
                                <div class="intent-row">
                                    <span class="intent-k">"status"</span>
                                    <span class="intent-v">{status.label()}</span>
                                </div>
                                <div class="intent-row">
                                    <span class="intent-k">"holder"</span>
                                    <span class="intent-v dim">{holder_line}</span>
                                </div>
                                {match (status, mine) {
                                    (FlatStatus::Available, _) => view! {
                                        <a class="intent-commit lease flat-cta" href=lease_href>
                                            {if lu == LandUse::Dual { "LEASE THIS FLAT · business or home" } else { "LEASE THIS FLAT" }}
                                        </a>
                                    }.into_any(),
                                    (_, true) => view! {
                                        <a class="intent-commit flat-cta" href=page.clone()>"MANAGE · deal, land, papers"</a>
                                    }.into_any(),
                                    (FlatStatus::OnRequest, false) => view! {
                                        <div class="use-card" style:--use=lu.color()>
                                            {match lu {
                                                LandUse::Hgb => view! {
                                                    <span class="use-t">{format!("{district} · ONE HGB TITLE")}</span>
                                                    <span class="use-s">
                                                        {sale.as_ref().map(|d| format!(
                                                            "{} flats · {:.1} ares · {} / are = {}",
                                                            d.flats, d.ares, fmt_usd(d.are_price), fmt_usd(d.total())
                                                        )).unwrap_or_default()}
                                                    </span>
                                                    <span class="use-s dim">"the district is sold whole — no single flats"</span>
                                                    {book
                                                        .filter(|t| !t.buyer_mark.is_empty())
                                                        .map(|t| {
                                                            let mark = LandUse::from_key(&t.buyer_mark);
                                                            view! {
                                                                <span class="use-s mark" style:color=mark.color()>
                                                                    {format!("buyer's mark: {} — the purpose the map sets for this place", mark.label().to_lowercase())}
                                                                </span>
                                                            }
                                                        })}
                                                }.into_any(),
                                                _ if book.map(|t| t.is_construction()).unwrap_or(false) => {
                                                    let city_held = book.map(|t| t.is_city_land()).unwrap_or(false);
                                                    view! {
                                                        <span class="use-t">{if city_held { "THE CITY'S CONSTRUCTION BUSINESS" } else { "CONSTRUCTION BUSINESS" }}</span>
                                                        <span class="use-s">
                                                            {if city_held {
                                                                "the valley builds here · open to a joint venture"
                                                            } else {
                                                                "for a building company · not private · on request"
                                                            }}
                                                        </span>
                                                    }.into_any()
                                                }
                                                _ if zone == "avalon" => view! {
                                                    <span class="use-t">"AVALON · SPECIAL PROJECT"</span>
                                                    <span class="use-s">"on request, as a joint venture"</span>
                                                    <ul class="use-tracks">
                                                        {AVALON_TRACKS.iter().map(|t| view! { <li>{*t}</li> }).collect_view()}
                                                    </ul>
                                                }.into_any(),
                                                _ => view! {
                                                    <span class="use-t">"BUSINESS GROUND"</span>
                                                    <span class="use-s">"opens to a business or a joint venture, on request"</span>
                                                }.into_any(),
                                            }}
                                        </div>
                                        {match req_ts {
                                            Some(ts) => view! {
                                                <div class="dd-done use" style:--use=lu.color()>
                                                    <span class="dd-done-t">"REQUEST SENT"</span>
                                                    <span class="dd-done-s">{format!("{} · the valley answers", fmt_ts(ts))}</span>
                                                </div>
                                            }.into_any(),
                                            None => {
                                                let key = req_key.clone();
                                                let target = if lu == LandUse::Hgb { format!("{zone} · whole district") } else { flat.id.clone() };
                                                view! {
                                                    <button
                                                        type="button"
                                                        class="intent-commit flat-cta use"
                                                        style:--use=lu.color()
                                                        on:click=move |_| {
                                                            jv.update(|m| request_jv(m, &key));
                                                            push_intent("YOU", req_intent, &target);
                                                            intents.set(load_intents());
                                                        }
                                                    >
                                                        {if lu == LandUse::Hgb { "REQUEST THE DISTRICT TITLE" } else { "REQUEST A JOINT VENTURE" }}
                                                    </button>
                                                }.into_any()
                                            }
                                        }}
                                    }.into_any(),
                                    (FlatStatus::Closed, false) if lu == LandUse::Special => view! {
                                        <div class="dd-done use" style:--use=lu.color()>
                                            <span class="dd-done-t">"HIGH-ENERGY GROUND"</span>
                                            <span class="dd-done-s">"one of the valley's special places · open to everyone who comes to feel it"</span>
                                        </div>
                                    }.into_any(),
                                    (FlatStatus::Closed, false) => view! {
                                        <div class="dd-done reserved">
                                            <span class="dd-done-t">
                                                {if book.map(|t| t.in_reregistration()).unwrap_or(false) { "TITLE IN RE-REGISTRATION" } else { "NOT ON OFFER" }}
                                            </span>
                                            <span class="dd-done-s">{closed_line}</span>
                                        </div>
                                    }.into_any(),
                                    (_, false) => match dd_ts {
                                        Some(ts) => view! {
                                            <div class="dd-done">
                                                <span class="dd-done-t">"DOCUMENTS REQUESTED"</span>
                                                <span class="dd-done-s">{format!("{} · awaiting the holder", fmt_ts(ts))}</span>
                                            </div>
                                        }.into_any(),
                                        None => view! {
                                            <button
                                                type="button"
                                                class="intent-commit flat-cta dd"
                                                on:click=move |_| {
                                                    dd.update(|m| request_dd(m, &fid_dd));
                                                    push_intent("YOU", "docs", &fid_dd);
                                                    intents.set(load_intents());
                                                }
                                            >"REQUEST DOCUMENTS · due diligence"</button>
                                        }.into_any(),
                                    },
                                }}
                                <a class="prism-link flat-open" href=page>"OPEN FLAT PAGE →"</a>
                            }.into_any()
                        }}
                        <div class="fleet-section" style="margin-top: 10px;">"YOUR REQUESTS"</div>
                        <div class="intent-queue">
                            {move || {
                                let q = intents.get();
                                if q.is_empty() {
                                    return view! {
                                        <div class="intent-empty">"nothing asked yet — lease a flat or request its papers"</div>
                                    }.into_any();
                                }
                                view! {
                                    <div class="intent-list">
                                        {q.into_iter().take(10).map(|it| {
                                            let act_cls = match it.action.as_str() {
                                                "lease" => "ii-act lease",
                                                "docs" => "ii-act docs",
                                                "jv" => "ii-act jv",
                                                "hgb" => "ii-act hgb",
                                                "build" => "ii-act build",
                                                _ => "ii-act",
                                            };
                                            let label = match it.action.as_str() {
                                                "docs" => "DUE DILIGENCE".to_string(),
                                                "jv" => "JOINT VENTURE".to_string(),
                                                "hgb" => "DISTRICT TITLE".to_string(),
                                                a => a.to_uppercase(),
                                            };
                                            view! {
                                                <div class="intent-item">
                                                    <span class="ii-id">{format!("#{:03}", it.id)}</span>
                                                    <span class=act_cls>{label}</span>
                                                    <span class="ii-flat">{it.flat.to_uppercase()}</span>
                                                </div>
                                            }
                                        }).collect_view()}
                                    </div>
                                }.into_any()
                            }}
                        </div>
                    </div>
                </section>
            </div>

            // ── the flat is already someone's — say so with a pulse, not an error ──
            {move || taken_popup.get().map(|(fid, name, zone, status)| {
                let book = terms(&fid);
                let lu = land_use(&fid);
                let holder = book
                    .and_then(|t| t.holder())
                    .map(|h| format!(" by {h}"))
                    .unwrap_or_default();
                let (title, line) = match (status, lu) {
                    (FlatStatus::Owned, _) => (
                        "THIS FLAT HAS A PULSE",
                        format!("{name} is already held{holder}. Land here changes hands once — pick a flat that still breathes free."),
                    ),
                    (FlatStatus::Transfer, _) => (
                        "RIGHTS IN TRANSIT",
                        format!("{name} is mid-handshake: a lease is moving through review right now. Watch the map — if the deal falls through, the flat opens again."),
                    ),
                    (FlatStatus::OnRequest, LandUse::Hgb) => (
                        "SOLD AS ONE DISTRICT",
                        format!("{name} is part of {} — the whole district goes as one HGB title at {} / are. Ask for the district title from the FLAT panel.", zone.to_uppercase(), book.and_then(|t| t.are_price_usd).map(fmt_usd).unwrap_or_default()),
                    ),
                    (FlatStatus::OnRequest, _) if book.map(|t| t.is_construction()).unwrap_or(false) => (
                        "CONSTRUCTION BUSINESS",
                        format!("{name} is set aside for the valley's building trade — a construction business, not a private home. Bring the company or a joint venture: ask from the FLAT panel."),
                    ),
                    (FlatStatus::OnRequest, _) if zone == "avalon" => (
                        "AVALON IS A SPECIAL PROJECT",
                        format!("{name} opens on request, as a joint venture: education from age 0 to maturity, a health resort, a cascade of reservoirs with aquatics. Bring a project that fits — the valley builds it with you. Ask from the FLAT panel."),
                    ),
                    (FlatStatus::OnRequest, _) => (
                        "BUSINESS GROUND",
                        format!("{name} is set aside for a business or a joint venture. Bring the business — ask from the FLAT panel and the valley answers."),
                    ),
                    (FlatStatus::Closed, _) if book.map(|t| t.in_reregistration()).unwrap_or(false) => (
                        "PAPERS IN MOTION",
                        format!("{name} is off sale while its title is re-registered. When the new papers settle, the map will say so — the bright green flats are open now."),
                    ),
                    (FlatStatus::Closed, LandUse::Special) => (
                        "HIGH-ENERGY GROUND",
                        format!("{name} is one of the valley's special places — where the energy runs strongest. It stays with the valley, open to everyone who comes to feel it."),
                    ),
                    (FlatStatus::Closed, LandUse::Lease | LandUse::Dual)
                        if book.map(|t| !t.is_city_land() && t.price().is_none()).unwrap_or(false) => (
                        "OPENS WITH THE SECOND WAVE",
                        format!("{name} joins the lease book with the second wave. The bright green flats are open now — start there."),
                    ),
                    (FlatStatus::Closed, LandUse::Commons) => (
                        "THIS GROUND BELONGS TO EVERYONE",
                        format!("{name} is city commons — the valley keeps it open for all of us. The green flats are the ones waiting for a name."),
                    ),
                    (FlatStatus::Closed, _) if book.map(|t| t.is_city_land()).unwrap_or(false) => (
                        "THIS GROUND BELONGS TO EVERYONE",
                        format!("{name} is {} — the valley keeps it for all of us. The green flats are the ones waiting for a name.", book.map(|t| t.kind_line()).unwrap_or_default()),
                    ),
                    (FlatStatus::Closed, _) => (
                        "NOT ON THE BOOK YET",
                        format!("{name} has no price in the plot sheet yet. The green flats are priced and waiting — start there."),
                    ),
                    (FlatStatus::Available, _) => ("", String::new()),
                };
                view! {
                    <div class="cyberia-sheet-backdrop" on:click=move |_| taken_popup.set(None)>
                        <div class="cyberia-sheet taken-sheet" on:click=move |ev| ev.stop_propagation()>
                            <div class="sheet-h">
                                <span class="panel-kicker">{title}</span>
                                <button class="sheet-x" on:click=move |_| taken_popup.set(None)>"✕"</button>
                            </div>
                            <p class="sheet-note taken-line">{line}</p>
                            <button class="intent-commit" on:click=move |_| taken_popup.set(None)>
                                "BACK TO THE MAP"
                            </button>
                        </div>
                    </div>
                }
            })}

            <div class="search-dock cyberia-dock">
                <span class="dock-count">
                    {move || {
                        let n = intents.get().len();
                        let m = flats.get().len();
                        let o = owned.get().len();
                        let l = leased.get().len();
                        format!(
                            "{n} intents · {m} flats · {}w+{o} fleets · {l} leases",
                            WORKERS.len(),
                        )
                    }}
                </span>
                <div class="cyberia-cta-bar dock-ctas">
                    <button class="cta-btn cta-lease cta-lg" on:click=move |_| {
                        let Some(fid) = selected_flat.get_untracked() else { return };
                        let list = flats.get_untracked();
                        let Some(flat) = list.iter().find(|f| f.id == fid) else { return };
                        let status = flat_status(&flat.id, &deals.get_untracked());
                        if status == FlatStatus::Available {
                            if let Some(w) = web_sys::window() {
                                let _ = w.location().set_href(&format!("/plot/{fid}/lease"));
                            }
                        } else {
                            taken_popup.set(Some((flat.id.clone(), bare_name(&flat.name), zone_key(flat).to_string(), status)));
                        }
                    }>
                        <span class="cta-ico">"🗺"</span>
                        <span class="cta-copy">
                            <span class="cta-title">"LEASE LAND"</span>
                            <span class="cta-sub">{move || {
                                selected_flat.get()
                                    .map(|f| {
                                        let name = flats.get().iter().find(|x| x.id == f).map(|x| bare_name(&x.name)).unwrap_or_else(|| f.to_uppercase());
                                        match (flat_status(&f, &deals.get()), land_use(&f)) {
                                            (FlatStatus::Available, _) => match terms(&f).and_then(|t| t.price()) {
                                                Some(p) => format!("{name} · {}", fmt_usd(p)),
                                                None => name.clone(),
                                            },
                                            (FlatStatus::OnRequest, LandUse::Hgb) => format!("{name} · whole-district HGB sale"),
                                            (FlatStatus::OnRequest, _) => format!("{name} · joint venture on request"),
                                            _ => format!("{name} · not on offer"),
                                        }
                                    })
                                    .unwrap_or_else(|| "pick a flat on the map".into())
                            }}</span>
                        </span>
                    </button>
                </div>
                <a href="https://x.com/cyberiacap" target="_blank" rel="noopener" class="dock-credit">
                    "🏴 a "<span style="color: var(--cyber-green);">"cyberia"</span>" project"
                </a>
            </div>
        </div>
    }
}
