//! Drawing on the one map (`specs/map.md`): the layers' dock, the tools, the
//! features painted over the globe at every zoom, the corners dragged into
//! place, a statement signed on the spot, and SIGN & PUBLISH.
//!
//! The features are painted on one canvas laid over the globe and placed by
//! the camera's eye each frame — a plot as its ring, a building as a prism
//! from its base to its height. The corners of the one feature being shaped
//! are elements of the page, so a finger can take them.

use crate::atlas::{self, deg, e7, Draft, Feature, Fix, Layer, Shown, Store, COLORS, P};
use crate::globe::{cam_fit, eye, point_at, Cam, Eye};
use crate::market::short;
use leptos::prelude::*;
use serde_json::Value;
use wasm_bindgen::JsCast;
use web_sys::CanvasRenderingContext2d as Ctx;

const TOOLS: [(&str, &str, &str); 4] = [
    ("point", "POINT", "a place: a gate, a spring, a corner stone"),
    ("line", "LINE", "a path, a fence, a pipe"),
    ("plot", "PLOT", "a piece of land: tap its corners"),
    ("building", "BUILDING", "a footprint and a height"),
];

/// The map's state on a page — signals, copied wherever they are needed.
#[derive(Clone, Copy)]
pub struct Atlas {
    remote: RwSignal<Vec<Layer>>,
    store: RwSignal<Store>,
    layers: Memo<Vec<Shown>>,
    open: RwSignal<bool>,
    tool: RwSignal<&'static str>,
    pen: RwSignal<Vec<P>>,
    /// the pen is down: the form of the new feature is shown
    naming: RwSignal<bool>,
    sel: RwSignal<Option<(String, String)>>,
    /// the corners of the selected feature while they are dragged
    shape: RwSignal<Option<Vec<P>>>,
    settings: RwSignal<bool>,
    verifying: RwSignal<bool>,
    fix: RwSignal<Option<Fix>>,
    wallet: RwSignal<bool>,
    busy: RwSignal<bool>,
    err: RwSignal<String>,
    said: RwSignal<String>,
    f_name: RwSignal<String>,
    f_note: RwSignal<String>,
    f_height: RwSignal<String>,
    f_props: RwSignal<String>,
    f_new: RwSignal<String>,
    f_rules: RwSignal<String>,
    f_addr: RwSignal<String>,
    f_role: RwSignal<String>,
}

impl Atlas {
    pub fn new() -> Self {
        let remote = RwSignal::new(Vec::<Layer>::new());
        let store = RwSignal::new(atlas::load_store());
        let layers = Memo::new(move |_| atlas::shown(&remote.get(), &store.get()));
        let s = |v: &str| RwSignal::new(v.to_string());
        let a = Atlas {
            remote, store, layers,
            open: RwSignal::new(false), tool: RwSignal::new(""), pen: RwSignal::new(Vec::new()), naming: RwSignal::new(false),
            sel: RwSignal::new(None), shape: RwSignal::new(None), settings: RwSignal::new(false), verifying: RwSignal::new(false),
            fix: RwSignal::new(None), wallet: RwSignal::new(crate::eth::load_eth().is_some()), busy: RwSignal::new(false), err: s(""), said: s(""),
            f_name: s(""), f_note: s(""), f_height: s("3"), f_props: s(""), f_new: s(""), f_rules: s(""), f_addr: s(""), f_role: s("witness"),
        };
        a.reload();
        a
    }

    fn reload(self) {
        leptos::task::spawn_local(async move {
            let got = atlas::fetch_layers().await;
            let _ = self.remote.try_set(got);
        });
    }

    fn edit_store(self, f: impl FnOnce(&mut Store)) {
        self.store.update(|s| {
            f(s);
            atlas::save_store(s);
        });
    }

    fn active(self) -> Option<Shown> {
        let id = self.store.with(|s| s.active.clone());
        self.layers.with(|l| l.iter().find(|x| x.id == id).cloned())
    }

    fn selected(self) -> Option<(Shown, Feature)> {
        let (lid, fid) = self.sel.get()?;
        let l = self.layers.with(|l| l.iter().find(|x| x.id == lid).cloned())?;
        let f = l.items.iter().find(|f| f.id == fid).cloned()?;
        Some((l, f))
    }

    fn select(self, l: &Shown, f: &Feature) {
        self.sel.set(Some((l.id.clone(), f.id.clone())));
        self.shape.set(None);
        self.verifying.set(false);
        self.fix.set(None);
        self.f_name.set(f.text("name"));
        self.f_note.set(f.text("note"));
        self.f_height.set(format!("{}", f.height_m()));
        self.f_props.set(f.props.iter().filter(|(k, _)| !["name", "note", "height_cm", "base_cm"].contains(&k.as_str())).map(|(k, v)| format!("{k} = {}", match v { Value::String(s) => s.clone(), v => v.to_string() })).collect::<Vec<_>>().join("\n"));
        self.open.set(true);
    }

    fn drop_pen(self) {
        self.tool.set("");
        self.pen.set(Vec::new());
        self.naming.set(false);
    }

    /// The props of the form, as a feature carries them.
    fn form_props(self, kind: &str) -> std::collections::BTreeMap<String, Value> {
        let mut props = std::collections::BTreeMap::new();
        for line in self.f_props.get_untracked().lines() {
            if let Some((k, v)) = line.split_once('=') {
                let key: String = k.trim().to_lowercase().chars().map(|c| if c.is_ascii_alphanumeric() { c } else { '_' }).take(40).collect();
                let v = v.trim();
                if key.as_bytes().first().map(|b| b.is_ascii_lowercase()).unwrap_or(false) && !v.is_empty() {
                    props.insert(key, v.parse::<i64>().map(Value::from).unwrap_or_else(|_| Value::String(v.to_string())));
                }
            }
        }
        for (k, s) in [("name", self.f_name), ("note", self.f_note)] {
            let v = s.get_untracked();
            if !v.trim().is_empty() {
                props.insert(k.to_string(), Value::String(v.trim().to_string()));
            }
        }
        if kind == "building" {
            let h = self.f_height.get_untracked().trim().parse::<f64>().unwrap_or(3.0).clamp(0.5, 1000.0);
            props.insert("height_cm".to_string(), Value::from((h * 100.0).round() as i64));
        }
        props
    }

