//! The map its people draw (`specs/map.md`): layers of features — points,
//! lines, plots, buildings — kept as files named by the hash of their bytes
//! and changed by acts a wallet signs (`gate/layers.py`).
//!
//! The page is local first: what is drawn lives in this browser as a draft
//! and shows at once; SIGN & PUBLISH sends a layer's pending changes as one
//! changeset under one signature. So drawing works with no network and no
//! wallet, and nothing reaches the shared map unsigned.
//!
//! Coordinates are whole numbers, degrees × 10⁷ (about a centimetre):
//! the same feature is the same bytes everywhere, and no float is hashed.

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::BTreeMap;
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_futures::JsFuture;

pub const E7: f64 = 10_000_000.0;
const STORE: &str = "cyberia.atlas.v1";
const TOKEN_KEY: &str = "cyberia.acts.token";
const R_M: f64 = 6_371_008.8;
pub const COLORS: [&str; 8] = ["#fcf000", "#00ff41", "#00d4ff", "#ff5c8a", "#ff9d00", "#b388ff", "#ffffff", "#ff3b30"];

pub type P = [i64; 2]; // [lon_e7, lat_e7]

pub fn e7(lon: f64, lat: f64) -> P {
    [(lon * E7).round() as i64, (lat * E7).round() as i64]
}

pub fn deg(p: P) -> (f64, f64) {
    (p[0] as f64 / E7, p[1] as f64 / E7)
}

/// What somebody signed about a feature, standing where they stood.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Attest {
    pub by: String,
    pub at: String,
    pub statement: String,
    #[serde(default)]
    pub role: String,
    #[serde(default)]
    pub method: String,
    #[serde(default)]
    pub accuracy_cm: Option<i64>,
    #[serde(default)]
    pub vertex: Option<usize>,
    #[serde(default)]
    pub distance_cm: Option<i64>,
    #[serde(default)]
    pub note: String,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Standing {
    #[serde(default)]
    pub confirm: u32,
    #[serde(default)]
    pub dispute: u32,
    #[serde(default)]
    pub on_spot: u32,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Feature {
    pub id: String,
    /// `point` · `line` · `plot` · `building`
    pub kind: String,
    pub geom: Vec<P>,
    #[serde(default)]
    pub props: BTreeMap<String, Value>,
    /// the published version this one is or replaces; empty for a feature never published
    #[serde(default)]
    pub version: String,
    #[serde(default)]
    pub by: String,
    #[serde(default)]
    pub at: String,
    #[serde(default)]
    pub attests: Vec<Attest>,
    #[serde(default)]
    pub standing: Standing,
    #[serde(default)]
    pub versions: u32,
}

impl Feature {
    pub fn text(&self, key: &str) -> String {
        match self.props.get(key) {
            Some(Value::String(s)) => s.clone(),
            Some(v) => v.to_string(),
            None => String::new(),
        }
    }

    pub fn num(&self, key: &str) -> Option<i64> {
        self.props.get(key).and_then(|v| v.as_i64())
    }

    pub fn name(&self) -> String {
        let n = self.text("name");
        if n.is_empty() { format!("{} {}", self.kind, &self.id[..4.min(self.id.len())]) } else { n }
    }

    pub fn closed(&self) -> bool {
        self.kind == "plot" || self.kind == "building"
    }

    /// A building's height above its base, metres; 0 for everything else.
    pub fn height_m(&self) -> f64 {
        if self.kind == "building" { self.num("height_cm").unwrap_or(300) as f64 / 100.0 } else { 0.0 }
    }

    pub fn base_m(&self) -> f64 {
        self.num("base_cm").unwrap_or(0) as f64 / 100.0
    }

    /// One line of measures: the area of what is closed, the length of what is not.
    pub fn measure(&self) -> String {
        match self.kind.as_str() {
            "point" => {
                let (lon, lat) = deg(self.geom[0]);
                format!("{lat:.6}, {lon:.6}")
            }
            "line" => fmt_len(length_m(&self.geom, false)),
            _ => format!("{} · {} around", fmt_area(area_m2(&self.geom)), fmt_len(length_m(&self.geom, true))),
        }
    }

    /// The feature as a changeset carries it.
    fn wire(&self) -> Value {
        json!({ "id": self.id, "kind": self.kind, "geom": self.geom, "props": self.props, "prev": self.version })
    }
}

/// A layer as the gate gives it.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Layer {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub about: String,
    #[serde(default)]
    pub public: bool,
    #[serde(default)]
    pub open: bool,
    #[serde(default)]
    pub owner: String,
    #[serde(default)]
    pub mine: bool,
    #[serde(default)]
    pub edit: bool,
    #[serde(default)]
    pub editors: Vec<String>,
    #[serde(default)]
    pub definition: Option<Definition>,
    #[serde(default)]
    pub items: Vec<Feature>,
}

