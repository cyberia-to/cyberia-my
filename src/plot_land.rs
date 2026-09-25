//! The land board of a flat: the real polygon on a 2 m grid, in metres.
//!
//! Three live tools — MEASURE, PRUNING, BUILD UNIT (2×2×2 m shower+toilet
//! cube). Every other figure from the cyberia-map catalog can be set down on
//! the grid to see how it sits, but build stays locked until phase 1.

use crate::land::{centroid, point_in_poly};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// grid cell edge, metres
pub const CELL_M: f64 = 2.0;
/// SVG units per metre
pub const PX_M: f64 = 10.0;
const PAD_M: f64 = 3.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Figure {
    pub id: &'static str,
    pub name: &'static str,
    pub sub: &'static str,
    pub cols: i32,
    pub rows: i32,
    /// phase-0 buildable — otherwise place-only, grey
    pub live: bool,
}

pub const FIGURES: &[Figure] = &[
    Figure {
        id: "unit",
        name: "UNIT-2",
        sub: "shower + toilet · 2×2×2 m",
        cols: 1,
        rows: 1,
        live: true,
    },
    Figure {
        id: "cube",
        name: "CUBE-4",
        sub: "4×4×4 m cell",
        cols: 2,
        rows: 2,
        live: false,
    },
    Figure {
        id: "tube",
        name: "TUBE",
        sub: "arch · 2×4 m",
        cols: 1,
        rows: 2,
        live: false,
    },
    Figure {
        id: "prysm",
        name: "PRYSM",
        sub: "A-frame · 4×2 m base",
        cols: 2,
        rows: 1,
        live: false,
    },
    Figure {
        id: "pyramid",
        name: "PYRAMID",
        sub: "4×4×4 m",
        cols: 2,
        rows: 2,
        live: false,
    },
    Figure {
        id: "sphere",
        name: "SPHERE",
        sub: "⌀4 m",
        cols: 2,
        rows: 2,
        live: false,
    },
];

pub fn figure(id: &str) -> Option<&'static Figure> {
    FIGURES.iter().find(|f| f.id == id)
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Placed {
    pub fig: String,
    pub c: i32,
    pub r: i32,
    #[serde(default)]
    pub built: bool,
}

/// Everything the owner did on this flat's board — this browser only.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct PlotState {
    #[serde(default)]
    pub pruned: Vec<(i32, i32)>,
    #[serde(default)]
    pub placed: Vec<Placed>,
}

fn state_key(plot_id: &str) -> String {
    format!("cyberia.plot.{plot_id}.v1")
}