    /// A tap on the map at a point of the layer. True when the map took it.
    pub fn tap(self, sx: f64, sy: f64) -> bool {
        let tool = self.tool.get_untracked();
        if !tool.is_empty() {
            if !self.naming.get_untracked() {
                if let Some((lon, lat)) = point_at(sx, sy) {
                    self.add_corner(e7(lon, lat));
                }
            }
            return true;
        }
        if self.shape.get_untracked().is_some() {
            return true;
        }
        let Some(eye) = eye() else { return false };
        let hidden = self.store.with_untracked(|s| s.hidden.clone());
        let hit = self.layers.with_untracked(|ls| {
            ls.iter().rev().filter(|l| !hidden.contains(&l.id)).find_map(|l| l.items.iter().rev().find(|f| hits(&eye, f, sx, sy)).map(|f| (l.clone(), f.clone())))
        });
        match hit {
            Some((l, f)) => {
                self.select(&l, &f);
                true
            }
            None => {
                if self.sel.get_untracked().is_some() {
                    self.sel.set(None);
                }
                false
            }
        }
    }

    fn add_corner(self, p: P) {
        self.pen.update(|g| {
            if g.last() != Some(&p) {
                g.push(p);
            }
        });
        if self.tool.get_untracked() == "point" {
            self.naming.set(true);
        }
    }

    fn run(self, what: &'static str, fut: impl std::future::Future<Output = Result<Layer, String>> + 'static, then: impl FnOnce(Atlas) + 'static) {
        self.busy.set(true);
        self.err.set(String::new());
        leptos::task::spawn_local(async move {
            match fut.await {
                Ok(l) => {
                    self.remote.update(|r| match r.iter_mut().find(|x| x.id == l.id) {
                        Some(x) => *x = l,
                        None => r.push(l),
                    });
                    self.said.set(what.to_string());
                    then(self);
                }
                Err(e) => self.err.set(e),
            }
            self.busy.set(false);
        });
    }
}

fn seg_dist(p: (f64, f64), a: (f64, f64), b: (f64, f64)) -> f64 {
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    let len = dx * dx + dy * dy;
    let t = if len < 1e-9 { 0.0 } else { (((p.0 - a.0) * dx + (p.1 - a.1) * dy) / len).clamp(0.0, 1.0) };
    ((p.0 - a.0 - t * dx).powi(2) + (p.1 - a.1 - t * dy).powi(2)).sqrt()
}

fn on_screen(eye: &Eye, g: &[P], up: f64) -> Option<Vec<(f64, f64)>> {
    g.iter().map(|p| { let (lon, lat) = deg(*p); eye.at(lon, lat, up) }).collect()
}

fn hits(eye: &Eye, f: &Feature, sx: f64, sy: f64) -> bool {
    let Some(q) = on_screen(eye, &f.geom, 0.0) else { return false };
    let p = (sx, sy);
    match f.kind.as_str() {
        "point" => (q[0].0 - sx).hypot(q[0].1 - sy) <= 16.0,
        "line" => q.windows(2).any(|w| seg_dist(p, w[0], w[1]) <= 10.0),
        _ => {
            let mut inside = false;
            for i in 0..q.len() {
                let (a, b) = (q[i], q[(i + 1) % q.len()]);
                if (a.1 > sy) != (b.1 > sy) && sx < (b.0 - a.0) * (sy - a.1) / (b.1 - a.1) + a.0 {
                    inside = !inside;
                }
            }
            inside || (0..q.len()).any(|i| seg_dist(p, q[i], q[(i + 1) % q.len()]) <= 8.0)
        }
    }
}

fn path(ctx: &Ctx, q: &[(f64, f64)], close: bool) {
    ctx.begin_path();
    for (i, (x, y)) in q.iter().enumerate() {
        if i == 0 { ctx.move_to(*x, *y) } else { ctx.line_to(*x, *y) }
    }
    if close {
        ctx.close_path();
    }
}

/// `#rrggbb` with an alpha.
fn tint(color: &str, a: f64) -> String {
    let c = color.trim_start_matches('#');
    match (c.len(), u32::from_str_radix(c, 16)) {
        (6, Ok(v)) => format!("rgba({},{},{},{a})", v >> 16, (v >> 8) & 255, v & 255),
        _ => color.to_string(),
    }
}

fn paint_feature(ctx: &Ctx, eye: &Eye, f: &Feature, geom: &[P], color: &str, on: bool, w: f64, h: f64, px_m: f64, labels: &mut Vec<Label>) {
    let Some(q) = on_screen(eye, geom, 0.0) else { return };
    // off the screen entirely: nothing to do
    let (x0, x1) = q.iter().fold((f64::MAX, f64::MIN), |a, p| (a.0.min(p.0), a.1.max(p.0)));
    let (y0, y1) = q.iter().fold((f64::MAX, f64::MIN), |a, p| (a.0.min(p.1), a.1.max(p.1)));
    if x1 < -40.0 || x0 > w + 40.0 || y1 < -40.0 || y0 > h + 40.0 {
        return;
    }
    let span = (x1 - x0).max(y1 - y0);
    ctx.set_line_width(if on { 3.0 } else { 1.6 });
    ctx.set_stroke_style_str(if on { "#ffffff" } else { color });
    let (cx, cy) = (q.iter().map(|p| p.0).sum::<f64>() / q.len() as f64, q.iter().map(|p| p.1).sum::<f64>() / q.len() as f64);
    match f.kind.as_str() {
        "point" => {
            ctx.begin_path();
            let _ = ctx.arc(q[0].0, q[0].1, if on { 8.0 } else { 6.0 }, 0.0, std::f64::consts::TAU);
            ctx.set_fill_style_str(color);
            ctx.fill();
            ctx.set_stroke_style_str(if on { "#ffffff" } else { "#000000" });
            ctx.stroke();
        }
        "line" => {
            path(ctx, &q, false);
            ctx.stroke();
        }
        _ if span < 5.0 => {
            // too small to read as a shape at this zoom: a dot holds its place
            ctx.set_fill_style_str(color);
            ctx.fill_rect(cx - 2.0, cy - 2.0, 4.0, 4.0);
        }
        "building" => {
            // the camera looks straight down, where a prism shows no wall: the roof is drawn lifted up the screen
            // by its height, the way a plan shows a building standing
            let (lift0, lift1) = (f.base_m() * px_m * 0.6, (f.base_m() + f.height_m()) * px_m * 0.6);
            let q: Vec<(f64, f64)> = q.iter().map(|p| (p.0, p.1 - lift0)).collect();
            let top: Vec<(f64, f64)> = q.iter().map(|p| (p.0, p.1 - (lift1 - lift0))).collect();
            path(ctx, &q, true);
            ctx.set_fill_style_str(&tint(color, 0.18));
            ctx.fill();
            // the walls, then the roof over them
            for i in 0..q.len() {
                let j = (i + 1) % q.len();
                path(ctx, &[q[i], q[j], top[j], top[i]], true);
                ctx.set_fill_style_str(&tint(color, 0.34));
                ctx.fill();
                ctx.stroke();
            }
            path(ctx, &top, true);
            ctx.set_fill_style_str(&tint(color, 0.6));
            ctx.fill();
            ctx.stroke();
        }
        _ => {
            path(ctx, &q, true);
            ctx.set_fill_style_str(&tint(color, if on { 0.34 } else { 0.18 }));
            ctx.fill();
            ctx.stroke();
        }
    }
    // the name, once the feature is large enough to carry it; what was signed on the spot beside it
    if f.kind == "point" || span > 70.0 {
        let mark = if f.standing.dispute > 0 { " !" } else if f.standing.on_spot > 0 { " ✓" } else { "" };
        let text = format!("{}{mark}", f.text("name"));
        if !text.trim().is_empty() {
            let point = f.kind == "point";
            labels.push(Label { text, x: if point { q[0].0 + 11.0 } else { cx }, y: if point { q[0].1 + 4.0 } else { cy + 4.0 }, left: point, bad: f.standing.dispute > 0, first: on });
        }
    }
}

