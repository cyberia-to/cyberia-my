//! The land market's client: acts signed by a wallet and sent to the gate
//! (`gate/gate.py`), and the state the gate folds from them. An act is a
//! short text — `cyberia.my act`, then `key: value` lines — that the wallet
//! shows and signs with `personal_sign`; the gate recovers the signer,
//! checks the right and commits the act. Nothing is written without a
//! signature, and nothing here trusts the page: the gate verifies again.

use crate::eth::{load_eth, personal_sign, EthWallet};
use serde::{Deserialize, Serialize};
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_futures::JsFuture;

const API: &str = "/api/acts";
const TOKEN_KEY: &str = "cyberia.acts.token";

#[derive(Clone, Debug, Default, PartialEq, Deserialize)]
pub struct Holder {
    pub plot: String,
    pub address: String,
    #[serde(default)]
    pub name: String,
}

#[derive(Clone, Debug, Default, PartialEq, Deserialize)]
pub struct Claim {
    pub id: String,
    pub plot: String,
    pub from: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub contact: String,
    pub status: String,
    #[serde(default)]
    pub reason: String,
    #[serde(default)]
    pub at: String,
}

#[derive(Clone, Debug, Default, PartialEq, Deserialize)]
pub struct Mandate {
    pub id: String,
    pub plot: String,
    pub holder: String,
    pub agent: String,
    pub scope: String,
    #[serde(default)]
    pub commission: f64,
    pub until: String,
    pub status: String,
}

#[derive(Clone, Debug, Default, PartialEq, Deserialize)]
pub struct Listing {
    pub id: String,
    pub plot: String,
    pub deal: String,
    pub price_usd: f64,
    #[serde(default)]
    pub rent_usd_year: Option<f64>,
    #[serde(default)]
    pub term_years: Option<u32>,
    #[serde(default)]
    pub words: String,
    #[serde(default)]
    pub contact: String,
    pub by: String,
    #[serde(rename = "as")]
    pub role: String,
    pub status: String,
    #[serde(default)]
    pub reason: String,
    #[serde(default)]
    pub at: String,
}

#[derive(Clone, Debug, Default, PartialEq, Deserialize)]
pub struct Profile {
    pub address: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub contact: String,
}

#[derive(Clone, Debug, Default, PartialEq, Deserialize)]
pub struct Role {
    pub address: String,
    pub role: String,
    #[serde(default)]
    pub name: String,
}

/// What one address sees of the market — everything for an admin.
#[derive(Clone, Debug, Default, PartialEq, Deserialize)]
pub struct View {
    pub address: String,
    /// `root` · `admin` · `agent` · `` (a holder or a stranger)
    #[serde(default)]
    pub role: String,
    #[serde(default)]
    pub profile: Option<Profile>,
    #[serde(default)]
    pub holders: Vec<Holder>,
    #[serde(default)]
    pub claims: Vec<Claim>,
    #[serde(default)]
    pub mandates: Vec<Mandate>,
    #[serde(default)]
    pub listings: Vec<Listing>,
    #[serde(default)]
    pub profiles: Vec<Profile>,
    #[serde(default)]
    pub roles: Vec<Role>,
}

impl View {
    pub fn is_admin(&self) -> bool {
        self.role == "root" || self.role == "admin"
    }

    pub fn me(&self, addr: &str) -> bool {
        self.address.eq_ignore_ascii_case(addr)
    }

    /// The name an address chose, or its short form.
    pub fn name_of(&self, addr: &str) -> String {
        self.profiles
            .iter()
            .find(|p| p.address.eq_ignore_ascii_case(addr))
            .map(|p| p.name.clone())
            .filter(|n| !n.is_empty())
            .or_else(|| self.roles.iter().find(|r| r.address.eq_ignore_ascii_case(addr)).map(|r| r.name.clone()).filter(|n| !n.is_empty()))
            .unwrap_or_else(|| short(addr))
    }
}

