//! Soft3 local user wallet — profile, balance, leases, intents, stocks, market.
//! Shared across map console, /me, elements, market.

use serde::{Deserialize, Serialize};

pub const PROFILE_KEY: &str = "cyberia_profile";
pub const BALANCE_KEY: &str = "cyberia_balance";
pub const LEASES_KEY: &str = "cyberia_leases";
pub const INTENTS_KEY: &str = "cyberia_intents";
pub const STOCKS_KEY: &str = "cyberia_stocks";
pub const ORDERS_KEY: &str = "cyberia_orders";
pub const BOOT_KEY: &str = "cyberia_economy_boot";

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Profile {
    pub handle: String,
    pub name: String,
    /// ISO-ish date string when first seen in this browser.
    pub since: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SoftBalance {
    /// Soft century-index units (mock spend/earn rail).
    pub cx: f64,
    /// Mock ETH float for bank-above-banks participation.
    pub eth: f64,
    /// Soft credit score 0–100 (activity-derived on dashboard).
    #[serde(default)]
    pub depth: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Lease {
    pub flat_id: String,
    pub flat_name: String,
    #[serde(default)]
    pub zone: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct IntentRec {
    pub id: u64,
    pub fleet: String,
    pub action: String,
    pub flat: String,
}

/// Qty of an element or product id in your book.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct StockLine {
    pub id: String,
    pub qty: f64,
}

/// Soft market order — no orgs, just peer/city lots in CX.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MarketOrder {
    pub id: u64,
    /// element or product catalog id
    pub good_id: String,
    pub qty: f64,
    /// CX per unit
    pub price_cx: f64,
    /// "buy" | "sell"
    pub side: String,
    /// handle or "cyber-valley" seed
    pub owner: String,
}

fn ls_get(key: &str) -> Option<String> {
    web_sys::window()
        .and_then(|w| w.local_storage().ok().flatten())
        .and_then(|ls| ls.get_item(key).ok().flatten())
}

fn ls_set(key: &str, raw: &str) {
    if let Some(ls) = web_sys::window().and_then(|w| w.local_storage().ok().flatten()) {
        let _ = ls.set_item(key, raw);
    }
}

fn today_label() -> String {
    // stable-enough "since" without chrono dep
    "2026".into()
}

pub fn load_profile() -> Profile {
    ls_get(PROFILE_KEY)
        .and_then(|raw| serde_json::from_str(&raw).ok())
        .unwrap_or_else(|| {
            let p = Profile {
                handle: "anon".into(),
                name: "Anonymous".into(),
                since: today_label(),
            };
            save_profile(&p);
            p
        })
}

pub fn save_profile(p: &Profile) {
    if let Ok(raw) = serde_json::to_string(p) {
        ls_set(PROFILE_KEY, &raw);
    }
}

pub fn load_balance() -> SoftBalance {
    ls_get(BALANCE_KEY)
        .and_then(|raw| serde_json::from_str(&raw).ok())
        .unwrap_or_else(|| {
            let b = SoftBalance {
                cx: 100.0,
                eth: 0.25,
                depth: 12.0,
            };
            save_balance(&b);
            b
        })
}

pub fn save_balance(b: &SoftBalance) {
    if let Ok(raw) = serde_json::to_string(b) {
        ls_set(BALANCE_KEY, &raw);
    }
}

pub fn load_leases() -> Vec<Lease> {
    ls_get(LEASES_KEY)
        .and_then(|raw| serde_json::from_str(&raw).ok())
        .unwrap_or_default()
}

pub fn save_leases(list: &[Lease]) {
    if let Ok(raw) = serde_json::to_string(list) {
        ls_set(LEASES_KEY, &raw);
    }
}

pub fn load_intents() -> Vec<IntentRec> {
    ls_get(INTENTS_KEY)
        .and_then(|raw| serde_json::from_str(&raw).ok())
        .unwrap_or_default()
}

pub fn save_intents(list: &[IntentRec]) {
    if let Ok(raw) = serde_json::to_string(list) {
        ls_set(INTENTS_KEY, &raw);
    }
}

/// Bump soft balance after buy/lease (mock economics).
pub fn debit_cx(amount: f64) {
    let mut b = load_balance();
    b.cx = (b.cx - amount).max(0.0);
    b.depth = (b.depth + amount * 0.15).min(99.0);
    save_balance(&b);
}

pub fn credit_cx(amount: f64) {
    let mut b = load_balance();
    b.cx += amount;
    save_balance(&b);
}

pub fn load_stocks() -> Vec<StockLine> {
    ls_get(STOCKS_KEY)
        .and_then(|raw| serde_json::from_str(&raw).ok())
        .unwrap_or_default()
}

pub fn save_stocks(list: &[StockLine]) {
    if let Ok(raw) = serde_json::to_string(list) {
        ls_set(STOCKS_KEY, &raw);
    }
}

pub fn stock_qty(id: &str) -> f64 {
    load_stocks()
        .into_iter()
        .find(|s| s.id == id)
        .map(|s| s.qty)
        .unwrap_or(0.0)
}

pub fn stock_add(id: &str, delta: f64) {
    let mut list = load_stocks();
    if let Some(line) = list.iter_mut().find(|s| s.id == id) {
        line.qty = (line.qty + delta).max(0.0);
        if line.qty < 1e-9 {
            list.retain(|s| s.id != id);
        }
    } else if delta > 0.0 {
        list.push(StockLine {
            id: id.to_string(),
            qty: delta,
        });
    }
    list.sort_by(|a, b| a.id.cmp(&b.id));
    save_stocks(&list);
}

pub fn stock_has(needs: &[(String, f64)]) -> bool {
    needs.iter().all(|(id, q)| stock_qty(id) + 1e-9 >= *q)
}

pub fn stock_consume(needs: &[(String, f64)]) -> bool {
    if !stock_has(needs) {
        return false;
    }
    for (id, q) in needs {
        stock_add(id, -q);
    }
    true
}

pub fn load_orders() -> Vec<MarketOrder> {
    ls_get(ORDERS_KEY)
        .and_then(|raw| serde_json::from_str(&raw).ok())
        .unwrap_or_default()
}

pub fn save_orders(list: &[MarketOrder]) {
    if let Ok(raw) = serde_json::to_string(list) {
        ls_set(ORDERS_KEY, &raw);
    }
}

pub fn next_order_id() -> u64 {
    load_orders().iter().map(|o| o.id).max().unwrap_or(0) + 1
}

pub fn push_intent(fleet: &str, action: &str, flat: &str) {
    let mut q = load_intents();
    let id = q.iter().map(|i| i.id).max().unwrap_or(0) + 1;
    q.insert(
        0,
        IntentRec {
            id,
            fleet: fleet.into(),
            action: action.into(),
            flat: flat.into(),
        },
    );
    if q.len() > 200 {
        q.truncate(200);
    }
    save_intents(&q);
}

/// First-run starter pack + city seed book (no orgs — valley as seed seller).
pub fn ensure_economy_boot() {
    if ls_get(BOOT_KEY).is_some() {
        return;
    }
    // starter stocks so BOM is reachable without empty trap
    let starters = [
        ("energy", 40.0),
        ("water", 20.0),
        ("labor", 30.0),
        ("wood", 8.0),
        ("food_raw", 5.0),
        ("fill", 4.0),
        ("gravel", 2.0),
    ];
    for (id, q) in starters {
        stock_add(id, q);
    }
    // city seed asks — deep book so market is never empty
    let mut orders = load_orders();
    let seed = [
        ("energy", 500.0, 0.4),
        ("water", 300.0, 0.5),
        ("labor", 200.0, 1.2),
        ("wood", 150.0, 2.0),
        ("food_raw", 200.0, 1.5),
        ("fill", 100.0, 1.0),
        ("gravel", 80.0, 1.8),
        ("biochar", 60.0, 3.0),
        ("bandwidth", 100.0, 0.8),
        ("compute", 40.0, 5.0),
        ("meal", 80.0, 4.0),
        ("cube_kit", 20.0, 45.0),
        ("camp_kit", 30.0, 18.0),
        ("trail_kit", 25.0, 12.0),
        ("soft_night", 15.0, 22.0),
        ("biochar_bag", 40.0, 8.0),
    ];
    let mut id = next_order_id();
    for (good, qty, price) in seed {
        orders.push(MarketOrder {
            id,
            good_id: good.into(),
            qty,
            price_cx: price,
            side: "sell".into(),
            owner: "cyber-valley".into(),
        });
        id += 1;
    }
    save_orders(&orders);
    ls_set(BOOT_KEY, "1");
    push_intent("SYSTEM", "boot", "economy");
}