/// What a layer says about itself beyond its name: its colour and its rules.
/// A rule is `when` → `color`; the first that holds paints the feature.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Definition {
    #[serde(default)]
    pub color: String,
    #[serde(default)]
    pub rules: Vec<Rule>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Rule {
    pub when: String,
    pub color: String,
}

/// What this browser holds of a layer that is not signed yet.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Draft {
    pub id: String,
    pub name: String,
    /// never published: the first signed change founds it
    #[serde(default)]
    pub fresh: bool,
    #[serde(default)]
    pub color: String,
    #[serde(default)]
    pub put: Vec<Feature>,
    #[serde(default)]
    pub remove: Vec<String>,
}

impl Draft {
    pub fn pending(&self) -> usize {
        self.put.len() + self.remove.len()
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Store {
    #[serde(default)]
    pub drafts: Vec<Draft>,
    #[serde(default)]
    pub hidden: Vec<String>,
    #[serde(default)]
    pub active: String,
}

fn storage() -> Option<web_sys::Storage> {
    web_sys::window().and_then(|w| w.local_storage().ok().flatten())
}

pub fn load_store() -> Store {
    storage().and_then(|ls| ls.get_item(STORE).ok().flatten()).and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default()
}

pub fn save_store(s: &Store) {
    if let (Some(ls), Ok(text)) = (storage(), serde_json::to_string(s)) {
        let _ = ls.set_item(STORE, &text);
    }
}

pub fn uuid() -> String {
    let mut b = [0u8; 16];
    let _ = getrandom::getrandom(&mut b);
    b[6] = (b[6] & 0x0f) | 0x40;
    b[8] = (b[8] & 0x3f) | 0x80;
    let h: String = b.iter().map(|x| format!("{x:02x}")).collect();
    format!("{}-{}-{}-{}-{}", &h[..8], &h[8..12], &h[12..16], &h[16..20], &h[20..])
}

/// One layer as the page shows it: what is published, with this browser's draft over it.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Shown {
    pub id: String,
    pub name: String,
    pub color: String,
    pub rules: Vec<Rule>,
    pub fresh: bool,
    pub mine: bool,
    pub edit: bool,
    pub public: bool,
    pub open: bool,
    pub owner: String,
    pub editors: Vec<String>,
    pub pending: usize,
    pub items: Vec<Feature>,
}

/// Fold the drafts over the published layers.
pub fn shown(remote: &[Layer], store: &Store) -> Vec<Shown> {
    let mut out: Vec<Shown> = Vec::new();
    // the connected wallet owns what it owns even before it has a session; the gate checks every act anyway
    let me = crate::eth::load_eth().map(|w| w.address).unwrap_or_default();
    for (i, l) in remote.iter().enumerate() {
        let d = store.drafts.iter().find(|d| d.id == l.id);
        let def = l.definition.clone().unwrap_or_default();
        let mut items: Vec<Feature> = l.items.iter().filter(|f| d.map(|d| !d.remove.contains(&f.id)).unwrap_or(true)).cloned().collect();
        if let Some(d) = d {
            for p in &d.put {
                match items.iter_mut().find(|f| f.id == p.id) {
                    Some(f) => *f = p.clone(),
                    None => items.push(p.clone()),
                }
            }
        }
        let color = d.map(|d| d.color.clone()).filter(|c| !c.is_empty()).or((!def.color.is_empty()).then(|| def.color.clone())).unwrap_or_else(|| COLORS[i % COLORS.len()].to_string());
        out.push(Shown {
            id: l.id.clone(), name: l.name.clone(), color, rules: def.rules, fresh: false, mine: l.mine || (!me.is_empty() && l.owner.eq_ignore_ascii_case(&me)),
            edit: l.edit || (!me.is_empty() && l.owner.eq_ignore_ascii_case(&me)), public: l.public, open: l.open,
            owner: l.owner.clone(), editors: l.editors.clone(), pending: d.map(|d| d.pending()).unwrap_or(0), items,
        });
    }
    for d in store.drafts.iter().filter(|d| d.fresh && !remote.iter().any(|l| l.id == d.id)) {
        out.push(Shown {
            id: d.id.clone(), name: d.name.clone(), color: if d.color.is_empty() { COLORS[0].to_string() } else { d.color.clone() }, rules: Vec::new(), fresh: true,
            mine: true, edit: true, public: true, open: false, owner: String::new(), editors: Vec::new(), pending: d.pending(), items: d.put.clone(),
        });
    }
    out
}

/// The draft of a layer, made on first touch.
pub fn draft_mut<'a>(store: &'a mut Store, id: &str, name: &str) -> &'a mut Draft {
    if !store.drafts.iter().any(|d| d.id == id) {
        store.drafts.push(Draft { id: id.to_string(), name: name.to_string(), ..Default::default() });
    }
    store.drafts.iter_mut().find(|d| d.id == id).unwrap()
}