/// `0x1234…abcd`
pub fn short(addr: &str) -> String {
    if addr.len() > 12 {
        format!("{}…{}", &addr[..6], &addr[addr.len() - 4..])
    } else {
        addr.to_string()
    }
}

/// A live listing as the public sees it.
#[derive(Clone, Debug, Default, PartialEq, Deserialize)]
pub struct Offer {
    pub id: String,
    pub plot: String,
    /// `sale` (the leasehold is assigned) · `lease` (a sublease)
    pub deal: String,
    pub price_usd: f64,
    #[serde(default)]
    pub rent_usd_year: Option<f64>,
    #[serde(default)]
    pub term_years: Option<u32>,
    #[serde(default)]
    pub words: String,
    pub seller: Seller,
}

#[derive(Clone, Debug, Default, PartialEq, Deserialize)]
pub struct Seller {
    /// `holder` · `agent`
    #[serde(rename = "as")]
    pub role: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub contact: String,
    #[serde(default)]
    pub accredited: bool,
}

impl Offer {
    pub fn deal_label(&self) -> &'static str {
        if self.deal == "lease" { "SUBLEASE" } else { "RESALE" }
    }

    pub fn by_line(&self) -> String {
        match (self.seller.role.as_str(), self.seller.accredited) {
            ("agent", true) => format!("through {} · accredited agent", self.seller.name),
            ("agent", false) => format!("through {} · the holder's agent", self.seller.name),
            _ => format!("by its holder · {}", self.seller.name),
        }
    }

    /// Where a buyer writes: the seller's Telegram, or the valley when none was left.
    pub fn ask_href(&self, plot_name: &str) -> String {
        let handle = self.seller.contact.trim().trim_start_matches('@').trim_start_matches("https://t.me/").to_string();
        let handle = if handle.is_empty() || !handle.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
            crate::orders::SUPPORT_HANDLE.to_string()
        } else {
            handle
        };
        let text = format!("{} {} — {}\nhttps://cyberia.my/plot/{}/", self.deal_label(), self.plot, plot_name, self.plot);
        format!("https://t.me/{handle}?text={}", String::from(js_sys::encode_uri_component(&text)))
    }
}

#[derive(Deserialize, Default)]
struct Public {
    #[serde(default)]
    listings: Vec<Offer>,
}

fn storage() -> Option<web_sys::Storage> {
    web_sys::window().and_then(|w| w.local_storage().ok().flatten())
}

fn token() -> Option<String> {
    storage().and_then(|ls| ls.get_item(TOKEN_KEY).ok().flatten()).filter(|t| !t.is_empty())
}

pub fn sign_out() {
    if let Some(ls) = storage() {
        let _ = ls.remove_item(TOKEN_KEY);
    }
}

async fn call(path: &str, body: Option<String>, auth: bool) -> Result<serde_json::Value, String> {
    let window = web_sys::window().ok_or("no window")?;
    let init = web_sys::RequestInit::new();
    init.set_method(if body.is_some() { "POST" } else { "GET" });
    let headers = js_sys::Object::new();
    let set = |k: &str, v: &str| {
        let _ = js_sys::Reflect::set(&headers, &JsValue::from_str(k), &JsValue::from_str(v));
    };
    if let Some(b) = &body {
        set("Content-Type", "application/json");
        init.set_body(&JsValue::from_str(b));
    }
    if auth {
        if let Some(t) = token() {
            set("Authorization", &format!("Bearer {t}"));
        }
    }
    init.set_headers(&headers);
    let resp = JsFuture::from(window.fetch_with_str_and_init(&format!("{API}{path}"), &init))
        .await
        .map_err(|_| "the gate does not answer".to_string())?;
    let resp: web_sys::Response = resp.dyn_into().map_err(|_| "no response")?;
    let text = JsFuture::from(resp.text().map_err(|_| "no body")?)
        .await
        .map_err(|_| "no body")?
        .as_string()
        .unwrap_or_default();
    let v: serde_json::Value = serde_json::from_str(&text).map_err(|_| "the gate does not answer".to_string())?;
    if resp.ok() {
        Ok(v)
    } else {
        Err(v.get("error").and_then(|e| e.as_str()).unwrap_or("refused").to_string())
    }
}

