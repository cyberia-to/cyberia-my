//! The files a flat hands out from its own page: the passport of everything
//! the map and the lease book know about it, and its boundary as KML for any
//! earth viewer. Both are built in the browser and offered as data URLs, so
//! they need no endpoint behind them.

use crate::deals::{holder, FlatStatus};
use crate::land::{area_m2, LandFlat};
use crate::plot_land::Frame;
use crate::terms::{bare_name, terms};

pub fn encode(s: &str) -> String {
    String::from(js_sys::encode_uri_component(s))
}

pub fn text_data_url(mime: &str, body: &str) -> String {
    format!("data:{mime};charset=utf-8,{}", String::from(js_sys::encode_uri_component(body)))
}

pub fn passport_json(flat: &LandFlat, frame: &Frame, status: FlatStatus) -> String {
    let doc = serde_json::json!({
        "id": flat.id,
        "name": bare_name(&flat.name),
        "holder": holder(&flat.id),
        "lease_book": terms(&flat.id),
        "zone": flat.zone,
        "phase": flat.phase,
        "status": status.label(),
        "area_m2": (area_m2(&flat.coords) * 10.0).round() / 10.0,
        "perimeter_m": (frame.perimeter_m() * 10.0).round() / 10.0,
        "vertices": flat.coords.len(),
        "coords_lonlat": flat.coords,
        "generated_ms": crate::deals::now_ms(),
        "source": "cyberia.my · phase 0 · soft3 local",
    });
    serde_json::to_string_pretty(&doc).unwrap_or_default()
}

pub fn boundary_kml(flat: &LandFlat) -> String {
    let ring: Vec<String> = flat
        .coords
        .iter()
        .map(|c| format!("{},{},0", c[0], c[1]))
        .collect();
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<kml xmlns=\"http://www.opengis.net/kml/2.2\"><Document><name>{name}</name><Placemark><name>{name}</name><description>{zone} · phase {phase} · cyberia.my</description><Polygon><outerBoundaryIs><LinearRing><coordinates>{coords}</coordinates></LinearRing></outerBoundaryIs></Polygon></Placemark></Document></kml>\n",
        name = bare_name(&flat.name),
        zone = flat.zone,
        phase = flat.phase,
        coords = ring.join(" "),
    )
}