/// Put a feature into a layer's draft: a new one, or a newer shape of one.
pub fn put(store: &mut Store, layer: &str, name: &str, f: Feature) {
    let d = draft_mut(store, layer, name);
    match d.put.iter_mut().find(|x| x.id == f.id) {
        Some(x) => *x = f,
        None => d.put.push(f),
    }
}

/// Take a feature off a layer: a published one waits for a signature, a drafted one is just dropped.
pub fn remove(store: &mut Store, layer: &str, name: &str, f: &Feature) {
    let d = draft_mut(store, layer, name);
    d.put.retain(|x| x.id != f.id);
    if !f.version.is_empty() && !d.remove.contains(&f.id) {
        d.remove.push(f.id.clone());
    }
}

// ── measures ──────────────────────────────────────────────────────────

fn local(g: &[P]) -> Vec<(f64, f64)> {
    let lat0 = g.iter().map(|p| p[1] as f64).sum::<f64>() / g.len().max(1) as f64 / E7;
    let k = lat0.to_radians().cos();
    g.iter().map(|p| ((p[0] as f64 / E7).to_radians() * k * R_M, (p[1] as f64 / E7).to_radians() * R_M)).collect()
}

pub fn area_m2(g: &[P]) -> f64 {
    let q = local(g);
    let mut a = 0.0;
    for i in 0..q.len() {
        let (x0, y0) = q[i];
        let (x1, y1) = q[(i + 1) % q.len()];
        a += x0 * y1 - x1 * y0;
    }
    a.abs() / 2.0
}

pub fn length_m(g: &[P], close: bool) -> f64 {
    let q = local(g);
    let n = if close { q.len() } else { q.len().saturating_sub(1) };
    (0..n).map(|i| { let (a, b) = (q[i], q[(i + 1) % q.len()]); ((a.0 - b.0).powi(2) + (a.1 - b.1).powi(2)).sqrt() }).sum()
}

pub fn dist_m(a: P, b: P) -> f64 {
    length_m(&[a, b], false)
}

pub fn centre(g: &[P]) -> P {
    let n = g.len().max(1) as i64;
    [g.iter().map(|p| p[0]).sum::<i64>() / n, g.iter().map(|p| p[1]).sum::<i64>() / n]
}

pub fn fmt_area(m2: f64) -> String {
    if m2 >= 10_000.0 { format!("{:.2} ha", m2 / 10_000.0) } else if m2 >= 100.0 { format!("{:.1} ares", m2 / 100.0) } else { format!("{m2:.0} m²") }
}

pub fn fmt_len(m: f64) -> String {
    if m >= 1000.0 { format!("{:.2} km", m / 1000.0) } else { format!("{m:.1} m") }
}

// ── a layer's rules ───────────────────────────────────────────────────

/// A feature's value under a name: a prop, or one of the things the map knows
/// about it (`kind`, `area_m2`, `length_m`, `confirm`, `dispute`, `on_spot`, `height_m`).
fn value_of(f: &Feature, key: &str) -> Option<String> {
    Some(match key {
        "kind" => f.kind.clone(),
        "area_m2" => format!("{:.0}", if f.closed() { area_m2(&f.geom) } else { 0.0 }),
        "length_m" => format!("{:.0}", length_m(&f.geom, f.closed())),
        "confirm" => f.standing.confirm.to_string(),
        "dispute" => f.standing.dispute.to_string(),
        "on_spot" => f.standing.on_spot.to_string(),
        "height_m" => format!("{:.0}", f.height_m()),
        _ => return f.props.get(key).map(|v| match v { Value::String(s) => s.clone(), v => v.to_string() }),
    })
}

