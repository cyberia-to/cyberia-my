//! Lease deals for phase-0 flats — soft3 local (this browser), no backend.
//!
//! A flat's colour on the map is its status. Two sources decide it: the lease
//! book (`terms.rs`, synced from the plot sheet — holder, type, price) and a
//! deal stored here. A signed or live deal wins; otherwise the book speaks.

use crate::terms::{terms, LandUse};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub const DEALS_KEY: &str = "cyberia.deals.v1";

/// Where a flat stands inside its land use. The colour on the map is the
/// land use (`terms::LandUse`); this is what you can do with the flat.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FlatStatus {
    /// lease / dual use, priced and free
    Available,
    /// joint venture, business, or a whole-district HGB sale
    OnRequest,
    /// a live deal in this browser
    Transfer,
    Owned,
    /// city land, no price, or no land use set
    Closed,
}

impl FlatStatus {
    pub fn label(self) -> &'static str {
        match self {
            FlatStatus::Available => "AVAILABLE",
            FlatStatus::OnRequest => "ON REQUEST",
            FlatStatus::Transfer => "IN TRANSFER",
            FlatStatus::Owned => "OWNED",
            FlatStatus::Closed => "NOT ON OFFER",
        }
    }

    /// fill strength on the map — the land use colour carries the hue
    pub fn alpha(self) -> f64 {
        match self {
            FlatStatus::Available => 0.62,
            FlatStatus::OnRequest => 0.50,
            FlatStatus::Transfer => 0.42,
            FlatStatus::Owned => 0.30,
            FlatStatus::Closed => 0.26,
        }
    }
}

/// Deal steps in order. The last one closes the deal — the flat becomes owned.
pub const STEPS: &[(&str, &str)] = &[
    ("request", "lease requested — the flat is spoken for"),
    ("review", "council review — plot, price, neighbours"),
    ("docs", "documents — passport, agreement, survey"),
    ("transfer", "rights in transfer — signatures on both sides"),
    ("signed", "signed — the flat is yours"),
];

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DealEvent {
    pub step: String,
    pub ts_ms: f64,
    pub note: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Deal {
    pub flat_id: String,
    pub flat_name: String,
    pub step: String,
    #[serde(default)]
    pub cancelled: bool,
    #[serde(default)]
    pub history: Vec<DealEvent>,
}

impl Deal {
    pub fn is_signed(&self) -> bool {
        !self.cancelled && self.step == "signed"
    }

    pub fn is_live(&self) -> bool {
        !self.cancelled && !self.is_signed()
    }

    pub fn step_index(&self) -> usize {
        STEPS.iter().position(|(s, _)| *s == self.step).unwrap_or(0)
    }

    pub fn step_line(&self) -> &'static str {
        STEPS
            .iter()
            .find(|(s, _)| *s == self.step)
            .map(|(_, line)| *line)
            .unwrap_or("")
    }
}

pub fn now_ms() -> f64 {
    js_sys::Date::now()
}

/// `2026-09-24 12:31` in the browser's local time.
pub fn fmt_ts(ms: f64) -> String {
    let d = js_sys::Date::new(&ms.into());
    format!(
        "{:04}-{:02}-{:02} {:02}:{:02}",
        d.get_full_year(),
        d.get_month() + 1,
        d.get_date(),
        d.get_hours(),
        d.get_minutes()
    )
}

pub fn load_deals() -> HashMap<String, Deal> {
    web_sys::window()
        .and_then(|w| w.local_storage().ok().flatten())
        .and_then(|ls| ls.get_item(DEALS_KEY).ok().flatten())
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

pub fn save_deals(m: &HashMap<String, Deal>) {
    if let Some(ls) = web_sys::window().and_then(|w| w.local_storage().ok().flatten()) {
        if let Ok(s) = serde_json::to_string(m) {
            let _ = ls.set_item(DEALS_KEY, &s);
        }
    }
}

/// Who holds the flat, per the lease book (the city excluded).
pub fn holder(id: &str) -> Option<&'static str> {
    terms(id).and_then(|t| t.holder())
}

