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
use crate::friends::friend_sales;
use crate::globe::{
    cam_drag, cam_fit, cam_hold, cam_north, cam_pan, cam_release, cam_stop, cam_twist, cam_world, cam_zoom, clear_region, level_for_zoom, locate, point_at, px_per_m, show_unit, DivisionPanel, GlobeAttrib, GlobeFar, GlobeLayer, Place, Unit, BASE,
    CITIES, SVG_Z,
};
use crate::nav::CyberiaNav;
use crate::wallet::{load_intents, load_leases, push_intent};
use leptos::prelude::*;
use leptos_router::hooks::use_query_map;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
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

const DOMAIN_POS_KEY: &str = "cyberia.domain_positions.v1";

fn triad_color(triad: &str) -> &'static str {
    match triad {
        "form" => "#00acff",  // cyan — rules
        "mass" => "#ff5b00",  // orange — matter
        "space" => "#304ffe", // blue — where
        "life" => "#00fe00",  // green — alive
        "word" => "#d500f9",  // violet — meaning
        "work" => "#fcf000",  // yellow — making
        "play" => "#ff0000",  // red — coordinate
        _ => "#00fe00",
    }
}

/// Canonical cybics triad order for domain board grouping.
const TRIAD_ORDER: &[&str] = &["form", "mass", "space", "life", "word", "work", "play"];

