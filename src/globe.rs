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

use crate::land::{load_map, FLAG_SVG};
use crate::nav::CyberiaNav;
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
const WORLD_ZOOM: u8 = 7; // the whole earth is present to this zoom
const MAX_TILE_ZOOM: u8 = 14;
const FOV_Y: f64 = 30.0 * PI / 180.0;
const Z_MAX: f64 = 19.5;
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
    fn new(lon: f64, lat: f64, z: f64, w: f64, h: f64) -> Self {
        let (l, p) = (lon.to_radians(), lat.to_radians());
        let f = 1.0 / (FOV_Y / 2.0).tan();
        let fpx = h / 2.0 * f;
        let ppr = 256.0 * 2f64.powf(z) / (2.0 * PI); // css px per radian under the camera
        View {
            e: [l.cos(), 0.0, -l.sin()],
            north: [-p.sin() * l.sin(), p.cos(), -p.sin() * l.cos()],
            up: [p.cos() * l.sin(), p.sin(), p.cos() * l.cos()],
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
    /// The point of the sphere under a screen pixel, as (lon, lat) degrees.
    fn unproject(&self, sx: f64, sy: f64) -> Option<(f64, f64)> {
        let dx = (sx - self.w / 2.0) / self.fpx;
        let dy = (self.h / 2.0 - sy) / self.fpx;
        let len = (dx * dx + dy * dy + 1.0).sqrt();
        let dir = [dx / len, dy / len, -1.0 / len];
        // |t·dir − c|² = 1 with c = (0, 0, −d)
        let b = dir[2] * self.d; // dir·c = −d·dir.z → b = −dir·c
        let disc = b * b - (self.d * self.d - 1.0);
        if disc < 0.0 {
            return None;
        }
        let t = -b - disc.sqrt();
        let hit = [dir[0] * t, dir[1] * t, dir[2] * t + self.d];
        let q = [
            self.e[0] * hit[0] + self.north[0] * hit[1] + self.up[0] * hit[2],
            self.e[1] * hit[0] + self.north[1] * hit[1] + self.up[1] * hit[2],
            self.e[2] * hit[0] + self.north[2] * hit[1] + self.up[2] * hit[2],
        ];
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
in vec3 a_pos; in vec2 a_uv;
uniform float u_fx, u_fy, u_d; uniform vec3 u_uv;
out vec2 v_uv; out float v_vis;
void main(){
  gl_Position = vec4(a_pos.x * u_fx, a_pos.y * u_fy, 0.0, -a_pos.z);
  v_vis = (a_pos.z + u_d - 1.0 / u_d) / (1.0 - 1.0 / u_d);
  v_uv = (a_uv + u_uv.yz) * u_uv.x;
}";
const TILE_FS: &str = "#version 300 es
precision highp float;
in vec2 v_uv; in float v_vis; out vec4 o;
uniform sampler2D u_tex;
void main(){
  if (v_vis < 0.0) discard;
  vec3 c = texture(u_tex, v_uv).rgb;
  c *= 0.45 + 0.55 * smoothstep(0.0, 0.45, v_vis);
  o = vec4(c, 1.0);
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
uniform float u_fx, u_fy, u_d;
out float v_vis;
void main(){
  gl_Position = vec4(a_pos.x * u_fx, a_pos.y * u_fy, 0.0, -a_pos.z);
  v_vis = (a_pos.z + u_d - 1.0 / u_d) / (1.0 - 1.0 / u_d);
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
}

struct Marker {
    q: V3,
    el: HtmlElement,
}

struct Fly {
    from: (f64, f64, f64),
    to: (f64, f64, f64),
    t0: f64,
    dur: f64,
}

pub struct City {
    pub name: &'static str,
    pub sub: &'static str,
    pub lon: f64,
    pub lat: f64,
    pub zoom: f64,
    pub href: &'static str,
    pub live: bool,
}

pub const CITIES: &[City] = &[
    City {
        name: "Cyber Valley",
        sub: "Gesing · Bali · 37 ha · 126 plots",
        lon: 115.0897935,
        lat: -8.2998539,
        zoom: 16.2,
        href: "/map",
        live: true,
    },
    City {
        name: "Cyberia",
        sub: "Armenia · city with a portal · founding",
        lon: 44.515,
        lat: 40.182,
        zoom: 10.5,
        href: "/cities#cyberia",
        live: false,
    },
];

struct Engine {
    gl: GL,
    canvas: HtmlCanvasElement,
    alive: bool,
    lon: f64,
    lat: f64,
    z: f64,
    z_min: f64,
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
    pointers: Vec<(i32, f64, f64)>,
    moved: f64,
    admin1_asked: bool,
    on_view: Box<dyn Fn(f64, f64, f64)>,
    last_report: f64,
    last_move: f64,
    url_synced: bool,
}

thread_local! {
    static ENGINE: RefCell<Option<Rc<RefCell<Engine>>>> = const { RefCell::new(None) };
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
        View::new(self.lon, self.lat, self.z, w, h)
    }
    fn clamp(&mut self) {
        self.lat = self.lat.clamp(-85.0, 85.0);
        self.lon = (self.lon + 180.0).rem_euclid(360.0) - 180.0;
        self.z = self.z.clamp(self.z_min, Z_MAX);
    }
    fn touch(&mut self) {
        self.dirty = true;
        self.last_move = now();
        self.url_synced = false;
    }

    fn covered(&self, k: Key) -> bool {
        k.0 <= WORLD_ZOOM
            || self.regions.iter().any(|r| r.0 == k.0 && k.1 >= r.1 && k.1 <= r.3 && k.2 >= r.2 && k.2 <= r.4)
    }

    fn tile(&mut self, k: Key) -> &mut Tile {
        self.tiles.entry(k).or_insert_with(|| {
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
            Tile { grid, verts, tex: None, loading: false, failed: false, used: 0 }
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
            let mut dlon = fl.to.0 - fl.from.0;
            if dlon > 180.0 {
                dlon -= 360.0;
            } else if dlon < -180.0 {
                dlon += 360.0;
            }
            let dist = ((dlon * dlon + (fl.to.1 - fl.from.1).powi(2)).sqrt() / 40.0).min(1.0);
            let dip = (PI * k).sin() * dist * (fl.from.2.min(fl.to.2) - self.z_min).clamp(0.0, 6.0);
            self.lon = fl.from.0 + dlon * s;
            self.lat = fl.from.1 + (fl.to.1 - fl.from.1) * s;
            self.z = fl.from.2 + (fl.to.2 - fl.from.2) * s - dip;
            if k >= 1.0 {
                self.fly = None;
            }
            self.clamp();
            self.touch();
        }

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
            self.clamp();
            self.dirty = true;
        }

        // idle: put the view into the address once the camera rests
        if !self.url_synced && self.fly.is_none() && t - self.last_move > 500.0 {
            self.url_synced = true;
            if let Some(hist) = web_sys::window().and_then(|w| w.history().ok()) {
                let url = format!("{BASE}/@{:.5},{:.5},{:.2}z", self.lat, self.lon, self.z);
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

        // imagery
        let wanted = self.wanted(&v);
        let mut loads: Vec<Key> = Vec::new();
        gl.use_program(Some(&self.tile_prog.p));
        gl.uniform1f(self.tile_prog.loc("u_fx"), fx);
        gl.uniform1f(self.tile_prog.loc("u_fy"), fy);
        gl.uniform1f(self.tile_prog.loc("u_d"), v.d as f32);
        gl.uniform1i(self.tile_prog.loc("u_tex"), 0);
        gl.active_texture(GL::TEXTURE0);
        gl.bind_vertex_array(Some(&self.tile_vao));
        let frame = self.frame;
        let mut scratch: Vec<f32> = Vec::new();
        for k in wanted {
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
                if src.0 == 0 {
                    break None;
                }
                src = (src.0 - 1, src.1 >> 1, src.2 >> 1);
            };
            let Some(tex) = tex else { continue };
            let up = (k.0 - src.0) as u32;
            let scale = 1.0 / (1u32 << up) as f32;
            let off = ((k.1 & ((1 << up) - 1)) as f32, (k.2 & ((1 << up) - 1)) as f32);
            let (grid, n_verts) = {
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
                    ]);
                }
                (g, tl.verts.len())
            };
            let _ = n_verts;
            let (ibo, count) = self.index_for(grid);
            gl.bind_buffer(GL::ARRAY_BUFFER, Some(&self.tile_vbo));
            gl.buffer_data_with_array_buffer_view(
                GL::ARRAY_BUFFER,
                &js_sys::Float32Array::from(scratch.as_slice()),
                GL::DYNAMIC_DRAW,
            );
            gl.bind_buffer(GL::ELEMENT_ARRAY_BUFFER, Some(&ibo));
            gl.bind_texture(GL::TEXTURE_2D, Some(&tex));
            gl.uniform3f(self.tile_prog.loc("u_uv"), scale, off.0, off.1);
            gl.draw_elements_with_i32(GL::TRIANGLES, count, GL::UNSIGNED_SHORT, 0);
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
            if self.z < l.min_z {
                continue;
            }
            let fade = ((self.z - l.min_z) / 0.8).clamp(0.0, 1.0) as f32;
            let c = l.color;
            gl.uniform4f(self.line_prog.loc("u_color"), c[0], c[1], c[2], c[3] * if l.min_z > 0.0 { fade } else { 1.0 });
            gl.bind_vertex_array(Some(&l.vao));
            gl.draw_arrays(GL::LINES, 0, l.count);
        }
        gl.use_program(Some(&self.vline_prog.p));
        gl.uniform1f(self.vline_prog.loc("u_fx"), fx);
        gl.uniform1f(self.vline_prog.loc("u_fy"), fy);
        gl.uniform1f(self.vline_prog.loc("u_d"), v.d as f32);
        gl.bind_vertex_array(Some(&self.vline_vao));
        for l in &self.cyberia {
            if self.z < l.min_z || l.pts.is_empty() || !v.visible(l.pts[0]) {
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
        gl.bind_vertex_array(None);

        // markers ride the same camera
        for m in &self.markers {
            let style = m.el.style();
            match v.to_screen(m.q) {
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

        if t - self.last_report > 90.0 {
            self.last_report = t;
            (self.on_view)(self.lat, self.lon, self.z);
        }

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
        self.fly = Some(Fly { from: (self.lon, self.lat, self.z), to: (lon, lat, z.clamp(self.z_min, Z_MAX)), t0: now(), dur });
        self.dirty = true;
    }

    /// Zoom by dz keeping the point under (sx, sy) where it is.
    fn zoom_at(&mut self, dz: f64, sx: f64, sy: f64) {
        let before = self.view().unproject(sx, sy);
        self.z = (self.z + dz).clamp(self.z_min, Z_MAX);
        if let (Some(b), Some(a)) = (before, self.view().unproject(sx, sy)) {
            let mut dlon = b.0 - a.0;
            if dlon > 180.0 {
                dlon -= 360.0;
            } else if dlon < -180.0 {
                dlon += 360.0;
            }
            self.lon += dlon;
            self.lat += b.1 - a.1;
        }
        self.fly = None;
        self.clamp();
        self.touch();
    }

    fn pan_px(&mut self, dx: f64, dy: f64) {
        let v = self.view();
        let ppr = v.fpx / (v.d - 1.0);
        self.lon -= (dx / ppr).to_degrees() / self.lat.to_radians().cos().max(0.08);
        self.lat += (dy / ppr).to_degrees();
        self.fly = None;
        self.clamp();
        self.touch();
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
    img.set_src(&format!("{TILES}/{}/{}/{}.jpg", k.0, k.1, k.2));
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

#[derive(Deserialize)]
struct TrailLine {
    coords: Vec<[f64; 2]>,
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
    let mut plots = Vec::new();
    for p in &map.phase0 {
        ring_segments(&p.coords, true, &mut plots);
    }
    let mut districts = Vec::new();
    for d in &map.districts {
        ring_segments(&d.coords, true, &mut districts);
    }
    let mut trails = Vec::new();
    let lines: Vec<TrailLine> = serde_json::from_str(include_str!("cyberia_trails.json")).unwrap_or_default();
    for t in &lines {
        ring_segments(&t.coords, false, &mut trails);
    }
    vec![
        PreciseLayer { pts: trails, color: [1.0, 1.0, 1.0, 0.45], min_z: 13.5 },
        PreciseLayer { pts: districts, color: [1.0, 0.84, 0.0, 0.85], min_z: 11.5 },
        PreciseLayer { pts: plots, color: [0.0, 1.0, 0.25, 0.95], min_z: 13.0 },
    ]
}

fn initial_view() -> Option<(f64, f64, f64)> {
    let path = web_sys::window()?.location().pathname().ok()?;
    let at = path.split("/@").nth(1)?;
    let mut it = at.trim_end_matches('/').split(',');
    let lat: f64 = it.next()?.parse().ok()?;
    let lon: f64 = it.next()?.parse().ok()?;
    let z: f64 = it.next()?.trim_end_matches('z').parse().ok()?;
    Some((lat, lon, z))
}

fn start(
    canvas: HtmlCanvasElement,
    overlay: HtmlElement,
    on_view: Box<dyn Fn(f64, f64, f64)>,
    on_pick: Rc<dyn Fn(usize)>,
) -> Result<(), String> {
    let gl: GL = canvas
        .get_context("webgl2")
        .ok()
        .flatten()
        .and_then(|c| c.dyn_into().ok())
        .ok_or("this browser has no WebGL2")?;

    let bg = Prog::new(&gl, BG_VS, BG_FS, &["a_pos"], &["u_r", "u_aspect"])?;
    let tile_prog = Prog::new(&gl, TILE_VS, TILE_FS, &["a_pos", "a_uv"], &["u_fx", "u_fy", "u_d", "u_uv", "u_tex"])?;
    let line_prog = Prog::new(&gl, LINE_VS, LINE_FS, &["a_pos"], &["u_fx", "u_fy", "u_d", "u_e", "u_n", "u_up", "u_color"])?;
    let vline_prog = Prog::new(&gl, VLINE_VS, LINE_FS, &["a_pos"], &["u_fx", "u_fy", "u_d", "u_color"])?;

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
    gl.vertex_attrib_pointer_with_i32(0, 3, GL::FLOAT, false, 20, 0);
    gl.enable_vertex_attrib_array(1);
    gl.vertex_attrib_pointer_with_i32(1, 2, GL::FLOAT, false, 20, 12);

    let vline_vao = gl.create_vertex_array().ok_or("vao")?;
    gl.bind_vertex_array(Some(&vline_vao));
    let vline_vbo = gl.create_buffer().ok_or("vbo")?;
    gl.bind_buffer(GL::ARRAY_BUFFER, Some(&vline_vbo));
    gl.enable_vertex_attrib_array(0);
    gl.vertex_attrib_pointer_with_i32(0, 3, GL::FLOAT, false, 0, 0);
    gl.bind_vertex_array(None);

    let doc = web_sys::window().and_then(|w| w.document()).ok_or("no document")?;
    let (lat, lon, z) = initial_view().unwrap_or((12.0, 100.0, 0.0));

    let engine = Rc::new(RefCell::new(Engine {
        gl,
        canvas: canvas.clone(),
        alive: true,
        lon,
        lat,
        z,
        z_min: 0.0,
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
        pointers: Vec::new(),
        moved: 0.0,
        admin1_asked: false,
        on_view,
        last_report: 0.0,
        last_move: 0.0,
        url_synced: true,
    }));

    // city markers: html over the canvas, placed by the camera every frame
    for (i, c) in CITIES.iter().enumerate() {
        let el: HtmlElement = doc.create_element("button").map_err(|_| "marker")?.unchecked_into();
        el.set_class_name(if c.live { "globe-marker live" } else { "globe-marker" });
        el.set_inner_html(&format!("<span class=\"gm-dot\"></span><span class=\"gm-name\">{}</span>", c.name));
        let (eng, pick) = (engine.clone(), on_pick.clone());
        let click = Closure::<dyn FnMut(web_sys::MouseEvent)>::new(move |ev: web_sys::MouseEvent| {
            ev.stop_propagation();
            let c = &CITIES[i];
            eng.borrow_mut().fly_to(c.lon, c.lat, c.zoom, 2600.0);
            pick(i);
        });
        el.add_event_listener_with_callback("click", click.as_ref().unchecked_ref()).ok();
        click.forget();
        overlay.append_child(&el).ok();
        engine.borrow_mut().markers.push(Marker { q: unit(c.lon, c.lat), el });
    }

    // input
    {
        let eng = engine.clone();
        let cv = canvas.clone();
        let down = Closure::<dyn FnMut(web_sys::PointerEvent)>::new(move |ev: web_sys::PointerEvent| {
            let _ = cv.set_pointer_capture(ev.pointer_id());
            let mut e = eng.borrow_mut();
            e.pointers.push((ev.pointer_id(), ev.offset_x() as f64, ev.offset_y() as f64));
            e.moved = 0.0;
            e.fly = None;
        });
        canvas.add_event_listener_with_callback("pointerdown", down.as_ref().unchecked_ref()).ok();
        down.forget();
    }
    {
        let eng = engine.clone();
        let mv = Closure::<dyn FnMut(web_sys::PointerEvent)>::new(move |ev: web_sys::PointerEvent| {
            let mut e = eng.borrow_mut();
            let (x, y) = (ev.offset_x() as f64, ev.offset_y() as f64);
            let Some(i) = e.pointers.iter().position(|p| p.0 == ev.pointer_id()) else { return };
            let (px, py) = (e.pointers[i].1, e.pointers[i].2);
            if e.pointers.len() == 1 {
                e.moved += (x - px).abs() + (y - py).abs();
                e.pan_px(x - px, y - py);
            } else if e.pointers.len() == 2 {
                let o = e.pointers[1 - i];
                let before = ((px - o.1).powi(2) + (py - o.2).powi(2)).sqrt().max(1.0);
                let after = ((x - o.1).powi(2) + (y - o.2).powi(2)).sqrt().max(1.0);
                e.zoom_at((after / before).log2(), (x + o.1) / 2.0, (y + o.2) / 2.0);
                e.pan_px((x - px) / 2.0, (y - py) / 2.0);
            }
            e.pointers[i] = (ev.pointer_id(), x, y);
        });
        canvas.add_event_listener_with_callback("pointermove", mv.as_ref().unchecked_ref()).ok();
        mv.forget();
    }
    for name in ["pointerup", "pointercancel"] {
        let eng = engine.clone();
        let up = Closure::<dyn FnMut(web_sys::PointerEvent)>::new(move |ev: web_sys::PointerEvent| {
            eng.borrow_mut().pointers.retain(|p| p.0 != ev.pointer_id());
        });
        canvas.add_event_listener_with_callback(name, up.as_ref().unchecked_ref()).ok();
        up.forget();
    }
    {
        let eng = engine.clone();
        let wheel = Closure::<dyn FnMut(web_sys::WheelEvent)>::new(move |ev: web_sys::WheelEvent| {
            ev.prevent_default();
            let k = if ev.ctrl_key() { 0.012 } else { 0.0028 };
            let dz = (-ev.delta_y() * k).clamp(-1.2, 1.2);
            eng.borrow_mut().zoom_at(dz, ev.offset_x() as f64, ev.offset_y() as f64);
        });
        let opts = web_sys::AddEventListenerOptions::new();
        opts.set_passive(false);
        canvas
            .add_event_listener_with_callback_and_add_event_listener_options("wheel", wheel.as_ref().unchecked_ref(), &opts)
            .ok();
        wheel.forget();
    }
    {
        let eng = engine.clone();
        let dbl = Closure::<dyn FnMut(web_sys::MouseEvent)>::new(move |ev: web_sys::MouseEvent| {
            let mut e = eng.borrow_mut();
            if let Some((lon, lat)) = e.view().unproject(ev.offset_x() as f64, ev.offset_y() as f64) {
                let z = e.z + 1.6;
                e.fly_to(lon, lat, z, 700.0);
            }
        });
        canvas.add_event_listener_with_callback("dblclick", dbl.as_ref().unchecked_ref()).ok();
        dbl.forget();
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
    Ok(())
}

fn with_engine(f: impl FnOnce(&mut Engine)) {
    ENGINE.with(|g| {
        if let Some(e) = g.borrow().as_ref() {
            f(&mut e.borrow_mut());
        }
    });
}

// ─── the page ─────────────────────────────────────────────────────────

fn fmt_coord(v: f64, pos: char, neg: char) -> String {
    format!("{:.4}°{}", v.abs(), if v >= 0.0 { pos } else { neg })
}

/// What one screen pixel spans on the ground under the camera.
fn ground_scale(lat: f64, z: f64) -> String {
    let m = 40_075_016.7 * lat.to_radians().cos() / (256.0 * 2f64.powf(z));
    if m >= 1000.0 {
        format!("{:.0} km/px", m / 1000.0)
    } else if m >= 1.0 {
        format!("{m:.0} m/px")
    } else {
        format!("{:.0} cm/px", m * 100.0)
    }
}

#[component]
pub fn GlobePage() -> impl IntoView {
    let canvas_ref = NodeRef::<leptos::html::Canvas>::new();
    let overlay_ref = NodeRef::<leptos::html::Div>::new();
    let hud = RwSignal::new((12.0f64, 100.0f64, 0.0f64));
    let picked = RwSignal::new(None::<usize>);
    let error = RwSignal::new(None::<String>);
    let started = StoredValue::new(false);

    Effect::new(move |_| {
        document().set_title("Cyberia — the globe");
        let (Some(canvas), Some(overlay)) = (canvas_ref.get(), overlay_ref.get()) else { return };
        if started.get_value() {
            return;
        }
        started.set_value(true);
        let overlay: HtmlElement = overlay.unchecked_into();
        let res = start(
            canvas,
            overlay,
            Box::new(move |lat, lon, z| hud.set((lat, lon, z))),
            Rc::new(move |i| picked.set(Some(i))),
        );
        if let Err(e) = res {
            error.set(Some(e));
        }
    });
    on_cleanup(|| {
        with_engine(|e| e.alive = false);
        ENGINE.with(|g| g.borrow_mut().take());
    });

    let near_valley = move || {
        let (lat, lon, z) = hud.get();
        z > 13.0 && (lat + 8.2998).abs() < 0.05 && (lon - 115.0898).abs() < 0.05
    };

    view! {
        <div class="globe-shell">
            <canvas node_ref=canvas_ref class="globe-canvas"></canvas>
            <div node_ref=overlay_ref class="globe-overlay"></div>

            <div class="site-chrome cyberia-chrome globe-chrome">
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
                            {format!("{} CITIES", CITIES.len())}
                        </div>
                        <CyberiaNav active="" />
                    </div>
                </div>
            </div>

            {move || error.get().map(|e| view! { <div class="globe-error">{e}</div> })}

            <div class="globe-zoom">
                <button type="button" title="zoom in" on:click=move |_| with_engine(|e| { let (w, h) = e.size(); e.zoom_at(1.0, w / 2.0, h / 2.0); })>"+"</button>
                <button type="button" title="zoom out" on:click=move |_| with_engine(|e| { let (w, h) = e.size(); e.zoom_at(-1.0, w / 2.0, h / 2.0); })>"−"</button>
                <button type="button" title="the whole earth" on:click=move |_| { picked.set(None); with_engine(|e| { let (lon, lat, z) = (e.lon, e.lat.clamp(-30.0, 30.0), e.z_min + 0.9); e.fly_to(lon, lat, z, 1600.0); }); }>"◯"</button>
            </div>

            <div class="globe-cities">
                <div class="globe-cities-h">"CITIES OF CYBERIA"</div>
                {CITIES.iter().enumerate().map(|(i, c)| view! {
                    <button
                        type="button"
                        class=move || if picked.get() == Some(i) { "globe-city on" } else { "globe-city" }
                        on:click=move |_| {
                            picked.set(Some(i));
                            with_engine(|e| e.fly_to(c.lon, c.lat, c.zoom, 2600.0));
                        }
                    >
                        <span class=if c.live { "gc-dot live" } else { "gc-dot" }></span>
                        <span class="gc-copy">
                            <span class="gc-name">{c.name}</span>
                            <span class="gc-sub">{c.sub}</span>
                        </span>
                    </button>
                }).collect_view()}
                {move || picked.get().map(|i| {
                    let c = &CITIES[i];
                    let href = if c.href.starts_with('/') { format!("{BASE}{}", c.href) } else { c.href.to_string() };
                    view! {
                        <a class="globe-open" href=href>
                            {if c.live { "OPEN THE LAND CONSOLE →" } else { "ABOUT THIS CITY →" }}
                        </a>
                    }
                })}
            </div>

            {move || near_valley().then(|| view! {
                <a class="globe-enter" href=format!("{BASE}/map")>
                    <span class="ge-title">"CYBER VALLEY"</span>
                    <span class="ge-sub">"plots · leases · fleets — open the land console →"</span>
                </a>
            })}

            <div class="globe-hud">
                {move || {
                    let (lat, lon, z) = hud.get();
                    format!("{}  {}  ·  z {:.1}  ·  {}", fmt_coord(lat, 'N', 'S'), fmt_coord(lon, 'E', 'W'), z, ground_scale(lat, z))
                }}
            </div>
            <div class="globe-attrib">
                "imagery: Sentinel-2 cloudless 2016 by "
                <a href="https://s2maps.eu" target="_blank" rel="noopener">"EOX"</a>
                " (CC BY 4.0) · lines: Natural Earth · served by cyberia"
            </div>
        </div>
    }
}