pub fn load_state(plot_id: &str) -> PlotState {
    web_sys::window()
        .and_then(|w| w.local_storage().ok().flatten())
        .and_then(|ls| ls.get_item(&state_key(plot_id)).ok().flatten())
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

pub fn save_state(plot_id: &str, st: &PlotState) {
    if let Some(ls) = web_sys::window().and_then(|w| w.local_storage().ok().flatten()) {
        if let Ok(s) = serde_json::to_string(st) {
            let _ = ls.set_item(&state_key(plot_id), &s);
        }
    }
}

/// The flat in its own metric frame: x east, y north, origin at the centroid.
#[derive(Clone, Debug, PartialEq)]
pub struct Frame {
    pub coords: Vec<[f64; 2]>,
    pub lon0: f64,
    pub lat0: f64,
    pub cos_lat: f64,
    pub min_x: f64,
    pub max_x: f64,
    pub min_y: f64,
    pub max_y: f64,
    /// grid origin (metres) — cell (0,0) starts here
    pub gx0: f64,
    pub gy0: f64,
    pub cols: i32,
    pub rows: i32,
}

const R_EARTH: f64 = 6_378_137.0;

impl Frame {
    pub fn new(coords: &[[f64; 2]]) -> Self {
        let (lon0, lat0) = centroid(coords).unwrap_or((0.0, 0.0));
        let cos_lat = lat0.to_radians().cos();
        let mut f = Frame {
            coords: coords.to_vec(),
            lon0,
            lat0,
            cos_lat,
            min_x: f64::INFINITY,
            max_x: f64::NEG_INFINITY,
            min_y: f64::INFINITY,
            max_y: f64::NEG_INFINITY,
            gx0: 0.0,
            gy0: 0.0,
            cols: 0,
            rows: 0,
        };
        for c in coords {
            let (x, y) = f.to_xy(c[0], c[1]);
            f.min_x = f.min_x.min(x);
            f.max_x = f.max_x.max(x);
            f.min_y = f.min_y.min(y);
            f.max_y = f.max_y.max(y);
        }
        f.gx0 = (f.min_x / CELL_M).floor() * CELL_M;
        f.gy0 = (f.min_y / CELL_M).floor() * CELL_M;
        f.cols = ((f.max_x - f.gx0) / CELL_M).ceil() as i32;
        f.rows = ((f.max_y - f.gy0) / CELL_M).ceil() as i32;
        f
    }

    pub fn to_xy(&self, lon: f64, lat: f64) -> (f64, f64) {
        (
            (lon - self.lon0).to_radians() * R_EARTH * self.cos_lat,
            (lat - self.lat0).to_radians() * R_EARTH,
        )
    }

    pub fn to_lonlat(&self, x: f64, y: f64) -> (f64, f64) {
        (
            self.lon0 + (x / (R_EARTH * self.cos_lat)).to_degrees(),
            self.lat0 + (y / R_EARTH).to_degrees(),
        )
    }

    pub fn inside(&self, x: f64, y: f64) -> bool {
        let (lon, lat) = self.to_lonlat(x, y);
        point_in_poly(lon, lat, &self.coords)
    }

    pub fn cell_center(&self, c: i32, r: i32) -> (f64, f64) {
        (
            self.gx0 + (c as f64 + 0.5) * CELL_M,
            self.gy0 + (r as f64 + 0.5) * CELL_M,
        )
    }

    pub fn cell_in_plot(&self, c: i32, r: i32) -> bool {
        let (x, y) = self.cell_center(c, r);
        self.inside(x, y)
    }

    pub fn cell_at(&self, x: f64, y: f64) -> (i32, i32) {
        (
            ((x - self.gx0) / CELL_M).floor() as i32,
            ((y - self.gy0) / CELL_M).floor() as i32,
        )
    }

    /// SVG frame: metres → viewBox units, north up.
    pub fn svg_w(&self) -> f64 {
        (self.max_x - self.min_x + 2.0 * PAD_M) * PX_M
    }

    pub fn svg_h(&self) -> f64 {
        (self.max_y - self.min_y + 2.0 * PAD_M) * PX_M
    }

    pub fn to_svg(&self, x: f64, y: f64) -> (f64, f64) {
        (
            (x - self.min_x + PAD_M) * PX_M,
            (self.max_y - y + PAD_M) * PX_M,
        )
    }

    pub fn from_svg(&self, sx: f64, sy: f64) -> (f64, f64) {
        (
            sx / PX_M + self.min_x - PAD_M,
            self.max_y + PAD_M - sy / PX_M,
        )
    }

    pub fn outline_path(&self) -> String {
        let mut s = String::new();
        for (i, c) in self.coords.iter().enumerate() {
            let (x, y) = self.to_xy(c[0], c[1]);
            let (sx, sy) = self.to_svg(x, y);
            s.push_str(if i == 0 { "M" } else { " L" });
            s.push_str(&format!("{sx:.2},{sy:.2}"));
        }
        s.push('Z');
        s
    }

    pub fn perimeter_m(&self) -> f64 {
        let n = self.coords.len();
        if n < 2 {
            return 0.0;
        }
        let mut p = 0.0;
        for i in 0..n {
            let a = self.to_xy(self.coords[i][0], self.coords[i][1]);
            let b = self.to_xy(self.coords[(i + 1) % n][0], self.coords[(i + 1) % n][1]);
            p += ((a.0 - b.0).powi(2) + (a.1 - b.1).powi(2)).sqrt();
        }
        p
    }

    /// Cells whose centre is on the flat.
    pub fn cells(&self) -> Vec<(i32, i32)> {
        let mut v = Vec::new();
        for r in 0..self.rows {
            for c in 0..self.cols {
                if self.cell_in_plot(c, r) {
                    v.push((c, r));
                }
            }
        }
        v
    }
}

/// Cells a figure would take when anchored at (c, r).
pub fn footprint_cells(fig: &Figure, c: i32, r: i32) -> Vec<(i32, i32)> {
    let mut v = Vec::with_capacity((fig.cols * fig.rows) as usize);
    for dr in 0..fig.rows {
        for dc in 0..fig.cols {
            v.push((c + dc, r + dr));
        }
    }
    v
}

pub fn occupied(placed: &[Placed]) -> HashSet<(i32, i32)> {
    let mut s = HashSet::new();
    for p in placed {
        if let Some(f) = figure(&p.fig) {
            s.extend(footprint_cells(f, p.c, p.r));
        }
    }
    s
}

/// Fits = every cell on the flat and none already taken.
pub fn fits(frame: &Frame, placed: &[Placed], fig: &Figure, c: i32, r: i32) -> bool {
    let taken = occupied(placed);
    footprint_cells(fig, c, r)
        .into_iter()
        .all(|(cc, rr)| frame.cell_in_plot(cc, rr) && !taken.contains(&(cc, rr)))
}

pub fn dist_m(a: (f64, f64), b: (f64, f64)) -> f64 {
    ((a.0 - b.0).powi(2) + (a.1 - b.1).powi(2)).sqrt()
}