/// A name waiting to be written: names are written after the shapes, and one that would cover another is left out.
struct Label {
    text: String,
    x: f64,
    y: f64,
    left: bool,
    bad: bool,
    /// the selected feature's name is written first, so it is never the one left out
    first: bool,
}

fn write_labels(ctx: &Ctx, mut labels: Vec<Label>) {
    labels.sort_by_key(|l| !l.first);
    let mut taken: Vec<(f64, f64, f64, f64)> = Vec::new();
    ctx.set_font("700 12px Play, sans-serif");
    for l in labels {
        let wide = l.text.chars().count() as f64 * 7.2 + 6.0;
        let (x0, x1) = if l.left { (l.x - 2.0, l.x + wide) } else { (l.x - wide / 2.0, l.x + wide / 2.0) };
        let (y0, y1) = (l.y - 12.0, l.y + 4.0);
        if taken.iter().any(|t| x0 < t.1 && x1 > t.0 && y0 < t.3 && y1 > t.2) {
            continue;
        }
        taken.push((x0, x1, y0, y1));
        ctx.set_text_align(if l.left { "left" } else { "center" });
        ctx.set_line_width(3.0);
        ctx.set_stroke_style_str("rgba(0,0,0,.85)");
        let _ = ctx.stroke_text(&l.text, l.x, l.y);
        ctx.set_fill_style_str(if l.bad { "#ff5c5c" } else { "#ffffff" });
        let _ = ctx.fill_text(&l.text, l.x, l.y);
    }
}

#[component]
pub fn AtlasLayer(atlas: Atlas, cam: RwSignal<Cam>) -> impl IntoView {
    let a = atlas;
    let canvas_ref = NodeRef::<leptos::html::Canvas>::new();
    let wrap_ref = NodeRef::<leptos::html::Div>::new();

    // paint: every frame of the camera, every change of what is drawn
    Effect::new(move |_| {
        let (_, _, z, w, h, _) = cam.get();
        let px_m = crate::globe::px_per_m(z);
        let layers = a.layers.get();
        let hidden = a.store.with(|s| s.hidden.clone());
        let (sel, shape, pen, tool) = (a.sel.get(), a.shape.get(), a.pen.get(), a.tool.get());
        let Some(canvas) = canvas_ref.get() else { return };
        let dpr = window().device_pixel_ratio().max(1.0);
        let (pw, ph) = ((w * dpr) as u32, (h * dpr) as u32);
        if canvas.width() != pw || canvas.height() != ph {
            canvas.set_width(pw);
            canvas.set_height(ph);
        }
        let Some(ctx) = canvas.get_context("2d").ok().flatten().and_then(|c| c.dyn_into::<Ctx>().ok()) else { return };
        let _ = ctx.set_transform(dpr, 0.0, 0.0, dpr, 0.0, 0.0);
        ctx.clear_rect(0.0, 0.0, w, h);
        let Some(eye) = eye() else { return };
        ctx.set_line_join("round");
        let mut labels = Vec::new();
        for l in layers.iter().filter(|l| !hidden.contains(&l.id)) {
            for f in &l.items {
                let on = sel.as_ref().map(|(lid, fid)| lid == &l.id && fid == &f.id).unwrap_or(false);
                let geom = if on { shape.as_deref().unwrap_or(&f.geom) } else { &f.geom };
                paint_feature(&ctx, &eye, f, geom, atlas::color_of(l, f), on, w, h, px_m, &mut labels);
            }
        }
        write_labels(&ctx, labels);
        // the pen: what is being drawn, dashed until it is put down
        if !pen.is_empty() {
            if let Some(q) = on_screen(&eye, &pen, 0.0) {
                let _ = ctx.set_line_dash(&js_sys::Array::of2(&6.0.into(), &5.0.into()));
                ctx.set_line_width(2.0);
                ctx.set_stroke_style_str("#ffffff");
                path(&ctx, &q, tool == "plot" || tool == "building");
                ctx.stroke();
                let _ = ctx.set_line_dash(&js_sys::Array::new());
                for (x, y) in &q {
                    ctx.begin_path();
                    let _ = ctx.arc(*x, *y, 4.5, 0.0, std::f64::consts::TAU);
                    ctx.set_fill_style_str("#fcf000");
                    ctx.fill();
                    ctx.set_stroke_style_str("#000");
                    ctx.stroke();
                }
            }
        }
    });

    // the corners of the feature being shaped: taken by a finger, moved over the ground
    let drag = RwSignal::new(None::<usize>);
    let place = move |i: usize, mid: bool| {
        move || {
            cam.track();
            let pos = a.shape.with(|s| {
                let g = s.as_ref()?;
                let p = if mid {
                    let (p, q) = (g[i], g[(i + 1) % g.len()]);
                    [(p[0] + q[0]) / 2, (p[1] + q[1]) / 2]
                } else {
                    *g.get(i)?
                };
                let (lon, lat) = deg(p);
                eye().and_then(|e| e.at(lon, lat, 0.0))
            });
            match pos {
                Some((x, y)) => format!("transform: translate({x:.1}px,{y:.1}px);"),
                None => "display: none;".to_string(),
            }
        }
    };
    let handles = move || {
        let n = a.shape.with(|s| s.as_ref().map(|g| g.len()).unwrap_or(0));
        let closed = a.selected().map(|(_, f)| f.closed()).unwrap_or(false);
        let min = a.selected().map(|(_, f)| match f.kind.as_str() { "point" => 1, "line" => 2, _ => 3 }).unwrap_or(1);
        let mids = if n < 2 { 0 } else if closed { n } else { n - 1 };
        view! {
            {(0..mids).map(|i| view! {
                <button type="button" class="globe-ui at-handle mid" style=place(i, true) title="add a corner here"
                    on:pointerdown=move |ev| ev.stop_propagation()
                    on:click=move |ev| {
                        ev.stop_propagation();
                        a.shape.update(|s| if let Some(g) = s {
                            let (p, q) = (g[i], g[(i + 1) % g.len()]);
                            g.insert(i + 1, [(p[0] + q[0]) / 2, (p[1] + q[1]) / 2]);
                        });
                    }></button>
            }).collect_view()}
            {(0..n).map(|i| view! {
                <button type="button" class="globe-ui at-handle" style=place(i, false) title="drag: move · double tap: remove"
                    on:pointerdown=move |ev| {
                        ev.stop_propagation();
                        ev.prevent_default();
                        if let Some(t) = ev.target().and_then(|t| t.dyn_into::<web_sys::Element>().ok()) {
                            let _ = t.set_pointer_capture(ev.pointer_id());
                        }
                        drag.set(Some(i));
                    }
                    on:pointermove=move |ev| {
                        if drag.get_untracked() != Some(i) { return }
                        ev.stop_propagation();
                        let Some(wrap) = wrap_ref.get_untracked() else { return };
                        let r = wrap.get_bounding_client_rect();
                        if let Some((lon, lat)) = point_at(ev.client_x() as f64 - r.left(), ev.client_y() as f64 - r.top()) {
                            a.shape.update(|s| if let Some(g) = s { if let Some(p) = g.get_mut(i) { *p = e7(lon, lat); } });
                        }
                    }
                    on:pointerup=move |ev| { ev.stop_propagation(); drag.set(None); }
                    on:click=move |ev| ev.stop_propagation()
                    on:dblclick=move |ev| {
                        ev.stop_propagation();
                        a.shape.update(|s| if let Some(g) = s { if g.len() > min { g.remove(i); } });
                    }></button>
            }).collect_view()}
        }
    };

    view! {
        <div node_ref=wrap_ref class="at-wrap">
            <canvas node_ref=canvas_ref class="at-canvas"></canvas>
            {handles}
            {move || (!a.tool.get().is_empty() && !a.naming.get()).then(|| view! { <div class="globe-cross at-cross"></div> })}
        </div>
        <Dock atlas=a cam=cam />
    }
}