/// Does a rule's condition hold: `key`, `key == value`, `!=`, `>`, `<`, `>=`, `<=`, `~` (contains),
/// joined by `and`. Numbers compare as numbers when both sides are; text ignores case.
pub fn holds(f: &Feature, when: &str) -> bool {
    when.split(" and ").all(|part| {
        let part = part.trim();
        for op in ["==", "!=", ">=", "<=", ">", "<", "~"] {
            if let Some((k, v)) = part.split_once(op) {
                let (k, v) = (k.trim(), v.trim().trim_matches('"').trim_matches('\''));
                let have = value_of(f, k).unwrap_or_default();
                return match (have.parse::<f64>(), v.parse::<f64>(), op) {
                    (Ok(a), Ok(b), "==") => a == b,
                    (Ok(a), Ok(b), "!=") => a != b,
                    (Ok(a), Ok(b), ">=") => a >= b,
                    (Ok(a), Ok(b), "<=") => a <= b,
                    (Ok(a), Ok(b), ">") => a > b,
                    (Ok(a), Ok(b), "<") => a < b,
                    (_, _, "==") => have.eq_ignore_ascii_case(v),
                    (_, _, "!=") => !have.eq_ignore_ascii_case(v),
                    (_, _, "~") => have.to_lowercase().contains(&v.to_lowercase()),
                    _ => false,
                };
            }
        }
        !part.is_empty() && value_of(f, part).map(|v| !v.is_empty() && v != "0").unwrap_or(false)
    })
}

pub fn color_of<'a>(l: &'a Shown, f: &Feature) -> &'a str {
    l.rules.iter().find(|r| holds(f, &r.when)).map(|r| r.color.as_str()).unwrap_or(l.color.as_str())
}

/// Rules as the panel edits them: one per line, `when -> colour`.
pub fn rules_text(rules: &[Rule]) -> String {
    rules.iter().map(|r| format!("{} -> {}", r.when, r.color)).collect::<Vec<_>>().join("\n")
}

pub fn parse_rules(text: &str) -> Vec<Rule> {
    text.lines().filter_map(|l| l.split_once("->")).map(|(w, c)| Rule { when: w.trim().to_string(), color: c.trim().to_string() }).filter(|r| !r.when.is_empty() && !r.color.is_empty()).collect()
}

// ── files in and out ──────────────────────────────────────────────────

fn feature(kind: &str, geom: Vec<P>, name: &str, note: &str) -> Option<Feature> {
    let mut geom = geom;
    if kind != "point" && geom.len() > 1 && geom.first() == geom.last() {
        geom.pop();
    }
    geom.dedup();
    let need = match kind { "point" => 1, "line" => 2, _ => 3 };
    if geom.len() < need {
        return None;
    }
    let mut props = BTreeMap::new();
    if !name.trim().is_empty() {
        props.insert("name".to_string(), Value::String(name.trim().chars().take(200).collect()));
    }
    if !note.trim().is_empty() {
        props.insert("note".to_string(), Value::String(note.trim().chars().take(3000).collect()));
    }
    Some(Feature { id: uuid(), kind: kind.to_string(), geom, props, ..Default::default() })
}

fn tag<'a>(xml: &'a str, name: &str) -> Option<&'a str> {
    let open = xml.find(&format!("<{name}"))?;
    let start = open + xml[open..].find('>')? + 1;
    let end = start + xml[start..].find(&format!("</{name}>"))?;
    Some(xml[start..end].trim())
}

fn unxml(s: &str) -> String {
    let s = s.trim().trim_start_matches("<![CDATA[").trim_end_matches("]]>");
    s.replace("&amp;", "&").replace("&lt;", "<").replace("&gt;", ">").replace("&quot;", "\"").replace("&apos;", "'").replace("<br>", "\n")
}

fn kml_coords(s: &str) -> Vec<P> {
    s.split_whitespace().filter_map(|t| {
        let mut it = t.split(',');
        Some(e7(it.next()?.parse().ok()?, it.next()?.parse().ok()?))
    }).collect()
}

