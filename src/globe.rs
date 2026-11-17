//! The globe — cyberia.my's home and its map engine.
//!
//! One camera over a unit sphere, from the whole planet to a plot: imagery
//! tiles (Web Mercator pyramid, served from our own front) draped on the
//! sphere, Natural Earth lines over them, the cyberia layers (districts,
//! plots, trails) at the deepest zooms, cities as markers. WebGL2 through
//! web-sys, no map library, no external API.
//!
//! Precision: tile and cyberia vertices are transformed into view space on
//! the CPU in f64 every frame, so the camera can go to a plot without f32
//! jitter; the world line layers are static f32 unit vectors (≈0.4 m).

use crate::land::load_map;
use leptos::prelude::*;
use serde::Deserialize;
use std::cell::RefCell;
use std::collections::HashMap;
use std::f64::consts::PI;
use std::rc::{Rc, Weak};
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;
use web_sys::{
    HtmlCanvasElement, HtmlElement, HtmlImageElement, WebGl2RenderingContext as GL, WebGlBuffer,
    WebGlProgram, WebGlTexture, WebGlUniformLocation, WebGlVertexArrayObject,
};

/// The path the app is served under ("" in production, "/pr/<name>" in a preview).
pub const BASE: &str = match option_env!("CYBERIA_BASE") {
    Some(b) => b,
    None => "",
};
/// Where the imagery pyramid lives (same origin by default).
const TILES: &str = match option_env!("CYBERIA_TILES") {
    Some(t) => t,
    None => "/tiles/s2",
};
/// The poles' own pyramid: equirectangular tiles (2^(z+1) × 2^z per zoom), because
/// Web Mercator ends at 85.05°. North of NORTH_FROM and south of SOUTH_FROM.
const POLAR: &str = match option_env!("CYBERIA_POLAR") {
    Some(t) => t,
    None => "/tiles/polar",
};
const GEO: u8 = 64; // a tile key with zoom ≥ GEO is a polar tile of zoom − GEO
const GEO_MIN: u8 = 3;
const NORTH_ZOOM: u8 = 6;
const SOUTH_ZOOM: u8 = 7;
const NORTH_FROM: f64 = 84.375;
const SOUTH_FROM: f64 = -60.0;
/// The divisions of the earth, level by level (scripts/build-divisions.py).
const DIV: &str = match option_env!("CYBERIA_DIVISIONS") {
    Some(t) => t,
    None => "/tiles/divisions",
};
/// The grid zoom of each level's index: a cell is 360/2^z degrees (level 0 has a list).
const DIV_GRID: [u32; 6] = [0, 4, 6, 7, 8, 9];
/// States whose franchise is open, by three-letter code.
const FRANCHISE_OPEN: &[&str] = &["IDN"];
const WORLD_ZOOM: u8 = 7; // the whole earth is present to this zoom
const MAX_TILE_ZOOM: u8 = 14;
const FOV_Y: f64 = 30.0 * PI / 180.0;
const Z_MAX: f64 = 26.0; // ≈ 2 m across the screen: tile-planning scale
/// From this zoom the land console's vector layer takes over from the engine's lines.
pub const SVG_Z: f64 = 13.5;
/// Web Mercator ground resolution at zoom 0, metres per css px (sphere, no latitude factor).
const M_PER_PX_Z0: f64 = 156_543.03;
const MERC_LAT: f64 = 85.051_128_78;

type V3 = [f64; 3];
type Key = (u8, u32, u32);

fn unit(lon: f64, lat: f64) -> V3 {
    let (l, p) = (lon.to_radians(), lat.to_radians());
    [p.cos() * l.sin(), p.sin(), p.cos() * l.cos()]
}
fn dot(a: V3, b: V3) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}
fn norm(a: V3) -> V3 {
    let l = dot(a, a).sqrt().max(1e-300);
    [a[0] / l, a[1] / l, a[2] / l]
}
/// East and north on the sphere at a point.
fn tangent(lon: f64, lat: f64) -> (V3, V3) {
    let (l, p) = (lon.to_radians(), lat.to_radians());
    ([l.cos(), 0.0, -l.sin()], [-p.sin() * l.sin(), p.cos(), -p.sin() * l.cos()])
}
fn smooth(x: f64) -> f64 {
    let t = x.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}
fn cross(a: V3, b: V3) -> V3 {
    [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]
}
/// Turn a vector about a unit axis by an angle.
fn turn(axis: V3, ang: f64, v: V3) -> V3 {
    let (sn, cs) = ang.sin_cos();
    let (x, k) = (cross(axis, v), dot(axis, v) * (1.0 - cs));
    [v[0] * cs + x[0] * sn + axis[0] * k, v[1] * cs + x[1] * sn + axis[1] * k, v[2] * cs + x[2] * sn + axis[2] * k]
}
/// The globe is a ball in the hand: below this zoom it keeps any heading it is given.
const FREE_Z: f64 = 8.5;
/// The sketch lines of the world (coast, borders, provinces) are gone from this zoom.
const WORLD_LINES_Z: f64 = 6.5;
/// From this zoom the map is north-up: a city's plan is drawn that way.
const LEVEL_Z: f64 = 11.0;
fn merc_lat(y: f64) -> f64 {
    (PI * (1.0 - 2.0 * y)).sinh().atan().to_degrees()
}
fn merc_y(lat: f64) -> f64 {
    let r = lat.clamp(-MERC_LAT, MERC_LAT).to_radians();
    (1.0 - r.tan().asinh() / PI) / 2.0
}

/// Everything a frame needs to place a point of the sphere on the screen.
#[derive(Clone, Copy)]
struct View {
    e: V3,
    north: V3,
    up: V3,
    d: f64,   // camera distance from the sphere's centre, radius = 1
    f: f64,   // 1 / tan(fov/2)
    fpx: f64, // focal length in css px
    w: f64,
    h: f64,
}

impl View {
    /// `hd`: the heading, radians the screen's up is turned from north (0 = north-up).
    fn new(lon: f64, lat: f64, hd: f64, z: f64, w: f64, h: f64) -> Self {
        let f = 1.0 / (FOV_Y / 2.0).tan();
        let fpx = h / 2.0 * f;
        let ppr = 256.0 * 2f64.powf(z) / (2.0 * PI); // css px per radian under the camera
        let (e, n) = tangent(lon, lat);
        let (sn, cs) = hd.sin_cos();
        View {
            e: [e[0] * cs + n[0] * sn, e[1] * cs + n[1] * sn, e[2] * cs + n[2] * sn],
            north: [n[0] * cs - e[0] * sn, n[1] * cs - e[1] * sn, n[2] * cs - e[2] * sn],
            up: unit(lon, lat),
            d: 1.0 + fpx / ppr,
            f,
            fpx,
            w,
            h,
        }
    }
    fn to_view(&self, q: V3) -> V3 {
        [dot(q, self.e), dot(q, self.north), dot(q, self.up) - self.d]
    }
    fn visible(&self, q: V3) -> bool {
        dot(q, self.up) > 1.0 / self.d
    }
    fn to_screen(&self, q: V3) -> Option<(f64, f64)> {
        if !self.visible(q) {
            return None;
        }
        let v = self.to_view(q);
        let x = self.f / (self.w / self.h) * v[0] / -v[2];
        let y = self.f * v[1] / -v[2];
        Some(((x * 0.5 + 0.5) * self.w, (0.5 - y * 0.5) * self.h))
    }
    /// The point of the sphere under a screen pixel.
    fn unproject_q(&self, sx: f64, sy: f64) -> Option<V3> {
        let dx = (sx - self.w / 2.0) / self.fpx;
        let dy = (self.h / 2.0 - sy) / self.fpx;
        let len = (dx * dx + dy * dy + 1.0).sqrt();
        let dir = [dx / len, dy / len, -1.0 / len];
        // |t·dir − c|² = 1 with c = (0, 0, −d)
        let b = dir[2] * self.d;
        let disc = b * b - (self.d * self.d - 1.0);
        if disc < 0.0 {
            return None;
        }
        let t = -b - disc.sqrt();
        let hit = [dir[0] * t, dir[1] * t, dir[2] * t + self.d];
        Some([
            self.e[0] * hit[0] + self.north[0] * hit[1] + self.up[0] * hit[2],
            self.e[1] * hit[0] + self.north[1] * hit[1] + self.up[1] * hit[2],
            self.e[2] * hit[0] + self.north[2] * hit[1] + self.up[2] * hit[2],
        ])
    }
    /// The same, as (lon, lat) degrees.
    fn unproject(&self, sx: f64, sy: f64) -> Option<(f64, f64)> {
        let q = self.unproject_q(sx, sy)?;
        Some((q[0].atan2(q[2]).to_degrees(), q[1].clamp(-1.0, 1.0).asin().to_degrees()))
    }
    /// Angular radius of what the screen can show, around the camera target.
    fn cap(&self) -> f64 {
        let horizon = (1.0 / self.d).acos();
        let rpx = 0.5 * (self.w * self.w + self.h * self.h).sqrt();
        let beta = (rpx / self.fpx).atan();
        let s = self.d * beta.sin();
        if s >= 1.0 {
            horizon
        } else {
            (s.asin() - beta).min(horizon)
        }
    }
}

// ─── shaders ──────────────────────────────────────────────────────────

const BG_VS: &str = "#version 300 es
in vec2 a_pos; out vec2 v;
void main(){ v = a_pos; gl_Position = vec4(a_pos, 0.0, 1.0); }";
const BG_FS: &str = "#version 300 es
precision highp float;
in vec2 v; out vec4 o;
uniform float u_r; uniform float u_aspect;
void main(){
  float t = length(vec2(v.x * u_aspect, v.y)) / u_r;
  vec3 c;
  if (t < 1.0) {
    c = mix(vec3(0.020, 0.060, 0.085), vec3(0.008, 0.026, 0.040), t * t);
    c += pow(t, 10.0) * vec3(0.0, 0.30, 0.20);
  } else {
    c = exp(-(t - 1.0) * 11.0) * vec3(0.0, 0.85, 0.50) * 0.40;
  }
  o = vec4(c, 1.0);
}";
const TILE_VS: &str = "#version 300 es
in vec3 a_pos; in vec2 a_uv; in float a_fade;
uniform float u_fx, u_fy, u_k, u_h; uniform vec3 u_uv;
out vec2 v_uv; out float v_vis; out float v_fade;
void main(){
  gl_Position = vec4(a_pos.x * u_fx, a_pos.y * u_fy, 0.0, -a_pos.z);
  v_vis = (a_pos.z + u_k) / u_h;
  v_uv = (a_uv + u_uv.yz) * u_uv.x;
  v_fade = a_fade;
}";
const TILE_FS: &str = "#version 300 es
precision highp float;
in vec2 v_uv; in float v_vis; in float v_fade; out vec4 o;
uniform sampler2D u_tex; uniform float u_tone;
void main(){
  if (v_vis < 0.0) discard;
  vec3 c = texture(u_tex, v_uv).rgb;
  // the south cap's sea is brighter than the mosaic's: bring it to the same water
  float sea = u_tone * smoothstep(0.06, 0.22, c.b - c.r);
  c = mix(c, vec3(0.105, 0.165, 0.305) * (0.85 + 0.3 * c.b), sea);
  c *= 0.45 + 0.55 * smoothstep(0.0, 0.45, v_vis);
  o = vec4(c, v_fade);
}";
// world lines: static unit vectors, the camera frame as uniforms
const LINE_VS: &str = "#version 300 es
in vec3 a_pos;
uniform vec3 u_e, u_n, u_up; uniform float u_fx, u_fy, u_d;
out float v_vis;
void main(){
  float qn = dot(a_pos, u_up);
  gl_Position = vec4(dot(a_pos, u_e) * u_fx, dot(a_pos, u_n) * u_fy, 0.0, u_d - qn);
  v_vis = (qn - 1.0 / u_d) / (1.0 - 1.0 / u_d);
}";
// cyberia lines: view-space positions computed on the CPU in f64
const VLINE_VS: &str = "#version 300 es
in vec3 a_pos;
uniform float u_fx, u_fy, u_k, u_h; uniform vec2 u_off;
out float v_vis;
void main(){
  gl_Position = vec4(a_pos.x * u_fx - u_off.x * a_pos.z, a_pos.y * u_fy - u_off.y * a_pos.z, 0.0, -a_pos.z);
  v_vis = (a_pos.z + u_k) / u_h;
}";
const LINE_FS: &str = "#version 300 es
precision highp float;
in float v_vis; out vec4 o;
uniform vec4 u_color;
void main(){
  if (v_vis < 0.0) discard;
  o = vec4(u_color.rgb, u_color.a * smoothstep(0.0, 0.06, v_vis));
}";

