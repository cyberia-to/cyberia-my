//! The titles the flats sit on. A flat is a piece of a larger parcel — a
//! certificate of the estate, most of them HGB held by the Company — and
//! the valley's map layer draws those parcels (`scripts/sync-titles.py`
//! reads its `certificates` folder into `cyberia_titles.json` and says which
//! flat's centre lies in which parcel). This is the upper level of the land:
//! title → flats.

use serde::Deserialize;
use std::sync::OnceLock;

const TITLES_JSON: &str = include_str!("cyberia_titles.json");

#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
pub struct Title {
    /// the parcel's number in the estate registry, as the layer writes it (`4`, `3.1`)
    #[serde(default)]
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub note: String,
    /// `certificate` · `agreement` · `pengikatan jual beli` · `wait` — the layer's words
    #[serde(default)]
    pub doc: String,
    /// the kind of right, `HGB` when the layer says so
    #[serde(default)]
    pub right: String,
    #[serde(default)]
    pub declared_ares: Option<f64>,
    /// `02.02.2053`
    #[serde(default)]
    pub expires: String,
    #[serde(default)]
    pub plots: Vec<String>,
}

impl Title {
    /// Who holds the parcel: a certificate with a right is the Company's;
    /// anything else is still on its way and is said as the layer says it.
    pub fn held_line(&self) -> String {
        match (self.doc.as_str(), self.right.is_empty()) {
            ("certificate", false) => format!("{} · PT Cyber Valley Estate", self.right),
            ("certificate", true) => "certificate · right not written on the map".to_string(),
            ("", _) => "no paper written on the map".to_string(),
            (doc, _) => format!("not a certificate yet · {doc}"),
        }
    }

    /// `parcel 4 · 400 ares declared · until 02.02.2053`
    pub fn facts(&self) -> String {
        let mut f = Vec::new();
        if !self.id.is_empty() {
            f.push(format!("parcel {}", self.id));
        }
        if let Some(a) = self.declared_ares {
            f.push(format!("{a:.0} ares declared"));
        }
        if !self.expires.is_empty() {
            f.push(format!("until {}", self.expires));
        }
        f.join(" · ")
    }
}

#[derive(Clone, Debug, Default, Deserialize)]
pub struct TitleBook {
    #[serde(default)]
    pub titles: Vec<Title>,
}

pub fn book() -> &'static TitleBook {
    static BOOK: OnceLock<TitleBook> = OnceLock::new();
    BOOK.get_or_init(|| serde_json::from_str(TITLES_JSON).unwrap_or_default())
}

/// The parcel a flat is a piece of.
pub fn title_of(plot: &str) -> Option<&'static Title> {
    book().titles.iter().find(|t| t.plots.iter().any(|p| p == plot))
}
