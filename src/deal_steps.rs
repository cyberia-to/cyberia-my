//! The track of `/plot/:id/deal` and the first of its seven steps. Every
//! panel is a view over the same handful of things the page holds — the flat
//! and its lease book, the Holder’s own choices, what the valley published,
//! the two fetches — so they travel together in a `Ctx`, which lives here
//! with the small parts every step shares. Steps four to seven, where the
//! money moves and the papers are signed, are in `deal_close.rs`.

use crate::deal::DealCfg;
use crate::deal_docs::{annex_c, open_sheet, Offer};
use crate::deals::{fmt_ts, now_ms, open_deal, Deal};
use crate::docs::library;
use crate::eth::EthWallet;
use crate::globe::BASE;
use crate::land::LandFlat;
use crate::orders::{
    order_href, ordered, record_order, OrderKind, OrderRec, PlotData, PublishedDeal,
    SUPPORT_HANDLE,
};
use crate::pay::{Form, SentPayment, PREMIUM_STEPS, TERM_YEARS};
use crate::terms::{fmt_usd, Terms};
use leptos::prelude::*;
use std::collections::HashMap;

fn encode(s: &str) -> String {
    String::from(js_sys::encode_uri_component(s))
}

pub(crate) fn tg(text: &str) -> String {
    format!("https://t.me/{SUPPORT_HANDLE}?text={}", encode(text))
}

/// Everything a step panel reads. Signals are handles, so a `Ctx` is cheap
/// to clone into the closures a panel builds.
#[derive(Clone)]
pub struct Ctx {
    pub id: String,
    pub name: String,
    pub flat: LandFlat,
    pub book: Option<&'static Terms>,
    pub price: Option<f64>,
    /// the bech32 of this browser’s neuron
    pub neuron: String,
    pub cfg: RwSignal<DealCfg>,
    pub deals: RwSignal<HashMap<String, Deal>>,
    pub pdata: RwSignal<Option<PlotData>>,
    pub cx: RwSignal<Option<f64>>,
    pub log: RwSignal<Vec<OrderRec>>,
    pub sent: RwSignal<Vec<SentPayment>>,
    pub eth: RwSignal<Option<EthWallet>>,
    pub published: Memo<Option<PublishedDeal>>,
    pub due: Memo<Option<f64>>,
    pub rent: Memo<Option<f64>>,
}

impl Ctx {
    /// The offer as it stands. Once the Holder has signed, the moment and
    /// the index fix come back from the deal rather than from the clock and
    /// the fetch: the text under a signature must not move beneath it.
    pub fn offer(&self) -> Offer {
        let c = self.cfg.get_untracked();
        let (ts_ms, cx_level) = match c.offer_ts_ms {
            Some(t) => (t, c.offer_cx),
            None => (c.requested_ms.unwrap_or_else(now_ms), self.cx.get_untracked()),
        };
        Offer {
            plot_id: self.id.clone(),
            plot_name: self.name.clone(),
            price_usd: self.price.unwrap_or(0.0),
            form: c.form,
            premium_pct: c.pct,
            party: c.party,
            origin: c.origin,
            cx_level,
            neuron: self.neuron.clone(),
            ts_ms,
        }
    }

    pub(crate) fn offer_fn(&self) -> impl Fn() -> Offer + Clone {
        let c = self.clone();
        move || c.offer()
    }
}

/// The panel of the step in view.
pub fn panel(i: usize, c: &Ctx) -> AnyView {
    use crate::deal_close::{notary_step, papers_step, payment_step, yours_step};
    match i {
        0 => terms_step(c),
        1 => identity_step(c),
        2 => dossier_step(c),
        3 => payment_step(c),
        4 => papers_step(c),
        5 => notary_step(c),
        _ => yours_step(c),
    }
}

/// One of a pair of choices, as a button that shows which way it stands.
pub(crate) fn pick(
    cfg: RwSignal<DealCfg>,
    k: &'static str,
    field: fn(&mut DealCfg) -> &mut String,
    v: &'static str,
    label: &'static str,
    sub: &'static str,
) -> AnyView {
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
    .into_any()
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

/// TERMS — who holds the lease, how it is paid, and the request itself.
pub fn terms_step(c: &Ctx) -> AnyView {
    let (cfg, deals, cx, due, rent, price) = (c.cfg, c.deals, c.cx, c.due, c.rent, c.price);
    let (id, name) = (c.id.clone(), c.name.clone());
    let offer = c.offer_fn();
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
                    {pick(cfg, "party", |c| &mut c.party, "person", "A PERSON", "you, in your own name")}
                    {pick(cfg, "party", |c| &mut c.party, "company", "A COMPANY", "a legal entity signs")}
                </div>
                <div class="dl-opts">
                    {pick(cfg, "origin", |c| &mut c.origin, "abroad", "FROM ABROAD", "a foreign passport or a foreign company")}
                    {pick(cfg, "origin", |c| &mut c.origin, "id", "INDONESIA", "KTP · an Indonesian entity")}
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
    p_terms().into_any()
}

/// IDENTITY — what the Holder sends, and the two keys of this browser.
pub fn identity_step(c: &Ctx) -> AnyView {
    let (cfg, log, eth) = (c.cfg, c.log, c.eth);
    let neuron_id = c.neuron.clone();
    let p_kyc = move || {
        let c = cfg.get();
        let sent_at = ordered(&log.get(), OrderKind::Kyc, "you");
        let short = { let b = neuron_id.clone(); if b.len() > 22 { format!("{}…{}", &b[..12], &b[b.len() - 6..]) } else { b } };
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
                        Some(w) => view! {
                            <b class="mono">{w.short()}</b>
                            <span class="dl-s">{if w.wallet.is_empty() { "the flat's token goes here".to_string() } else { format!("{} · the flat's token goes here", w.wallet) }}</span>
                        }.into_any(),
                        None => view! {
                            <span class="dl-s">"optional · where the flat's token is sent"</span>
                            <crate::wallet_ui::WalletConnect on_connected=Callback::new(move |w| eth.set(Some(w))) />
                        }.into_any(),
                    }}
                </div>
            </div>
        }
    };
    p_kyc().into_any()
}

/// DOSSIER — the papers of the land and of the Company, and the mark of reading.
pub fn dossier_step(c: &Ctx) -> AnyView {
    let (cfg, log, pdata, book) = (c.cfg, c.log, c.pdata, c.book);
    let (id, name, flat) = (c.id.clone(), c.name.clone(), c.flat.clone());
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
    p_dd().into_any()
}
