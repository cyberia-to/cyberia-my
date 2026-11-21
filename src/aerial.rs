//! AERIAL — a flat seen from above, where it lies: our own satellite tiles
//! under it, the neighbours in the colour of what they are, the valley's
//! places signed with their distance, the flat itself lit. One picture that
//! says "here", used by the map's preview and by the flat page.

use crate::deals::{flat_status, Deal, FlatStatus};
use crate::globe::TILES;
use crate::land::{centroid, load_map, LandFlat};
use crate::plot_land::Frame;
use crate::terms::{bare_name, land_use};
use leptos::prelude::*;
use std::collections::HashMap;

/// The deepest zoom our pyramid carries over Bali.
pub(crate) const TILE_Z: u32 = 16;

/// Names on the map that are plumbing, not places a buyer walks to.
fn is_plumbing(name: &str) -> bool {
    let n = name.to_lowercase();
    n == "wc" || n.starts_with("parking") || n == "stairs"
}

/// A named place near a flat.
#[derive(Clone, Debug, PartialEq)]
pub struct Near {
    pub name: String,
    pub dist_m: f64,
    /// east, north of the flat's centre, metres
    pub xy: (f64, f64),
}

/// The valley's places by distance from the flat's centre, nearest first,
/// one per name.
pub fn nearby_places(flat: &LandFlat) -> Vec<Near> {
    let frame = Frame::new(&flat.coords);
    let map = load_map();
    let mut v: Vec<Near> = map
        .places
        .iter()
        .filter(|p| !is_plumbing(&p.name))
        .filter_map(|p| {
            let c = p.coords.first()?;
            let (x, y) = frame.to_xy(c[0], c[1]);
            Some(Near { name: p.name.to_lowercase(), dist_m: (x * x + y * y).sqrt(), xy: (x, y) })
        })
        .collect();
    v.sort_by(|a, b| a.dist_m.partial_cmp(&b.dist_m).unwrap_or(std::cmp::Ordering::Equal));
    let mut seen = std::collections::HashSet::new();
    v.retain(|n| seen.insert(n.name.clone()));
    v
}

/// How many flats of a district can be leased now, and how many it has.
pub fn district_stock(zone: &str, deals: &HashMap<String, Deal>) -> (usize, usize) {
    let map = load_map();
    let all: Vec<&LandFlat> = map.phase0.iter().filter(|f| f.zone == zone).collect();
    let open = all.iter().filter(|f| flat_status(&f.id, deals) == FlatStatus::Available).count();
    (open, all.len())
}

/// Web Mercator tile → its north-west corner, degrees.
pub(crate) fn tile_nw(x: i64, y: i64, z: u32) -> (f64, f64) {
    let n = (1u64 << z) as f64;
    let lon = x as f64 / n * 360.0 - 180.0;
    let lat = (std::f64::consts::PI * (1.0 - 2.0 * y as f64 / n)).sinh().atan().to_degrees();
    (lon, lat)
}

pub(crate) fn tile_of(lon: f64, lat: f64, z: u32) -> (i64, i64) {
    let n = (1u64 << z) as f64;
    let x = ((lon + 180.0) / 360.0 * n).floor() as i64;
    let y = ((1.0 - lat.to_radians().tan().asinh() / std::f64::consts::PI) / 2.0 * n).floor() as i64;
    (x, y)
}

