//! Orders the valley fulfils on the ground — a triangulation survey, photos
//! and a drone video of a flat — and the data they bring back.
//!
//! An order is a Telegram message: the hard force bot takes `/start` with
//! the order in its payload and posts the task to the crew's group; until
//! the bot is live the same text goes as a direct message to support. The
//! order is logged in this browser so the cabinet can show it. What comes
//! back is published as `/plot-data/<id>.json` — fetched here, no rebuild.

use crate::deals::now_ms;
use serde::{Deserialize, Serialize};

/// Support — direct messages, KYC, every question.
pub const SUPPORT_HANDLE: &str = "st_joy";
/// The community group newcomers join.
pub const COMMUNITY_GROUP: &str = "cyberialand";
/// The hard force bot, once it runs; orders go to it as `/start` payloads.
pub const HARDFORCE_BOT: Option<&str> = None;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum OrderKind {
    Survey,
    Media,
    Kyc,
}

impl OrderKind {
    pub fn key(self) -> &'static str {
        match self {
            OrderKind::Survey => "survey",
            OrderKind::Media => "media",
            OrderKind::Kyc => "kyc",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            OrderKind::Survey => "ORDER SURVEY",
            OrderKind::Media => "ORDER PHOTO · DRONE VIDEO",
            OrderKind::Kyc => "SEND KYC",
        }
    }

    pub fn line(self) -> &'static str {
        match self {
            OrderKind::Survey => "triangulation on the slope by the hard force · the figure goes to the page and the lease",
            OrderKind::Media => "photos and a drone flight over the flat · published on this page",
            OrderKind::Kyc => "passport page and a selfie, in a direct message · nothing is stored on our servers",
        }
    }

    /// The message the order sends.
    pub fn text(self, plot_id: &str, plot_name: &str) -> String {
        match self {
            OrderKind::Survey => format!(
                "ORDER SURVEY {plot_id}\n{plot_name} — triangulation survey of the flat.\nhttps://cyberia.my/plot/{plot_id}/"
            ),
            OrderKind::Media => format!(
                "ORDER MEDIA {plot_id}\n{plot_name} — photos and a drone video of the flat.\nhttps://cyberia.my/plot/{plot_id}/"
            ),
            OrderKind::Kyc => "KYC — I am starting a lease at Cyber Valley and will send my passport page and a selfie here.".to_string(),
        }
    }
}

fn encode(s: &str) -> String {
    String::from(js_sys::encode_uri_component(s))
}

/// Where the order goes: the bot's deep link, or support's direct messages
/// with the text already typed.
pub fn order_href(kind: OrderKind, plot_id: &str, plot_name: &str) -> String {
    match (HARDFORCE_BOT, kind) {
        (Some(bot), OrderKind::Survey | OrderKind::Media) => {
            // start payloads allow [A-Za-z0-9_-]: `survey_sinwood-7`, kind before the first `_`
            format!("https://t.me/{bot}?start={}_{plot_id}", kind.key())
        }
        _ => format!(
            "https://t.me/{SUPPORT_HANDLE}?text={}",
            encode(&kind.text(plot_id, plot_name))
        ),
    }
}

pub fn support_href() -> String {
    format!("https://t.me/{SUPPORT_HANDLE}")
}

pub fn community_href() -> String {
    format!("https://t.me/{COMMUNITY_GROUP}")
}

/// Orders placed from this browser.
pub const ORDERS_LOG_KEY: &str = "cyberia.orders.v1";

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct OrderRec {
    pub kind: OrderKind,
    pub plot: String,
    pub ts_ms: f64,
}

pub fn load_orders_log() -> Vec<OrderRec> {
    web_sys::window()
        .and_then(|w| w.local_storage().ok().flatten())
        .and_then(|ls| ls.get_item(ORDERS_LOG_KEY).ok().flatten())
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

/// One live order per kind and flat: a second press refreshes the time.
pub fn record_order(log: &mut Vec<OrderRec>, kind: OrderKind, plot: &str) {
    log.retain(|o| !(o.kind == kind && o.plot == plot));
    log.insert(
        0,
        OrderRec {
            kind,
            plot: plot.to_string(),
            ts_ms: now_ms(),
        },
    );
    if let Some(ls) = web_sys::window().and_then(|w| w.local_storage().ok().flatten()) {
        if let Ok(s) = serde_json::to_string(log) {
            let _ = ls.set_item(ORDERS_LOG_KEY, &s);
        }
    }
}

pub fn ordered(log: &[OrderRec], kind: OrderKind, plot: &str) -> Option<f64> {
    log.iter().find(|o| o.kind == kind && o.plot == plot).map(|o| o.ts_ms)
}

/// What the ground brought back — `/plot-data/<id>.json`, written by the
/// hard force bot into the pages repo.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct PlotData {
    #[serde(default)]
    pub survey: Option<Survey>,
    #[serde(default)]
    pub media: Vec<Media>,
    #[serde(default)]
    pub updated: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Survey {
    pub m2: f64,
    #[serde(default)]
    pub on: String,
    #[serde(default)]
    pub method: String,
    #[serde(default)]
    pub by: String,
    /// the survey sheet — an IPFS gateway URL
    #[serde(default)]
    pub file: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Media {
    /// `photo` | `video`
    pub kind: String,
    pub url: String,
    #[serde(default)]
    pub cid: String,
    #[serde(default)]
    pub taken: String,
    #[serde(default)]
    pub note: String,
}

/// None when the flat has no published data yet (404) or the fetch failed.
pub async fn fetch_plot_data(plot_id: &str) -> Option<PlotData> {
    use wasm_bindgen::JsCast;
    use wasm_bindgen_futures::JsFuture;
    let window = web_sys::window()?;
    let resp = JsFuture::from(window.fetch_with_str(&format!("/plot-data/{plot_id}.json")))
        .await
        .ok()?;
    let resp: web_sys::Response = resp.dyn_into().ok()?;
    if !resp.ok() {
        return None;
    }
    let text = JsFuture::from(resp.text().ok()?).await.ok()?.as_string()?;
    serde_json::from_str(&text).ok()
}
