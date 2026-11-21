//! `/plot/:id/deal` — the deal of one flat as a track of seven steps. The
//! flat page sells; this page carries the work that follows a yes: who the
//! Holder is, the dossier, the money and its control, the papers made for
//! this flat and signed, the notary, the handover.
//!
//! Every step has three sources of truth and shows which one speaks: what
//! this browser did (a request sent, a payment reported, a text signed),
//! what the valley published for the flat (`/plot-data/<id>.json`, steps
//! by date), and what is still ahead. Steps are not locked: KYC and the
//! dossier are often done side by side, and the track says so.

use crate::deal_steps::{panel, Ctx};
use crate::deals::{cancel_deal, fmt_ts, load_deals};
use crate::eth::load_eth;
use crate::globe::BASE;
use crate::land::{area_m2, load_map, FLAG_SVG};
use crate::nav::CyberiaNav;
use crate::orders::{
    load_orders_log, ordered, support_href, OrderKind, PlotData, PublishedDeal, SUPPORT_HANDLE,
};
use crate::pay::{load_sent, sent_for, yearly_rent, Form, TERM_YEARS};
use crate::signal::{neuron, DocSignature};
use crate::terms::{bare_name, fmt_usd, land_use, size_line, terms};
use leptos::prelude::*;
use leptos_router::hooks::use_params_map;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ─── what this browser knows about the deal ───────────────────────────

pub const CFG_KEY: &str = "cyberia.dealcfg.v1";

/// The Holder's choices and own acts on one flat's deal — this browser only.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DealCfg {
    pub form: Form,
    pub pct: u32,
    /// `person` | `company`
    pub party: String,
    /// `id` (Indonesia) | `abroad`
    pub origin: String,
    /// `bank` | `eth`
    pub rail: String,
    /// `person` (at the notary's table) | `proxy` (by power of attorney)
    pub notary: String,
    #[serde(default)]
    pub requested_ms: Option<f64>,
    #[serde(default)]
    pub dd_read_ms: Option<f64>,
    #[serde(default)]
    pub notary_asked_ms: Option<f64>,
    /// the offer as it was signed, and the signature under it
    #[serde(default)]
    pub offer_text: Option<String>,
    #[serde(default)]
    pub offer_sig: Option<DocSignature>,
    /// the moment the offer was made and the index fix read then, frozen
    /// when it was signed — the text under the signature must not move
    #[serde(default)]
    pub offer_ts_ms: Option<f64>,
    #[serde(default)]
    pub offer_cx: Option<f64>,
}

impl Default for DealCfg {
    fn default() -> Self {
        DealCfg {
            form: Form::Upfront,
            pct: 30,
            party: "person".into(),
            origin: "abroad".into(),
            rail: "bank".into(),
            notary: "person".into(),
            requested_ms: None,
            dd_read_ms: None,
            notary_asked_ms: None,
            offer_text: None,
            offer_sig: None,
            offer_ts_ms: None,
            offer_cx: None,
        }
    }
}

impl DealCfg {
    pub fn share(&self) -> u32 {
        if self.form == Form::Upfront { 100 } else { self.pct }
    }
}