/// `radius_m`: how far around the flat the picture reaches. `labels`: sign
/// the places and the neighbours (the big view) or stay quiet (the preview).
#[component]
pub fn Aerial(
    flat: LandFlat,
    radius_m: f64,
    #[prop(into)] deals: Signal<HashMap<String, Deal>>,
    #[prop(default = true)] labels: bool,
) -> impl IntoView {
    let frame = Frame::new(&flat.coords);
    let r = radius_m;
    let side = 2.0 * r;
    // metres east/north of the centre → viewBox units, north up
    let to_v = move |x: f64, y: f64| (x + r, r - y);
    let path_of = {
        let frame = frame.clone();
        move |coords: &[[f64; 2]]| {
            let mut s = String::new();
            for (i, c) in coords.iter().enumerate() {
                let (x, y) = frame.to_xy(c[0], c[1]);
                let (vx, vy) = to_v(x, y);
                s.push_str(if i == 0 { "M" } else { " L" });
                s.push_str(&format!("{vx:.1},{vy:.1}"));
            }
            s.push('Z');
            s
        }
    };

    // the tiles under the picture
    let (lon_w, lat_n) = frame.to_lonlat(-r, r);
    let (lon_e, lat_s) = frame.to_lonlat(r, -r);
    let (tx0, ty0) = tile_of(lon_w, lat_n, TILE_Z);
    let (tx1, ty1) = tile_of(lon_e, lat_s, TILE_Z);
    let mut tiles = Vec::new();
    for ty in ty0..=ty1 {
        for tx in tx0..=tx1 {
            let (lw, ln) = tile_nw(tx, ty, TILE_Z);
            let (le, ls) = tile_nw(tx + 1, ty + 1, TILE_Z);
            let (x0, y0) = frame.to_xy(lw, ln);
            let (x1, y1) = frame.to_xy(le, ls);
            let (vx, vy) = to_v(x0, y0);
            tiles.push((format!("{TILES}/{TILE_Z}/{tx}/{ty}.jpg"), vx, vy, x1 - x0, y0 - y1));
        }
    }

    let map = load_map();
    let me = flat.id.clone();
    let around: Vec<LandFlat> = map
        .phase0
        .iter()
        .filter(|f| f.id != me)
        .filter(|f| {
            centroid(&f.coords)
                .map(|(lon, lat)| {
                    let (x, y) = frame.to_xy(lon, lat);
                    x.abs() < r * 1.25 && y.abs() < r * 1.25
                })
                .unwrap_or(false)
        })
        .cloned()
        .collect();
    let around_paths: Vec<(String, String, String, (f64, f64))> = around
        .iter()
        .map(|f| {
            let c = centroid(&f.coords).map(|(lon, lat)| frame.to_xy(lon, lat)).unwrap_or((0.0, 0.0));
            (f.id.clone(), bare_name(&f.name), path_of(&f.coords), to_v(c.0, c.1))
        })
        .collect();
    let font = side / 64.0;
    // the nearest places that fit: a label too close under another is left out
    let mut places: Vec<Near> = Vec::new();
    if labels {
        for n in nearby_places(&flat) {
            if n.dist_m < 32.0 || n.xy.0.abs() > r * 0.8 || n.xy.1.abs() > r * 0.6 {
                continue;
            }
            let clear = places.iter().all(|p| (p.xy.1 - n.xy.1).abs() > font * 1.5 || (p.xy.0 - n.xy.0).abs() > font * 14.0);
            if clear {
                places.push(n);
            }
            if places.len() >= 7 {
                break;
            }
        }
    }
    let own = path_of(&flat.coords);
    let ink = land_use(&flat.id).color();
    // a round scale bar: 20 m in the preview, 100 m in the wide view
    let bar_m = if r <= 120.0 { 20.0 } else if r <= 260.0 { 50.0 } else { 100.0 };
    let uid = flat.id.replace(|c: char| !c.is_ascii_alphanumeric(), "");

    view! {
        <svg class="aerial" viewBox=format!("0 0 {side:.0} {side:.0}") preserveAspectRatio="xMidYMid slice">
            <defs>
                <radialGradient id=format!("av-{uid}") cx="50%" cy="50%" r="62%">
                    <stop offset="55%" stop-color="#000" stop-opacity="0" />
                    <stop offset="100%" stop-color="#000" stop-opacity="0.82" />
                </radialGradient>
            </defs>
            <g class="aerial-tiles">
                {tiles.into_iter().map(|(href, x, y, w, h)| view! {
                    <image href=href x=format!("{x:.1}") y=format!("{y:.1}") width=format!("{:.1}", w + 0.6) height=format!("{:.1}", h + 0.6) preserveAspectRatio="none" />
                }).collect_view()}
            </g>
            <rect x="0" y="0" width=format!("{side:.0}") height=format!("{side:.0}") fill=format!("url(#av-{uid})") />
            // the neighbours: what can be leased keeps its colour, the rest goes quiet
            {around_paths.into_iter().map(|(id, name, d, c)| {
                let lu = land_use(&id);
                let id_s = id.clone();
                let open = Memo::new(move |_| flat_status(&id_s, &deals.get()) == FlatStatus::Available);
                // a group, not an anchor: inside an svg the view's `a` would be an html element
                let href = format!("{}/plot/{id}", crate::globe::BASE);
                view! {
                    <g class="aerial-nb" on:click=move |_| {
                        if let Some(w) = web_sys::window() {
                            let _ = w.location().set_href(&href);
                        }
                    }>
                        <title>{name.clone()}</title>
                        <path d=d
                            fill=move || if open.get() { lu.color() } else { "#9a9a9a" }
                            fill-opacity=move || if open.get() { "0.30" } else { "0.14" }
                            stroke=move || if open.get() { lu.color() } else { "#bdbdbd" }
                            stroke-opacity="0.75" stroke-width="1" vector-effect="non-scaling-stroke" />
                        {labels.then(|| view! {
                            <text x=format!("{:.1}", c.0) y=format!("{:.1}", c.1) text-anchor="middle" class="aerial-nb-t" font-size=format!("{:.1}", font * 0.72)>
                                {name.rsplit('-').next().unwrap_or("").to_string()}
                            </text>
                        })}
                    </g>
                }
            }).collect_view()}
            // the flat itself: lit, with a ring that breathes
            <path d=own.clone() class="aerial-ring" stroke=ink fill="none" vector-effect="non-scaling-stroke" />
            <path d=own class="aerial-own" fill=ink fill-opacity="0.5" stroke="#fff" stroke-width="2.6" vector-effect="non-scaling-stroke" />
            {places.into_iter().map(|n| {
                let (x, y) = to_v(n.xy.0, n.xy.1);
                view! {
                    <g class="aerial-place">
                        <circle cx=format!("{x:.1}") cy=format!("{y:.1}") r=format!("{:.1}", font * 0.28) />
                        <text x=format!("{:.1}", x + font * 0.5) y=format!("{:.1}", y + font * 0.32) font-size=format!("{font:.1}")>
                            {format!("{} · {:.0} m", n.name, n.dist_m)}
                        </text>
                    </g>
                }
            }).collect_view()}
            // north and the scale, bottom left
            <g class="aerial-scale" transform=format!("translate({:.1},{:.1})", side * 0.04, side * 0.955)>
                <line x1="0" y1="0" x2=format!("{bar_m:.0}") y2="0" vector-effect="non-scaling-stroke" />
                <text x=format!("{:.1}", bar_m + font * 0.4) y=format!("{:.1}", font * 0.32) font-size=format!("{:.1}", font * 0.85)>{format!("{bar_m:.0} m · north up")}</text>
            </g>
        </svg>
    }
}
