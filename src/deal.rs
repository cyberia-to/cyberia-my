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

use crate::deal_docs::{annex_b, annex_c, annex_d, annex_e, open_sheet, Offer};
use crate::deals::{cancel_deal, fmt_ts, load_deals, now_ms, open_deal};
use crate::docs::library;
use crate::eth::{connect, has_provider, load_eth};
use crate::globe::BASE;
use crate::land::{area_m2, load_map, FLAG_SVG};
use crate::nav::CyberiaNav;
use crate::orders::{
    load_orders_log, order_href, ordered, record_order, support_href, OrderKind, PlotData,
    PublishedDeal, SUPPORT_HANDLE,
};
use crate::pay::{
    load_sent, new_sent, record_sent, reference, sent_for, sent_text, yearly_rent, Form, BANK,
    ETH_ADDRESS, PREMIUM_STEPS, TERM_YEARS,
};
use crate::signal::{neuron, sign_document, verify_document, DocSignature};
use crate::terms::{bare_name, fmt_usd, land_use, size_line, terms};
use leptos::prelude::*;
use leptos_router::hooks::use_params_map;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

fn encode(s: &str) -> String {
    String::from(js_sys::encode_uri_component(s))
}

fn tg(text: &str) -> String {
    format!("https://t.me/{SUPPORT_HANDLE}?text={}", encode(text))
}

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