fn program(gl: &GL, vs: &str, fs: &str, attribs: &[&str]) -> Result<WebGlProgram, String> {
    let compile = |kind: u32, src: &str| -> Result<web_sys::WebGlShader, String> {
        let s = gl.create_shader(kind).ok_or("no shader")?;
        gl.shader_source(&s, src);
        gl.compile_shader(&s);
        if gl.get_shader_parameter(&s, GL::COMPILE_STATUS).as_bool().unwrap_or(false) {
            Ok(s)
        } else {
            Err(gl.get_shader_info_log(&s).unwrap_or_default())
        }
    };
    let p = gl.create_program().ok_or("no program")?;
    gl.attach_shader(&p, &compile(GL::VERTEX_SHADER, vs)?);
    gl.attach_shader(&p, &compile(GL::FRAGMENT_SHADER, fs)?);
    for (i, name) in attribs.iter().enumerate() {
        gl.bind_attrib_location(&p, i as u32, name);
    }
    gl.link_program(&p);
    if gl.get_program_parameter(&p, GL::LINK_STATUS).as_bool().unwrap_or(false) {
        Ok(p)
    } else {
        Err(gl.get_program_info_log(&p).unwrap_or_default())
    }
}

struct Prog {
    p: WebGlProgram,
    u: HashMap<&'static str, Option<WebGlUniformLocation>>,
}
impl Prog {
    fn new(gl: &GL, vs: &str, fs: &str, attribs: &[&str], uniforms: &[&'static str]) -> Result<Self, String> {
        let p = program(gl, vs, fs, attribs)?;
        let u = uniforms.iter().map(|n| (*n, gl.get_uniform_location(&p, n))).collect();
        Ok(Prog { p, u })
    }
    fn loc(&self, name: &str) -> Option<&WebGlUniformLocation> {
        self.u.get(name).and_then(|o| o.as_ref())
    }
}

// ─── engine ───────────────────────────────────────────────────────────

struct Tile {
    grid: usize,
    verts: Vec<V3>, // (grid+1)² unit vectors, row by row from the north-west corner
    fade: Vec<f32>, // opacity per vertex; empty = opaque
    tex: Option<WebGlTexture>,
    loading: bool,
    failed: bool,
    used: u32,
}

struct StaticLayer {
    vao: WebGlVertexArrayObject,
    count: i32,
    color: [f32; 4],
    min_z: f64,
}

struct PreciseLayer {
    pts: Vec<V3>, // pairs: a segment per two points
    color: [f32; 4],
    min_z: f64,
    max_z: f64,
}

struct Marker {
    q: V3,
    el: HtmlElement,
}

/// A division of the earth as the page shows it: a state (level 0) or a unit inside it.
#[derive(Clone, PartialEq, Debug)]
pub struct Unit {
    pub id: u64,
    pub level: u8,
    pub name: String,
    /// what the source calls this level here: province, regency, …
    pub kind: String,
    /// the state's three-letter code
    pub code: String,
    pub franchise: bool,
    /// the land inside the outline, km² (outer rings on the sphere; lakes and enclaves count)
    pub area_km2: f64,
}

/// A unit's file.
#[derive(Deserialize)]
struct UnitFile {
    n: String,
    l: u8,
    #[serde(default)]
    t: String,
    c: String,
    p: Vec<Vec<f64>>, // rings: lon, lat, lon, lat, …
}

#[derive(Deserialize)]
struct Country {
    id: u64,
    c: String,
    b: [f64; 4],
}

struct Fly {
    from: (V3, f64, f64), // the point under the camera, zoom, heading
    to: (V3, f64, f64),
    t0: f64,
    dur: f64,
}

pub struct City {
    pub name: &'static str,
    pub sub: &'static str,
    pub lon: f64,
    pub lat: f64,
    /// metres the screen's short side shows when the camera arrives
    pub fit_m: f64,
}

pub const CITIES: &[City] = &[City {
    name: "Cyber Valley",
    sub: "Gesing · Bali · 37 ha · 126 plots",
    lon: 115.0897935,
    lat: -8.2998539,
    fit_m: 1700.0,
}];

struct Engine {
    gl: GL,
    canvas: HtmlCanvasElement,
    alive: bool,
    lon: f64,
    lat: f64,
    hd: f64, // heading, radians from north-up
    spin: Option<(V3, f64)>,      // the ball keeps turning: axis, radians per ms
    drag_spin: Option<(V3, f64)>, // the last move of the hand
    drag_t: f64,
    z: f64,
    z_min: f64,
    last_t: f64,
    highlight: Vec<V3>, // the selected unit's outline
    open: Vec<V3>,      // the outlines of the states whose franchise is open
    dirty: bool,
    frame: u32,
    fly: Option<Fly>,
    bg: Prog,
    bg_vao: WebGlVertexArrayObject,
    tile_prog: Prog,
    tile_vao: WebGlVertexArrayObject,
    tile_vbo: WebGlBuffer,
    index: HashMap<usize, (WebGlBuffer, i32)>,
    line_prog: Prog,
    vline_prog: Prog,
    vline_vao: WebGlVertexArrayObject,
    vline_vbo: WebGlBuffer,
    tiles: HashMap<Key, Tile>,
    inflight: usize,
    regions: Vec<(u8, u32, u32, u32, u32)>,
    world: Vec<StaticLayer>,
    cyberia: Vec<PreciseLayer>,
    markers: Vec<Marker>,
    admin1_asked: bool,
    id: u32,
    fit: Option<f64>,
    on_view: Box<dyn Fn(f64, f64, f64, f64, f64, f64)>,
    last_move: f64,
    url_synced: bool,
}

thread_local! {
    static ENGINE: RefCell<Option<Rc<RefCell<Engine>>>> = const { RefCell::new(None) };
    static NEXT_ID: std::cell::Cell<u32> = const { std::cell::Cell::new(0) };
}

fn now() -> f64 {
    web_sys::window().and_then(|w| w.performance()).map(|p| p.now()).unwrap_or(0.0)
}

impl Engine {
    fn size(&self) -> (f64, f64) {
        (self.canvas.client_width().max(1) as f64, self.canvas.client_height().max(1) as f64)
    }
    fn view(&self) -> View {
        let (w, h) = self.size();
        View::new(self.lon, self.lat, self.hd, self.z, w, h)
    }
    fn clamp(&mut self) {
        self.lat = self.lat.clamp(-90.0, 90.0);
        self.lon = (self.lon + 180.0).rem_euclid(360.0) - 180.0;
        self.z = self.z.clamp(self.z_min, Z_MAX);
    }
    fn touch(&mut self) {
        self.dirty = true;
        self.last_move = now();
        self.url_synced = false;
    }

    /// Set the camera by the point under it and the direction of the screen's up.
    fn set_frame(&mut self, up2: V3, n2: V3) {
        let up2 = norm(up2);
        self.lat = up2[1].clamp(-1.0, 1.0).asin().to_degrees();
        if up2[0].abs() + up2[2].abs() > 1e-12 {
            self.lon = up2[0].atan2(up2[2]).to_degrees();
        }
        let (e_t, n_t) = tangent(self.lon, self.lat);
        self.hd = if self.z >= LEVEL_Z { 0.0 } else { (-dot(n2, e_t)).atan2(dot(n2, n_t)) };
    }

    /// Put the camera over another point of the sphere, carrying the screen's
    /// orientation along the way (no twist), so a move can cross a pole.
    fn move_to(&mut self, up2: V3) {
        let v = self.view();
        let up2 = norm(up2);
        let c = 1.0 + dot(v.up, up2);
        if c < 1e-9 {
            return;
        }
        let k = dot(up2, v.north) / c;
        self.set_frame(up2, [v.north[0] - (v.up[0] + up2[0]) * k, v.north[1] - (v.up[1] + up2[1]) * k, v.north[2] - (v.up[2] + up2[2]) * k]);
    }

    /// The hand on the ball: the point of the earth that was under (x0, y0) goes
    /// to (x1, y1) and stays under the finger. Off the planet's disc it pans.
    fn drag(&mut self, x0: f64, y0: f64, x1: f64, y1: f64) {
        let v = self.view();
        let (Some(a), Some(b)) = (v.unproject_q(x0, y0), v.unproject_q(x1, y1)) else {
            self.pan_px(x1 - x0, y1 - y0);
            return;
        };
        let axis = cross(b, a);
        let len = dot(axis, axis).sqrt();
        if len > 1e-15 {
            let axis = [axis[0] / len, axis[1] / len, axis[2] / len];
            let ang = len.atan2(dot(a, b)).min(0.5);
            self.set_frame(turn(axis, ang, v.up), turn(axis, ang, v.north));
            let t = now();
            let dt = (t - self.drag_t).clamp(4.0, 200.0);
            self.drag_spin = Some((axis, (ang / dt).min(0.004)));
            self.drag_t = t;
        }
        self.fly = None;
        self.spin = None;
        self.clamp();
        self.touch();
    }

    /// The hand lets go: a ball that was moving keeps turning and slows down.
    fn release(&mut self) {
        if let Some(s) = self.drag_spin.take() {
            if now() - self.drag_t < 70.0 && self.z < FREE_Z && s.1 > 3e-5 {
                self.spin = Some(s);
                self.dirty = true;
            }
        }
    }

    /// Turn the view about the point under the camera, at once.
    fn twist(&mut self, da: f64) {
        if self.z >= LEVEL_Z {
            return;
        }
        self.hd = (self.hd + da + PI).rem_euclid(2.0 * PI) - PI;
        self.fly = None;
        self.spin = None;
        self.touch();
    }

