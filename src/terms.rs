//! The lease book: land use, size, holder and price per flat.
//!
//! `scripts/sync-plot-terms.py` builds `plot_terms.json` from two valley
//! sources: the land-use map (plot fill colour = land use) and the plot sheet
//! (type · ares · owner · purpose · price). The map says what the ground is
//! for; the sheet says who holds it and what it costs.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::OnceLock;

const TERMS_JSON: &str = include_str!("plot_terms.json");

/// The city itself — its land is shared, never leased out.
const CITY: &str = "cyber valley";

/// What the ground is for — the land-use map's colour legend, painted in the
/// prysm emotion palette so the map stays acid, not pastel.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LandUse {
    Lease,
    Venture,
    Commons,
    Hgb,
    Dual,
    Special,
    Unclassified,
}

impl LandUse {
    pub const ALL: [LandUse; 7] = [
        LandUse::Lease,
        LandUse::Dual,
        LandUse::Hgb,
        LandUse::Venture,
        LandUse::Commons,
        LandUse::Special,
        LandUse::Unclassified,
    ];

    pub fn from_key(k: &str) -> Self {
        match k {
            "lease" => LandUse::Lease,
            "venture" => LandUse::Venture,
            "commons" => LandUse::Commons,
            "hgb" => LandUse::Hgb,
            "dual" => LandUse::Dual,
            "special" => LandUse::Special,
            _ => LandUse::Unclassified,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            LandUse::Lease => "RESIDENCE",
            LandUse::Venture => "BUSINESS",
            LandUse::Commons => "CITY LAND",
            LandUse::Hgb => "HGB SALE · WHOLE DISTRICT",
            LandUse::Dual => "DUAL USE",
            LandUse::Special => "SPECIAL PLACE",
            LandUse::Unclassified => "UNCLASSIFIED",
        }
    }

    /// legend-sized
    pub fn short(self) -> &'static str {
        match self {
            LandUse::Lease => "RESIDENCE",
            LandUse::Venture => "BUSINESS",
            LandUse::Commons => "CITY LAND",
            LandUse::Hgb => "FOR SALE",
            LandUse::Dual => "DUAL USE",
            LandUse::Special => "SPECIAL",
            LandUse::Unclassified => "UNSET",
        }
    }

    pub fn line(self) -> &'static str {
        match self {
            LandUse::Lease => "a home on a 25-year leasehold",
            LandUse::Venture => "ground for a business",
            LandUse::Commons => "belongs to the city · shared by everyone",
            LandUse::Hgb => "the whole district sold as one HGB title",
            LandUse::Dual => "business and housing on one flat",
            LandUse::Special => "high-energy ground · held by the valley",
            LandUse::Unclassified => "land use not set on the map yet",
        }
    }

    pub fn color(self) -> &'static str {
        match self {
            LandUse::Lease => "#00fe00",
            LandUse::Venture => "#ff5b00",
            LandUse::Commons => "#d500f9",
            LandUse::Hgb => "#00acff",
            LandUse::Dual => "#fcf000",
            LandUse::Special => "#ff0000",
            LandUse::Unclassified => "#4b4b4d",
        }
    }
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq)]
pub struct Terms {
    /// map fill `#rrggbb`
    #[serde(default)]
    pub fill: String,
    #[serde(default)]
    pub land_use: String,
    #[serde(default)]
    pub address: String,
    /// private | community | commons | commerce
    #[serde(default)]
    pub kind: String,
    #[serde(default)]
    pub ares: Option<f64>,
    #[serde(default)]
    pub owner: String,
    #[serde(default)]
    pub purpose: String,
    #[serde(default)]
    pub are_price_usd: Option<f64>,
    #[serde(default)]
    pub plot_price_usd: Option<f64>,
    /// the map's colour when the sheet overruled it (community wins)
    #[serde(default)]
    pub map_use: String,
    /// inside a district sold whole: the map colour marks a special-purpose place
    #[serde(default)]
    pub buyer_mark: String,
    /// e.g. `construction business`
    #[serde(default)]
    pub note: String,
    /// `re-registration` — off sale while the title is being re-issued
    #[serde(default)]
    pub hold: String,
    /// the business niche the valley keeps the flat for — `spa`, `chill`
    #[serde(default)]
    pub niche: String,
    /// surveyed on the ground — outranks the sheet's ares and the drawing
    #[serde(default)]
    pub measured_m2: Option<f64>,
    #[serde(default)]
    pub measured_on: String,
}

impl Terms {
    pub fn land_use(&self) -> LandUse {
        LandUse::from_key(&self.land_use)
    }

    /// Held by a person (the sheet's owner column), the city excluded.
    pub fn holder(&self) -> Option<&str> {
        let o = self.owner.trim();
        (!o.is_empty() && !o.eq_ignore_ascii_case(CITY)).then_some(o)
    }

    /// The sheet sets it aside for the city (commons), or the city holds it.
    pub fn is_city_land(&self) -> bool {
        self.kind == "commons" || self.owner.trim().eq_ignore_ascii_case(CITY)
    }