/// What the Holder sends for KYC, by who they are.
fn kyc_list(cfg: &DealCfg) -> Vec<&'static str> {
    match (cfg.party.as_str(), cfg.origin.as_str()) {
        ("company", "id") => vec![
            "NIB and the deed of establishment with its latest amendment",
            "NPWP of the company",
            "KTP of the director who signs",
            "the board's resolution to take the lease, if the articles ask for one",
        ],
        ("company", _) => vec![
            "certificate of incorporation and the articles",
            "a register or extract naming the directors",
            "passport page of the director who signs",
            "the board's resolution to take the lease",
        ],
        (_, "id") => vec!["KTP", "NPWP", "a selfie with the KTP"],
        _ => vec!["the photo page of your passport", "a selfie with the passport", "your current visa or stay permit, if you are in Indonesia"],
    }
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
    let eth_note = RwSignal::new(String::new());
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

    let offer = {
        let (id, name, neuron_id) = (id.clone(), name.clone(), me.bech32.clone());
        move || {
            let c = cfg.get_untracked();
            Offer {
                plot_id: id.clone(),
                plot_name: name.clone(),
                price_usd: price.unwrap_or(0.0),
                form: c.form,
                premium_pct: c.pct,
                party: c.party,
                origin: c.origin,
                cx_level: cx.get_untracked(),
                neuron: neuron_id.clone(),
                ts_ms: c.requested_ms.unwrap_or_else(now_ms),
            }
        }
    };

    // ── step panels ───────────────────────────────────────────────────

    let pick = move |k: &'static str, field: fn(&mut DealCfg) -> &mut String, v: &'static str, label: &'static str, sub: &'static str| {
        let on = Memo::new(move |_| {
            let mut c = cfg.get();
            *field(&mut c) == v
        });
        view! {
            <button type="button" class=move || if on.get() { "dl-opt is-on" } else { "dl-opt" } data-k=k
                on:click=move |_| cfg.update(|c| *field(c) = v.to_string())>
                <span class="dl-opt-t">{label}</span>
                <span class="dl-opt-s">{sub}</span>
            </button>
        }
    };

    let p_terms = {
        let (id, name) = (id.clone(), name.clone());
        let offer = offer.clone();
        move || {
            let (id, name) = (id.clone(), name.clone());
            let offer = offer.clone();
            let locked = cfg.get().requested_ms.is_some();
            view! {
                <div class="dl-q">"who holds the lease"</div>
                <div class="dl-opts">
                    {pick("party", |c| &mut c.party, "person", "A PERSON", "you, in your own name")}
                    {pick("party", |c| &mut c.party, "company", "A COMPANY", "a legal entity signs")}
                </div>
                <div class="dl-opts">
                    {pick("origin", |c| &mut c.origin, "abroad", "FROM ABROAD", "a foreign passport or a foreign company")}
                    {pick("origin", |c| &mut c.origin, "id", "INDONESIA", "KTP · an Indonesian entity")}
                </div>
                <div class="dl-q">"how you pay"</div>
                <div class="dl-opts">
                    {[Form::Upfront, Form::Index].into_iter().map(|f| view! {
                        <button type="button" class=move || if cfg.get().form == f { "dl-opt is-on" } else { "dl-opt" }
                            on:click=move |_| cfg.update(|c| c.form = f)>
                            <span class="dl-opt-t">{f.label()}</span>
                            <span class="dl-opt-s">{f.sub()}</span>
                        </button>
                    }).collect_view()}
                </div>
                {move || (cfg.get().form == Form::Index).then(|| view! {
                    <div class="dl-pct">
                        <span class="dl-k">"premium"</span>
                        {PREMIUM_STEPS.iter().map(|&s| view! {
                            <button type="button" class=move || if cfg.get().pct == s { "pay-step is-on" } else { "pay-step" } on:click=move |_| cfg.update(|c| c.pct = s)>{format!("{s}%")}</button>
                        }).collect_view()}
                    </div>
                })}
                <div class="dl-sum">
                    <div><span class="dl-k">"price"</span><b>{price.map(fmt_usd).unwrap_or_else(|| "on request".into())}</b></div>
                    <div><span class="dl-k">{move || format!("at signing · {}%", cfg.get().share())}</span><b class="go">{move || due.get().map(fmt_usd).unwrap_or_default()}</b></div>
                    {move || (cfg.get().form == Form::Index).then(|| {
                        let r = rent.get().unwrap_or(0.0);
                        view! {
                            <div><span class="dl-k">{format!("rent, each of {TERM_YEARS} years")}</span><b class="cx">{match cx.get() {
                                Some(l) => format!("{:.0} CX ≈ {}", r / l, fmt_usd(r)),
                                None => format!("≈ {}", fmt_usd(r)),
                            }}</b></div>
                        }
                    })}
                </div>
                {if locked {
                    view! { <p class="dl-note">"the request is with the valley — change a choice here and tell support in the chat"</p> }.into_any()
                } else {
                    view! {
                        <button type="button" class="dl-go" on:click=move |_| {
                            cfg.update(|c| c.requested_ms = Some(now_ms()));
                            deals.update(|d| open_deal(d, &id, &name));
                            let o = offer();
                            let text = format!("LEASE REQUEST {}\n{}", o.plot_id, o.canonical());
                            if let Some(w) = web_sys::window() {
                                let _ = w.open_with_url_and_target(&tg(&text), "_blank");
                            }
                        }>"SEND THE REQUEST →"</button>
                        <p class="dl-note">"opens a chat with the valley with your terms filled in · the flat is marked as requested by you"</p>
                    }.into_any()
                }}
            }
        }
    };

    let p_kyc = move || {
        let c = cfg.get();
        let sent_at = ordered(&log.get(), OrderKind::Kyc, "you");
        let short = { let b = me.bech32.clone(); if b.len() > 22 { format!("{}…{}", &b[..12], &b[b.len() - 6..]) } else { b } };
        view! {
            <div class="dl-q">{format!("send these — {}, {}", if c.party == "company" { "a company" } else { "a person" }, if c.origin == "id" { "Indonesia" } else { "from abroad" })}</div>
            <ul class="dl-list">
                {kyc_list(&c).into_iter().map(|s| view! { <li>{s}</li> }).collect_view()}
            </ul>
            <a class="dl-go" href=order_href(OrderKind::Kyc, "you", "") target="_blank" rel="noopener"
                on:click=move |_| log.update(|l| record_order(l, OrderKind::Kyc, "you"))>
                {if sent_at.is_some() { "SEND AGAIN IN A DIRECT MESSAGE →" } else { "SEND IN A DIRECT MESSAGE →" }}
            </a>
            <p class="dl-note">"to support, person to person · nothing is stored on our servers"</p>
            <div class="dl-q">"your keys"</div>
            <div class="dl-keys">
                <div><span class="dl-k">"neuron"</span><b class="mono">{short}</b><span class="dl-s">"made in this browser · signs your offer in step 5"</span></div>
                <div>
                    <span class="dl-k">"ethereum"</span>
                    {move || match eth.get() {
                        Some(w) => view! { <b class="mono">{w.short()}</b><span class="dl-s">"the flat's token goes here"</span> }.into_any(),
                        None => view! {
                            <button type="button" class="dl-mini" on:click=move |_| {
                                if !has_provider() { eth_note.set("no wallet in this browser — optional, add it any time in the cabinet".into()); return; }
                                leptos::task::spawn_local(async move {
                                    match connect().await { Ok(w) => eth.set(Some(w)), Err(e) => eth_note.set(e) }
                                });
                            }>"CONNECT A WALLET"</button>
                            <span class="dl-s">{move || { let n = eth_note.get(); if n.is_empty() { "optional · where the flat's token is sent".to_string() } else { n } }}</span>
                        }.into_any(),
                    }}
                </div>
            </div>
        }
    };

    let p_dd = {
        let (id, name, flat) = (id.clone(), name.clone(), flat.clone());
        move || {
            let lib = library();
            let count = |k: &str| lib.iter().find(|s| s.key == k).map(|s| (s.items.len(), s.items.iter().filter(|i| !i.cid.is_empty()).count())).unwrap_or((0, 0));
            let (land, land_p) = count("land");
            let (co, co_p) = count("company");
            let (sign, _) = count("sign");
            let survey = pdata.get().and_then(|d| d.survey).map(|s| format!("{:.2} m² · {}", s.m2, s.on))
                .or_else(|| book.and_then(|t| t.measured_m2.map(|m| format!("{m:.2} m² · {}", t.measured_on))));
            let read = cfg.get().dd_read_ms;
            let flat_c = flat.clone();
            view! {
                <div class="dl-tiles">
                    <a class="dl-tile" href=format!("{BASE}/docs#land") target="_blank">
                        <b>{land}</b><span>"papers of the land"</span><i>{format!("{land_p} on IPFS by content hash")}</i>
                    </a>
                    <a class="dl-tile" href=format!("{BASE}/docs#company") target="_blank">
                        <b>{co}</b><span>"papers of the Company"</span><i>{format!("{co_p} on IPFS · PT Cyber Valley Estate")}</i>
                    </a>
                    <a class="dl-tile" href=format!("{BASE}/docs#sign") target="_blank">
                        <b>{sign}</b><span>"instruments you sign"</span><i>"the agreement and annexes A–G"</i>
                    </a>
                </div>
                <div class="dl-q">"this flat's ground"</div>
                <div class="dl-rows">
                    <div><span class="dl-k">"survey"</span>
                        {match survey {
                            Some(s) => view! { <b>{s}</b> }.into_any(),
                            None => view! {
                                <b class="dim">"not surveyed yet"</b>
                                <a class="dl-mini" href=order_href(OrderKind::Survey, &id, &name) target="_blank" rel="noopener"
                                    on:click={ let id = id.clone(); move |_| log.update(|l| record_order(l, OrderKind::Survey, &id)) }>"ORDER A SURVEY"</a>
                            }.into_any(),
                        }}
                    </div>
                    <div><span class="dl-k">"land office search"</span><b class="dim">"ordered through the notary within 30 days of signing"</b></div>
                </div>
                <div class="dl-actions">
                    <button type="button" class="dl-mini" on:click=move |_| open_sheet(&annex_c(&flat_c, cfg.get_untracked().dd_read_ms))>"OPEN THE INVENTORY · ANNEX C"</button>
                </div>
                {match read {
                    Some(ts) => view! { <p class="dl-note ok">{format!("✓ you marked the dossier as read · {}", fmt_ts(ts))}</p> }.into_any(),
                    None => view! {
                        <button type="button" class="dl-go" on:click=move |_| cfg.update(|c| c.dd_read_ms = Some(now_ms()))>"I HAVE READ THE DOSSIER"</button>
                        <p class="dl-note">"your own mark — it goes into the record of handover, Annex C"</p>
                    }.into_any(),
                }}
            }
        }
    };

    let p_pay = {
        let (id, name) = (id.clone(), name.clone());
        move || {
            let (id, name) = (id.clone(), name.clone());
            let Some(due_usd) = due.get() else {
                return view! { <p class="dl-note">"the price of this flat is set on request — the valley names it in the chat, then this step opens"</p> }.into_any();
            };
            let pd = published.get();
            let paid = pd.as_ref().map(|p| p.paid_usd).unwrap_or(0.0);
            let payments = pd.map(|p| p.payments).unwrap_or_default();
            let mine = sent_for(&sent.get(), &id);
            let pct = ((paid / due_usd) * 100.0).clamp(0.0, 100.0);
            let id_ref = id.clone();
            view! {
                <div class="dl-meter">
                    <div class="dl-meter-top"><span>{format!("{} received of {}", fmt_usd(paid), fmt_usd(due_usd))}</span><span>{format!("{pct:.0}%")}</span></div>
                    <div class="dl-meter-bar"><i style:width=format!("{pct:.1}%")></i></div>
                </div>
                <div class="dl-opts">
                    {pick("rail", |c| &mut c.rail, "bank", "BANK TRANSFER", "USD · EUR · AUD · IDR")}
                    {pick("rail", |c| &mut c.rail, "eth", "ETHEREUM", "ETH or USDT on mainnet")}
                </div>
                {move || if cfg.get().rail == "bank" {
                    let id_now = id_ref.clone();
                    view! {
                        <div class="dl-bank">
                            <div><span class="dl-k">"account name"</span><b>{BANK.holder}</b></div>
                            <div><span class="dl-k">"account number"</span><b class="mono">{BANK.number}</b></div>
                            <div><span class="dl-k">"SWIFT"</span><b class="mono">{BANK.swift}</b></div>
                            <div><span class="dl-k">"reference"</span><b class="mono">{move || reference(&id_now, cfg.get().share())}</b></div>
                            <div><span class="dl-k">"bank"</span><b>{format!("{} · {} · {}", BANK.bank, BANK.branch, BANK.bank_address)}</b></div>
                            <div><span class="dl-k">"currencies"</span><b>{BANK.currencies}</b></div>
                            <div><span class="dl-k">"holder address"</span><b>{BANK.holder_address}</b></div>
                        </div>
                        <p class="dl-note">"USD, EUR and AUD land as sent; IDR settles at Bank Indonesia's JISDOR rate on the invoice date"</p>
                    }.into_any()
                } else {
                    view! {
                        <div class="dl-bank">
                            <div><span class="dl-k">"address"</span><b class="mono">{ETH_ADDRESS.unwrap_or("given by support in a direct message")}</b></div>
                        </div>
                        <p class="dl-note">"ETH or USDT on Ethereum mainnet · the Company's address, named 2026-10-22"</p>
                    }.into_any()
                }}
                <button type="button" class="dl-go" on:click=move |_| {
                    let c = cfg.get_untracked();
                    let p = new_sent(&id, due_usd, c.share(), c.form, &c.rail);
                    let text = sent_text(&p, &name);
                    sent.update(|l| record_sent(l, p));
                    if let Some(w) = web_sys::window() {
                        let _ = w.open_with_url_and_target(&tg(&text), "_blank");
                    }
                }>{format!("I HAVE SENT {} →", fmt_usd(due_usd))}</button>
                <p class="dl-note">"tells the valley in the chat · the Company confirms the receipt and it shows here as received"</p>
                {(!mine.is_empty() || !payments.is_empty()).then(|| view! {
                    <div class="dl-q">"the ledger of this deal"</div>
                    <div class="dl-ledger">
                        {payments.into_iter().map(|p| view! {
                            <div class="ok"><span>"RECEIVED"</span><b>{fmt_usd(p.usd)}</b><i>{format!("{} · {} · {}", p.method, p.reference, p.on)}</i></div>
                        }).collect_view()}
                        {mine.into_iter().map(|p| view! {
                            <div><span>"SENT"</span><b>{fmt_usd(p.usd)}</b><i>{format!("{} · {} · {} · awaiting receipt", p.method, p.reference, fmt_ts(p.ts_ms))}</i></div>
                        }).collect_view()}
                    </div>
                })}
            }.into_any()
        }
    };

    let p_docs = {
        let flat = flat.clone();
        let offer = offer.clone();
        move || {
            let c = cfg.get();
            let index_form = c.form == Form::Index;
            let sig = c.offer_sig.clone();
            let signed_text = c.offer_text.clone();
            let valid = match (&signed_text, &sig) {
                (Some(t), Some(s)) => verify_document(t, s),
                _ => false,
            };
            // the offer changed after it was signed: the signature no longer covers it
            let stale = signed_text.as_ref().map(|t| *t != offer().canonical()).unwrap_or(false);
            let (f_b, o_d, o_e, o_sign, o_send) = (flat.clone(), offer.clone(), offer.clone(), offer.clone(), offer.clone());
            let sig_d = sig.clone();
            let row = |made: bool, title: &'static str, what: &'static str, key: &'static str| view! {
                <a class="dl-doc" href=format!("{BASE}/docs/{key}") target="_blank">
                    <span class=if made { "dl-doc-tag made" } else { "dl-doc-tag" }>{if made { "FOR THIS FLAT" } else { "TEMPLATE" }}</span>
                    <b>{title}</b><i>{what}</i>
                </a>
            };
            view! {
                <div class="dl-q">"the set for this deal"</div>
                <div class="dl-docs">
                    {row(false, "Lease Agreement", "Perjanjian Hak Sewa · the text the notary draws the deed from", "signable-pack")}
                    {row(false, "Annex A — Estate Registry", "the Title the flat sits on", "signable-04-annex-a")}
                    <button type="button" class="dl-doc" on:click=move |_| open_sheet(&annex_b(&f_b))>
                        <span class="dl-doc-tag made">"MADE FOR THIS FLAT"</span><b>"Annex B — Plot Boundary"</b><i>"corners, sides, neighbours, the drawing · open the filled sheet"</i>
                    </button>
                    <button type="button" class="dl-doc" on:click={ let f = flat.clone(); move |_| open_sheet(&annex_c(&f, cfg.get_untracked().dd_read_ms)) }>
                        <span class="dl-doc-tag made">"MADE FOR THIS DEAL"</span><b>"Annex C — Due Diligence Pack"</b><i>"every paper by its content hash, and your mark of reading"</i>
                    </button>
                    <button type="button" class="dl-doc" on:click=move |_| open_sheet(&annex_d(&o_d(), sig_d.as_ref()))>
                        <span class="dl-doc-tag made">"MADE FOR THIS DEAL"</span><b>"Annex D — Offer of Record"</b><i>"the page and the price you were shown · carries your signature"</i>
                    </button>
                    {index_form.then(|| view! {
                        <button type="button" class="dl-doc" on:click=move |_| open_sheet(&annex_e(&o_e()))>
                            <span class="dl-doc-tag made">"MADE FOR THIS DEAL"</span><b>"Annex E — Century Index"</b><i>"your rent in CX and the rule that converts it"</i>
                        </button>
                    })}
                    {row(false, "Annex F — Utilities, Access and Mobility", "self-sufficiency norms · filled per flat by the valley", "signable-09-annex-f")}
                    {row(false, "Annex G — Handover Condition", "the record of handover · filled on the day", "signable-10-annex-g")}
                </div>
                <div class="dl-q">"sign the offer"</div>
                {match (sig, valid && !stale) {
                    (Some(s), true) => view! {
                        <div class="dl-signed">
                            <b>"✓ SIGNED BY YOUR NEURON · THE SIGNATURE VERIFIES"</b>
                            <span class="mono">{format!("particle {}…{}", &s.particle[..12], &s.particle[s.particle.len() - 8..])}</span>
                            <span>{format!("{} · {}", fmt_ts(s.ts_ms), s.neuron)}</span>
                        </div>
                        <button type="button" class="dl-go" on:click=move |_| {
                            let o = o_send();
                            let text = format!("SIGNED OFFER {}\nparticle: {}\nneuron: {}\npubkey: {}\nsignature: {}\n\n{}", o.plot_id, s.particle, s.neuron, s.pubkey_hex, s.sig_hex, o.canonical());
                            if let Some(w) = web_sys::window() {
                                let _ = w.open_with_url_and_target(&tg(&text), "_blank");
                            }
                        }>"SEND THE SIGNED OFFER →"</button>
                        <p class="dl-note">"the Company countersigns; the notary draws the Indonesian deed from the same figures"</p>
                    }.into_any(),
                    (had, _) => view! {
                        {had.is_some().then(|| view! { <p class="dl-note warn">"your terms changed after you signed — the old signature no longer covers the offer; sign it again"</p> })}
                        <button type="button" class="dl-go" on:click=move |_| {
                            let text = o_sign().canonical();
                            let s = sign_document(&text);
                            cfg.update(|c| { c.offer_text = Some(text.clone()); c.offer_sig = Some(s.clone()); });
                        }>"SIGN THE OFFER WITH YOUR NEURON"</button>
                        <p class="dl-note">"an electronic signature made in this browser: the offer's text is hashed into a particle and signed with your key (ADR-036) · anyone can verify it · the deed itself is signed at the notary"</p>
                    }.into_any(),
                }}
            }
        }
    };

    let p_notary = {
        let (id, name) = (id.clone(), name.clone());
        move || {
            let (id, name) = (id.clone(), name.clone());
            let asked = cfg.get().notary_asked_ms;
            view! {
                <div class="dl-q">"how you sign the deed"</div>
                <div class="dl-opts">
                    {pick("notary", |c| &mut c.notary, "person", "IN PERSON", "at the notary's table in Bali, one visit")}
                    {pick("notary", |c| &mut c.notary, "proxy", "BY PROXY", "a power of attorney, signed where you are")}
                </div>
                <ul class="dl-list">
                    {move || if cfg.get().notary == "proxy" {
                        view! {
                            <li>"you sign a power of attorney naming your proxy — notarised and, from abroad, legalised or apostilled"</li>
                            <li>"the original travels to Bali; the proxy signs the deed for you"</li>
                            <li>"the notary reads the deed in Bahasa Indonesia; your second-language copy is beside it"</li>
                        }.into_any()
                    } else {
                        view! {
                            <li>"bring the passport or KTP you sent for KYC"</li>
                            <li>"the notary reads the deed in Bahasa Indonesia; a sworn interpreter sits with you if you ask"</li>
                            <li>"both sides sign; you leave with your copy of the deed"</li>
                        }.into_any()
                    }}
                    <li>"the land office search is ordered by the notary within 30 days of signing and pinned to the dossier"</li>
                </ul>
                <a class="dl-go" href=tg(&format!("NOTARY {id}\n{name} — I am ready to set a date for the deed.\nhow: {}", if cfg.get_untracked().notary == "proxy" { "by proxy" } else { "in person" }))
                    target="_blank" rel="noopener" on:click=move |_| cfg.update(|c| c.notary_asked_ms = Some(now_ms()))>
                    {if asked.is_some() { "ASK AGAIN FOR A DATE →" } else { "SET A DATE WITH THE NOTARY →" }}
                </a>
            }
        }
    };

    let p_yours = {
        let id = id.clone();
        move || {
            let owned = published.get().map(|p| p.status == "owned").unwrap_or(false);
            view! {
                <div class="dl-rows">
                    <div><span class="dl-k">"handover"</span><b class=if owned { "" } else { "dim" }>{if owned { "the flat is handed over" } else { "Annex G is filled on the ground, with you or your proxy" }}</b></div>
                    <div><span class="dl-k">"the flat's token"</span><b class="dim">"minted to your Ethereum address once the deed is signed"</b>
                        <a class="dl-mini" href=format!("/plot/{id}/metadata.json") target="_blank">"TOKEN METADATA"</a></div>
                    <div><span class="dl-k">"your room"</span><b class="dim">"every flat you hold, with its ledger"</b>
                        <a class="dl-mini" href=format!("{BASE}/cabinet")>"OPEN THE CABINET"</a></div>
                </div>
                {owned.then(|| view! { <p class="dl-note ok">"✓ the leasehold is yours for the term"</p> })}
            }
        }
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
                        {move || match current.get() {
                            0 => p_terms().into_any(),
                            1 => p_kyc().into_any(),
                            2 => p_dd().into_any(),
                            3 => p_pay().into_any(),
                            4 => p_docs().into_any(),
                            5 => p_notary().into_any(),
                            _ => p_yours().into_any(),
                        }}
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