fn load_cfgs() -> HashMap<String, DealCfg> {
    web_sys::window()
        .and_then(|w| w.local_storage().ok().flatten())
        .and_then(|ls| ls.get_item(CFG_KEY).ok().flatten())
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

pub fn load_cfg(id: &str) -> DealCfg {
    load_cfgs().remove(id).unwrap_or_default()
}

fn save_cfg(id: &str, cfg: &DealCfg) {
    let mut all = load_cfgs();
    all.insert(id.to_string(), cfg.clone());
    if let Some(ls) = web_sys::window().and_then(|w| w.local_storage().ok().flatten()) {
        if let Ok(s) = serde_json::to_string(&all) {
            let _ = ls.set_item(CFG_KEY, &s);
        }
    }
}

/// Carry the share the buyer picked on the flat page into their deal. It is
/// their own choice until the request is sent or the offer signed; after
/// that the figures the valley holds and the signature covers stand.
pub fn set_share(id: &str, share: u32) {
    let mut cfg = load_cfg(id);
    if cfg.requested_ms.is_some() || cfg.offer_sig.is_some() {
        return;
    }
    if share >= 100 {
        cfg.form = Form::Upfront;
    } else {
        cfg.form = Form::Index;
        cfg.pct = share.clamp(10, 90);
    }
    save_cfg(id, &cfg);
}

/// Has this browser started a deal on the flat?
pub fn deal_started(id: &str) -> bool {
    load_cfgs().get(id).map(|c| c.requested_ms.is_some()).unwrap_or(false)
}

// ─── the track ────────────────────────────────────────────────────────

/// Where a step stands, and who says so.
#[derive(Clone, Debug, PartialEq)]
enum State {
    /// confirmed — by the valley's published date, or by the Holder's own act where that is enough
    Done(String),
    /// the Holder did their part; the valley answers
    Waiting(String),
    Todo,
}

/// key (as the valley publishes it) · title · the one line under it
const STEPS: &[(&str, &str, &str)] = &[
    ("request", "TERMS", "how you pay, who holds — the request"),
    ("kyc", "IDENTITY", "who the Holder is"),
    ("dd", "DOSSIER", "the land and the Company, read"),
    ("premium", "PAYMENT", "sent, received, counted"),
    ("docs", "PAPERS", "made for this flat, signed"),
    ("notary", "NOTARY", "the deed, in person or by proxy"),
    ("signed", "YOURS", "handover and the flat's token"),
];

fn states(
    cfg: &DealCfg,
    published: &Option<PublishedDeal>,
    kyc_sent: Option<f64>,
    sent_usd: f64,
    due_usd: Option<f64>,
) -> Vec<State> {
    let date = |k: &str| published.as_ref().and_then(|p| p.steps.get(k).cloned()).filter(|d| !d.is_empty());
    let status = published.as_ref().map(|p| p.status.as_str()).unwrap_or("");
    let paid = published.as_ref().map(|p| p.paid_usd).unwrap_or(0.0);
    let by = |k: &str| date(k).map(|d| State::Done(format!("confirmed {d}")));
    vec![
        by("request")
            .or_else(|| (!status.is_empty() && status != "available").then(|| State::Done("accepted by the valley".into())))
            .or_else(|| cfg.requested_ms.map(|t| State::Waiting(format!("sent {} · the valley answers", fmt_ts(t)))))
            .unwrap_or(State::Todo),
        by("kyc")
            .or_else(|| kyc_sent.map(|t| State::Waiting(format!("sent {} · being checked", fmt_ts(t)))))
            .unwrap_or(State::Todo),
        by("dd")
            .or_else(|| cfg.dd_read_ms.map(|t| State::Done(format!("read {}", fmt_ts(t)))))
            .unwrap_or(State::Todo),
        by("premium")
            .or_else(|| match due_usd {
                Some(due) if paid > 0.0 && paid + 0.5 >= due => Some(State::Done(format!("{} received", fmt_usd(paid)))),
                _ => None,
            })
            .or_else(|| (paid > 0.0).then(|| State::Waiting(format!("{} received so far", fmt_usd(paid)))))
            .or_else(|| (sent_usd > 0.0).then(|| State::Waiting(format!("{} reported sent · awaiting receipt", fmt_usd(sent_usd)))))
            .unwrap_or(State::Todo),
        by("docs")
            .or_else(|| cfg.offer_sig.as_ref().map(|s| State::Waiting(format!("offer signed {} · the Company countersigns", fmt_ts(s.ts_ms)))))
            .unwrap_or(State::Todo),
        by("notary")
            .or_else(|| cfg.notary_asked_ms.map(|t| State::Waiting(format!("asked {} · a date is being set", fmt_ts(t)))))
            .unwrap_or(State::Todo),
        by("signed")
            .or_else(|| (status == "owned").then(|| State::Done("the flat is held".into())))
            .unwrap_or(State::Todo),
    ]
}

#[component]
pub fn DealPage() -> impl IntoView {
    let params = use_params_map();
    let id = params.get_untracked().get("id").unwrap_or_default();
    let map = load_map();
    let Some(flat) = map.phase0.iter().find(|f| f.id == id).cloned() else {
        return view! {
            <div class="page-shell" style="padding:40px;">
                <h1 style="color: var(--cyber-red);">"404"</h1>
                <a href=format!("{BASE}/map") style="color: var(--cyber-green);">"← back to the map"</a>
            </div>
        }
        .into_any();
    };
    let name = bare_name(&flat.name);
    let book = terms(&id);
    let price = book.and_then(|t| t.price());
    let ink = land_use(&id).color();
    let size = size_line(book, area_m2(&flat.coords));
    document().set_title(&format!("{name} — your deal · Cyber Valley"));

    let cfg = RwSignal::new(load_cfg(&id));
    {
        let id = id.clone();
        Effect::new(move |_| save_cfg(&id, &cfg.get()));
    }
    let deals = RwSignal::new(load_deals());
    let pdata = RwSignal::new(None::<PlotData>);
    {
        let id = id.clone();
        leptos::task::spawn_local(async move {
            pdata.set(crate::orders::fetch_plot_data(&id).await);
        });
    }
    let cx = RwSignal::new(None::<f64>);
    leptos::task::spawn_local(async move {
        cx.set(crate::orders::fetch_cx_level().await);
    });
    let log = RwSignal::new(load_orders_log());
    let sent = RwSignal::new(load_sent());
    let eth = RwSignal::new(load_eth());
    let me = neuron();

    let published = Memo::new(move |_| pdata.get().and_then(|d| d.deal).filter(|d| !d.status.is_empty()));
    let due = Memo::new(move |_| price.map(|p| p * cfg.get().share() as f64 / 100.0));
    let rent = Memo::new(move |_| price.map(|p| yearly_rent(p, cfg.get().share())));
    let st = Memo::new({
        let id = id.clone();
        move |_| {
            let sent_usd: f64 = sent_for(&sent.get(), &id).iter().map(|p| p.usd).sum();
            states(&cfg.get(), &published.get(), ordered(&log.get(), OrderKind::Kyc, "you"), sent_usd, due.get())
        }
    });
    // the step in view: the first one still to do, until the Holder picks another
    let picked = RwSignal::new(None::<usize>);
    let current = Memo::new(move |_| {
        picked.get().unwrap_or_else(|| st.get().iter().position(|s| *s == State::Todo).unwrap_or(STEPS.len() - 1))
    });
    let done_n = Memo::new(move |_| st.get().iter().filter(|s| matches!(s, State::Done(_))).count());

    let ctx = Ctx {
        id: id.clone(),
        name: name.clone(),
        flat: flat.clone(),
        book,
        price,
        neuron: me.bech32.clone(),
        cfg,
        deals,
        pdata,
        cx,
        log,
        sent,
        eth,
        published,
        due,
        rent,
    };

    let id_cancel = id.clone();
    let page_href = format!("{BASE}/plot/{id}");
    let summary = move || {
        let c = cfg.get();
        match (price, c.form) {
            (None, _) => "price on request".to_string(),
            (Some(p), Form::Upfront) => format!("{} · paid in full at signing", fmt_usd(p)),
            (Some(p), Form::Index) => format!("{} · {}% at signing, the rest as rent in CX", fmt_usd(p), c.pct),
        }
    };

    view! {
        <div class="page-shell cities-shell plot-shell">
            <div class="site-chrome cyberia-chrome">
                <div class="chrome-inner">
                    <div class="header-row1">
                        <div class="logo-zone">
                            <h1 class="logo">
                                <a href=format!("{BASE}/") class="brand-flag" title="home" inner_html=FLAG_SVG></a>
                                <span style="color: var(--cyber-green);">"cyber"</span>
                                <span style="color: var(--cyber-green); margin: 0 1px;">"•"</span>
                                <span style="color: #fff;">"ia"</span>
                            </h1>
                        </div>
                        <div class="cyberia-phase-pill is-real">
                            <span class="phase-dot" style:background=ink></span>
                            {move || format!("{} · DEAL · {} OF {}", name.clone(), done_n.get(), STEPS.len())}
                        </div>
                        <CyberiaNav active="map" />
                    </div>
                </div>
            </div>
            <div class="dl" style:--ink=ink>
                <aside class="dl-track">
                    <a class="dl-back" href=page_href>"← the flat"</a>
                    <div class="dl-flat">{bare_name(&flat.name)}</div>
                    <div class="dl-flat-s">{format!("{size} · {TERM_YEARS}-year leasehold")}</div>
                    <div class="dl-flat-s">{summary}</div>
                    <div class="dl-progress"><i style:width=move || format!("{:.0}%", done_n.get() as f64 / STEPS.len() as f64 * 100.0)></i></div>
                    <div class="dl-steps">
                        {STEPS.iter().enumerate().map(|(i, (_, title, line))| view! {
                            <button type="button"
                                class=move || {
                                    let s = st.get();
                                    let base = match &s[i] { State::Done(_) => "dl-step done", State::Waiting(_) => "dl-step wait", State::Todo => "dl-step" };
                                    if current.get() == i { format!("{base} on") } else { base.to_string() }
                                }
                                on:click=move |_| picked.set(Some(i))>
                                <span class="dl-step-n">{move || match &st.get()[i] { State::Done(_) => "✓".to_string(), State::Waiting(_) => "…".to_string(), State::Todo => (i + 1).to_string() }}</span>
                                <span class="dl-step-c">
                                    <b>{*title}</b>
                                    <i>{move || match &st.get()[i] { State::Done(s) | State::Waiting(s) => s.clone(), State::Todo => line.to_string() }}</i>
                                </span>
                            </button>
                        }).collect_view()}
                    </div>
                    <a class="dl-help" href=support_href() target="_blank" rel="noopener">{format!("a person answers · @{SUPPORT_HANDLE}")}</a>
                    {move || cfg.get().requested_ms.is_some().then(|| {
                        let id_c = id_cancel.clone();
                        view! {
                            <button type="button" class="dl-withdraw" on:click=move |_| {
                                deals.update(|d| cancel_deal(d, &id_c, "withdrawn by the requester"));
                                cfg.update(|c| c.requested_ms = None);
                            }>"withdraw the request"</button>
                        }
                    })}
                </aside>
                <section class="dl-panel">
                    <div class="dl-panel-h">
                        <span class="dl-panel-n">{move || format!("STEP {} OF {}", current.get() + 1, STEPS.len())}</span>
                        <h2>{move || STEPS[current.get()].1}</h2>
                        <p>{move || match &st.get()[current.get()] {
                            State::Done(s) => format!("done · {s}"),
                            State::Waiting(s) => s.clone(),
                            State::Todo => STEPS[current.get()].2.to_string(),
                        }}</p>
                    </div>
                    <div class="dl-body">
                        {move || panel(current.get(), &ctx)}
                    </div>
                    <div class="dl-nav">
                        <button type="button" class="dl-mini" disabled={move || current.get() == 0} on:click=move |_| picked.set(Some(current.get_untracked().saturating_sub(1)))>"← back"</button>
                        <button type="button" class="dl-mini" disabled={move || current.get() + 1 == STEPS.len()} on:click=move |_| picked.set(Some((current.get_untracked() + 1).min(STEPS.len() - 1)))>"next step →"</button>
                    </div>
                </section>
            </div>
        </div>
    }
    .into_any()
}