    /// Turn the view to a heading, smoothly, the short way or by `delta`.
    fn turn_by(&mut self, delta: f64, dur: f64) {
        let q = unit(self.lon, self.lat);
        self.spin = None;
        self.fly = Some(Fly { from: (q, self.z, self.hd), to: (q, self.z, self.hd + delta), t0: now(), dur });
        self.dirty = true;
    }

    fn covered(&self, k: Key) -> bool {
        k.0 >= GEO
            || k.0 <= WORLD_ZOOM
            || self.regions.iter().any(|r| r.0 == k.0 && k.1 >= r.1 && k.1 <= r.3 && k.2 >= r.2 && k.2 <= r.4)
    }

    fn tile(&mut self, k: Key) -> &mut Tile {
        self.tiles.entry(k).or_insert_with(|| {
            if k.0 >= GEO {
                // equirectangular: vertices evenly in longitude and latitude
                let g = k.0 - GEO;
                let h = 180.0 / (1u32 << g) as f64;
                let grid = match g {
                    0..=3 => 12,
                    4 => 8,
                    5 => 6,
                    _ => 4,
                };
                let mut verts = Vec::with_capacity((grid + 1) * (grid + 1));
                let mut fade = Vec::with_capacity(verts.capacity());
                for j in 0..=grid {
                    let lat = (90.0 - (k.2 as f64 + j as f64 / grid as f64) * h).clamp(-90.0, 90.0);
                    for i in 0..=grid {
                        let lon = -180.0 + (k.1 as f64 + i as f64 / grid as f64) * h;
                        verts.push(unit(lon, lat));
                        // the south cap comes in over three degrees below its edge
                        fade.push(if lat > 0.0 { 1.0 } else { smooth((SOUTH_FROM - lat) / 3.0) as f32 });
                    }
                }
                return Tile { grid, verts, fade, tex: None, loading: false, failed: false, used: 0 };
            }
            let grid = match k.0 {
                0..=2 => 24,
                3..=5 => 12,
                6..=8 => 6,
                _ => 2,
            };
            let n = (1u64 << k.0) as f64;
            let mut verts = Vec::with_capacity((grid + 1) * (grid + 1));
            for j in 0..=grid {
                let lat = merc_lat((k.2 as f64 + j as f64 / grid as f64) / n);
                for i in 0..=grid {
                    let lon = (k.1 as f64 + i as f64 / grid as f64) / n * 360.0 - 180.0;
                    verts.push(unit(lon, lat));
                }
            }
            Tile { grid, verts, fade: Vec::new(), tex: None, loading: false, failed: false, used: 0 }
        })
    }

    fn index_for(&mut self, grid: usize) -> (WebGlBuffer, i32) {
        if let Some(b) = self.index.get(&grid) {
            return b.clone();
        }
        let mut idx: Vec<u16> = Vec::with_capacity(grid * grid * 6);
        let row = (grid + 1) as u16;
        for j in 0..grid as u16 {
            for i in 0..grid as u16 {
                let a = j * row + i;
                idx.extend_from_slice(&[a, a + row, a + 1, a + 1, a + row, a + row + 1]);
            }
        }
        let buf = self.gl.create_buffer().unwrap();
        self.gl.bind_buffer(GL::ELEMENT_ARRAY_BUFFER, Some(&buf));
        self.gl.buffer_data_with_array_buffer_view(
            GL::ELEMENT_ARRAY_BUFFER,
            &js_sys::Uint16Array::from(idx.as_slice()),
            GL::STATIC_DRAW,
        );
        let out = (buf, idx.len() as i32);
        self.index.insert(grid, out.clone());
        out
    }

    /// The tiles the screen needs, at the zoom that keeps them near 256 px.
    fn wanted(&self, v: &View) -> Vec<Key> {
        let theta = v.cap();
        let mut tz = ((self.z + 0.6).floor() as i32).clamp(0, MAX_TILE_ZOOM as i32) as u8;
        loop {
            let list = self.enumerate(tz, theta);
            if list.len() <= 140 || tz == 0 {
                return list;
            }
            tz -= 1;
        }
    }

    fn enumerate(&self, tz: u8, theta: f64) -> Vec<Key> {
        let n = 1i64 << tz;
        let nf = n as f64;
        let (lon0, lat0) = (self.lon.to_radians(), self.lat.to_radians());
        let y0 = ((merc_y((lat0 + theta).to_degrees().min(89.9)) * nf).floor() as i64).clamp(0, n - 1);
        let y1 = ((merc_y((lat0 - theta).to_degrees().max(-89.9)) * nf).floor() as i64).clamp(0, n - 1);
        let all_lon = lat0.abs() + theta >= PI / 2.0 - 0.02 || theta.sin() >= lat0.cos();
        let (x0, span) = if all_lon {
            (0, n)
        } else {
            let dl = (theta.sin() / lat0.cos()).asin();
            let a = (((lon0 - dl + PI) / (2.0 * PI)) * nf).floor() as i64;
            let b = (((lon0 + dl + PI) / (2.0 * PI)) * nf).floor() as i64;
            (a, (b - a + 1).min(n))
        };
        let margin = 2.0 * PI / nf * 0.8;
        let mut out = Vec::new();
        for y in y0..=y1 {
            let clat = merc_lat((y as f64 + 0.5) / nf);
            for i in 0..span {
                let x = (x0 + i).rem_euclid(n);
                let clon = (x as f64 + 0.5) / nf * 360.0 - 180.0;
                let ang = dot(unit(clon, clat), unit(self.lon, self.lat)).clamp(-1.0, 1.0).acos();
                if ang <= theta + margin {
                    out.push((tz, x as u32, y as u32));
                }
            }
        }
        out
    }