fn field(label: &'static str, hint: &'static str, sig: RwSignal<String>) -> impl IntoView {
    view! {
        <label class="at-field">
            <span>{label}</span>
            <input type="text" placeholder=hint prop:value=move || sig.get() on:input=move |ev| sig.set(event_target_value(&ev)) />
        </label>
    }
}

fn area(label: &'static str, hint: &'static str, rows: u32, sig: RwSignal<String>) -> impl IntoView {
    view! {
        <label class="at-field">
            <span>{label}</span>
            <textarea rows=rows placeholder=hint prop:value=move || sig.get() on:input=move |ev| sig.set(event_target_value(&ev))></textarea>
        </label>
    }
}

/// Publish a layer's draft in parts small enough for one signature each. The cap
/// is on bytes, not on features: the gate refuses a changeset over
/// `atlas::MAX_FILE` and a body over `atlas::MAX_BODY`, and one plot may be a
/// ring of 5000 corners — a handful of those passes a cap that a count of 1500
/// never reaches. So a part is halved until it fits, and it is measured before
/// the wallet is asked to sign it.
async fn publish_all(d: Draft) -> Result<Layer, String> {
    const PART: usize = 1500;
    let mut rest = d.clone();
    loop {
        let mut part = rest.clone();
        part.put.truncate(PART);
        while part.put.len() > 1 && !atlas::fits(&atlas::changeset(&part)) {
            let n = part.put.len();
            part.put.truncate(n - n / 2);
        }
        if !atlas::fits(&atlas::changeset(&part)) {
            return Err("one change of this layer is larger than the gate takes (2 MiB): draw its shape with fewer corners, or put less in its props".to_string());
        }
        rest.put.drain(..part.put.len());
        if !rest.put.is_empty() {
            part.remove.clear();
        } else {
            rest.remove.clear();
        }
        let l = atlas::publish(&part, "").await?;
        rest.fresh = false;
        if rest.pending() == 0 {
            return Ok(l);
        }
    }
}