fn kml_placemarks(xml: &str) -> Vec<Feature> {
    let mut out = Vec::new();
    for block in xml.split("<Placemark").skip(1) {
        let block = block.split("</Placemark>").next().unwrap_or("");
        let name = tag(block, "name").map(unxml).unwrap_or_default();
        let note = tag(block, "description").map(unxml).unwrap_or_default();
        for (el, kind) in [("Polygon", "plot"), ("LineString", "line"), ("Point", "point")] {
            for part in block.split(&format!("<{el}")).skip(1) {
                let part = part.split(&format!("</{el}>")).next().unwrap_or("");
                // a polygon's outer ring; what is cut out of it is not carried yet
                let ring = if el == "Polygon" { tag(part, "outerBoundaryIs").unwrap_or(part) } else { part };
                if let Some(f) = tag(ring, "coordinates").and_then(|c| feature(kind, kml_coords(c), &name, &note)) {
                    out.push(f);
                }
            }
        }
    }
    out
}

/// A KML file as layers: every folder is one (My Maps writes a layer as a folder);
/// what lies outside folders is a layer named after the file.
pub fn from_kml(xml: &str, file_name: &str) -> Vec<(String, Vec<Feature>)> {
    let mut out = Vec::new();
    let mut rest = String::new();
    let mut parts = xml.split("<Folder");
    rest.push_str(parts.next().unwrap_or(""));
    for part in parts {
        let (inside, after) = part.split_once("</Folder>").unwrap_or((part, ""));
        let name = inside.split("<Placemark").next().and_then(|head| tag(head, "name")).map(unxml).unwrap_or_else(|| file_name.to_string());
        let items = kml_placemarks(inside);
        if !items.is_empty() {
            out.push((name, items));
        }
        rest.push_str(after);
    }
    let loose = kml_placemarks(&rest);
    if !loose.is_empty() {
        out.push((file_name.to_string(), loose));
    }
    out
}

fn gj_points(v: &Value) -> Vec<P> {
    v.as_array().map(|a| a.iter().filter_map(|p| Some(e7(p.get(0)?.as_f64()?, p.get(1)?.as_f64()?))).collect()).unwrap_or_default()
}

fn gj_geometry(g: &Value, name: &str, note: &str, props: &BTreeMap<String, Value>, out: &mut Vec<Feature>) {
    let c = &g["coordinates"];
    let mut add = |kind: &str, geom: Vec<P>| {
        if let Some(mut f) = feature(kind, geom, name, note) {
            for (k, v) in props {
                f.props.entry(k.clone()).or_insert(v.clone());
            }
            if kind == "plot" && f.props.contains_key("height_cm") {
                f.kind = "building".to_string();
            }
            out.push(f);
        }
    };
    match g["type"].as_str().unwrap_or("") {
        "Point" => add("point", gj_points(&json!([c]))),
        "MultiPoint" => gj_points(c).into_iter().for_each(|p| add("point", vec![p])),
        "LineString" => add("line", gj_points(c)),
        "MultiLineString" => c.as_array().into_iter().flatten().for_each(|l| add("line", gj_points(l))),
        "Polygon" => add("plot", gj_points(&c[0])),
        "MultiPolygon" => c.as_array().into_iter().flatten().for_each(|p| add("plot", gj_points(&p[0]))),
        "GeometryCollection" => g["geometries"].as_array().into_iter().flatten().for_each(|x| gj_geometry(x, name, note, props, out)),
        _ => {}
    }
}

/// A prop as the map keeps it: a short lowercase key, text or a whole number.
fn clean_props(p: &Value) -> BTreeMap<String, Value> {
    let mut out = BTreeMap::new();
    for (k, v) in p.as_object().into_iter().flatten() {
        let key: String = k.to_lowercase().chars().map(|c| if c.is_ascii_alphanumeric() { c } else { '_' }).take(40).collect();
        if key.is_empty() || !key.as_bytes()[0].is_ascii_lowercase() || key == "name" || key == "note" || out.len() >= 50 {
            continue;
        }
        match v {
            Value::String(s) if !s.is_empty() => { out.insert(key, Value::String(s.chars().take(3000).collect())); }
            Value::Number(n) if n.is_i64() => { out.insert(key, v.clone()); }
            Value::Number(n) => { out.insert(key, Value::String(n.to_string())); }
            Value::Bool(b) => { out.insert(key, Value::String(b.to_string())); }
            _ => {}
        }
    }
    out
}

