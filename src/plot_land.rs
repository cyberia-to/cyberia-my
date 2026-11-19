//! The plan of a flat: the real polygon in its own metric frame, north up,
//! on a 2 m grid, every side that can carry its length signed — what a buyer
//! reads before leasing it.

use crate::land::{centroid, point_in_poly};

/// grid cell edge, metres
pub const CELL_M: f64 = 2.0;
/// SVG units per metre
pub const PX_M: f64 = 10.0;
/// the plan never breathes less than this around the flat, metres
const PAD_M: f64 = 4.0;
/// a sign sits this many fonts outside the line it signs
const SIGN_OFF: f64 = 0.9;
/// one glyph of a sign is this wide, in fonts — Play's digit advance
const SIGN_CH: f64 = 0.45;

/// how wide a sign is set, in SVG units
fn sign_w(text: &str, font: f64) -> f64 {
    text.chars().count() as f64 * SIGN_CH * font
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
    /// the frame's margin in SVG units — wide enough to hold the signs
    pub pad_px: f64,
}

const R_EARTH: f64 = 6_378_137.0;

/// One side of the flat: its length, the middle of its line in SVG units,
/// and the sign the plan sets on it.
#[derive(Clone, Debug, PartialEq)]
pub struct Side {
    pub len_m: f64,
    pub mid: (f64, f64),
    /// unit normal pointing out of the flat, in SVG axes
    pub out: (f64, f64),
    /// the length as the plan writes it, or nothing when the sign would be
    /// wider than the side it signs
    pub sign: Option<String>,
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
            pad_px: 0.0,
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
        f.pad_px = f.sign_reach();
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

    /// The widest the drawing runs, in SVG units — the flat itself, no margin.
    fn span_px(&self) -> f64 {
        (self.max_x - self.min_x).max(self.max_y - self.min_y) * PX_M
    }

    /// The size of a side's sign, in SVG units. It follows the drawing, so a
    /// 5 are flat and a 50 are flat read alike at any size; the floor only
    /// bites under about 30 m across, where the drawing is scaled up anyway.
    pub fn font(&self) -> f64 {
        (self.span_px() / 34.0).max(9.0)
    }

    /// How far outside its line a sign sits, in SVG units.
    pub fn sign_off(&self) -> f64 {
        self.font() * SIGN_OFF
    }

    /// How far beyond the outline the signs reach, in SVG units: their offset
    /// plus half the widest of them east and west, or a line of text north
    /// and south — the margin that must contain them.
    fn sign_reach(&self) -> f64 {
        let font = self.font();
        let widest = self
            .side_lens()
            .into_iter()
            .filter_map(|len_m| Self::sign(len_m, font))
            .map(|t| sign_w(&t, font) / 2.0)
            .fold(0.0, f64::max);
        (self.sign_off() + widest.max(font)).max(PAD_M * PX_M)
    }

    /// SVG frame: metres → viewBox units, north up.
    pub fn svg_w(&self) -> f64 {
        (self.max_x - self.min_x) * PX_M + 2.0 * self.pad_px
    }

    pub fn svg_h(&self) -> f64 {
        (self.max_y - self.min_y) * PX_M + 2.0 * self.pad_px
    }

    pub fn to_svg(&self, x: f64, y: f64) -> (f64, f64) {
        (
            (x - self.min_x) * PX_M + self.pad_px,
            (self.max_y - y) * PX_M + self.pad_px,
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
        self.side_lens().iter().sum()
    }

    /// The length of every side, in ring order, metres.
    fn side_lens(&self) -> Vec<f64> {
        let ring = self.ring_xy();
        let n = ring.len();
        if n < 2 {
            return Vec::new();
        }
        (0..n)
            .map(|i| {
                let (a, b) = (ring[i], ring[(i + 1) % n]);
                (b.0 - a.0).hypot(b.1 - a.1)
            })
            .collect()
    }

    /// The sign of a side of this length, or nothing when the sign would be
    /// wider than the side it signs: a short side stays unsigned rather than
    /// smear its length over its neighbours.
    fn sign(len_m: f64, font: f64) -> Option<String> {
        let text = format!("{len_m:.1} m");
        if sign_w(&text, font) <= len_m * PX_M {
            Some(text)
        } else {
            None
        }
    }

    /// Every side of the flat with its length, for the signed plan.
    pub fn sides(&self) -> Vec<Side> {
        let ring = self.ring_xy();
        let n = ring.len();
        if n < 2 {
            return Vec::new();
        }
        let font = self.font();
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
                    if ccw {
                        (dy / len_m, -dx / len_m)
                    } else {
                        (-dy / len_m, dx / len_m)
                    }
                } else {
                    (0.0, 0.0)
                };
                Side {
                    len_m,
                    mid,
                    out: (nx, -ny),
                    sign: Self::sign(len_m, font),
                }
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