pub fn flat_status(id: &str, deals: &HashMap<String, Deal>) -> FlatStatus {
    match deals.get(id) {
        Some(d) if d.is_signed() => return FlatStatus::Owned,
        Some(d) if d.is_live() => return FlatStatus::Transfer,
        _ => {}
    }
    let Some(t) = terms(id) else {
        return FlatStatus::Closed;
    };
    if t.holder().is_some() {
        return FlatStatus::Owned;
    }
    if t.in_reregistration() {
        return FlatStatus::Closed;
    }
    match t.land_use() {
        LandUse::Lease | LandUse::Dual if !t.is_city_land() && t.price().is_some() => {
            FlatStatus::Available
        }
        // the city's own business ground stays open to a joint venture
        LandUse::Venture => FlatStatus::OnRequest,
        LandUse::Hgb if !t.is_city_land() => FlatStatus::OnRequest,
        _ => FlatStatus::Closed,
    }
}

/// Start a deal at `request`. A cancelled deal restarts from scratch.
pub fn open_deal(deals: &mut HashMap<String, Deal>, id: &str, name: &str) {
    let fresh = Deal {
        flat_id: id.to_string(),
        flat_name: name.to_string(),
        step: "request".into(),
        cancelled: false,
        history: vec![DealEvent {
            step: "request".into(),
            ts_ms: now_ms(),
            note: "lease requested from the map".into(),
        }],
    };
    match deals.get_mut(id) {
        Some(d) if !d.cancelled => {}
        _ => {
            deals.insert(id.to_string(), fresh);
        }
    }
    save_deals(deals);
}

pub fn advance_deal(deals: &mut HashMap<String, Deal>, id: &str, note: &str) {
    if let Some(d) = deals.get_mut(id) {
        let i = d.step_index();
        if d.cancelled || i + 1 >= STEPS.len() {
            return;
        }
        d.step = STEPS[i + 1].0.into();
        d.history.push(DealEvent {
            step: d.step.clone(),
            ts_ms: now_ms(),
            note: note.into(),
        });
    }
    save_deals(deals);
}

pub fn cancel_deal(deals: &mut HashMap<String, Deal>, id: &str, note: &str) {
    if let Some(d) = deals.get_mut(id) {
        if d.is_signed() {
            return;
        }
        d.cancelled = true;
        d.history.push(DealEvent {
            step: "cancelled".into(),
            ts_ms: now_ms(),
            note: note.into(),
        });
    }
    save_deals(deals);
}

/// Due-diligence requests on flats that are not yours: flat id → when asked.
pub const DD_KEY: &str = "cyberia.dd.v1";

pub fn load_dd() -> HashMap<String, f64> {
    web_sys::window()
        .and_then(|w| w.local_storage().ok().flatten())
        .and_then(|ls| ls.get_item(DD_KEY).ok().flatten())
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

pub fn request_dd(m: &mut HashMap<String, f64>, id: &str) {
    m.entry(id.to_string()).or_insert_with(now_ms);
    if let Some(ls) = web_sys::window().and_then(|w| w.local_storage().ok().flatten()) {
        if let Ok(s) = serde_json::to_string(m) {
            let _ = ls.set_item(DD_KEY, &s);
        }
    }
}

/// Requests for on-request land: flat id (or `district:<zone>` for a whole
/// HGB district) → when asked.
pub const JV_KEY: &str = "cyberia.jv.v1";

pub fn load_jv() -> HashMap<String, f64> {
    web_sys::window()
        .and_then(|w| w.local_storage().ok().flatten())
        .and_then(|ls| ls.get_item(JV_KEY).ok().flatten())
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

pub fn request_jv(m: &mut HashMap<String, f64>, id: &str) {
    m.entry(id.to_string()).or_insert_with(now_ms);
    if let Some(ls) = web_sys::window().and_then(|w| w.local_storage().ok().flatten()) {
        if let Ok(s) = serde_json::to_string(m) {
            let _ = ls.set_item(JV_KEY, &s);
        }
    }
}