    /// Draw one frame; returns the tiles whose images should start loading.
    fn render(&mut self, t: f64) -> Vec<Key> {
        // camera animation
        if let Some(fl) = &self.fly {
            let k = ((t - fl.t0) / fl.dur).clamp(0.0, 1.0);
            let s = k * k * (3.0 - 2.0 * k);
            let (a, b) = (fl.from.0, fl.to.0);
            let ang = dot(a, b).clamp(-1.0, 1.0).acos();
            // along the great circle between the two points
            let q = if ang < 1e-9 {
                b
            } else {
                let (wa, wb) = (((1.0 - s) * ang).sin() / ang.sin(), (s * ang).sin() / ang.sin());
                norm([a[0] * wa + b[0] * wb, a[1] * wa + b[1] * wb, a[2] * wa + b[2] * wb])
            };
            let dist = (ang.to_degrees() / 40.0).min(1.0);
            let dip = (PI * k).sin() * dist * (fl.from.1.min(fl.to.1) - self.z_min).clamp(0.0, 6.0);
            self.lat = q[1].clamp(-1.0, 1.0).asin().to_degrees();
            if q[0].abs() + q[2].abs() > 1e-12 {
                self.lon = q[0].atan2(q[2]).to_degrees();
            }
            self.z = fl.from.1 + (fl.to.1 - fl.from.1) * s - dip;
            self.hd = fl.from.2 + (fl.to.2 - fl.from.2) * s;
            if k >= 1.0 {
                self.fly = None;
                self.hd = (self.hd + PI).rem_euclid(2.0 * PI) - PI;
            }
            self.clamp();
            self.touch();
        } else {
            let dt = (t - self.last_t).clamp(0.0, 100.0);
            // the turn the hand left the ball with
            if let Some((axis, rate)) = self.spin {
                let v = self.view();
                self.set_frame(turn(axis, rate * dt, v.up), turn(axis, rate * dt, v.north));
                let rate = rate * (-dt / 900.0).exp();
                self.spin = (rate > 4e-6 && self.z < FREE_Z).then_some((axis, rate));
                self.clamp();
                self.touch();
            }
            // coming down to a city the map levels: its plan is drawn north-up
            if self.hd != 0.0 && self.z > FREE_Z {
                let w = smooth((self.z - FREE_Z) / (LEVEL_Z - FREE_Z));
                self.hd *= (-dt / 1000.0 * 10.0 * w).exp();
                if self.z >= LEVEL_Z || self.hd.abs() < 0.0005 {
                    self.hd = 0.0;
                }
                self.dirty = true;
            }
        }
        self.last_t = t;

        // canvas size follows the element
        let (w, h) = self.size();
        let dpr = web_sys::window().map(|w| w.device_pixel_ratio()).unwrap_or(1.0).min(2.0);
        let (pw, ph) = ((w * dpr) as u32, (h * dpr) as u32);
        if self.canvas.width() != pw || self.canvas.height() != ph {
            self.canvas.set_width(pw);
            self.canvas.set_height(ph);
            // the smallest zoom keeps the whole globe on the screen
            let radius = 0.40 * w.min(h);
            let fpx = h / 2.0 / (FOV_Y / 2.0).tan();
            // radius = fpx / sqrt(d² − 1) → d, then ppr = fpx / (d − 1)
            let d = ((fpx / radius).powi(2) + 1.0).sqrt();
            self.z_min = ((fpx / (d - 1.0)) * 2.0 * PI / 256.0).log2();
            if let Some(m) = self.fit.take() {
                self.z = (M_PER_PX_Z0 * w.min(h) / m).log2();
            }
            self.clamp();
            self.dirty = true;
        }

        // idle: put the view into the address once the camera rests
        if !self.url_synced && self.fly.is_none() && t - self.last_move > 500.0 {
            self.url_synced = true;
            if let Some(hist) = web_sys::window().and_then(|w| w.history().ok()) {
                let search = web_sys::window().and_then(|w| w.location().search().ok()).unwrap_or_default();
let turned = if self.hd.abs() > 0.009 { format!(",{:.0}h", self.hd.to_degrees()) } else { String::new() };
                let url = format!("{BASE}/@{:.6},{:.6},{:.2}z{turned}{search}", self.lat, self.lon, self.z);
                let _ = hist.replace_state_with_url(&JsValue::NULL, "", Some(&url));
            }
        }

        if !self.dirty {
            return Vec::new();
        }
        self.dirty = false;
        self.frame += 1;
        let v = self.view();
        let gl = self.gl.clone();
        let (fx, fy) = ((v.f / (v.w / v.h)) as f32, v.f as f32);
        gl.viewport(0, 0, pw as i32, ph as i32);
        gl.disable(GL::DEPTH_TEST);
        gl.disable(GL::BLEND);

        // space, the ocean under the tiles, the glow at the limb
        gl.use_program(Some(&self.bg.p));
        gl.uniform1f(self.bg.loc("u_r"), (v.f / (v.d * v.d - 1.0).sqrt()) as f32);
        gl.uniform1f(self.bg.loc("u_aspect"), (v.w / v.h) as f32);
        gl.bind_vertex_array(Some(&self.bg_vao));
        gl.draw_arrays(GL::TRIANGLE_STRIP, 0, 4);

        // imagery: the north cap under the Mercator pyramid, the south cap over it
        let wanted = self.wanted(&v);
        let (north, south) = self.wanted_polar(&v);
        let mut loads: Vec<Key> = Vec::new();
        gl.use_program(Some(&self.tile_prog.p));
        gl.uniform1f(self.tile_prog.loc("u_fx"), fx);
        gl.uniform1f(self.tile_prog.loc("u_fy"), fy);
        gl.uniform1f(self.tile_prog.loc("u_k"), (v.d - 1.0 / v.d) as f32);
        gl.uniform1f(self.tile_prog.loc("u_h"), (1.0 - 1.0 / v.d) as f32);
        gl.uniform1i(self.tile_prog.loc("u_tex"), 0);
        gl.uniform1f(self.tile_prog.loc("u_tone"), 0.0);
        gl.active_texture(GL::TEXTURE0);
        gl.bind_vertex_array(Some(&self.tile_vao));
        let frame = self.frame;
        let mut scratch: Vec<f32> = Vec::new();
        for k in north.into_iter().chain(wanted) {
            self.draw_tile(&v, k, &mut loads, &mut scratch);
        }
        if !south.is_empty() {
            gl.enable(GL::BLEND);
            gl.blend_func(GL::SRC_ALPHA, GL::ONE_MINUS_SRC_ALPHA);
            gl.uniform1f(self.tile_prog.loc("u_tone"), 1.0);
            for k in south {
                self.draw_tile(&v, k, &mut loads, &mut scratch);
            }
        }
        // the planet's base picture is always asked for
        if self.tile((0, 0, 0)).tex.is_none() && !self.tile((0, 0, 0)).loading {
            loads.push((0, 0, 0));
        }

        // lines
        gl.enable(GL::BLEND);
        gl.blend_func(GL::SRC_ALPHA, GL::ONE_MINUS_SRC_ALPHA);
        gl.use_program(Some(&self.line_prog.p));
        gl.uniform1f(self.line_prog.loc("u_fx"), fx);
        gl.uniform1f(self.line_prog.loc("u_fy"), fy);
        gl.uniform1f(self.line_prog.loc("u_d"), v.d as f32);
        gl.uniform3f(self.line_prog.loc("u_e"), v.e[0] as f32, v.e[1] as f32, v.e[2] as f32);
        gl.uniform3f(self.line_prog.loc("u_n"), v.north[0] as f32, v.north[1] as f32, v.north[2] as f32);
        gl.uniform3f(self.line_prog.loc("u_up"), v.up[0] as f32, v.up[1] as f32, v.up[2] as f32);
        for l in &self.world {
            // the world's lines are a sketch of the planet (1:50m): closer in they would
            // double the exact boundaries beside them, so they fade out by WORLD_LINES_Z
            if self.z < l.min_z || self.z >= WORLD_LINES_Z {
                continue;
            }
            let near = (1.0 - smooth((self.z - (WORLD_LINES_Z - 1.5)) / 1.5)) as f32;
            let fade = ((self.z - l.min_z) / 0.8).clamp(0.0, 1.0) as f32;
            let c = l.color;
            gl.uniform4f(self.line_prog.loc("u_color"), c[0], c[1], c[2], c[3] * near * if l.min_z > 0.0 { fade } else { 1.0 });
            gl.bind_vertex_array(Some(&l.vao));
            gl.draw_arrays(GL::LINES, 0, l.count);
        }
        gl.use_program(Some(&self.vline_prog.p));
        gl.uniform1f(self.vline_prog.loc("u_fx"), fx);
        gl.uniform1f(self.vline_prog.loc("u_fy"), fy);
        gl.uniform1f(self.vline_prog.loc("u_k"), (v.d - 1.0 / v.d) as f32);
        gl.uniform1f(self.vline_prog.loc("u_h"), (1.0 - 1.0 / v.d) as f32);
        gl.bind_vertex_array(Some(&self.vline_vao));
        gl.uniform2f(self.vline_prog.loc("u_off"), 0.0, 0.0);
        for l in &self.cyberia {
            if self.z < l.min_z || self.z > l.max_z || l.pts.is_empty() || !v.visible(l.pts[0]) {
                continue;
            }
            scratch.clear();
            for q in &l.pts {
                let p = v.to_view(*q);
                scratch.extend_from_slice(&[p[0] as f32, p[1] as f32, p[2] as f32]);
            }
            let fade = ((self.z - l.min_z) / 0.8).clamp(0.0, 1.0) as f32;
            gl.bind_buffer(GL::ARRAY_BUFFER, Some(&self.vline_vbo));
            gl.buffer_data_with_array_buffer_view(
                GL::ARRAY_BUFFER,
                &js_sys::Float32Array::from(scratch.as_slice()),
                GL::DYNAMIC_DRAW,
            );
            gl.uniform4f(self.vline_prog.loc("u_color"), l.color[0], l.color[1], l.color[2], l.color[3] * fade);
            gl.draw_arrays(GL::LINES, 0, l.pts.len() as i32);
        }
        let open_a = (1.0 - smooth((self.z - 6.5) / 2.0)) as f32 * 0.7;
        let px = (2.0 / (v.w * dpr) as f32, 2.0 / (v.h * dpr) as f32);
        for (pts, color, bold) in [(&self.open, [0.0f32, 1.0, 0.25, open_a], false), (&self.highlight, [1.0f32, 0.84, 0.0, 1.0], true)] {
            if pts.is_empty() || color[3] <= 0.0 {
                continue;
            }
            scratch.clear();
            for q in pts {
                let p = v.to_view(*q);
                scratch.extend_from_slice(&[p[0] as f32, p[1] as f32, p[2] as f32]);
            }
            gl.bind_buffer(GL::ARRAY_BUFFER, Some(&self.vline_vbo));
            gl.buffer_data_with_array_buffer_view(GL::ARRAY_BUFFER, &js_sys::Float32Array::from(scratch.as_slice()), GL::DYNAMIC_DRAW);
            gl.uniform4f(self.vline_prog.loc("u_color"), color[0], color[1], color[2], color[3]);
            // a line of one pixel drawn five times, one pixel apart, reads as a bold one
            let passes: &[(f32, f32)] = if bold {
                &[(0.0, 0.0), (1.0, 0.0), (-1.0, 0.0), (0.0, 1.0), (0.0, -1.0), (1.0, 1.0), (-1.0, -1.0), (1.0, -1.0), (-1.0, 1.0)]
            } else {
                &[(0.0, 0.0)]
            };
            for (ox, oy) in passes {
                gl.uniform2f(self.vline_prog.loc("u_off"), ox * px.0, oy * px.1);
                gl.draw_arrays(GL::LINES, 0, pts.len() as i32);
            }
        }
        gl.bind_vertex_array(None);

        // markers ride the same camera
        for m in &self.markers {
            let style = m.el.style();
            match v.to_screen(m.q).filter(|_| self.z < SVG_Z) {
                Some((x, y)) => {
                    let _ = style.set_property("display", "flex");
                    let _ = style.set_property("transform", &format!("translate({x:.1}px,{y:.1}px)"));
                }
                None => {
                    let _ = style.set_property("display", "none");
                }
            }
        }

        // forget tiles the screen has not used for a while
        if self.tiles.len() > 420 {
            let old: Vec<Key> = self
                .tiles
                .iter()
                .filter(|(k, tl)| k.0 > 2 && !tl.loading && frame - tl.used > 240)
                .map(|(k, _)| *k)
                .collect();
            for k in old {
                if let Some(tl) = self.tiles.remove(&k) {
                    gl.delete_texture(tl.tex.as_ref());
                }
            }
        }

        (self.on_view)(self.lat, self.lon, self.z, w, h, self.hd);

        // few at a time, the coarse first
        loads.sort();
        loads.dedup();
        let room = 10usize.saturating_sub(self.inflight);
        loads.truncate(room);
        for k in &loads {
            self.tile(*k).loading = true;
        }
        self.inflight += loads.len();
        loads
    }

    fn image_ready(&mut self, k: Key, img: &HtmlImageElement) {
        let gl = self.gl.clone();
        self.inflight = self.inflight.saturating_sub(1);
        let tex = gl.create_texture();
        gl.bind_texture(GL::TEXTURE_2D, tex.as_ref());
        let ok = gl
            .tex_image_2d_with_u32_and_u32_and_html_image_element(
                GL::TEXTURE_2D,
                0,
                GL::RGB as i32,
                GL::RGB,
                GL::UNSIGNED_BYTE,
                img,
            )
            .is_ok();
        gl.generate_mipmap(GL::TEXTURE_2D);
        gl.tex_parameteri(GL::TEXTURE_2D, GL::TEXTURE_MIN_FILTER, GL::LINEAR_MIPMAP_LINEAR as i32);
        gl.tex_parameteri(GL::TEXTURE_2D, GL::TEXTURE_MAG_FILTER, GL::LINEAR as i32);
        gl.tex_parameteri(GL::TEXTURE_2D, GL::TEXTURE_WRAP_S, GL::CLAMP_TO_EDGE as i32);
        gl.tex_parameteri(GL::TEXTURE_2D, GL::TEXTURE_WRAP_T, GL::CLAMP_TO_EDGE as i32);
        let tl = self.tile(k);
        tl.loading = false;
        if ok {
            tl.tex = tex;
        } else {
            tl.failed = true;
        }
        self.dirty = true;
    }

    fn image_failed(&mut self, k: Key) {
        self.inflight = self.inflight.saturating_sub(1);
        let tl = self.tile(k);
        tl.loading = false;
        tl.failed = true;
        self.dirty = true;
    }

    fn add_world_layer(&mut self, lonlat: &[f32], color: [f32; 4], min_z: f64) {
        let mut q: Vec<f32> = Vec::with_capacity(lonlat.len() / 2 * 3);
        for c in lonlat.chunks_exact(2) {
            let u = unit(c[0] as f64, c[1] as f64);
            q.extend_from_slice(&[u[0] as f32, u[1] as f32, u[2] as f32]);
        }
        let gl = &self.gl;
        let vao = gl.create_vertex_array().unwrap();
        gl.bind_vertex_array(Some(&vao));
        let vbo = gl.create_buffer();
        gl.bind_buffer(GL::ARRAY_BUFFER, vbo.as_ref());
        gl.buffer_data_with_array_buffer_view(GL::ARRAY_BUFFER, &js_sys::Float32Array::from(q.as_slice()), GL::STATIC_DRAW);
        gl.enable_vertex_attrib_array(0);
        gl.vertex_attrib_pointer_with_i32(0, 3, GL::FLOAT, false, 0, 0);
        gl.bind_vertex_array(None);
        self.world.push(StaticLayer { vao, count: (q.len() / 3) as i32, color, min_z });
        self.dirty = true;
    }

    fn fly_to(&mut self, lon: f64, lat: f64, z: f64, dur: f64) {
        self.fly = Some(Fly {
            from: (unit(self.lon, self.lat), self.z, self.hd),
            // a flight to a city lands north-up; one over the planet keeps the heading
            to: (unit(lon, lat), z.clamp(self.z_min, Z_MAX), if z > FREE_Z { 0.0 } else { self.hd }),
            t0: now(),
            dur,
        });
        self.spin = None;
        self.dirty = true;
    }