    /// The sheet's type in the valley's words: private → residence,
    /// community / commerce → business, commons → city land.
    pub fn kind_word(&self) -> &'static str {
        match self.kind.as_str() {
            "private" => "residence",
            "community" | "commerce" => "business",
            "commons" => "city land",
            _ => "",
        }
    }

    pub fn price(&self) -> Option<f64> {
        self.plot_price_usd.filter(|p| *p > 0.0)
    }

    pub fn in_reregistration(&self) -> bool {
        self.hold == "re-registration"
    }

    pub fn is_construction(&self) -> bool {
        self.note == "construction business"
    }

    /// `residence` · `business · spa` · `construction business of the city · laba`
    pub fn kind_line(&self) -> String {
        if !self.note.is_empty() {
            return if self.is_city_land() {
                format!("{} of the city · {}", self.note, self.purpose)
            } else {
                self.note.clone()
            };
        }
        let base = match (self.kind_word(), self.purpose.is_empty()) {
            ("", _) => "not in the plot sheet".to_string(),
            (w, true) => w.to_string(),
            (w, false) => format!("{w} · {}", self.purpose),
        };
        if self.niche.is_empty() {
            base
        } else {
            format!("{base} · {} niche", self.niche)
        }
    }
}

fn book() -> &'static HashMap<String, Terms> {
    static BOOK: OnceLock<HashMap<String, Terms>> = OnceLock::new();
    BOOK.get_or_init(|| serde_json::from_str(TERMS_JSON).unwrap_or_default())
}

pub fn terms(id: &str) -> Option<&'static Terms> {
    book().get(id)
}

pub fn land_use(id: &str) -> LandUse {
    terms(id).map(|t| t.land_use()).unwrap_or(LandUse::Unclassified)
}

/// A whole-district HGB title: every flat of the district, one price.
pub struct DistrictSale {
    pub flats: usize,
    pub ares: f64,
    pub are_price: f64,
}

impl DistrictSale {
    pub fn total(&self) -> f64 {
        self.ares * self.are_price
    }
}

/// `areas` — (flat id, zone, drawn m²) for every flat on the map; the sheet's
/// ares win where the sheet has them.
pub fn district_sale(zone: &str, areas: &[(String, String, f64)]) -> Option<DistrictSale> {
    let mut flats = 0;
    let mut ares = 0.0;
    let mut are_price = None;
    for (id, z, m2) in areas {
        if z != zone {
            continue;
        }
        let Some(t) = terms(id) else { continue };
        if t.land_use() != LandUse::Hgb {
            continue;
        }
        flats += 1;
        ares += t.ares.unwrap_or(m2 / 100.0);
        are_price = are_price.or(t.are_price_usd);
    }
    Some(DistrictSale {
        flats,
        ares,
        are_price: are_price?,
    })
}

/// `48000` → `$48,000`
pub fn fmt_usd(v: f64) -> String {
    let n = v.round() as i64;
    let s = n.abs().to_string();
    let mut out = String::new();
    for (i, ch) in s.chars().enumerate() {
        if i > 0 && (s.len() - i) % 3 == 0 {
            out.push(',');
        }
        out.push(ch);
    }
    format!("{}${out}", if n < 0 { "-" } else { "" })
}

/// `$48,000 · $5,000 / are` — or None when the book has no price.
pub fn price_line(t: &Terms) -> Option<String> {
    let p = t.price()?;
    Some(match t.are_price_usd {
        Some(a) if a > 0.0 => format!("{} · {} / are", fmt_usd(p), fmt_usd(a)),
        _ => fmt_usd(p),
    })
}

/// `1138.24` → `1,138.24`
fn fmt_m2(v: f64) -> String {
    let whole = fmt_usd(v.trunc()).trim_start_matches('$').to_string();
    let frac = ((v - v.trunc()) * 100.0).round() as i64;
    if frac == 0 {
        whole
    } else {
        format!("{whole}.{frac:02}")
    }
}

fn fmt_ares(a: f64) -> String {
    let s = format!("{a:.2}");
    let s = s.trim_end_matches('0').trim_end_matches('.');
    format!("{s} ares")
}

/// The flat's size as one fact, in ares — surveyed first, then the sheet,
/// else the drawn polygon. Previews show only this.
pub fn size_line(t: Option<&Terms>, drawn_m2: f64) -> String {
    match (t.and_then(|t| t.measured_m2), t.and_then(|t| t.ares)) {
        (Some(m), _) => fmt_ares(m / 100.0),
        (None, Some(a)) => fmt_ares(a),
        (None, None) => fmt_ares((drawn_m2 / 10.0).round() / 10.0),
    }
}

/// The flat page's full account of its size. A survey triangulates the
/// slope, so on mountain ground it reads larger than the map, which measures
/// the horizontal projection — the page says both.
pub fn size_detail(t: Option<&Terms>, drawn_m2: f64) -> String {
    match (t.and_then(|t| t.measured_m2), t.and_then(|t| t.ares)) {
        (Some(m), _) => format!(
            "{} · {} m² surveyed on the slope · {drawn_m2:.0} m² in plan on the map",
            fmt_ares(m / 100.0),
            fmt_m2(m)
        ),
        (None, Some(a)) => format!("{} · {:.0} m²", fmt_ares(a), a * 100.0),
        (None, None) => format!("{} · {drawn_m2:.0} m² drawn on the map", fmt_ares((drawn_m2 / 10.0).round() / 10.0)),
    }
}

/// `sinwood-25:@alex_dzin` → `SINWOOD-25` — the KML's inline handle is a
/// drawing note, the lease book says who holds the flat.
pub fn bare_name(name: &str) -> String {
    let head = name.split(":@").next().unwrap_or(name);
    // KML spells a few as `sinwood -24`
    let joined: String = head.split_whitespace().collect::<Vec<_>>().join(" ");
    // `sinwood-1:laba` → `SINWOOD-1 · LABA`
    joined
        .replace(" -", "-")
        .replace("- ", "-")
        .replace(':', " · ")
        .to_uppercase()
}

/// Avalon's special project — the tracks a joint venture there can take.
pub const AVALON_TRACKS: &[&str] = &[
    "education · from age 0 to maturity",
    "health resort",
    "reservoir cascade · aquatics",
];