pub fn from_geojson(text: &str) -> Vec<Feature> {
    let Ok(v) = serde_json::from_str::<Value>(text) else { return Vec::new() };
    let mut out = Vec::new();
    let feats: Vec<&Value> = match v["type"].as_str() {
        Some("FeatureCollection") => v["features"].as_array().map(|a| a.iter().collect()).unwrap_or_default(),
        _ => vec![&v],
    };
    for f in feats {
        let (g, p) = if f["type"] == "Feature" { (&f["geometry"], &f["properties"]) } else { (f, &Value::Null) };
        let name = p["name"].as_str().or(p["title"].as_str()).unwrap_or("");
        let note = p["note"].as_str().or(p["description"].as_str()).unwrap_or("");
        gj_geometry(g, name, note, &clean_props(p), &mut out);
    }
    out
}

fn coord(p: P) -> Value {
    json!([p[0] as f64 / E7, p[1] as f64 / E7])
}

pub fn to_geojson(l: &Shown) -> String {
    let feats: Vec<Value> = l.items.iter().map(|f| {
        let geometry = match f.kind.as_str() {
            "point" => json!({ "type": "Point", "coordinates": coord(f.geom[0]) }),
            "line" => json!({ "type": "LineString", "coordinates": f.geom.iter().map(|p| coord(*p)).collect::<Vec<_>>() }),
            _ => {
                let mut ring: Vec<Value> = f.geom.iter().map(|p| coord(*p)).collect();
                ring.push(coord(f.geom[0]));
                json!({ "type": "Polygon", "coordinates": [ring] })
            }
        };
        let mut props = f.props.clone();
        props.insert("kind".to_string(), Value::String(f.kind.clone()));
        props.insert("id".to_string(), Value::String(f.id.clone()));
        json!({ "type": "Feature", "id": f.id, "properties": props, "geometry": geometry })
    }).collect();
    serde_json::to_string_pretty(&json!({ "type": "FeatureCollection", "name": l.name, "features": feats })).unwrap_or_default()
}

fn xml(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}

pub fn to_kml(l: &Shown) -> String {
    let mut out = format!("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<kml xmlns=\"http://www.opengis.net/kml/2.2\"><Document><name>{}</name><Folder><name>{}</name>\n", xml(&l.name), xml(&l.name));
    for f in &l.items {
        let pts = |g: &[P]| g.iter().map(|p| format!("{:.7},{:.7},0", p[0] as f64 / E7, p[1] as f64 / E7)).collect::<Vec<_>>().join(" ");
        let geom = match f.kind.as_str() {
            "point" => format!("<Point><coordinates>{}</coordinates></Point>", pts(&f.geom)),
            "line" => format!("<LineString><coordinates>{}</coordinates></LineString>", pts(&f.geom)),
            _ => {
                let mut ring = f.geom.clone();
                ring.push(f.geom[0]);
                format!("<Polygon><outerBoundaryIs><LinearRing><coordinates>{}</coordinates></LinearRing></outerBoundaryIs></Polygon>", pts(&ring))
            }
        };
        out.push_str(&format!("<Placemark><name>{}</name><description>{}</description>{geom}</Placemark>\n", xml(&f.text("name")), xml(&f.text("note"))));
    }
    out.push_str("</Folder></Document></kml>\n");
    out
}

// ── the gate ──────────────────────────────────────────────────────────

/// The bytes of a value as the gate names them: sorted keys, no spaces.
fn canon(v: &Value) -> String {
    match v {
        Value::Object(m) => {
            let mut keys: Vec<&String> = m.keys().collect();
            keys.sort();
            format!("{{{}}}", keys.iter().map(|k| format!("{}:{}", Value::String((*k).clone()), canon(&m[*k]))).collect::<Vec<_>>().join(","))
        }
        Value::Array(a) => format!("[{}]", a.iter().map(canon).collect::<Vec<_>>().join(",")),
        v => v.to_string(),
    }
}

fn name_of(data: &str) -> String {
    use sha2::{Digest, Sha256};
    let h: String = Sha256::digest(data.as_bytes()).iter().map(|b| format!("{b:02x}")).collect();
    format!("sha256:{h}")
}

fn token() -> Option<String> {
    storage().and_then(|ls| ls.get_item(TOKEN_KEY).ok().flatten()).filter(|t| !t.is_empty())
}