    /// Zoom by dz keeping the point under (sx, sy) where it is.
    fn zoom_at(&mut self, dz: f64, sx: f64, sy: f64) {
        let before = self.view().unproject_q(sx, sy);
        self.z = (self.z + dz).clamp(self.z_min, Z_MAX);
        if let (Some(b), Some(a)) = (before, self.view().unproject_q(sx, sy)) {
            let up = unit(self.lon, self.lat);
            self.move_to([up[0] + b[0] - a[0], up[1] + b[1] - a[1], up[2] + b[2] - a[2]]);
        }
        self.fly = None;
        self.spin = None;
        self.clamp();
        self.touch();
    }

    fn pan_px(&mut self, dx: f64, dy: f64) {
        let v = self.view();
        let ppr = v.fpx / (v.d - 1.0);
        // the ground follows the finger: the camera goes the other way, along the screen's axes
        let (a, b) = ((dx / ppr).clamp(-1.0, 1.0), (dy / ppr).clamp(-1.0, 1.0));
        self.move_to([
            v.up[0] - v.e[0] * a + v.north[0] * b,
            v.up[1] - v.e[1] * a + v.north[1] * b,
            v.up[2] - v.e[2] * a + v.north[2] * b,
        ]);
        self.fly = None;
        self.spin = None;
        self.clamp();
        self.touch();
    }

    /// The polar tiles the screen needs: (north cap, south cap).
    fn wanted_polar(&self, v: &View) -> (Vec<Key>, Vec<Key>) {
        let theta = v.cap();
        let cam = unit(self.lon, self.lat);
        let tz = (self.z + 0.6).floor() as i32;
        let mut out = (Vec::new(), Vec::new());
        for north in [true, false] {
            let reach = if north {
                self.lat + theta.to_degrees() > NORTH_FROM
            } else {
                self.lat - theta.to_degrees() < SOUTH_FROM
            };
            if !reach {
                continue;
            }
            let top = if north { NORTH_ZOOM } else { SOUTH_ZOOM } as i32;
            let mut g = (tz - 1).clamp(GEO_MIN as i32, top) as u8;
            let list = loop {
                let n = 1u32 << g;
                let h = 180.0 / n as f64;
                let margin = h.to_radians() * 0.8;
                let mut list = Vec::new();
                for y in 0..n {
                    let keep = if north { (y as f64) * h < 90.0 - NORTH_FROM } else { 90.0 - (y as f64 + 1.0) * h < SOUTH_FROM };
                    if !keep {
                        continue;
                    }
                    let clat = 90.0 - (y as f64 + 0.5) * h;
                    for x in 0..2 * n {
                        let clon = -180.0 + (x as f64 + 0.5) * h;
                        if dot(unit(clon, clat), cam).clamp(-1.0, 1.0).acos() <= theta + margin {
                            list.push((GEO + g, x, y));
                        }
                    }
                }
                if list.len() <= 120 || g == GEO_MIN {
                    break list;
                }
                g -= 1;
            };
            if north {
                out.0 = list;
            } else {
                out.1 = list;
            }
        }
        out
    }

