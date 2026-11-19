//! Paying for a flat — the two forms of the lease, the Company's bank
//! account, and the record of what this browser reports as sent. What the
//! Company has received is published by the valley in `/plot-data`.

use crate::deals::now_ms;
use serde::{Deserialize, Serialize};

/// The Company's account, from its bank letter — company data, public.
pub struct BankAccount {
    pub holder: &'static str,
    pub holder_address: &'static str,
    pub number: &'static str,
    pub currencies: &'static str,
    pub bank: &'static str,
    pub branch: &'static str,
    pub bank_address: &'static str,
    pub swift: &'static str,
}

pub const BANK: BankAccount = BankAccount {
    holder: "PT CYBER VALLEY ESTATE",
    holder_address: "Desa Gesing, Kecamatan Banjar, Kabupaten Buleleng, Bali 81152",
    number: "4205703735",
    currencies: "IDR · USD · AUD · EUR",
    bank: "PermataBank",
    branch: "Pura Bagus Taruna",
    bank_address: "Jl. Pura Bagus Taruna 522X, Legian, Kuta, Badung 80361",
    swift: "BBBAIDJA",
};

/// The Company's Ethereum address for the lease — named by the Company
/// on 2026-10-22; EIP-55 casing verified.
pub const ETH_ADDRESS: Option<&str> = Some("0x15534f1875977148EBe5FFb8A3D51C9F996bD0ac");

/// Premium shares the holder can pick below the upfront 100%.
pub const PREMIUM_STEPS: &[u32] = &[10, 20, 30, 40, 50, 60, 70, 80, 90];

/// The leasehold's term, in years.
pub const TERM_YEARS: u32 = 25;

/// The estate's discount rate at a 100% premium, and the spread it adds for
/// carrying the tenant's default risk as the premium shrinks to zero — the
/// same figures the lease pricer at cx.cyberia.my shows.
const RATE_BASE: f64 = 0.09;
const RATE_SPREAD: f64 = 0.05;

/// The yearly rent, in today's dollars, that carries the part of the price
/// not paid as premium: the remainder spread over the term as an annuity at
/// the estate's rate `9% + 5%·(1 − premium)`. The rent is written in CX, so
/// it keeps its value without an indexation term — the index does that.
pub fn yearly_rent(price: f64, premium_pct: u32) -> f64 {
    let p = premium_pct.min(100) as f64 / 100.0;
    let remainder = price * (1.0 - p);
    if remainder <= 0.0 {
        return 0.0;
    }
    let r = RATE_BASE + RATE_SPREAD * (1.0 - p);
    let annuity: f64 = (1..=TERM_YEARS).map(|t| (1.0 + r).powi(-(t as i32))).sum();
    remainder / annuity
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Form {
    /// the whole price at signing
    Upfront,
    /// a premium at signing, the rest as a yearly rent written in CX
    Index,
}

impl Form {
    pub fn label(self) -> &'static str {
        match self {
            Form::Upfront => "PAY IT ALL NOW",
            Form::Index => "PREMIUM + YEARLY RENT",
        }
    }

    pub fn sub(self) -> &'static str {
        match self {
            Form::Upfront => "the whole price at signing · no rent",
            Form::Index => "part at signing · the rest as rent in CX",
        }
    }

    pub fn key(self) -> &'static str {
        match self {
            Form::Upfront => "upfront",
            Form::Index => "index",
        }
    }
}

/// The transfer reference the bank statement will show.
pub fn reference(plot_id: &str, pct: u32) -> String {
    format!("LEASE {} PREMIUM {pct}%", plot_id.to_uppercase())
}

/// A payment this browser reported as sent; the valley confirms receipt.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SentPayment {
    pub plot: String,
    pub usd: f64,
    pub pct: u32,
    pub form: Form,
    /// `bank` | `eth`
    pub method: String,
    pub reference: String,
    pub ts_ms: f64,
}

pub const SENT_KEY: &str = "cyberia.pay.v1";

pub fn load_sent() -> Vec<SentPayment> {
    web_sys::window()
        .and_then(|w| w.local_storage().ok().flatten())
        .and_then(|ls| ls.get_item(SENT_KEY).ok().flatten())
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

pub fn record_sent(list: &mut Vec<SentPayment>, p: SentPayment) {
    list.insert(0, p);
    if let Some(ls) = web_sys::window().and_then(|w| w.local_storage().ok().flatten()) {
        if let Ok(s) = serde_json::to_string(list) {
            let _ = ls.set_item(SENT_KEY, &s);
        }
    }
}

pub fn sent_for(list: &[SentPayment], plot: &str) -> Vec<SentPayment> {
    list.iter().filter(|p| p.plot == plot).cloned().collect()
}

/// The message to support once a transfer is on its way.
pub fn sent_text(p: &SentPayment, plot_name: &str) -> String {
    format!(
        "PAYMENT SENT {}\n{plot_name} · {} · {} · {}% premium\nUSD {:.0} via {}\nreference: {}\nhttps://cyberia.my/plot/{}/",
        p.plot,
        p.form.key(),
        if p.method == "eth" { "Ethereum" } else { "bank transfer" },
        p.pct,
        p.usd,
        p.method,
        p.reference,
        p.plot,
    )
}

pub fn new_sent(plot: &str, usd: f64, pct: u32, form: Form, method: &str) -> SentPayment {
    SentPayment {
        plot: plot.to_string(),
        usd,
        pct,
        form,
        method: method.to_string(),
        reference: reference(plot, pct),
        ts_ms: now_ms(),
    }
}