async fn call(path: &str, body: Option<String>) -> Result<Value, String> {
    let window = web_sys::window().ok_or("no window")?;
    let init = web_sys::RequestInit::new();
    init.set_method(if body.is_some() { "POST" } else { "GET" });
    let headers = js_sys::Object::new();
    let set = |k: &str, v: &str| {
        let _ = js_sys::Reflect::set(&headers, &JsValue::from_str(k), &JsValue::from_str(v));
    };
    if let Some(b) = &body {
        set("Content-Type", "application/json");
        init.set_body(&JsValue::from_str(b));
    }
    if let Some(t) = token() {
        set("Authorization", &format!("Bearer {t}"));
    }
    init.set_headers(&headers);
    let resp = JsFuture::from(window.fetch_with_str_and_init(&format!("/api/map{path}"), &init)).await.map_err(|_| "the gate does not answer".to_string())?;
    let resp: web_sys::Response = resp.dyn_into().map_err(|_| "no response")?;
    let text = JsFuture::from(resp.text().map_err(|_| "no body")?).await.map_err(|_| "no body")?.as_string().unwrap_or_default();
    let v: Value = serde_json::from_str(&text).map_err(|_| "the gate does not answer here — the drawing stays in this browser".to_string())?;
    if resp.ok() { Ok(v) } else { Err(v.get("error").and_then(|e| e.as_str()).unwrap_or("refused").to_string()) }
}

/// Every layer this address may see, whole. Empty when the gate does not answer.
pub async fn fetch_layers() -> Vec<Layer> {
    let Ok(v) = call("/layers", None).await else { return Vec::new() };
    let mut out = Vec::new();
    for c in v["layers"].as_array().into_iter().flatten() {
        if let Some(id) = c["id"].as_str() {
            if let Ok(l) = call(&format!("/layer/{id}"), None).await {
                if let Ok(l) = serde_json::from_value::<Layer>(l) {
                    out.push(l);
                }
            }
        }
    }
    out
}

fn now_iso() -> String {
    let d = js_sys::Date::new_0();
    format!("{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z", d.get_utc_full_year(), d.get_utc_month() + 1, d.get_utc_date(), d.get_utc_hours(), d.get_utc_minutes(), d.get_utc_seconds())
}

/// Sign one act of the map and hand it to the gate, with the file it names.
async fn act(kind: &str, layer: &str, fields: &[(&str, String)], file: Option<String>) -> Result<Layer, String> {
    let w = crate::eth::load_eth().ok_or("connect your wallet first")?;
    let mut lines = vec!["cyberia.my map".to_string(), format!("kind: {kind}"), format!("id: {}", uuid()), format!("at: {}", now_iso()), format!("layer: {layer}")];
    if let Some(f) = &file {
        lines.push(format!("file: {}", name_of(f)));
    }
    for (k, v) in fields {
        let v = v.replace('\n', " ");
        if !v.trim().is_empty() {
            lines.push(format!("{k}: {}", v.trim()));
        }
    }
    let message = lines.join("\n");
    let signature = crate::eth::personal_sign(&w.address, &message).await?;
    let body = json!({ "message": message, "signature": signature, "file": file.unwrap_or_default() });
    let v = call("/act", Some(body.to_string())).await?;
    serde_json::from_value(v["layer"].clone()).map_err(|_| "the gate's answer is not a layer".to_string())
}

/// Publish a layer's draft: one changeset, one signature. The first one founds the layer.
pub async fn publish(d: &Draft, note: &str) -> Result<Layer, String> {
    let file = canon(&json!({ "layer": d.id, "put": d.put.iter().map(|f| f.wire()).collect::<Vec<_>>(), "remove": d.remove }));
    let mut fields = vec![("put", d.put.len().to_string()), ("remove", d.remove.len().to_string()), ("note", note.to_string())];
    if d.fresh {
        fields.push(("name", d.name.clone()));
    }
    act("change", &d.id, &fields, Some(file)).await
}

/// Amend a layer: its name, who sees it, who may add to it, its colour and rules.
pub async fn amend(layer: &str, fields: &[(&str, String)], def: Option<&Definition>) -> Result<Layer, String> {
    let file = def.map(|d| canon(&serde_json::to_value(d).unwrap_or_default()));
    act("layer", layer, fields, file).await
}

pub async fn grant(layer: &str, address: &str, role: &str) -> Result<Layer, String> {
    act("grant", layer, &[("address", address.to_string()), ("role", role.to_string())], None).await
}