#[component]
fn Dock(atlas: Atlas, cam: RwSignal<Cam>) -> impl IntoView {
    let a = atlas;
    let file_ref = NodeRef::<leptos::html::Input>::new();
    let pending = move || a.store.with(|s| s.drafts.iter().map(|d| d.pending()).sum::<usize>());

    // the rules textarea is seeded once per layer the settings panel opens on. every
    // store or remote write re-runs what reads a.active(), and a reseed on each of
    // them would throw away the rules being typed and sign the published ones back.
    Effect::new(move |prev: Option<Option<String>>| {
        let l = a.settings.get().then(|| a.active()).flatten();
        let id = l.as_ref().map(|l| l.id.clone());
        if id.is_some() && prev.flatten() != id {
            a.f_rules.set(atlas::rules_text(&l.unwrap().rules));
        }
        id
    });

    let new_layer = move || {
        let name = a.f_new.get_untracked().trim().to_string();
        if name.is_empty() { return }
        let id = atlas::uuid();
        a.edit_store(|s| {
            let n = s.drafts.len();
            s.drafts.push(Draft { id: id.clone(), name, fresh: true, color: COLORS[n % COLORS.len()].to_string(), ..Default::default() });
            s.active = id;
        });
        a.f_new.set(String::new());
    };

    let import = move |ev: web_sys::Event| {
        let Some(input) = ev.target().and_then(|t| t.dyn_into::<web_sys::HtmlInputElement>().ok()) else { return };
        let Some(file) = input.files().and_then(|f| f.get(0)) else { return };
        let fname = file.name();
        input.set_value("");
        leptos::task::spawn_local(async move {
            let text = match atlas::read_file(file).await {
                Ok(t) => t,
                Err(e) => return a.err.set(e),
            };
            let stem = fname.rsplit_once('.').map(|x| x.0).unwrap_or(&fname).to_string();
            let sets: Vec<(String, Vec<Feature>)> = if fname.to_lowercase().ends_with(".kml") || text.trim_start().starts_with("<?xml") || text.contains("<kml") {
                atlas::from_kml(&text, &stem)
            } else {
                let items = atlas::from_geojson(&text);
                if items.is_empty() { Vec::new() } else { vec![(stem, items)] }
            };
            if sets.is_empty() {
                return a.err.set("nothing to draw in this file — a KML or a GeoJSON with points, lines or polygons is read (a KMZ is a zip: unpack it first)".to_string());
            }
            let (n_layers, n_items) = (sets.len(), sets.iter().map(|s| s.1.len()).sum::<usize>());
            let first = sets[0].1.first().map(|f| atlas::centre(&f.geom));
            a.edit_store(|s| {
                for (i, (name, items)) in sets.into_iter().enumerate() {
                    let id = atlas::uuid();
                    let n = s.drafts.len();
                    s.drafts.push(Draft { id: id.clone(), name, fresh: true, color: COLORS[n % COLORS.len()].to_string(), put: items, ..Default::default() });
                    if i == 0 { s.active = id; }
                }
            });
            a.err.set(String::new());
            a.said.set(format!("{n_items} features in {n_layers} layers, from {fname} — in this browser until you sign them"));
            if let Some(p) = first {
                let (lon, lat) = deg(p);
                cam_fit(lon, lat, 1500.0, 1800.0);
            }
        });
    };

    let layers_list = move || {
        let layers = a.layers.get();
        let (active, hidden) = a.store.with(|s| (s.active.clone(), s.hidden.clone()));
        if layers.is_empty() {
            return view! { <p class="at-note">"no layer yet — name one below, or open a KML from My Maps: every folder becomes a layer"</p> }.into_any();
        }
        layers.into_iter().map(|l| {
            let (id1, id2, id3) = (l.id.clone(), l.id.clone(), l.id.clone());
            let off = hidden.contains(&l.id);
            let first = l.items.first().map(|f| atlas::centre(&f.geom));
            view! {
                <div class=if active == l.id { "at-layer on" } else { "at-layer" }>
                    <button type="button" class=if off { "at-eye off" } else { "at-eye" } title="show or hide"
                        on:click=move |_| a.edit_store(|s| if let Some(i) = s.hidden.iter().position(|x| x == &id1) { s.hidden.remove(i); } else { s.hidden.push(id1.clone()); })>
                        <i style=format!("background:{}", l.color)></i>
                    </button>
                    <button type="button" class="at-layer-n" on:click=move |_| { a.edit_store(|s| s.active = id2.clone()); a.settings.set(false); }>
                        <b>{l.name.clone()}</b>
                        <span>{format!("{} · {}", l.items.len(), if l.fresh { "only in this browser".to_string() } else if l.mine { "yours".to_string() } else if l.edit { "you edit".to_string() } else if l.open { "open to all".to_string() } else { format!("by {}", short(&l.owner)) })}</span>
                    </button>
                    {(l.pending > 0).then(|| view! { <em class="at-pend" title="changes not signed yet">{l.pending}</em> })}
                    <button type="button" class="at-mini" title="fly to it" on:click=move |_| if let Some(p) = first { let (lon, lat) = deg(p); cam_fit(lon, lat, 900.0, 1600.0); }>"◎"</button>
                    <button type="button" class="at-mini" title="the layer's settings" on:click=move |_| { a.edit_store(|s| s.active = id3.clone()); a.settings.update(|o| *o = !*o); }>"⚙"</button>
                </div>
            }
        }).collect_view().into_any()
    };

    // the tools and the pen
    let tools = move || {
        let Some(l) = a.active() else { return ().into_any() };
        if !(l.edit || l.open) {
            return view! { <p class="at-note">{format!("{} is drawn by {} — you look, you do not draw here", l.name, short(&l.owner))}</p> }.into_any();
        }
        let tool = a.tool.get();
        if tool.is_empty() {
            return view! {
                <div class="at-tools">
                    {TOOLS.iter().map(|(k, t, line)| view! {
                        <button type="button" class="at-tool" title=*line on:click=move |_| { a.sel.set(None); a.shape.set(None); a.pen.set(Vec::new()); a.naming.set(false); a.f_name.set(String::new()); a.f_note.set(String::new()); a.f_props.set(String::new()); a.tool.set(k); }>{*t}</button>
                    }).collect_view()}
                </div>
            }.into_any();
        }
        let need = match tool { "point" => 1, "line" => 2, _ => 3 };
        let (lid, lname) = (l.id.clone(), l.name.clone());
        view! {
            <div class="at-pen">
                <div class="at-pen-h">
                    <b>{tool.to_uppercase()}</b>
                    <span>{move || {
                        let g = a.pen.get();
                        match (tool, g.len()) {
                            (_, 0) => "tap the map, or move it under the crosshair".to_string(),
                            ("point", _) => "placed".to_string(),
                            ("line", n) => format!("{n} points · {}", atlas::fmt_len(atlas::length_m(&g, false))),
                            (_, n) if n < 3 => format!("{n} corners"),
                            (_, n) => format!("{n} corners · {}", atlas::fmt_area(atlas::area_m2(&g))),
                        }
                    }}</span>
                </div>
                {move || if a.naming.get() {
                    let (lid, lname) = (lid.clone(), lname.clone());
                    view! {
                        {field("name", "what it is called", a.f_name)}
                        {(tool == "building").then(|| field("height, metres", "3", a.f_height))}
                        {area("note", "", 2, a.f_note)}
                        <div class="at-row">
                            <button type="button" class="at-go" on:click=move |_| {
                                let f = Feature { id: atlas::uuid(), kind: tool.to_string(), geom: a.pen.get_untracked(), props: a.form_props(tool), ..Default::default() };
                                a.edit_store(|s| atlas::put(s, &lid, &lname, f));
                                a.drop_pen();
                                a.said.set("drawn — in this browser until you sign it".to_string());
                            }>"PUT IT ON THE LAYER"</button>
                            <button type="button" class="at-mini wide" on:click=move |_| a.naming.set(false)>"BACK"</button>
                        </div>
                    }.into_any()
                } else {
                    view! {
                        <div class="at-row">
                            <button type="button" class="at-go" on:click=move |_| {
                                let (lat, lon, ..) = cam.get_untracked();
                                a.add_corner(e7(lon, lat));
                            }>"＋ AT THE CROSSHAIR"</button>
                            <button type="button" class="at-mini wide" disabled=move || a.pen.with(|g| g.is_empty()) on:click=move |_| a.pen.update(|g| { g.pop(); })>"UNDO"</button>
                        </div>
                        <div class="at-row">
                            <button type="button" class="at-go" disabled=move || a.pen.with(|g| g.len() < need) on:click=move |_| a.naming.set(true)>"FINISH"</button>
                            <button type="button" class="at-mini wide" on:click=move |_| a.drop_pen()>"CANCEL"</button>
                        </div>
                    }.into_any()
                }}
            </div>
        }.into_any()
    };

    // the feature in hand
    let card = move || {
        let Some((l, f)) = a.selected() else { return ().into_any() };
        let can = l.edit || (l.open && (f.by.is_empty() || crate::eth::load_eth().map(|w| w.address.eq_ignore_ascii_case(&f.by)).unwrap_or(false)));
        let shaping = a.shape.get().is_some();
        let published = !f.version.is_empty();
        let (f1, f2, f3, f4, f5) = (f.clone(), f.clone(), f.clone(), f.clone(), f.clone());
        let (l1, l2, l3, l4) = (l.clone(), l.clone(), l.clone(), l.clone());
        let centre = atlas::centre(&f.geom);
        let stand = match (f.standing.on_spot, f.standing.confirm, f.standing.dispute) {
            (0, 0, 0) if published => "nobody has signed for it on the spot yet".to_string(),
            (0, 0, 0) => "not signed yet — publish it first".to_string(),
            (s, c, d) => format!("{c} confirm ({s} standing on it){}", if d > 0 { format!(" · {d} dispute") } else { String::new() }),
        };
        let attests = f.attests.clone();
        view! {
            <div class="at-card">
                <div class="at-card-h">
                    <i style=format!("background:{}", atlas::color_of(&l, &f))></i>
                    <b>{f.name()}</b>
                    <button type="button" class="at-mini" title="let it go" on:click=move |_| { a.sel.set(None); a.shape.set(None); }>"✕"</button>
                </div>
                <div class="at-facts">
                    <span>{f.kind.to_uppercase()}</span><span>{f.measure()}</span>
                    {(f.kind == "building").then(|| view! { <span>{format!("{} m high", f.height_m())}</span> })}
                    <span>{l.name.clone()}</span>
                    {(!f.by.is_empty()).then(|| view! { <span>{format!("by {}", short(&f.by))}</span> })}
                    {(f.versions > 1).then(|| view! { <span>{format!("{} versions", f.versions)}</span> })}
                </div>
                <div class={if f.standing.dispute > 0 { "at-stand bad" } else if f.standing.on_spot > 0 { "at-stand ok" } else { "at-stand" }}>{stand}</div>
                {can.then(|| view! {
                    {field("name", "", a.f_name)}
                    {(f.kind == "building").then(|| field("height, metres", "3", a.f_height))}
                    {area("note", "", 2, a.f_note)}
                    {area("props — one a line, key = value", "status = sold\nprice_usd = 42000", 2, a.f_props)}
                    <div class="at-row">
                        <button type="button" class="at-go" on:click=move |_| {
                            let mut x = f1.clone();
                            x.props = a.form_props(&x.kind);
                            if let Some(g) = a.shape.get_untracked() { x.geom = g; }
                            a.edit_store(|s| atlas::put(s, &l1.id, &l1.name, x));
                            a.shape.set(None);
                            a.said.set("changed — in this browser until you sign it".to_string());
                        }>{if shaping { "KEEP THE SHAPE" } else { "KEEP THE CHANGE" }}</button>
                        {(!shaping).then(|| view! { <button type="button" class="at-mini wide" on:click=move |_| a.shape.set(Some(f2.geom.clone()))>"MOVE CORNERS"</button> })}
                        {shaping.then(|| view! { <button type="button" class="at-mini wide" on:click=move |_| a.shape.set(None)>"UNDO SHAPE"</button> })}
                    </div>
                    {shaping.then(|| view! { <p class="at-note">"drag a corner · tap a small dot to add one · double tap a corner to remove it"</p> })}
                })}
                <div class="at-row">
                    <button type="button" class="at-mini wide" on:click=move |_| { let (lon, lat) = deg(centre); cam_fit(lon, lat, if f3.kind == "point" { 120.0 } else { (atlas::length_m(&f3.geom, true) / 2.0).max(60.0) }, 1400.0); }>"FLY TO IT"</button>
                    {published.then(|| view! { <button type="button" class="at-mini wide" on:click=move |_| a.verifying.update(|v| *v = !*v)>"VERIFY ON THE SPOT"</button> })}
                    {can.then(|| view! { <button type="button" class="at-mini wide bad" on:click=move |_| { a.edit_store(|s| atlas::remove(s, &l2.id, &l2.name, &f4)); a.sel.set(None); a.shape.set(None); }>"REMOVE"</button> })}
                </div>
                {move || a.verifying.get().then(|| {
                    let (f5, l3, l4) = (f5.clone(), l3.clone(), l4.clone());
                    let near = move |fix: &Fix| -> (Option<usize>, f64) {
                        // the corner the signer stands at, when one is within reach of the reading; else the feature as a whole
                        let (i, d) = f5.geom.iter().enumerate().map(|(i, p)| (i, atlas::dist_m(*p, fix.at))).fold((0, f64::MAX), |a, b| if b.1 < a.1 { b } else { a });
                        ((f5.geom.len() > 1 && d <= (fix.accuracy_cm as f64 / 100.0).max(3.0) * 2.0).then_some(i), d)
                    };
                    let near2 = near.clone();
                    let (fa, fb) = (f.clone(), f.clone());
                    let sign = move |statement: &'static str, f: Feature, lid: String| {
                        let fix = a.fix.get_untracked();
                        let vertex = fix.as_ref().and_then(|x| near2(x).0);
                        let (role, note) = (a.f_role.get_untracked(), a.f_note.get_untracked());
                        a.run("signed — your word stands on this version of it", async move { atlas::attest(&lid, &f, statement, &role, fix.as_ref(), vertex, &note).await }, |a| { a.verifying.set(false); a.fix.set(None); });
                    };
                    let sign2 = sign.clone();
                    view! {
                        <div class="at-verify">
                            <p class="at-note">"stand on it — at a corner when you can. the phone reads your position for a few seconds; you sign what it read and what you say. it holds for this version of the feature: a moved boundary is verified again."</p>
                            {move || match a.fix.get() {
                                None => view! {
                                    <button type="button" class="at-go" disabled=move || a.busy.get() on:click=move |_| {
                                        a.busy.set(true);
                                        a.err.set(String::new());
                                        leptos::task::spawn_local(async move {
                                            match atlas::read_fix(6.0).await {
                                                Ok(x) => a.fix.set(Some(x)),
                                                Err(e) => a.err.set(e),
                                            }
                                            a.busy.set(false);
                                        });
                                    }>{move || if a.busy.get() { "READING YOUR POSITION…" } else { "READ MY POSITION" }}</button>
                                }.into_any(),
                                Some(x) => {
                                    let (v, d) = near(&x);
                                    let coarse = x.accuracy_cm > 2500;
                                    view! {
                                        <div class=if coarse { "at-stand bad" } else { "at-stand ok" }>
                                            {format!("±{:.1} m · {} readings within {:.1} m · {}", x.accuracy_cm as f64 / 100.0, x.samples, x.spread_cm as f64 / 100.0, match v { Some(i) => format!("{d:.1} m from corner {}", i + 1), None => format!("{d:.1} m from its nearest corner") })}
                                        </div>
                                        {coarse.then(|| view! { <p class="at-note">"the reading is coarser than 25 m — step into the open and read again; a dispute can still be signed"</p> })}
                                        <button type="button" class="at-mini wide" on:click=move |_| a.fix.set(None)>"READ AGAIN"</button>
                                    }.into_any()
                                }
                            }}
                            <label class="at-field">
                                <span>"you are"</span>
                                <select on:change=move |ev| a.f_role.set(event_target_value(&ev))>
                                    <option value="witness">"a witness"</option>
                                    <option value="holder">"its holder"</option>
                                    <option value="neighbour">"a neighbour"</option>
                                    <option value="surveyor">"a surveyor"</option>
                                </select>
                            </label>
                            <div class="at-row">
                                <button type="button" class="at-go" disabled=move || a.busy.get() || a.fix.with(|x| x.as_ref().map(|x| x.accuracy_cm > 2500).unwrap_or(true))
                                    on:click={ let (s, f, l) = (sign.clone(), fa.clone(), l3.id.clone()); move |_| s("confirm", f.clone(), l.clone()) }>"CONFIRM · SIGN"</button>
                                <button type="button" class="at-mini wide bad" disabled=move || a.busy.get()
                                    on:click={ let (s, f, l) = (sign2.clone(), fb.clone(), l4.id.clone()); move |_| s("dispute", f.clone(), l.clone()) }>"DISPUTE · SIGN"</button>
                            </div>
                        </div>
                    }
                })}
                {(!attests.is_empty()).then(|| view! {
                    <div class="at-words">
                        {attests.into_iter().rev().take(6).map(|w| view! {
                            <div class=if w.statement == "confirm" { "ok" } else { "bad" }>
                                <b>{w.statement.to_uppercase()}</b>
                                <span>{format!("{} · {}{}{}", w.role, short(&w.by), w.distance_cm.map(|d| format!(" · {:.1} m off ±{:.0} m", d as f64 / 100.0, w.accuracy_cm.unwrap_or(0) as f64 / 100.0)).unwrap_or_else(|| " · not on the spot".to_string()), w.vertex.map(|v| format!(" · corner {}", v + 1)).unwrap_or_default())}</span>
                            </div>
                        }).collect_view()}
                    </div>
                })}
            </div>
        }.into_any()
    };

    // the layer's settings: who sees it, who draws on it, how it paints
    let settings = move || {
        if !a.settings.get() { return ().into_any() }
        let Some(l) = a.active() else { return ().into_any() };
        let (l1, l2, l3, l4, l5, l6) = (l.clone(), l.clone(), l.clone(), l.clone(), l.clone(), l.clone());
        let swatches = COLORS.iter().map(|c| {
            let l = l.clone();
            view! {
                <button type="button" class=if l.color == *c { "at-sw on" } else { "at-sw" } style=format!("background:{c}")
                    on:click=move |_| a.edit_store(|s| atlas::draft_mut(s, &l.id, &l.name).color = c.to_string())></button>
            }
        }).collect_view();
        view! {
            <div class="at-set">
                <div class="at-q">{format!("{} — settings", l.name)}</div>
                <div class="at-sws">{swatches}</div>
                <div class="at-row">
                    <button type="button" class="at-mini wide" on:click=move |_| atlas::download(&format!("{}.geojson", l1.name), "application/geo+json", &atlas::to_geojson(&l1))>"SAVE AS GEOJSON"</button>
                    <button type="button" class="at-mini wide" on:click=move |_| atlas::download(&format!("{}.kml", l2.name), "application/vnd.google-earth.kml+xml", &atlas::to_kml(&l2))>"SAVE AS KML"</button>
                </div>
                {if l.fresh {
                    view! {
                        <p class="at-note">"this layer is only in this browser. its first signed change founds it and makes you its owner."</p>
                        <button type="button" class="at-mini wide bad" on:click=move |_| { a.edit_store(|s| { s.drafts.retain(|d| d.id != l3.id); s.active.clear(); }); a.settings.set(false); a.sel.set(None); }>"THROW THE LAYER AWAY"</button>
                    }.into_any()
                } else if l.mine {
                    let rules_l = l4.clone();
                    view! {
                        {area("rules — one a line: when -> colour", "status == sold -> #ff3b30\nconfirm > 0 -> #00ff41\narea_m2 > 1000 and kind == plot -> #00d4ff", 4, a.f_rules)}
                        <p class="at-note">"a rule reads a feature's props and what the map knows of it: kind, area_m2, length_m, height_m, confirm, dispute, on_spot. the first rule that holds paints it."</p>
                        <div class="at-row">
                            <button type="button" class="at-go" disabled=move || a.busy.get() on:click=move |_| {
                                let def = atlas::Definition { color: rules_l.color.clone(), rules: atlas::parse_rules(&a.f_rules.get_untracked()) };
                                let id = rules_l.id.clone();
                                a.run("the layer's colour and rules are signed", async move { atlas::amend(&id, &[], Some(&def)).await }, |_| {});
                            }>"SIGN THE COLOUR AND THE RULES"</button>
                        </div>
                        <div class="at-row">
                            <button type="button" class="at-mini wide" disabled=move || a.busy.get() on:click={ let l = l5.clone(); move |_| { let (id, v) = (l.id.clone(), if l.public { "no" } else { "yes" }); a.run("signed", async move { atlas::amend(&id, &[("public", v.to_string())], None).await }, |_| {}); } }>
                                {if l.public { "EVERYONE SEES IT · MAKE IT PRIVATE" } else { "PRIVATE · SHOW IT TO EVERYONE" }}
                            </button>
                        </div>
                        <div class="at-row">
                            <button type="button" class="at-mini wide" disabled=move || a.busy.get() on:click={ let l = l6.clone(); move |_| { let (id, v) = (l.id.clone(), if l.open { "no" } else { "yes" }); a.run("signed", async move { atlas::amend(&id, &[("open", v.to_string())], None).await }, |_| {}); } }>
                                {if l.open { "ANYONE ADDS TO IT · CLOSE IT" } else { "ONLY EDITORS DRAW · OPEN IT TO ALL" }}
                            </button>
                        </div>
                        {field("an editor's wallet address", "0x…", a.f_addr)}
                        <div class="at-row">
                            <button type="button" class="at-mini wide" disabled=move || a.busy.get() || a.f_addr.get().trim().len() != 42 on:click={ let id = l.id.clone(); move |_| { let (id, addr) = (id.clone(), a.f_addr.get_untracked().trim().to_string()); a.run("the editor is named", async move { atlas::grant(&id, &addr, "editor").await }, |a| a.f_addr.set(String::new())); } }>"NAME AN EDITOR · SIGN"</button>
                        </div>
                        {l.editors.iter().map(|e| { let (id, addr, shown) = (l.id.clone(), e.clone(), short(e)); view! {
                            <div class="at-row"><span class="at-note">{format!("editor {shown}")}</span>
                                <button type="button" class="at-mini" title="take the right back" on:click=move |_| { let (id, addr) = (id.clone(), addr.clone()); a.run("the editor is removed", async move { atlas::grant(&id, &addr, "none").await }, |_| {}); }>"✕"</button></div>
                        }}).collect_view()}
                    }.into_any()
                } else {
                    view! { <p class="at-note">{format!("owned by {} · {}", short(&l.owner), if l.open { "open: anyone adds features of their own" } else { "closed: its owner and editors draw" })}</p> }.into_any()
                }}
            </div>
        }.into_any()
    };

    let connect = move || Callback::new(move |w: crate::eth::EthWallet| {
        leptos::task::spawn_local(async move {
            // a session lets the gate show this address its private layers; publishing needs only the signature
            let _ = crate::market::sign_in(w).await;
            a.wallet.set(crate::eth::load_eth().is_some());
            a.reload();
        });
    });

    // what is not signed yet
    let publish = move || {
        let pend = a.active().filter(|l| l.pending > 0);
        if pend.is_none() && !a.wallet.get() {
            return view! {
                <div class="at-pub quiet">
                    <p class="at-note">"connect a wallet: the layers you own open for drawing, your private ones show, and you sign what you publish"</p>
                    <crate::wallet_ui::WalletConnect on_connected=connect() />
                </div>
            }.into_any();
        }
        let Some(l) = pend else { return ().into_any() };
        if !a.wallet.get() {
            return view! {
                <div class="at-pub">
                    <b>{format!("{} changes on {} are not signed", l.pending, l.name)}</b>
                    <p class="at-note">"they are in this browser only. connect a wallet to sign and publish them."</p>
                    <crate::wallet_ui::WalletConnect on_connected=connect() />
                </div>
            }.into_any();
        }
        let lid = l.id.clone();
        view! {
            <div class="at-pub">
                <b>{format!("{} changes on {} are not signed", l.pending, l.name)}</b>
                <div class="at-row">
                    <button type="button" class="at-go" disabled=move || a.busy.get() on:click=move |_| {
                        let Some(d) = a.store.with_untracked(|s| s.drafts.iter().find(|d| d.id == lid).cloned()) else { return };
                        let id = d.id.clone();
                        a.run("published — signed, committed, on the map for everyone who may see the layer", publish_all(d), move |a| {
                            a.edit_store(|s| match s.drafts.iter_mut().find(|x| x.id == id) {
                                // the colour chosen here stays; what was pending is published
                                Some(x) => { x.put.clear(); x.remove.clear(); x.fresh = false; }
                                None => {}
                            });
                        });
                    }>{move || if a.busy.get() { "WAITING FOR YOUR WALLET…" } else { "SIGN & PUBLISH" }}</button>
                    <button type="button" class="at-mini wide bad" title="drop what is not signed" on:click={ let id = l.id.clone(); let fresh = l.fresh; move |_| a.edit_store(|s| if fresh { s.drafts.retain(|d| d.id != id) } else if let Some(d) = s.drafts.iter_mut().find(|d| d.id == id) { d.put.clear(); d.remove.clear(); }) }>"DISCARD"</button>
                </div>
            </div>
        }.into_any()
    };

    view! {
        <div class=move || if a.open.get() { "globe-ui at-dock open" } else { "globe-ui at-dock" }>
            <button type="button" class="at-toggle" on:click=move |_| a.open.update(|o| *o = !*o)>
                <b>"✎ DRAW"</b>
                <span>{move || { let n = a.layers.with(|l| l.len()); if n == 0 { "your layers on the map".to_string() } else { format!("{n} layers") } }}</span>
                {move || { let n = pending(); (n > 0).then(|| view! { <em class="at-pend">{n}</em> }) }}
            </button>
            {move || a.open.get().then(|| view! {
                <div class="at-body">
                    {move || { let e = a.err.get(); (!e.is_empty()).then(|| view! {
                        <p class="at-note warn">{e.clone()}</p>
                        {e.contains("reload").then(|| view! {
                            <div class="at-row">
                                <button type="button" class="at-mini wide" on:click=move |_| { a.err.set(String::new()); a.reload(); }>"RELOAD THE MAP"</button>
                                <button type="button" class="at-mini wide bad" title="sign your shape over the newer one" on:click=move |_| {
                                    leptos::task::spawn_local(async move {
                                        let got = atlas::fetch_layers().await;
                                        a.edit_store(|s| for d in s.drafts.iter_mut() {
                                            if let Some(l) = got.iter().find(|l| l.id == d.id) {
                                                for p in d.put.iter_mut() {
                                                    if let Some(head) = l.items.iter().find(|f| f.id == p.id) { p.version = head.version.clone(); }
                                                }
                                            }
                                        });
                                        a.remote.set(got);
                                        a.err.set(String::new());
                                        a.said.set("your changes now stand on the newest versions — sign them when you are sure".to_string());
                                    });
                                }>"KEEP MINE OVER THEIRS"</button>
                            </div>
                        })}
                    }) }}
                    {move || { let s = a.said.get(); (!s.is_empty()).then(|| view! { <p class="at-note ok">{s}</p> }) }}
                    <div class="at-q">"LAYERS"</div>
                    <div class="at-layers">{layers_list}</div>
                    <div class="at-row">
                        <input class="at-new" type="text" maxlength="60" placeholder="a new layer's name" prop:value=move || a.f_new.get() on:input=move |ev| a.f_new.set(event_target_value(&ev))
                            on:keydown=move |ev| if ev.key() == "Enter" { new_layer() } />
                        <button type="button" class="at-mini wide" disabled=move || a.f_new.get().trim().is_empty() on:click=move |_| new_layer()>"＋ LAYER"</button>
                        <button type="button" class="at-mini wide" title="a KML from My Maps or Earth, or a GeoJSON" on:click=move |_| if let Some(i) = file_ref.get_untracked() { i.click() }>"OPEN A FILE"</button>
                        <input node_ref=file_ref type="file" accept=".kml,.geojson,.json,application/json" style="display:none" on:change=import />
                    </div>
                    {settings}
                    {tools}
                    {card}
                    {publish}
                </div>
            })}
        </div>
    }
}
