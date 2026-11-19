//! The plan of a flat: the real polygon in its own metric frame, north up,
//! on a 2 m grid, every side signed with its length — what a buyer reads
//! before leasing it.

use crate::land::{centroid, point_in_poly};

/// grid cell edge, metres
pub const CELL_M: f64 = 2.0;
/// SVG units per metre
pub const PX_M: f64 = 10.0;
const PAD_M: f64 = 4.0;

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

/// One side of the flat: its length and where its label sits, in SVG units.
#[derive(Clone, Debug, PartialEq)]
pub struct Edge {
    pub len_m: f64,
    pub mid: (f64, f64),
    /// unit normal pointing out of the flat, in SVG axes
    pub out: (f64, f64),
}

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

    fn ring_xy(&self) -> Vec<(f64, f64)> {
        let mut v: Vec<(f64, f64)> = self.coords.iter().map(|c| self.to_xy(c[0], c[1])).collect();
        // a closed ring repeats its first vertex; the plan wants each side once
        if v.len() > 1 && v.first() == v.last() {
            v.pop();
        }
        v
    }

    pub fn outline_path(&self) -> String {
        let mut s = String::new();
        for (i, (x, y)) in self.ring_xy().into_iter().enumerate() {
            let (sx, sy) = self.to_svg(x, y);
            s.push_str(if i == 0 { "M" } else { " L" });
            s.push_str(&format!("{sx:.2},{sy:.2}"));
        }
        s.push('Z');
        s
    }

    pub fn perimeter_m(&self) -> f64 {
        self.edges().iter().map(|e| e.len_m).sum()
    }

    /// Every side of the flat with its length, for the signed plan.
    pub fn edges(&self) -> Vec<Edge> {
        let ring = self.ring_xy();
        let n = ring.len();
        if n < 2 {
            return Vec::new();
        }
        // orientation decides which normal points outward
        let mut twice_area = 0.0;
        for i in 0..n {
            let (a, b) = (ring[i], ring[(i + 1) % n]);
            twice_area += a.0 * b.1 - b.0 * a.1;
        }
        let ccw = twice_area > 0.0;
        (0..n)
            .map(|i| {
                let (a, b) = (ring[i], ring[(i + 1) % n]);
                let (dx, dy) = (b.0 - a.0, b.1 - a.1);
                let len_m = (dx * dx + dy * dy).sqrt();
                let mid = self.to_svg((a.0 + b.0) / 2.0, (a.1 + b.1) / 2.0);
                // outward normal in metres (x east, y north), then flip y for SVG
                let (nx, ny) = if len_m > 0.0 {
                    if ccw { (dy / len_m, -dx / len_m) } else { (-dy / len_m, dx / len_m) }
                } else {
                    (0.0, 0.0)
                };
                Edge { len_m, mid, out: (nx, -ny) }
            })
            .collect()
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