/// A position read on the spot: the mean of the samples, how far they spread.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Fix {
    pub at: P,
    pub accuracy_cm: i64,
    pub samples: u32,
    pub spread_cm: i64,
}

pub async fn attest(layer: &str, f: &Feature, statement: &str, role: &str, fix: Option<&Fix>, vertex: Option<usize>, note: &str) -> Result<Layer, String> {
    let mut fields = vec![("feature", f.id.clone()), ("version", f.version.clone()), ("statement", statement.to_string()), ("role", role.to_string())];
    match fix {
        Some(x) => fields.extend([
            ("method", "phone".to_string()), ("lat_e7", x.at[1].to_string()), ("lon_e7", x.at[0].to_string()), ("accuracy_cm", x.accuracy_cm.to_string()),
            ("samples", x.samples.to_string()), ("spread_cm", x.spread_cm.to_string()),
        ]),
        None => fields.push(("method", "manual".to_string())),
    }
    if let Some(v) = vertex {
        fields.push(("vertex", v.to_string()));
    }
    fields.push(("note", note.to_string()));
    act("attest", layer, &fields, None).await
}

/// Read the phone's position for a few seconds and average it. The browser asks the person first.
pub async fn read_fix(seconds: f64) -> Result<Fix, String> {
    let js = format!(
        "new Promise((ok, no) => {{ if (!navigator.geolocation) return no('this browser gives no position'); const s = []; \
         const id = navigator.geolocation.watchPosition(p => s.push([p.coords.longitude, p.coords.latitude, p.coords.accuracy]), e => {{ if (!s.length) {{ navigator.geolocation.clearWatch(id); no(e.message || 'no position'); }} }}, {{ enableHighAccuracy: true, maximumAge: 0, timeout: 20000 }}); \
         setTimeout(() => {{ navigator.geolocation.clearWatch(id); s.length ? ok(JSON.stringify(s)) : no('no position came in time — step outside and try again'); }}, {}); }})",
        (seconds * 1000.0) as u32
    );
    let promise: js_sys::Promise = js_sys::eval(&js).map_err(|_| "no position".to_string())?.dyn_into().map_err(|_| "no position".to_string())?;
    let text = JsFuture::from(promise).await.map_err(|e| e.as_string().unwrap_or_else(|| "no position".to_string()))?.as_string().unwrap_or_default();
    let s: Vec<[f64; 3]> = serde_json::from_str(&text).map_err(|_| "no position".to_string())?;
    // the better half of the samples: a phone's first readings are its worst
    let mut s = s;
    s.sort_by(|a, b| a[2].partial_cmp(&b[2]).unwrap_or(std::cmp::Ordering::Equal));
    s.truncate((s.len() + 1) / 2);
    let n = s.len() as f64;
    let at = e7(s.iter().map(|x| x[0]).sum::<f64>() / n, s.iter().map(|x| x[1]).sum::<f64>() / n);
    let spread = s.iter().map(|x| dist_m(e7(x[0], x[1]), at)).fold(0.0, f64::max);
    Ok(Fix { at, accuracy_cm: (s.iter().map(|x| x[2]).sum::<f64>() / n * 100.0).round() as i64, samples: s.len() as u32, spread_cm: (spread * 100.0).round() as i64 })
}

/// Read a file the person picked, as text.
pub async fn read_file(file: web_sys::File) -> Result<String, String> {
    let blob: &web_sys::Blob = file.as_ref();
    let text = js_sys::Reflect::get(blob, &JsValue::from_str("text")).ok().and_then(|f| f.dyn_into::<js_sys::Function>().ok()).ok_or("this browser reads no files")?;
    let promise: js_sys::Promise = text.call0(blob).map_err(|_| "the file does not open".to_string())?.dyn_into().map_err(|_| "the file does not open".to_string())?;
    JsFuture::from(promise).await.map_err(|_| "the file does not open".to_string())?.as_string().ok_or_else(|| "the file is not text".to_string())
}

/// Hand the person a file to keep.
pub fn download(name: &str, mime: &str, text: &str) {
    let Some(doc) = web_sys::window().and_then(|w| w.document()) else { return };
    let Ok(a) = doc.create_element("a") else { return };
    let _ = a.set_attribute("href", &format!("data:{mime};charset=utf-8,{}", String::from(js_sys::encode_uri_component(text))));
    let _ = a.set_attribute("download", name);
    if let Ok(el) = a.dyn_into::<web_sys::HtmlElement>() {
        el.click();
    }
}