    /// Draw one tile with the best picture at hand; ask for its own if it is missing.
    fn draw_tile(&mut self, v: &View, k: Key, loads: &mut Vec<Key>, scratch: &mut Vec<f32>) {
        let frame = self.frame;
        let parent = |k: Key| (k.0 != 0 && k.0 != GEO + GEO_MIN).then(|| (k.0 - 1, k.1 >> 1, k.2 >> 1));
        // the deepest ancestor (or the tile itself) that exists on the server
        let mut want = k;
        while !self.covered(want) && want.0 > 0 {
            want = (want.0 - 1, want.1 >> 1, want.2 >> 1);
        }
        {
            let tl = self.tile(want);
            tl.used = frame;
            if tl.tex.is_none() && !tl.loading && !tl.failed {
                loads.push(want);
            }
        }
        // the deepest loaded ancestor carries the picture meanwhile
        let mut src = want;
        let tex = loop {
            if let Some(tx) = self.tiles.get_mut(&src).and_then(|tl| {
                tl.used = frame;
                tl.tex.clone()
            }) {
                break Some(tx);
            }
            match parent(src) {
                Some(p) => src = p,
                None => break None,
            }
        };
        let Some(tex) = tex else { return };
        let up = (k.0 - src.0) as u32;
        let scale = 1.0 / (1u32 << up) as f32;
        let off = ((k.1 & ((1 << up) - 1)) as f32, (k.2 & ((1 << up) - 1)) as f32);
        let grid = {
            let tl = self.tile(k);
            tl.used = frame;
            scratch.clear();
            let g = tl.grid;
            for (i, q) in tl.verts.iter().enumerate() {
                let p = v.to_view(*q);
                scratch.extend_from_slice(&[
                    p[0] as f32,
                    p[1] as f32,
                    p[2] as f32,
                    (i % (g + 1)) as f32 / g as f32,
                    (i / (g + 1)) as f32 / g as f32,
                    tl.fade.get(i).copied().unwrap_or(1.0),
                ]);
            }
            g
        };
        let (ibo, count) = self.index_for(grid);
        let gl = &self.gl;
        gl.bind_buffer(GL::ARRAY_BUFFER, Some(&self.tile_vbo));
        gl.buffer_data_with_array_buffer_view(GL::ARRAY_BUFFER, &js_sys::Float32Array::from(scratch.as_slice()), GL::DYNAMIC_DRAW);
        gl.bind_buffer(GL::ELEMENT_ARRAY_BUFFER, Some(&ibo));
        gl.bind_texture(GL::TEXTURE_2D, Some(&tex));
        gl.uniform3f(self.tile_prog.loc("u_uv"), scale, off.0, off.1);
        gl.draw_elements_with_i32(GL::TRIANGLES, count, GL::UNSIGNED_SHORT, 0);
    }
}

fn start_load(this: &Rc<RefCell<Engine>>, k: Key) {
    let Ok(img) = HtmlImageElement::new() else { return };
    img.set_cross_origin(Some("anonymous"));
    let weak: Weak<RefCell<Engine>> = Rc::downgrade(this);
    let (w2, img2) = (weak.clone(), img.clone());
    img.set_onload(Some(
        Closure::once_into_js(move || {
            if let Some(e) = w2.upgrade() {
                e.borrow_mut().image_ready(k, &img2);
            }
        })
        .unchecked_ref(),
    ));
    img.set_onerror(Some(
        Closure::once_into_js(move || {
            if let Some(e) = weak.upgrade() {
                e.borrow_mut().image_failed(k);
            }
        })
        .unchecked_ref(),
    ));
    if k.0 >= GEO {
        img.set_src(&format!("{POLAR}/{}/{}/{}.jpg", k.0 - GEO, k.1, k.2));
    } else {
        img.set_src(&format!("{TILES}/{}/{}/{}.jpg", k.0, k.1, k.2));
    }
}

async fn fetch_f32(url: String) -> Option<Vec<f32>> {
    let resp = JsFuture::from(web_sys::window()?.fetch_with_str(&url)).await.ok()?;
    let resp: web_sys::Response = resp.dyn_into().ok()?;
    if !resp.ok() {
        return None;
    }
    let buf = JsFuture::from(resp.array_buffer().ok()?).await.ok()?;
    Some(js_sys::Float32Array::new(&buf).to_vec())
}

async fn fetch_text(url: String) -> Option<String> {
    let resp = JsFuture::from(web_sys::window()?.fetch_with_str(&url)).await.ok()?;
    let resp: web_sys::Response = resp.dyn_into().ok()?;
    if !resp.ok() {
        return None;
    }
    JsFuture::from(resp.text().ok()?).await.ok()?.as_string()
}

#[derive(Deserialize)]
struct Manifest {
    ranges: Vec<Range>,
}
#[derive(Deserialize)]
struct Range {
    z: u8,
    x0: u32,
    y0: u32,
    x1: u32,
    y1: u32,
}

async fn fetch_manifest() -> Option<Manifest> {
    let resp = JsFuture::from(web_sys::window()?.fetch_with_str(&format!("{TILES}/manifest.json"))).await.ok()?;
    let resp: web_sys::Response = resp.dyn_into().ok()?;
    let text = JsFuture::from(resp.text().ok()?).await.ok()?.as_string()?;
    serde_json::from_str(&text).ok()
}

fn ring_segments(coords: &[[f64; 2]], close: bool, out: &mut Vec<V3>) {
    for w in coords.windows(2) {
        out.push(unit(w[0][0], w[0][1]));
        out.push(unit(w[1][0], w[1][1]));
    }
    if close && coords.len() > 2 {
        let (a, b) = (coords[coords.len() - 1], coords[0]);
        out.push(unit(a[0], a[1]));
        out.push(unit(b[0], b[1]));
    }
}

fn cyberia_layers() -> Vec<PreciseLayer> {
    let map = load_map();
    let mut districts = Vec::new();
    for d in &map.districts {
        ring_segments(&d.coords, true, &mut districts);
    }
    vec![PreciseLayer { pts: districts, color: [1.0, 0.84, 0.0, 0.85], min_z: 10.5, max_z: SVG_Z }]
}

/// An address of the globe: /@lat,lon,zoomz and, when the view is turned, ,headingh.
fn initial_view() -> Option<(f64, f64, f64, f64)> {
    let path = web_sys::window()?.location().pathname().ok()?;
    let at = path.split("/@").nth(1)?;
    let mut it = at.trim_end_matches('/').split(',');
    let lat: f64 = it.next()?.parse().ok()?;
    let lon: f64 = it.next()?.parse().ok()?;
    let z: f64 = it.next()?.trim_end_matches('z').parse().ok()?;
    let hd: f64 = it.next().and_then(|h| h.trim_end_matches('h').parse().ok()).unwrap_or(0.0);
    Some((lat, lon, z, hd.to_radians()))
}

/// Start the engine on a canvas. `home` is (lon, lat, metres to fit) when the page
/// opens on a place; an address of the form /@lat,lon,zoomz wins over it. Input comes
/// from the page through the `cam_*` functions: the engine listens to nothing itself.
fn start(
    canvas: HtmlCanvasElement,
    overlay: HtmlElement,
    home: Option<(f64, f64, f64)>,
    on_view: Box<dyn Fn(f64, f64, f64, f64, f64, f64)>,
    on_pick: Rc<dyn Fn(usize)>,
) -> Result<u32, String> {
    let gl: GL = canvas
        .get_context("webgl2")
        .ok()
        .flatten()
        .and_then(|c| c.dyn_into().ok())
        .ok_or("this browser has no WebGL2")?;

    let bg = Prog::new(&gl, BG_VS, BG_FS, &["a_pos"], &["u_r", "u_aspect"])?;
    let tile_prog = Prog::new(&gl, TILE_VS, TILE_FS, &["a_pos", "a_uv", "a_fade"], &["u_fx", "u_fy", "u_k", "u_h", "u_uv", "u_tex", "u_tone"])?;
    let line_prog = Prog::new(&gl, LINE_VS, LINE_FS, &["a_pos"], &["u_fx", "u_fy", "u_d", "u_e", "u_n", "u_up", "u_color"])?;
    let vline_prog = Prog::new(&gl, VLINE_VS, LINE_FS, &["a_pos"], &["u_fx", "u_fy", "u_k", "u_h", "u_color", "u_off"])?;

    let bg_vao = gl.create_vertex_array().ok_or("vao")?;
    gl.bind_vertex_array(Some(&bg_vao));
    let quad = gl.create_buffer();
    gl.bind_buffer(GL::ARRAY_BUFFER, quad.as_ref());
    gl.buffer_data_with_array_buffer_view(
        GL::ARRAY_BUFFER,
        &js_sys::Float32Array::from([-1.0f32, -1.0, 1.0, -1.0, -1.0, 1.0, 1.0, 1.0].as_slice()),
        GL::STATIC_DRAW,
    );
    gl.enable_vertex_attrib_array(0);
    gl.vertex_attrib_pointer_with_i32(0, 2, GL::FLOAT, false, 0, 0);

    let tile_vao = gl.create_vertex_array().ok_or("vao")?;
    gl.bind_vertex_array(Some(&tile_vao));
    let tile_vbo = gl.create_buffer().ok_or("vbo")?;
    gl.bind_buffer(GL::ARRAY_BUFFER, Some(&tile_vbo));
    gl.enable_vertex_attrib_array(0);
    gl.vertex_attrib_pointer_with_i32(0, 3, GL::FLOAT, false, 24, 0);
    gl.enable_vertex_attrib_array(1);
    gl.vertex_attrib_pointer_with_i32(1, 2, GL::FLOAT, false, 24, 12);
    gl.enable_vertex_attrib_array(2);
    gl.vertex_attrib_pointer_with_i32(2, 1, GL::FLOAT, false, 24, 20);

    let vline_vao = gl.create_vertex_array().ok_or("vao")?;
    gl.bind_vertex_array(Some(&vline_vao));
    let vline_vbo = gl.create_buffer().ok_or("vbo")?;
    gl.bind_buffer(GL::ARRAY_BUFFER, Some(&vline_vbo));
    gl.enable_vertex_attrib_array(0);
    gl.vertex_attrib_pointer_with_i32(0, 3, GL::FLOAT, false, 0, 0);
    gl.bind_vertex_array(None);

    let doc = web_sys::window().and_then(|w| w.document()).ok_or("no document")?;
    let addressed = initial_view();
    let (lat, lon, z, hd) = addressed.unwrap_or(match home {
        Some((lon, lat, _)) => (lat, lon, 15.0, 0.0),
        None => (12.0, 100.0, 0.0, 0.0),
    });
    let fit = if addressed.is_some() { None } else { home.map(|h| h.2) };
    let id = NEXT_ID.with(|n| {
        n.set(n.get() + 1);
        n.get()
    });

    let engine = Rc::new(RefCell::new(Engine {
        gl,
        canvas: canvas.clone(),
        alive: true,
        lon,
        lat,
        hd,
        spin: None,
        drag_spin: None,
        drag_t: 0.0,
        z,
        z_min: 0.0,
        last_t: 0.0,
        highlight: Vec::new(),
        open: Vec::new(),
        dirty: true,
        frame: 0,
        fly: None,
        bg,
        bg_vao,
        tile_prog,
        tile_vao,
        tile_vbo,
        index: HashMap::new(),
        line_prog,
        vline_prog,
        vline_vao,
        vline_vbo,
        tiles: HashMap::new(),
        inflight: 0,
        regions: Vec::new(),
        world: Vec::new(),
        cyberia: cyberia_layers(),
        markers: Vec::new(),
        admin1_asked: false,
        id,
        fit,
        on_view,
        last_move: 0.0,
        url_synced: true,
    }));

    // city markers: html over the canvas, placed by the camera every frame
    for (i, c) in CITIES.iter().enumerate() {
        let el: HtmlElement = doc.create_element("button").map_err(|_| "marker")?.unchecked_into();
        el.set_class_name("globe-marker live");
        el.set_inner_html(&format!("<span class=\"gm-dot\"></span><span class=\"gm-name\">{}</span>", c.name));
        let (eng, pick) = (engine.clone(), on_pick.clone());
        let click = Closure::<dyn FnMut(web_sys::MouseEvent)>::new(move |ev: web_sys::MouseEvent| {
            ev.stop_propagation();
            let c = &CITIES[i];
            {
                let mut e = eng.borrow_mut();
                let (w, h) = e.size();
                e.fly_to(c.lon, c.lat, (M_PER_PX_Z0 * w.min(h) / c.fit_m).log2(), 2600.0);
            }
            pick(i);
        });
        el.add_event_listener_with_callback("click", click.as_ref().unchecked_ref()).ok();
        click.forget();
        overlay.append_child(&el).ok();
        engine.borrow_mut().markers.push(Marker { q: unit(c.lon, c.lat), el });
    }

    // data: what exists beyond the world pyramid, then the line layers
    {
        let eng = engine.clone();
        wasm_bindgen_futures::spawn_local(async move {
            if let Some(m) = fetch_manifest().await {
                let mut e = eng.borrow_mut();
                e.regions = m.ranges.iter().filter(|r| r.z > WORLD_ZOOM).map(|r| (r.z, r.x0, r.y0, r.x1, r.y1)).collect();
                e.dirty = true;
            }
        });
    }
    for (file, color, min_z) in [
        ("coast", [0.45f32, 1.0, 0.70, 0.55], 0.0),
        ("borders", [0.45f32, 1.0, 0.70, 0.38], 0.0),
    ] {
        let eng = engine.clone();
        wasm_bindgen_futures::spawn_local(async move {
            if let Some(v) = fetch_f32(format!("{BASE}/globe/{file}.bin")).await {
                eng.borrow_mut().add_world_layer(&v, color, min_z);
            }
        });
    }

    // the states whose franchise is open are outlined from the start
    wasm_bindgen_futures::spawn_local(async move {
        let ids: Vec<u64> = countries().await.iter().filter(|c| FRANCHISE_OPEN.contains(&c.c.as_str())).map(|c| c.id).collect();
        let mut open = Vec::new();
        for id in ids {
            if let Some(u) = unit_file(id).await {
                open.extend(outline(&u.p));
            }
        }
        with_engine(|e| {
            e.open = open;
            e.dirty = true;
        });
    });

    // the frame loop
    let cb: Rc<RefCell<Option<Closure<dyn FnMut(f64)>>>> = Rc::new(RefCell::new(None));
    let cb2 = cb.clone();
    let eng = engine.clone();
    *cb.borrow_mut() = Some(Closure::new(move |t: f64| {
        if !eng.borrow().alive || !eng.borrow().canvas.is_connected() {
            cb2.borrow_mut().take();
            return;
        }
        let loads = eng.borrow_mut().render(t);
        for k in loads {
            start_load(&eng, k);
        }
        // the provinces arrive when the camera first comes close enough to read them
        let ask = {
            let mut e = eng.borrow_mut();
            let ask = !e.admin1_asked && e.z > 3.4;
            if ask {
                e.admin1_asked = true;
            }
            ask
        };
        if ask {
            let e2 = eng.clone();
            wasm_bindgen_futures::spawn_local(async move {
                if let Some(v) = fetch_f32(format!("{BASE}/globe/admin1.bin")).await {
                    e2.borrow_mut().add_world_layer(&v, [0.45, 1.0, 0.70, 0.22], 3.6);
                }
            });
        }
        if let Some(c) = cb2.borrow().as_ref() {
            let _ = web_sys::window().unwrap().request_animation_frame(c.as_ref().unchecked_ref());
        }
    }));
    web_sys::window()
        .unwrap()
        .request_animation_frame(cb.borrow().as_ref().unwrap().as_ref().unchecked_ref())
        .map_err(|_| "raf")?;

    ENGINE.with(|g| *g.borrow_mut() = Some(engine));
    Ok(id)
}

fn with_engine(f: impl FnOnce(&mut Engine)) {
    ENGINE.with(|g| {
        if let Some(e) = g.borrow().as_ref() {
            f(&mut e.borrow_mut());
        }
    });
}

// ─── the camera, for the page that hosts the layer ────────────────────

/// Zoom by `dz` levels around a point of the layer (css px), or its centre.
pub fn cam_zoom(dz: f64, at: Option<(f64, f64)>) {
    with_engine(|e| {
        let (w, h) = e.size();
        let (x, y) = at.unwrap_or((w / 2.0, h / 2.0));
        e.zoom_at(dz, x, y);
    });
}

/// The hand on the ball: the point that was under (x0, y0) follows the finger to (x1, y1).
pub fn cam_drag(x0: f64, y0: f64, x1: f64, y1: f64) {
    with_engine(|e| e.drag(x0, y0, x1, y1));
}

/// The hand lets go: the ball keeps the turn it had.
pub fn cam_release() {
    with_engine(|e| e.release());
}

/// A hand on the ball stops it.
pub fn cam_stop() {
    with_engine(|e| {
        e.spin = None;
        e.drag_spin = None;
    });
}

/// Turn the view about its centre by an angle (radians, clockwise), at once.
pub fn cam_twist(da: f64) {
    with_engine(|e| e.twist(da));
}

/// Turn the view about its centre by an angle, smoothly.
pub fn cam_turn(delta: f64) {
    with_engine(|e| {
        if e.z < LEVEL_Z {
            e.turn_by(delta, 420.0);
        }
    });
}

/// North up again, the short way round.
pub fn cam_north() {
    with_engine(|e| {
        let d = -e.hd;
        e.turn_by(d, 600.0);
    });
}

/// Drag the ground by a screen offset.
pub fn cam_pan(dx: f64, dy: f64) {
    with_engine(|e| e.pan_px(dx, dy));
}

/// Fly to a place so that `metres` fit the short side of the layer.
pub fn cam_fit(lon: f64, lat: f64, metres: f64, dur: f64) {
    with_engine(|e| {
        let (w, h) = e.size();
        e.fly_to(lon, lat, (M_PER_PX_Z0 * w.min(h) / metres).log2(), dur);
    });
}

/// Back out to the whole earth.
pub fn cam_world() {
    with_engine(|e| {
        let (lon, lat, z) = (e.lon, e.lat.clamp(-30.0, 30.0), e.z_min + 0.9);
        e.fly_to(lon, lat, z, 1600.0);
    });
}

// ─── divisions: the unit under a point, level by level ────────────────

thread_local! {
    static UNITS: RefCell<HashMap<u64, Rc<UnitFile>>> = RefCell::new(HashMap::new());
    static CELLS: RefCell<HashMap<(u8, u32, u32), Rc<Vec<f64>>>> = RefCell::new(HashMap::new());
    static COUNTRIES: RefCell<Option<Rc<Vec<Country>>>> = const { RefCell::new(None) };
}

async fn countries() -> Rc<Vec<Country>> {
    if let Some(c) = COUNTRIES.with(|c| c.borrow().clone()) {
        return c;
    }
    let list: Vec<Country> = match fetch_text(format!("{DIV}/countries.json")).await {
        Some(t) => serde_json::from_str(&t).unwrap_or_default(),
        None => Vec::new(),
    };
    let list = Rc::new(list);
    if !list.is_empty() {
        COUNTRIES.with(|c| *c.borrow_mut() = Some(list.clone()));
    }
    list
}

async fn unit_file(id: u64) -> Option<Rc<UnitFile>> {
    if let Some(u) = UNITS.with(|m| m.borrow().get(&id).cloned()) {
        return Some(u);
    }
    let text = fetch_text(format!("{DIV}/u/{}/{}/{id}.json", id / 1_000_000, (id % 1_000_000) / 1000)).await?;
    let u = Rc::new(serde_json::from_str::<UnitFile>(&text).ok()?);
    UNITS.with(|m| m.borrow_mut().insert(id, u.clone()));
    Some(u)
}

/// A cell of a level's index: flat [id, min_lon, min_lat, max_lon, max_lat, …].
async fn cell(level: u8, x: u32, y: u32) -> Rc<Vec<f64>> {
    if let Some(c) = CELLS.with(|m| m.borrow().get(&(level, x, y)).cloned()) {
        return c;
    }
    let flat: Vec<f64> = match fetch_text(format!("{DIV}/t/{level}/{x}/{y}.json")).await {
        Some(t) => serde_json::from_str(&t).unwrap_or_default(),
        None => Vec::new(), // no unit of this level here
    };
    let flat = Rc::new(flat);
    CELLS.with(|m| m.borrow_mut().insert((level, x, y), flat.clone()));
    flat
}

fn inside(rings: &[Vec<f64>], lon: f64, lat: f64) -> bool {
    rings.iter().any(|r| {
        let n = r.len() / 2;
        if n < 3 {
            return false;
        }
        let mut c = false;
        let mut j = n - 1;
        for i in 0..n {
            let (xi, yi, xj, yj) = (r[2 * i], r[2 * i + 1], r[2 * j], r[2 * j + 1]);
            if (yi > lat) != (yj > lat) && lon < (xj - xi) * (lat - yi) / (yj - yi) + xi {
                c = !c;
            }
            j = i;
        }
        c
    })
}

/// The area of rings on the sphere, km²: Σ (λ₂ − λ₁)(2 + sin φ₁ + sin φ₂) · R²/2 per ring.
fn area_km2(rings: &[Vec<f64>]) -> f64 {
    const R_KM: f64 = 6371.0088;
    let mut total = 0.0;
    for r in rings {
        let n = r.len() / 2;
        let mut sum = 0.0;
        for a in 0..n {
            let b = (a + 1) % n;
            let mut dl = (r[2 * b] - r[2 * a]).to_radians();
            // a step across the date line is the short way round
            if dl > PI {
                dl -= 2.0 * PI;
            } else if dl < -PI {
                dl += 2.0 * PI;
            }
            sum += dl * (2.0 + r[2 * a + 1].to_radians().sin() + r[2 * b + 1].to_radians().sin());
        }
        total += (sum * R_KM * R_KM / 2.0).abs();
    }
    total
}

fn fmt_area(km2: f64) -> String {
    // thousands apart, so a state and a village both read at a glance
    let group = |v: f64| {
        let s = format!("{:.0}", v);
        let mut out = String::new();
        for (i, ch) in s.chars().enumerate() {
            if i > 0 && (s.len() - i) % 3 == 0 {
                out.push(' ');
            }
            out.push(ch);
        }
        out
    };
    if km2 >= 100.0 {
        format!("{} km²", group(km2))
    } else if km2 >= 1.0 {
        format!("{km2:.1} km²")
    } else {
        format!("{:.1} ha", km2 * 100.0)
    }
}

fn outline(rings: &[Vec<f64>]) -> Vec<V3> {
    let mut out = Vec::new();
    for r in rings {
        let n = r.len() / 2;
        for a in 0..n {
            let b = (a + 1) % n;
            let (x1, y1, x2, y2) = (r[2 * a], r[2 * a + 1], r[2 * b], r[2 * b + 1]);
            // not the seams a source cuts along the date line and at the pole
            if (x1.abs() > 179.99 && x2.abs() > 179.99) || (y1 < -89.9 && y2 < -89.9) {
                continue;
            }
            out.push(unit(x1, y1));
            out.push(unit(x2, y2));
        }
    }
    out
}

/// The point of the earth under a point of the layer (css px): (lon, lat).
pub fn point_at(sx: f64, sy: f64) -> Option<(f64, f64)> {
    ENGINE.with(|g| g.borrow().as_ref().and_then(|e| e.borrow().view().unproject(sx, sy)))
}

/// Every division the point is in, from the state down: `found` is called once per
/// level that has a unit there, in order. Nothing is stored about parents — each
/// level is asked for the unit under the point.
pub async fn locate(lon: f64, lat: f64, found: impl Fn(Unit)) {
    for level in 0u8..6 {
        // candidates: (id, box) whose box holds the point
        let mut cand: Vec<(u64, f64)> = Vec::new();
        if level == 0 {
            for c in countries().await.iter() {
                if lon >= c.b[0] && lon <= c.b[2] && lat >= c.b[1] && lat <= c.b[3] {
                    cand.push((c.id, (c.b[2] - c.b[0]) * (c.b[3] - c.b[1])));
                }
            }
        } else {
            let size = 360.0 / (1u32 << DIV_GRID[level as usize]) as f64;
            let flat = cell(level, ((lon + 180.0) / size) as u32, ((90.0 - lat) / size) as u32).await;
            for u in flat.chunks_exact(5) {
                if lon >= u[1] && lon <= u[3] && lat >= u[2] && lat <= u[4] {
                    cand.push((u[0] as u64, (u[3] - u[1]) * (u[4] - u[2])));
                }
            }
        }
        // the smallest box first: an enclave wins over what surrounds it
        cand.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
        for (id, _) in cand {
            let Some(u) = unit_file(id).await else { continue };
            if inside(&u.p, lon, lat) {
                found(Unit {
                    id,
                    level: u.l,
                    name: u.n.clone(),
                    kind: u.t.clone(),
                    code: u.c.clone(),
                    franchise: FRANCHISE_OPEN.contains(&u.c.as_str()),
                    area_km2: area_km2(&u.p),
                });
                break;
            }
        }
    }
}

/// Outline a unit the page already holds (it came from `locate`).
pub fn show_unit(id: u64) {
    let Some(u) = UNITS.with(|m| m.borrow().get(&id).cloned()) else { return };
    with_engine(|e| {
        e.highlight = outline(&u.p);
        e.dirty = true;
    });
}

/// Drop the outline of the selected unit.
pub fn clear_region() {
    with_engine(|e| {
        e.highlight.clear();
        e.dirty = true;
    });
}

/// The level a tap selects at a zoom: the state from afar, deeper as the camera comes down.
pub fn level_for_zoom(z: f64) -> u8 {
    match z {
        z if z < 4.6 => 0,
        z if z < 6.6 => 1,
        z if z < 8.6 => 2,
        z if z < 10.0 => 3,
        z if z < 11.5 => 4,
        _ => 5,
    }
}

/// Css px per metre on the ground at a zoom.
pub fn px_per_m(z: f64) -> f64 {
    2f64.powf(z) / M_PER_PX_Z0
}

fn fmt_coord(v: f64, pos: char, neg: char) -> String {
    format!("{:.4}°{}", v.abs(), if v >= 0.0 { pos } else { neg })
}

/// What one screen pixel spans on the ground under the camera.
fn ground_scale(z: f64) -> String {
    let m = 1.0 / px_per_m(z);
    if m >= 1000.0 {
        format!("{:.0} km/px", m / 1000.0)
    } else if m >= 1.0 {
        format!("{m:.0} m/px")
    } else {
        format!("{:.1} cm/px", m * 100.0)
    }
}

/// The camera as the page sees it: lat, lon, zoom, layer width and height in css px,
/// and the heading (radians the screen's up is turned clockwise from north).
pub type Cam = (f64, f64, f64, f64, f64, f64);

/// The globe as a layer: a canvas and its markers, filling the positioned parent.
/// It reports the camera into `cam` on every drawn frame.
#[component]
pub fn GlobeLayer(
    cam: RwSignal<Cam>,
    picked: RwSignal<Option<usize>>,
    /// (lon, lat, metres to fit) when the page opens on a place
    home: Option<(f64, f64, f64)>,
) -> impl IntoView {
    let canvas_ref = NodeRef::<leptos::html::Canvas>::new();
    let overlay_ref = NodeRef::<leptos::html::Div>::new();
    let error = RwSignal::new(None::<String>);
    let engine_id = StoredValue::new(0u32);

    Effect::new(move |_| {
        let (Some(canvas), Some(overlay)) = (canvas_ref.get(), overlay_ref.get()) else { return };
        if engine_id.get_value() != 0 {
            return;
        }
        let overlay: HtmlElement = overlay.unchecked_into();
        let res = start(
            canvas,
            overlay,
            home,
            Box::new(move |lat, lon, z, w, h, hd| {
                let _ = cam.try_set((lat, lon, z, w, h, hd));
            }),
            Rc::new(move |i| {
                let _ = picked.try_set(Some(i));
            }),
        );
        match res {
            Ok(id) => engine_id.set_value(id),
            Err(e) => error.set(Some(e)),
        }
    });
    on_cleanup(move || {
        // only this layer's engine: a new page may already have started its own
        let id = engine_id.try_get_value().unwrap_or(0);
        ENGINE.with(|g| {
            let mine = g.borrow().as_ref().map(|e| e.borrow().id == id).unwrap_or(false);
            if mine {
                if let Some(e) = g.borrow_mut().take() {
                    e.borrow_mut().alive = false;
                }
            }
        });
    });

    view! {
        <canvas node_ref=canvas_ref class="globe-canvas"></canvas>
        <div node_ref=overlay_ref class="globe-overlay"></div>
        {move || error.get().map(|e| view! { <div class="globe-error">{e}</div> })}
    }
}

/// What the layer shows over the planet: the cities, the way to found one,
/// the zoom buttons and the readout. The host hides it once the land console is on.
#[component]
pub fn GlobeFar(cam: RwSignal<Cam>, picked: RwSignal<Option<usize>>) -> impl IntoView {
    let founding = RwSignal::new(false);
    let name = RwSignal::new(String::new());

    view! {
        <div class="globe-ui globe-zoom">
            <button type="button" title="zoom in" on:click=move |_| cam_zoom(1.0, None)>"+"</button>
            <button type="button" title="zoom out" on:click=move |_| cam_zoom(-1.0, None)>"−"</button>
            <button type="button" title="the whole earth" on:click=move |_| { picked.set(None); cam_world(); }>"◯"</button>
            <span class="gz-sep"></span>
            <button type="button" title="turn left (Q)" on:click=move |_| cam_turn(-PI / 8.0)>"⟲"</button>
            <button type="button" class="gz-compass" title="north up" on:click=move |_| cam_north()>
                <span class="gz-needle" style=move || format!("transform: rotate({:.3}rad)", cam.get().5)>"▲"</span>
            </button>
            <button type="button" title="turn right (E)" on:click=move |_| cam_turn(PI / 8.0)>"⟳"</button>
        </div>
        <div class="globe-ui globe-hint">"drag: turn the globe · shift-drag or two fingers: twist · wheel or pinch: zoom · tap: select"</div>

        <div class="globe-ui globe-cities">
            <div class="globe-cities-h">"CITIES OF CYBERIA"</div>
            {CITIES.iter().enumerate().map(|(i, c)| view! {
                <button
                    type="button"
                    class=move || if picked.get() == Some(i) { "globe-city on" } else { "globe-city" }
                    on:click=move |_| {
                        // a second tap on the selected city lets it go
                        if picked.get_untracked() == Some(i) {
                            picked.set(None);
                            return;
                        }
                        picked.set(Some(i));
                        founding.set(false);
                        cam_fit(c.lon, c.lat, c.fit_m, 2600.0);
                    }
                >
                    <span class="gc-dot live"></span>
                    <span class="gc-copy">
                        <span class="gc-name">{c.name}</span>
                        <span class="gc-sub">{c.sub}</span>
                    </span>
                </button>
            }).collect_view()}
            <button
                type="button"
                class=move || if founding.get() { "globe-city found on" } else { "globe-city found" }
                on:click=move |_| founding.update(|f| *f = !*f)
            >
                <span class="gc-dot"></span>
                <span class="gc-copy">
                    <span class="gc-name">"Found a city"</span>
                    <span class="gc-sub">"pick a place on the globe"</span>
                </span>
            </button>
            {move || founding.get().then(|| view! {
                <div class="globe-found">
                    <div class="gf-step">"1 · THE PLACE"</div>
                    <div class="gf-note">"move the globe until the crosshair marks it"</div>
                    <div class="gf-coord">
                        {move || {
                            let (lat, lon, ..) = cam.get();
                            format!("{}  {}", fmt_coord(lat, 'N', 'S'), fmt_coord(lon, 'E', 'W'))
                        }}
                    </div>
                    <div class="gf-step">"2 · THE NAME"</div>
                    <input
                        class="gf-input"
                        type="text"
                        maxlength="40"
                        placeholder="name of the city"
                        prop:value=move || name.get()
                        on:input=move |ev| name.set(event_target_value(&ev))
                    />
                    <button type="button" class="gf-commit" disabled=true title="founding is not open yet">
                        "FOUND CITY — SOON"
                    </button>
                    <div class="gf-note">"founding is not open yet · nothing is saved"</div>
                </div>
            })}
        </div>

        {move || founding.get().then(|| view! { <div class="globe-cross"></div> })}

        <div class="globe-ui globe-hud">
            {move || {
let (lat, lon, z, _, _, hd) = cam.get();
                let turned = if hd.abs() > 0.009 { format!("  ·  north {:.0}°", hd.to_degrees().rem_euclid(360.0)) } else { String::new() };
                format!("{}  {}  ·  z {:.1}  ·  {}{turned}", fmt_coord(lat, 'N', 'S'), fmt_coord(lon, 'E', 'W'), z, ground_scale(z))
            }}
        </div>
    }
}

fn level_name(level: u8) -> String {
    // one plain name per level: the sources' own words for a level are uneven
    match level {
        0 => "STATE".to_string(),
        1 => "REGION".to_string(),
        2 => "SUBREGION".to_string(),
        n => format!("LEVEL {n}"),
    }
}

/// A unit's outline as an SVG path fitted into a w × h box (north up, true proportions).
fn unit_path(id: u64, w: f64, h: f64, pad: f64) -> String {
    let Some(u) = UNITS.with(|m| m.borrow().get(&id).cloned()) else { return String::new() };
    let (mut x0, mut y0, mut x1, mut y1) = (f64::MAX, f64::MAX, f64::MIN, f64::MIN);
    for r in &u.p {
        for c in r.chunks_exact(2) {
            x0 = x0.min(c[0]);
            x1 = x1.max(c[0]);
            y0 = y0.min(c[1]);
            y1 = y1.max(c[1]);
        }
    }
    if x0 > x1 {
        return String::new();
    }
    // a degree of longitude is shorter away from the equator
    let kx = ((y0 + y1) * 0.5).to_radians().cos().max(0.05);
    let (sw, sh) = (((x1 - x0) * kx).max(1e-9), (y1 - y0).max(1e-9));
    let k = ((w - 2.0 * pad) / sw).min((h - 2.0 * pad) / sh);
    let (ox, oy) = ((w - sw * k) * 0.5, (h - sh * k) * 0.5);
    let mut d = String::new();
    for r in &u.p {
        for (i, c) in r.chunks_exact(2).enumerate() {
            let (x, y) = (ox + (c[0] - x0) * kx * k, oy + (y1 - c[1]) * k);
            d.push_str(&format!("{}{x:.1},{y:.1}", if i == 0 { "M" } else { "L" }));
        }
        d.push('Z');
    }
    d
}

/// The right panel of a tapped point, on the plan of a plot's panel: the render of
/// the selected division, every level the point is in, the facts and the action.
#[component]
pub fn DivisionPanel(
    /// the tapped point (lat, lon), the divisions it is in from the state down, the selected one
    point: RwSignal<Option<(f64, f64)>>,
    chain: RwSignal<Vec<Unit>>,
    selected: RwSignal<Option<u64>>,
) -> impl IntoView {
    let close = move || {
        clear_region();
        point.set(None);
        chain.set(Vec::new());
        selected.set(None);
    };
    // the division the panel is about: the selected one, else the deepest found
    let current = move || {
        let list = chain.get();
        let sel = selected.get();
        list.iter().find(|u| Some(u.id) == sel).or(list.last()).cloned()
    };
    view! {
        <section class="cyberia-panel cyberia-right division-panel">
            <div class="cyberia-panel-h">
                <span class="panel-kicker">"RENDER"</span>
                <span class="panel-sub">"division · outline"</span>
            </div>
            <div class="render-3d">
                {move || match current() {
                    Some(u) => view! {
                        <div class="render-stage">
                            <div class="plot-slab" style:--prism-color="#ffd400">
                                <svg class="plot-slab-svg" viewBox="0 0 120 120">
                                    <path d=unit_path(u.id, 120.0, 120.0, 14.0) class="plot-slab-path" />
                                </svg>
                            </div>
                            <div class="prism-meta">
                                <div class="prism-name">{u.name.clone()}</div>
                                <div class="prism-sub">
                                    <span style="color:#ffd400">{level_name(u.level)}</span>
                                    {if u.kind.is_empty() { String::new() } else { format!(" · {}", u.kind.to_lowercase()) }}
                                </div>
                                <div class="prism-area">{fmt_area(u.area_km2)}</div>
                            </div>
                        </div>
                    }.into_any(),
                    None => view! { <div class="intent-empty">"looking up the divisions here…"</div> }.into_any(),
                }}
            </div>

            <div class="cyberia-panel-h" style="margin-top: 24px;">
                <span class="panel-kicker">"DIVISION"</span>
                <span class="panel-sub">"tap the globe · pick a level"</span>
                <button class="panel-x" type="button" title="deselect (Esc)" on:click=move |_| close()>"✕"</button>
            </div>
            <div class="intent-form">
                {move || {
                    let list = chain.get();
                    let (lat, lon) = point.get().unwrap_or((0.0, 0.0));
                    let state = list.first().cloned();
                    let franchise = state.as_ref().map(|u| u.franchise).unwrap_or(false);
                    let cur = current();
                    view! {
                        <div class="gr-levels">
                            {list.iter().cloned().map(|u| {
                                let id = u.id;
                                view! {
                                    <button
                                        type="button"
                                        class=move || if selected.get() == Some(id) { "gr-level on" } else { "gr-level" }
                                        style=format!("margin-left:{}px", 10 * u.level as u32)
                                        on:click=move |_| {
                                            // a second tap on the selected level lets it go
                                            if selected.get_untracked() == Some(id) {
                                                selected.set(None);
                                                clear_region();
                                            } else {
                                                selected.set(Some(id));
                                                show_unit(id);
                                            }
                                        }
                                    >
                                        <span class="gl-kind">{level_name(u.level)}</span>
                                        <span class="gl-area">{fmt_area(u.area_km2)}</span>
                                        <span class="gl-name">{u.name.clone()}</span>
                                    </button>
                                }
                            }).collect_view()}
                        </div>
                        <div class="intent-row">
                            <span class="intent-k">"point"</span>
                            <span class="intent-v dim">{format!("{} {}", fmt_coord(lat, 'N', 'S'), fmt_coord(lon, 'E', 'W'))}</span>
                        </div>
                        <div class="intent-row">
                            <span class="intent-k">"state"</span>
                            <span class="intent-v">{state.as_ref().map(|u| u.name.clone()).unwrap_or_else(|| "—".into())}</span>
                        </div>
                        <div class="intent-row">
                            <span class="intent-k">"division"</span>
                            <span class="intent-v">{cur.as_ref().map(|u| u.name.clone()).unwrap_or_else(|| "—".into())}</span>
                        </div>
                        <div class="intent-row">
                            <span class="intent-k">"level"</span>
                            <span class="intent-v dim">{cur.as_ref().map(|u| format!("{} · {} of {}", level_name(u.level).to_lowercase(), u.level as usize + 1, list.len())).unwrap_or_else(|| "—".into())}</span>
                        </div>
                        <div class="intent-row">
                            <span class="intent-k">"area"</span>
                            <span class="intent-v">{cur.as_ref().map(|u| fmt_area(u.area_km2)).unwrap_or_else(|| "—".into())}</span>
                        </div>
                        <div class="intent-row">
                            <span class="intent-k">"share of the level above"</span>
                            <span class="intent-v dim">{cur.as_ref().and_then(|u| {
                                let parent = list.iter().filter(|p| p.level < u.level).next_back()?;
                                (parent.area_km2 > 0.0).then(|| format!("{:.2}% of {}", (u.area_km2 / parent.area_km2 * 100.0).min(100.0), parent.name))
                            }).unwrap_or_else(|| "—".into())}</span>
                        </div>
                        <div class="intent-row">
                            <span class="intent-k">"franchise"</span>
                            <span class="intent-v" style=if franchise { "color:#00ff41" } else { "color:#8a8a8a" }>
                                {if franchise { "OPEN" } else { "NOT OPEN YET" }}
                            </span>
                        </div>
                        <div class="deal-note division-note">
                            {if franchise { "cities of cyberia can be founded here" } else { "today the franchise is open in Indonesia only" }}
                        </div>
                        <button type="button" class="gf-commit division-cta" disabled=true title="founding is not open yet">
                            {if franchise { "FOUND CITY HERE — SOON" } else { "FRANCHISE NOT OPEN HERE" }}
                        </button>
                    }
                }}
            </div>
        </section>
    }
}

/// The credit the imagery licence asks for; shown at every zoom.
#[component]
pub fn GlobeAttrib() -> impl IntoView {
    view! {
        <div class="globe-ui globe-attrib">
            "imagery: Sentinel-2 cloudless 2016 by "
            <a href="https://s2maps.eu" target="_blank" rel="noopener">"EOX"</a>
            " (CC BY 4.0) · Antarctica: Blue Marble by NASA · divisions: "
            <a href="https://www.geoboundaries.org" target="_blank" rel="noopener">"geoBoundaries"</a>
            " · lines: Natural Earth · served by cyberia"
        </div>
    }
}