/// The live listings — public, no wallet needed.
pub async fn fetch_offers() -> Vec<Offer> {
    call("/public", None, false)
        .await
        .ok()
        .and_then(|v| serde_json::from_value::<Public>(v).ok())
        .map(|p| p.listings)
        .unwrap_or_default()
}

/// What the signed-in address sees, when a session is kept.
pub async fn fetch_me() -> Option<View> {
    token()?;
    match call("/me", None, true).await {
        Ok(v) => serde_json::from_value(v).ok(),
        Err(_) => {
            sign_out();
            None
        }
    }
}

fn now_iso() -> String {
    let d = js_sys::Date::new_0();
    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        d.get_utc_full_year(),
        d.get_utc_month() + 1,
        d.get_utc_date(),
        d.get_utc_hours(),
        d.get_utc_minutes(),
        d.get_utc_seconds()
    )
}

fn uuid() -> String {
    let mut b = [0u8; 16];
    let _ = getrandom::getrandom(&mut b);
    b[6] = (b[6] & 0x0f) | 0x40;
    b[8] = (b[8] & 0x3f) | 0x80;
    let h: String = b.iter().map(|x| format!("{x:02x}")).collect();
    format!("{}-{}-{}-{}-{}", &h[..8], &h[8..12], &h[12..16], &h[16..20], &h[20..])
}

async fn wallet() -> Result<EthWallet, String> {
    match load_eth() {
        Some(w) => Ok(w),
        None => Err("connect your wallet first".to_string()),
    }
}

/// Prove a connected wallet's address to the gate: one signature, a
/// session of thirty days.
pub async fn sign_in(w: EthWallet) -> Result<View, String> {
    let nonce = call("/nonce", None, false).await?.get("nonce").and_then(|n| n.as_str()).unwrap_or_default().to_string();
    let message = format!("cyberia.my sign in\naddress: {}\nnonce: {nonce}\nat: {}", w.address, now_iso());
    let signature = personal_sign(&w.address, &message).await?;
    #[derive(Serialize)]
    struct Body<'a> {
        message: &'a str,
        signature: &'a str,
    }
    let v = call("/signin", Some(serde_json::to_string(&Body { message: &message, signature: &signature }).unwrap_or_default()), false).await?;
    if let (Some(t), Some(ls)) = (v.get("token").and_then(|t| t.as_str()), storage()) {
        let _ = ls.set_item(TOKEN_KEY, t);
    }
    serde_json::from_value(v.get("view").cloned().unwrap_or_default()).map_err(|_| "the gate's answer is not a view".to_string())
}

/// Sign one act with the wallet and hand it to the gate. `fields` are the
/// act's own lines; kind, id and time are added here. Returns the signer's
/// view after the act.
pub async fn act(kind: &str, fields: &[(&str, String)]) -> Result<View, String> {
    let w = wallet().await?;
    let mut lines = vec![
        "cyberia.my act".to_string(),
        format!("kind: {kind}"),
        format!("id: {}", uuid()),
        format!("at: {}", now_iso()),
    ];
    for (k, v) in fields {
        let v = v.replace('\n', " ");
        let v = v.trim();
        if !v.is_empty() {
            lines.push(format!("{k}: {v}"));
        }
    }
    let message = lines.join("\n");
    let signature = personal_sign(&w.address, &message).await?;
    #[derive(Serialize)]
    struct Body<'a> {
        message: &'a str,
        signature: &'a str,
    }
    let v = call("/act", Some(serde_json::to_string(&Body { message: &message, signature: &signature }).unwrap_or_default()), false).await?;
    serde_json::from_value(v.get("view").cloned().unwrap_or_default()).map_err(|_| "the gate's answer is not a view".to_string())
}