fn triad_question(triad: &str) -> &'static str {
    match triad {
        "form" => "what are the rules?",
        "mass" => "what is it made of?",
        "space" => "where does it happen?",
        "life" => "what lives?",
        "word" => "what does it mean?",
        "work" => "who does the work?",
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

fn load_map() -> MapData {
    serde_json::from_str(MAP_JSON).expect("cyberia_map.json")
}

/// SVG units per metre: the site fitted into the padded viewBox with ONE scale
/// for both axes, so the plane is conformal and lies exactly over the globe.
fn svg_per_m(bbox: &BBox, w: f64, h: f64, pad: f64) -> f64 {
    let (ew, ns) = site_span_m(bbox);
    ((w - 2.0 * pad) / ew.max(1.0)).min((h - 2.0 * pad) / ns.max(1.0))
}

/// Metres per degree at the site: (east, north).
fn m_per_deg(bbox: &BBox) -> (f64, f64) {
    let lat0 = (bbox.min_lat + bbox.max_lat) * 0.5;
    (111_320.0 * lat0.to_radians().cos(), 111_320.0)
}

/// Project lon/lat into a viewBox-local SVG plane (y flipped), centred on the site.
fn project(lon: f64, lat: f64, bbox: &BBox, w: f64, h: f64, pad: f64) -> (f64, f64) {
    let k = svg_per_m(bbox, w, h, pad);
    let (me, mn) = m_per_deg(bbox);
    let lon0 = (bbox.min_lon + bbox.max_lon) * 0.5;
    let lat0 = (bbox.min_lat + bbox.max_lat) * 0.5;
    (w * 0.5 + (lon - lon0) * me * k, h * 0.5 - (lat - lat0) * mn * k)
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

fn line_path(coords: &[[f64; 2]], bbox: &BBox, w: f64, h: f64, pad: f64) -> String {
    let mut s = String::new();
    for (i, c) in coords.iter().enumerate() {
        let (x, y) = project(c[0], c[1], bbox, w, h, pad);
        s.push_str(if i == 0 { "M" } else { " L" });
        s.push_str(&format!("{x:.2},{y:.2}"));
    }
    s
}

/// Roads, paths and canyons between the plots — synced from the valley map
/// by `scripts/sync-plot-terms.py`.
#[derive(Clone, Debug, Deserialize)]
struct Trail {
    kind: String,
    width_m: f64,
    /// the line colour on the valley map
    #[serde(default)]
    color: String,
    coords: Vec<[f64; 2]>,
}

fn trails() -> &'static [Trail] {
    static TRAILS: std::sync::OnceLock<Vec<Trail>> = std::sync::OnceLock::new();
    TRAILS.get_or_init(|| serde_json::from_str(include_str!("cyberia_trails.json")).unwrap_or_default())
}

/// Keep HUD-sized marks (labels, dots) constant on screen while viewBox zooms.
/// Anchor at (x,y) in SVG/world units.
fn screen_stable_tf(x: f64, y: f64, zoom: f64) -> String {
    let s = 1.0 / zoom.max(1e-9);
    // the plan turns with the globe; its words stay level on the screen
    let turn = -PLAN_TURN.with(|t| t.get());
    format!("translate({x:.2},{y:.2}) rotate({turn:.2}) scale({s:.4})")
}

thread_local! {
    /// How far the plan is turned on the screen, degrees clockwise (the camera's heading).
    static PLAN_TURN: std::cell::Cell<f64> = const { std::cell::Cell::new(0.0) };
}

/// SVG camera: viewBox centered at `center` with size (W/z)×(H/z).
/// Pure vector zoom — no CSS scale raster blur.
fn map_view_box(center: (f64, f64), zoom: f64, w: f64, h: f64) -> String {
    let z = zoom.max(1e-9);
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
fn view_width_m(zoom: f64, map_w: f64, svg_k: f64) -> f64 {
    (map_w / zoom.max(1e-9)) / svg_k.max(1e-9)
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
    let z = zoom.max(1e-9);
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

/// Ops map — fleets rail + flats.
#[component]
pub fn ValleyConsole() -> impl IntoView {
    view! { <MapConsole domains_board=false /> }
}

/// The home: the same map, opened on the whole planet.
#[component]
pub fn GlobeHome() -> impl IntoView {
    view! { <MapConsole domains_board=false planet=true /> }
}

/// Domains board — same camera, left rail = 21 cybics (draggable shill points).
#[component]
pub fn DomainsBoard() -> impl IntoView {
    view! { <MapConsole domains_board=true /> }
}

#[component]
fn MapConsole(domains_board: bool, #[prop(default = false)] planet: bool) -> impl IntoView {
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
    // deals first: loading them drops legacy auto-opened requests from the journal
    let deals = RwSignal::new(load_deals());
    // the valley's published deals (reserved · transfer · owned) paint over this browser's own
    leptos::task::spawn_local(async move {
        let index = crate::orders::fetch_index().await;
        if !index.is_empty() {
            deals.update(|d| crate::deals::merge_published(d, &index));
        }
    });
    let intents = RwSignal::new(load_intents());
    let dd = RwSignal::new(load_dd());
    let jv = RwSignal::new(load_jv());
    let leased = RwSignal::new(
        load_leases()
            .into_iter()
            .map(|l| l.flat_id)
            .collect::<Vec<String>>(),
    );
    let flats = RwSignal::new(map.phase0.clone()); // live land geometry (split/merge)

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
    let svg_k = svg_per_m(&map.bbox, MAP_W, MAP_H, MAP_PAD);
    let map_zoom = RwSignal::new(1.0_f64);
    let map_center = RwSignal::new((MAP_W * 0.5, MAP_H * 0.5)); // world/SVG units
    let map_hover = RwSignal::new(None::<(String, String, f64)>); // id, label, m2
    let map_dragging = RwSignal::new(false);
    // true once pointer moved past threshold — suppress plot-click select after pan
    let map_did_drag = RwSignal::new(false);
    let map_drag_last = RwSignal::new((0.0_f64, 0.0_f64));
    let map_wrap_ref = NodeRef::<leptos::html::Div>::new();
    // fingers on the map: (pointer id, x, y) — two of them pinch
    let touches = StoredValue::new(Vec::<(i32, f64, f64)>::new());

    // ONE camera: the globe's. It reports every frame; near the site the SVG
    // plane follows it exactly, far from it the console steps aside.
    let cam = RwSignal::new((0.0_f64, 0.0_f64, 0.0_f64, 0.0_f64, 0.0_f64, 0.0_f64));
    let picked = RwSignal::new(None::<usize>);
    // a tap on the globe: the point (lat, lon), every division it is in from the
    // state down, and the one that is selected and outlined
    let point = RwSignal::new(None::<(f64, f64)>);
    let chain = RwSignal::new(Vec::<Unit>::new());
    let unit_sel = RwSignal::new(None::<u64>);
    let tap_no = StoredValue::new(0u32);
    let drop_point = move || {
        tap_no.update_value(|n| *n += 1);
        clear_region();
        point.set(None);
        chain.set(Vec::new());
        unit_sel.set(None);
    };
    let near = RwSignal::new(!planet);
    // the plan comes in before the console does: it fades in over the last zoom level of the approach
    let plan_on = RwSignal::new(!planet);
    let plan_alpha = RwSignal::new(if planet { 0.0 } else { 1.0 });
    let site_lon = (map.bbox.min_lon + map.bbox.max_lon) * 0.5;
    let site_lat = (map.bbox.min_lat + map.bbox.max_lat) * 0.5;
    let site_fit_m = {
        let (ew, ns) = site_span_m(&map.bbox);
        ew.max(ns) * 1.1
    };
    {
        let bbox = map_bbox.clone();
        Effect::new(move |_| {
            let (lat, lon, z, w, h, hd) = cam.get();
            if w < 2.0 || h < 2.0 {
                return;
            }
            let at_site = (lat - site_lat).abs() < 0.2 && (lon - site_lon).abs() < 0.2;
            // the console comes on at SVG_Z and stays until a little below it, so a hand
            // resting at the threshold does not flip the page back and forth
            let n = at_site && z >= if near.get_untracked() { SVG_Z - 0.35 } else { SVG_Z };
            if near.get_untracked() != n {
                near.set(n);
            }
            let from = SVG_Z - 0.9;
            let p = at_site && z >= from;
            if plan_on.get_untracked() != p {
                plan_on.set(p);
            }
            if p {
                let t = ((z - from) / 0.9).clamp(0.0, 1.0);
                let a = t * t * (3.0 - 2.0 * t);
                if (plan_alpha.get_untracked() - a).abs() > 0.004 {
                    plan_alpha.set(a);
                }
                PLAN_TURN.with(|t| t.set(hd.to_degrees()));
                map_center.set(project(lon, lat, &bbox, MAP_W, MAP_H, MAP_PAD));
                map_zoom.set(px_per_m(z) / (svg_k * (w / MAP_W).min(h / MAP_H)));
            }
        });
    }
    // cities founded in this browser by the old catalog form are gone with the form
    if let Some(ls) = web_sys::window().and_then(|w| w.local_storage().ok().flatten()) {
        let _ = ls.remove_item("cyberia_found_cities");
    }

    // a point of the earth is chosen (a tap, or a place found by name): every
    // division it is in is looked up, and one is selected — `prefer` if it is
    // there, else the level the zoom reads
    let tap_at = move |lon: f64, lat: f64, prefer: Option<(u64, u8)>| {
        drop_point();
        let tap = tap_no.get_value();
        point.set(Some((lat, lon)));
        let want = prefer.map(|p| p.1).unwrap_or_else(|| level_for_zoom(cam.get_untracked().2));
        wasm_bindgen_futures::spawn_local(async move {
            locate(lon, lat, move |u: Unit| {
                if tap_no.try_get_value() != Some(tap) {
                    return; // a later tap took over
                }
                if u.level <= want {
                    unit_sel.set(Some(u.id));
                    show_unit(u.id);
                }
                chain.update(|c| c.push(u));
            })
            .await;
            // open water: nothing to select
            if tap_no.try_get_value() == Some(tap) && chain.get_untracked().is_empty() {
                drop_point();
            }
        });
    };
    // a place found by name: fly so that it fills the view, then select it
    let go_place = move |p: Place| {
        picked.set(None);
        cam_fit(p.lon, p.lat, (p.span_deg * 111_320.0 * 1.6).max(2_000.0), 1800.0);
        tap_at(p.lon, p.lat, Some((p.id, p.level)));
    };

    let zoom_by = move |factor: f64| cam_zoom(factor.log2(), None);
    let zoom_at = move |factor: f64, cx: f64, cy: f64| cam_zoom(factor.log2(), Some((cx, cy)));
    let pan_by = move |dx_screen: f64, dy_screen: f64| cam_pan(dx_screen, dy_screen);
    let reset_cam = move || cam_fit(site_lon, site_lat, site_fit_m, 900.0);
    let home = (!planet).then_some((site_lon, site_lat, site_fit_m));

    // the keyboard steers the map from anywhere on the page. Keys are read by their
    // place on the keyboard (code), so every layout steers; held keys move the
    // camera smoothly, and every key is written on its button in the pad
    let held = StoredValue::new(std::collections::HashSet::<String>::new());
    let apply_held = move || {
        held.with_value(|h| {
            let on = |a: &str, b: &str| (h.contains(a) || h.contains(b)) as i32 as f64;
            let far = if h.contains("ShiftLeft") || h.contains("ShiftRight") { 3.0 } else { 1.0 };
            cam_hold(0, (on("ArrowLeft", "KeyA") - on("ArrowRight", "KeyD")) * far);
            cam_hold(1, (on("ArrowUp", "KeyW") - on("ArrowDown", "KeyS")) * far);
            cam_hold(2, on("Equal", "NumpadAdd") - on("Minus", "NumpadSubtract"));
            cam_hold(3, on("KeyE", "KeyE") - on("KeyQ", "KeyQ"));
        });
    };
    const STEER: [&str; 15] = [
        "ArrowLeft", "ArrowRight", "ArrowUp", "ArrowDown", "KeyA", "KeyD", "KeyW", "KeyS", "Equal", "Minus", "NumpadAdd", "NumpadSubtract", "KeyQ",
        "KeyE", "ShiftLeft",
    ];
    let key_down = window_event_listener(leptos::ev::keydown, move |ev| {
        if ev.ctrl_key() || ev.meta_key() || ev.alt_key() {
            return;
        }
        // typing is not steering
        let typing = ev
            .target()
            .and_then(|t| t.dyn_into::<web_sys::Element>().ok())
            .map(|el| {
                let tag = el.tag_name().to_ascii_lowercase();
                tag == "input" || tag == "textarea" || tag == "select" || el.has_attribute("contenteditable")
            })
            .unwrap_or(false);
        if typing {
            return;
        }
        let code = ev.code();
        if STEER.contains(&code.as_str()) || code == "ShiftRight" {
            if code != "ShiftLeft" && code != "ShiftRight" {
                ev.prevent_default();
            }
            held.update_value(|h| {
                h.insert(code);
            });
            apply_held();
            return;
        }
        if ev.repeat() {
            return;
        }
        let mut used = true;
        match code.as_str() {
            "KeyN" => cam_north(),
            "Digit0" | "Numpad0" => {
                picked.set(None);
                cam_world();
            }
            "KeyH" => reset_cam(),
            // nothing selected: the division, the city and the plot let go
            "Escape" => {
                drop_point();
                picked.set(None);
                selected_flat.set(None);
            }
            _ => used = false,
        }
        if used {
            ev.prevent_default();
        }
    });
    let key_up = window_event_listener(leptos::ev::keyup, move |ev| {
        let code = ev.code();
        held.update_value(|h| {
            h.remove(&code);
        });
        apply_held();
    });
    // a window that loses the focus never sends the key's release
    let blur = window_event_listener(leptos::ev::blur, move |_| {
        held.update_value(|h| h.clear());
        apply_held();
    });
    on_cleanup(move || {
        key_down.remove();
        key_up.remove();
        blur.remove();
    });

    Effect::new(move |_| {
        let title = if domains_board {
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

    view! {
        <div class=move || {
            let mut c = String::from("page-shell cyberia-shell globe-host");
            if !near.get() {
                c.push_str(" globe-far");
                if point.get().is_some() {
                    c.push_str(" globe-picked");
                }
            }
            if !plan_on.get() {
                c.push_str(" plan-off");
            }
            c
        }>
            <div class="site-chrome cyberia-chrome">
            <div class="chrome-inner">
            <div class="header-row1">
                <div class="logo-zone">
                    <h1 class="logo">
                        <a href=format!("{BASE}/") class="brand-flag" title="the globe" inner_html=FLAG_SVG></a>
                        <span style="color: var(--cyber-green);">"cyber"</span>
                        <span style="color: var(--cyber-green); margin: 0 1px;">"•"</span>
                        <span style="color: #fff;">"ia"</span>
                    </h1>
                </div>
                <div class="cyberia-phase-pill">
                    <span class="phase-dot"></span>
                    {move || {
                        if !near.get() {
                            format!("{} CITY · THE GLOBE", CITIES.len())
                        } else if domains_board {
                            let n = domains.get().len();
                            format!("21 CYBICS · {n} DOMAINS · DRAG")
                        } else {
                            let n = flats.get().len();
                            format!("PHASE 0 · {n} PLOTS · GESING")
                        }
                    }}
                </div>
                // the bar's two places are places of this map: the camera flies, no page loads
                {move || view! {
                    <CyberiaNav
                        active=if near.get() { nav_active } else { "earth" }
                        on_go=Callback::new(move |place: &'static str| {
                            if place == "earth" {
                                picked.set(None);
                                cam_world();
                            } else {
                                reset_cam();
                            }
                        })
                    />
                }}
            </div>
            </div>
            </div>

            <div class=if domains_board { "cyberia-stage" } else { "cyberia-stage no-rail" }>
                // LEFT — DOMAINS board only; the fleets rail left the main screen
                {domains_board.then(|| view! {
                <section class="cyberia-panel cyberia-fleets">
                    <div class="cyberia-panel-h">
                        <span class="panel-kicker">"DOMAINS"</span>
                        <span class="panel-sub">
                            {move || format!("{} cybics · drag on map", domains.get().len())}
                        </span>
                    </div>
                    <div class="fleet-list">
                    {move || {
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
                    }}
                    </div>
                    <div class="cyberia-hint">
                        "Select a domain · drag its glow on the map · positions save in this browser"
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
                                let m = view_width_m(z, MAP_W, svg_k);
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
                            // by the delta: a wheel notch and a trackpad stroke both feel right
                            let k = if ev.ctrl_key() { 0.012 } else { 0.0028 };
                            let factor = 2f64.powf((-dy * k).clamp(-1.2, 1.2));
                            if let Some(el) = map_wrap_ref.get_untracked() {
                                let rect = el.get_bounding_client_rect();
                                let cx = ev.client_x() as f64 - rect.left();
                                let cy = ev.client_y() as f64 - rect.top();
                                zoom_at(factor, cx, cy);
                            } else {
                                zoom_by(factor);
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
                                .and_then(|el| el.closest(".map-zoom-rail, .map-cam-btn, .domain-dot, .globe-ui").ok().flatten())
                                .is_some();
                            if on_ui {
                                return;
                            }
                            cam_stop(); // a hand on a turning ball stops it
                            touches.update_value(|t| {
                                t.retain(|p| p.0 != ev.pointer_id());
                                t.push((ev.pointer_id(), ev.client_x() as f64, ev.client_y() as f64));
                            });
                            map_dragging.set(true);
                            // a second finger is a pinch, never a tap
                            map_did_drag.set(touches.with_value(|t| t.len() > 1));
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
                                let _ = (ww, wh);
                                let Some((lon, lat)) = point_at(cx, cy) else { return };
                                domains.update(|list| {
                                    if let Some(d) = list.iter_mut().find(|d| d.id == dom_id) {
                                        d.coords = vec![[lon, lat]];
                                    }
                                });
                                map_drag_last.set((ev.client_x() as f64, ev.client_y() as f64));
                                return;
                            }

                            if !map_dragging.get_untracked() { return; }
                            let cx = ev.client_x() as f64;
                            let cy = ev.client_y() as f64;
                            // —— two fingers: zoom about their midpoint, pan with it ——
                            let pinch = touches.with_value(|t| {
                                if t.len() != 2 { return None; }
                                let i = t.iter().position(|p| p.0 == ev.pointer_id())?;
                                Some((t[i], t[1 - i]))
                            });
                            if let Some((me, other)) = pinch {
                                let before = ((me.1 - other.1).powi(2) + (me.2 - other.2).powi(2)).sqrt().max(1.0);
                                let after = ((cx - other.1).powi(2) + (cy - other.2).powi(2)).sqrt().max(1.0);
                                let (ox, oy) = map_wrap_ref
                                    .get_untracked()
                                    .map(|el| { let r = el.get_bounding_client_rect(); (r.left(), r.top()) })
                                    .unwrap_or((0.0, 0.0));
                                zoom_at(after / before, (cx + other.1) / 2.0 - ox, (cy + other.2) / 2.0 - oy);
                                pan_by((cx - me.1) / 2.0, (cy - me.2) / 2.0);
                                // the two fingers turn about each other: the view turns with them
                                cam_twist((cy - other.2).atan2(cx - other.1) - (me.2 - other.2).atan2(me.1 - other.1));
                                touches.update_value(|t| {
                                    if let Some(p) = t.iter_mut().find(|p| p.0 == ev.pointer_id()) {
                                        *p = (ev.pointer_id(), cx, cy);
                                    }
                                });
                                map_did_drag.set(true);
                                return;
                            }
                            touches.update_value(|t| {
                                if let Some(p) = t.iter_mut().find(|p| p.0 == ev.pointer_id()) {
                                    *p = (ev.pointer_id(), cx, cy);
                                }
                            });
                            let (lx, ly) = map_drag_last.get_untracked();
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
                            let (ox, oy, ww, wh) = map_wrap_ref
                                .get_untracked()
                                .map(|el| { let r = el.get_bounding_client_rect(); (r.left(), r.top(), r.width(), r.height()) })
                                .unwrap_or((0.0, 0.0, MAP_W, MAP_H));
                            if ev.shift_key() {
                                // shift: twist the view about its centre
                                let (mx, my) = (ox + ww / 2.0, oy + wh / 2.0);
                                cam_twist((cy - my).atan2(cx - mx) - (ly - my).atan2(lx - mx));
                            } else {
                                // the point under the finger stays under the finger
                                cam_drag(lx - ox, ly - oy, cx - ox, cy - oy);
                            }
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
                            touches.update_value(|t| t.retain(|p| p.0 != ev.pointer_id()));
                            // the finger that stays goes on panning from where it is
                            match touches.with_value(|t| t.first().copied()) {
                                Some(p) => map_drag_last.set((p.1, p.2)),
                                None => {
                                    map_dragging.set(false);
                                    cam_release(); // a ball that was moving keeps turning
                                }
                            }
                            // release now, before the browser synthesizes the click that
                            // follows — otherwise the capturing wrapper (not the plot the
                            // user actually tapped) stays the click's target and the plot
                            // never sees it, so a tap can never select anything
                            if let Some(el) = map_wrap_ref.get_untracked() {
                                let _ = el.release_pointer_capture(ev.pointer_id());
                            }
                        }
                        on:pointercancel=move |_| {
                            touches.update_value(|t| t.clear());
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
                        on:click=move |ev| {
                            // a click that ends a drag selects nothing
                            if map_did_drag.get_untracked() {
                                map_did_drag.set(false);
                                return;
                            }
                            let Some(target) = ev.target().and_then(|t| t.dyn_into::<web_sys::Element>().ok()) else { return };
                            if near.get_untracked() {
                                // bare ground between the plots: the plot lets go
                                if target.tag_name().eq_ignore_ascii_case("svg") {
                                    selected_flat.set(None);
                                }
                                return;
                            }
                            if target.closest(".globe-ui, .globe-marker, button, a, input").ok().flatten().is_some() {
                                return;
                            }
                            let Some(el) = map_wrap_ref.get_untracked() else { return };
                            let r = el.get_bounding_client_rect();
                            // the point under the tap, then every division it is in; space clears
                            match point_at(ev.client_x() as f64 - r.left(), ev.client_y() as f64 - r.top()) {
                                Some((lon, lat)) => tap_at(lon, lat, None),
                                None => drop_point(),
                            }
                        }
                    >
                        // the globe: imagery and the world's lines, under everything
                        <GlobeLayer cam=cam picked=picked home=home />
                        <GlobeFar cam=cam picked=picked on_place=go_place />
                        <GlobeAttrib />

                        // the pad: every way to move the camera, each button with its key
                        <div class="nav-pad globe-ui" aria-label="map navigation">
                            <button type="button" title="turn the view left · hold Q"
                                on:pointerdown=move |_| cam_hold(3, -1.0)
                                on:pointerup=move |_| cam_hold(3, 0.0)
                                on:pointerleave=move |_| cam_hold(3, 0.0)
                                on:pointercancel=move |_| cam_hold(3, 0.0)
                            >"⟲"<kbd>"Q"</kbd></button>
                            <button type="button" title="up · hold W or ↑ (shift: fast)"
                                on:pointerdown=move |_| cam_hold(1, 1.0)
                                on:pointerup=move |_| cam_hold(1, 0.0)
                                on:pointerleave=move |_| cam_hold(1, 0.0)
                                on:pointercancel=move |_| cam_hold(1, 0.0)
                            >"↑"<kbd>"W"</kbd></button>
                            <button type="button" title="turn the view right · hold E"
                                on:pointerdown=move |_| cam_hold(3, 1.0)
                                on:pointerup=move |_| cam_hold(3, 0.0)
                                on:pointerleave=move |_| cam_hold(3, 0.0)
                                on:pointercancel=move |_| cam_hold(3, 0.0)
                            >"⟳"<kbd>"E"</kbd></button>
                            <button type="button" title="left · hold A or ← (shift: fast)"
                                on:pointerdown=move |_| cam_hold(0, 1.0)
                                on:pointerup=move |_| cam_hold(0, 0.0)
                                on:pointerleave=move |_| cam_hold(0, 0.0)
                                on:pointercancel=move |_| cam_hold(0, 0.0)
                            >"←"<kbd>"A"</kbd></button>
                            <button type="button" class="np-north" title="north up · N" on:click=move |_| cam_north()>
                                <span class="np-needle" style=move || format!("transform: rotate({:.3}rad)", cam.get().5)>"▲"</span>
                                <kbd>"N"</kbd>
                            </button>
                            <button type="button" title="right · hold D or → (shift: fast)"
                                on:pointerdown=move |_| cam_hold(0, -1.0)
                                on:pointerup=move |_| cam_hold(0, 0.0)
                                on:pointerleave=move |_| cam_hold(0, 0.0)
                                on:pointercancel=move |_| cam_hold(0, 0.0)
                            >"→"<kbd>"D"</kbd></button>
                            <button type="button" title="zoom out · hold −"
                                on:pointerdown=move |_| cam_hold(2, -1.0)
                                on:pointerup=move |_| cam_hold(2, 0.0)
                                on:pointerleave=move |_| cam_hold(2, 0.0)
                                on:pointercancel=move |_| cam_hold(2, 0.0)
                            >"−"<kbd>"−"</kbd></button>
                            <button type="button" title="down · hold S or ↓ (shift: fast)"
                                on:pointerdown=move |_| cam_hold(1, -1.0)
                                on:pointerup=move |_| cam_hold(1, 0.0)
                                on:pointerleave=move |_| cam_hold(1, 0.0)
                                on:pointercancel=move |_| cam_hold(1, 0.0)
                            >"↓"<kbd>"S"</kbd></button>
                            <button type="button" title="zoom in · hold +"
                                on:pointerdown=move |_| cam_hold(2, 1.0)
                                on:pointerup=move |_| cam_hold(2, 0.0)
                                on:pointerleave=move |_| cam_hold(2, 0.0)
                                on:pointercancel=move |_| cam_hold(2, 0.0)
                            >"+"<kbd>"+"</kbd></button>
                            <button type="button" class="np-wide" title="the whole earth · 0" on:click=move |_| { picked.set(None); cam_world(); }>"◯ EARTH"<kbd>"0"</kbd></button>
                            <button type="button" class="np-wide2" title="Cyber Valley · H" on:click=move |_| reset_cam()>"⌂"<kbd>"H"</kbd></button>
                        </div>

                        {
                            let m = map_for_svg.clone();
                            let m_places = map_for_svg.clone();
                            let m_cap = map_for_svg.clone();
                            let m_districts = map_for_svg.clone();
                            let m_trails = map_for_svg.clone();
                            let m_friends = map_for_svg.clone();
                            const W: f64 = 960.0;
                            const H: f64 = 720.0;
                            const PAD: f64 = 20.0;
                            view! {
                                <div class="map-world" style=move || format!("transform: rotate({:.4}rad); opacity: {:.3}", cam.get().5, plan_alpha.get())>
                                <svg
                                    class="flat-map"
                                    viewBox=move || map_view_box(map_center.get(), map_zoom.get(), W, H)
                                    preserveAspectRatio="xMidYMid meet"
                                >
                                    <defs>
                                        <pattern id="hatch-owned" width="4" height="4" patternUnits="userSpaceOnUse" patternTransform="rotate(45)">
                                            <line x1="0" y1="0" x2="0" y2="4" stroke="rgba(255,255,255,0.75)" stroke-width="1.1" />
                                        </pattern>
                                    </defs>
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
                                                        // one colour per land use, exactly the prysm hex — status
                                                        // rides on top as texture, never on the fill
                                                        fill=land_use(&flat.id).color()
                                                        stroke=move || {
                                                            let sel = selected_flat.get().as_deref() == Some(id_fill.as_str());
                                                            // one neutral seam for every plot: the district is in the name, not the colour
                                                            if sel { "#ffffff" } else { "#0a0a0a" }
                                                        }
                                                        stroke-width=move || {
                                                            let sel = selected_flat.get().as_deref() == Some(id_sw.as_str());
                                                            let hov = map_hover.get().as_ref().map(|t| t.0.as_str()) == Some(id_sw.as_str());
                                                            // the district seam: thin idle, bold when hot
                                                            if sel { "3.0" } else if hov { "2.4" } else { "1.2" }
                                                        }
                                                        stroke-linejoin="round"
                                                        stroke-linecap="round"
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

                                    // friends' sales — a title sale, the sale code's blue, solid;
                                    // a click opens the listing's own page
                                    {friend_sales().iter().map(|f| {
                                        let d = poly_path(&f.coords, &m_friends.bbox, W, H, PAD);
                                        let href = format!("/friend/{}", f.slug());
                                        view! {
                                            <g class="friend-sale"
                                                on:click=move |ev| {
                                                    if map_did_drag.get_untracked() {
                                                        map_did_drag.set(false);
                                                        ev.stop_propagation();
                                                        return;
                                                    }
                                                    ev.stop_propagation();
                                                    if let Some(w) = web_sys::window() {
                                                        let _ = w.location().set_href(&href);
                                                    }
                                                }
                                            >
                                                <path
                                                    d=d
                                                    fill=LandUse::Hgb.color()
                                                    stroke=LandUse::Hgb.color()
                                                    stroke-width="1.2"
                                                    stroke-linejoin="round"
                                                    vector-effect="non-scaling-stroke"
                                                    class="friend-path"
                                                />
                                            </g>
                                        }
                                    }).collect_view()}

                                    // roads, paths and canyons — true width in metres, over the plots
                                    {
                                        let upm = svg_per_m(&m_trails.bbox, W, H, PAD);
                                        // paths first, roads over them, canyons on top
                                        let mut list: Vec<&Trail> = trails().iter().collect();
                                        list.sort_by_key(|t| match t.kind.as_str() { "path" => 0, "road" => 1, _ => 2 });
                                        list.into_iter().map(|t| {
                                            let d = line_path(&t.coords, &m_trails.bbox, W, H, PAD);
                                            // paths wear their colour from the valley map, see-through
                                            let (stroke, dash, opacity) = match t.kind.as_str() {
                                                "road" => ("#777777".to_string(), "none", "0.95"),
                                                "canyon" => ("#304ffe".to_string(), "6 4", "0.85"),
                                                _ if !t.color.is_empty() => (t.color.clone(), "none", "0.5"),
                                                _ => ("#4b4b4d".to_string(), "none", "0.5"),
                                            };
                                            view! {
                                                <path
                                                    d=d
                                                    fill="none"
                                                    stroke=stroke
                                                    stroke-width=format!("{:.2}", t.width_m * upm)
                                                    stroke-dasharray=dash
                                                    stroke-linecap="round"
                                                    stroke-linejoin="round"
                                                    opacity=opacity
                                                    class=format!("trail {}", t.kind)
                                                    pointer-events="none"
                                                />
                                            }
                                        }).collect_view()
                                    }

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
                                            (FlatStatus::Available, _) => terms(&id).and_then(price_line).unwrap_or_else(|| "price on request".into()),
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
                            {move || if domains_board {
                                "drag domain glows to place · pan empty space · scroll zoom · saved in browser"
                            } else {
                                "drag pan · scroll zoom · tap a flat"
                            }}
                        </div>
                    </div>
                    <div class="flat-legend">
                        <div class="leg-row">
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
                        <span class="leg dash" title="a lease request you opened in this browser">"IN TRANSFER"</span>
                        </div>
                        <div class="leg-row">
                        <span class="leg dim">"lines"</span>
                        <span class="leg line" style:--sw="#777777">"ROAD"</span>
                        <span class="leg line thin" style:--sw="#097138">"PATH"</span>
                        <span class="leg line dashed" style:--sw="#304ffe">"CANYON"</span>
                        </div>
                    </div>
                </section>

                // RIGHT, over the planet — the tapped point's divisions, on the plot panel's plan
                {move || (!near.get() && point.get().is_some()).then(|| view! {
                    <DivisionPanel point=point chain=chain selected=unit_sel />
                })}

                // RIGHT — 3D + INTENT
                <section class="cyberia-panel cyberia-right">
                    <div class="cyberia-panel-h">
                        <span class="panel-kicker">"RENDER"</span>
                        <span class="panel-sub">"flat · 3d"</span>
                    </div>
                    <div class="render-3d">
                        {move || {
                            let fid = selected_flat.get().unwrap_or_default();
                            let list = flats.get();
                            let flat = list.iter().find(|f| f.id == fid).cloned();
                            let flat_name = flat
                                .as_ref()
                                .map(|f| bare_name(&f.name))
                                .unwrap_or_else(|| fid.to_uppercase());
                            let n = flat.as_ref().map(|f| f.coords.len()).unwrap_or(0);
                            let lu = land_use(&fid);
                            let status = flat_status(&fid, &deals.get());

                            // real footprint of the selected flat, in its own
                            // local frame (12% margin so the outline clears the edge)
                            const RW: f64 = 120.0;
                            const RH: f64 = 120.0;
                            const RPAD: f64 = 14.0;
                            let plot_path = flat
                                .as_ref()
                                .map(|f| {
                                    let (min_lon, max_lon, min_lat, max_lat) = poly_bbox(&f.coords);
                                    let mlon = ((max_lon - min_lon) * 0.12).max(1e-7);
                                    let mlat = ((max_lat - min_lat) * 0.12).max(1e-7);
                                    let bb = BBox {
                                        min_lon: min_lon - mlon,
                                        max_lon: max_lon + mlon,
                                        min_lat: min_lat - mlat,
                                        max_lat: max_lat + mlat,
                                    };
                                    poly_path(&f.coords, &bb, RW, RH, RPAD)
                                })
                                .unwrap_or_default();

                            view! {
                                <div class="render-stage">
                                    <div class="plot-slab" style:--prism-color=lu.color()>
                                        <svg class="plot-slab-svg" viewBox="0 0 120 120">
                                            <path d=plot_path class="plot-slab-path" />
                                        </svg>
                                    </div>
                                    <div class="prism-meta">
                                        <div class="prism-name">{flat_name}</div>
                                        <div class="prism-sub">
                                            <span style:color=lu.color()>{lu.label()}</span>
                                            {format!(" · {} · {n} verts", status.label().to_lowercase())}
                                        </div>
                                        <a class="prism-link" href=format!("/plot/{fid}")>"OPEN FLAT →"</a>
                                    </div>
                                </div>
                            }
                        }}
                    </div>

                    <div class="cyberia-panel-h" style="margin-top: 24px;">
                        <span class="panel-kicker">"FLAT"</span>
                        <span class="panel-sub">"pick on the map · lease or ask for its papers"</span>
                        {move || selected_flat.get().is_some().then(|| view! {
                            <button class="panel-x" type="button" title="deselect (Esc)" on:click=move |_| selected_flat.set(None)>"✕"</button>
                        })}
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
                                LandUse::Venture if status == FlatStatus::OnRequest => {
                                    if zone == "avalon" { "joint-venture terms".to_string() } else { "on request".to_string() }
                                }
                                _ if status == FlatStatus::Closed => "—".to_string(),
                                _ if status == FlatStatus::Available => {
                                    book.and_then(price_line).unwrap_or_else(|| "price on request".into())
                                }
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
                                            {match lu {
                                                LandUse::Dual => "LEASE THIS FLAT · business or home",
                                                LandUse::Venture => "LEASE FOR A BUSINESS",
                                                _ => "LEASE THIS FLAT",
                                            }}
                                            {book.filter(|t| !t.niche.is_empty()).map(|t| format!(" · {}", t.niche.to_uppercase()))}
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
                                                                "for a building company · on request"
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
                                                    <span class="use-s">"no price in the book yet · ask and the valley answers"</span>
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
                                                        {if lu == LandUse::Hgb {
                                                            "REQUEST THE DISTRICT TITLE"
                                                        } else if zone == "avalon" || book.map(|t| t.is_construction()).unwrap_or(false) {
                                                            "REQUEST A JOINT VENTURE"
                                                        } else {
                                                            "ASK FOR THIS GROUND"
                                                        }}
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
                                let q: Vec<_> = intents.get().into_iter().filter(|i| i.fleet == "YOU").collect();
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

            <div class="search-dock cyberia-dock">
                <span class="dock-count">
                    {move || {
                        let d = deals.get();
                        let (mut open, mut ask, mut held) = (0, 0, 0);
                        for f in flats.get().iter() {
                            match flat_status(&f.id, &d) {
                                FlatStatus::Available => open += 1,
                                FlatStatus::OnRequest => ask += 1,
                                FlatStatus::Owned | FlatStatus::Transfer => held += 1,
                                FlatStatus::Closed => {}
                            }
                        }
                        let mine = intents.get().iter().filter(|i| i.fleet == "YOU").count();
                        format!("{open} flats open · {ask} on request · {held} held · {mine} your requests")
                    }}
                </span>
                <div class="cyberia-cta-bar dock-ctas">
                    <button class="cta-btn cta-lease cta-lg" on:click=move |_| {
                        let Some(fid) = selected_flat.get_untracked() else { return };
                        let list = flats.get_untracked();
                        let Some(flat) = list.iter().find(|f| f.id == fid) else { return };
                        // every flat answers on its own page — open or not, the deal panel says why
                        if let Some(w) = web_sys::window() {
                            let _ = w.location().set_href(&format!("{BASE}/plot/{}/lease", flat.id));
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
                                                None => format!("{name} · price on request"),
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
