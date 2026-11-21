//! `/cabinet` — a Holder's own room, as a menu on the left and one thing on
//! the right, like the desk and the way to sell:
//!
//!   YOU       where you stand, and the one thing to do next
//!   IDENTITY  the neuron of this browser, the Ethereum wallet, KYC
//!   FLATS     your shelves, and the ways to a flat
//!   MARKET    your offers and mandates, once the wallet has signed in
//!   PAPERS    the dossiers and what you sign
//!   HELP      the people who answer
//!
//! Everything personal lives in this browser; the valley's files live on IPFS.

use crate::deals::{fmt_ts, load_deals};
use crate::docs::library;
use crate::eth::{forget_eth, load_eth};
use crate::globe::BASE;
use crate::land::FLAG_SVG;
use crate::market::{self, View};
use crate::nav::CyberiaNav;
use crate::orders::{
    community_href, load_orders_log, order_href, ordered, record_order, support_href, OrderKind,
    SUPPORT_HANDLE,
};
use crate::pay::load_sent;
use crate::sell::enter;
use crate::signal::neuron;
use crate::terms::fmt_usd;
use leptos::prelude::*;

/// How many flats this browser has on its shelves — a deal, a payment, a
/// star. The flats themselves live on `/my`.
fn shelved() -> usize {
    let mut ids: std::collections::BTreeSet<String> = load_deals().into_iter().filter(|(_, d)| !d.cancelled).map(|(k, _)| k).collect();
    ids.extend(load_sent().into_iter().map(|p| p.plot));
    ids.extend(crate::my::load_watch());
    ids.len()
}

/// menu · its line · its submenus
const MENU: &[(&str, &str, &[&str])] = &[
    ("YOU", "where you stand", &[]),
    ("IDENTITY", "neuron, wallet, KYC", &["NEURON", "WALLET", "KYC"]),
    ("FLATS", "your shelves", &[]),
    ("MARKET", "your offers and mandates", &[]),
    ("PAPERS", "what stands behind a flat", &[]),
    ("HELP", "the people who answer", &[]),
];

#[component]
pub fn CabinetPage() -> impl IntoView {
    document().set_title("Cyberia — cabinet");
    let log = RwSignal::new(load_orders_log());
    let eth = RwSignal::new(load_eth());
    let view = RwSignal::new(None::<View>);
    let busy = RwSignal::new(false);
    let err = RwSignal::new(String::new());
    leptos::task::spawn_local(async move {
        if let Some(v) = market::fetch_me().await {
            view.set(Some(v));
        }
    });
    let menu = RwSignal::new(0usize);
    let sub = RwSignal::new(0usize);
    let go = move |m: usize, s: usize| {
        menu.set(m);
        sub.set(s);
    };
    let n = neuron();
    let kyc_sent = move || ordered(&log.get(), OrderKind::Kyc, "you");
    let flats_n = shelved();

    // the mark beside each menu: done, waiting, or how much is there
    let mark = move |i: usize| -> (String, &'static str) {
        match i {
            1 => match (eth.get().is_some(), kyc_sent().is_some()) {
                (true, true) => ("✓".into(), "done"),
                (w, k) => (format!("{}/3", 1 + w as usize + k as usize), "wait"),
            },
            2 => (flats_n.to_string(), if flats_n > 0 { "done" } else { "" }),
            3 => match view.get() {
                Some(v) => ((v.listings.len() + v.mandates.len()).to_string(), "done"),
                None => ("·".into(), ""),
            },
            _ => ("·".into(), ""),
        }
    };

    let short_neuron = { let b = n.bech32.clone(); if b.len() > 20 { format!("{}…{}", &b[..12], &b[b.len() - 6..]) } else { b } };
    let neuron_full = n.bech32.clone();
    let sn = short_neuron.clone();

    let p_you = move || {
        let w = eth.get();
        let k = kyc_sent();
        // the one thing to do next, in the order a holder needs them
        let (next_t, next_s, next_m, next_sub): (&str, &str, usize, usize) = if flats_n == 0 {
            ("PICK YOUR FIRST FLAT", "the map shows what is open, cheapest first", 2, 0)
        } else if k.is_none() {
            ("SEND YOUR KYC", "a passport page and a selfie, person to person", 1, 2)
        } else if w.is_none() {
            ("CONNECT A WALLET", "where your flat's token is sent, and your signature on the market", 1, 1)
        } else {
            ("OPEN MY PLOTS", "your flats on the map, each with its next step", 2, 0)
        };
        view! {
            <div class="ad-tiles cb-tiles">
                <button type="button" class="ad-tile go" on:click=move |_| go(1, 0)><span>"neuron"</span><b>"✓"</b><i>{sn.clone()}</i></button>
                <button type="button" class={ let on = w.is_some(); if on { "ad-tile go" } else { "ad-tile warn" } } on:click=move |_| go(1, 1)>
                    <span>"wallet"</span><b>{if w.is_some() { "✓" } else { "—" }}</b><i>{w.as_ref().map(|w| format!("{} · {}", w.short(), w.wallet)).unwrap_or_else(|| "not connected".into())}</i>
                </button>
                <button type="button" class=if k.is_some() { "ad-tile go" } else { "ad-tile warn" } on:click=move |_| go(1, 2)>
                    <span>"KYC"</span><b>{if k.is_some() { "…" } else { "—" }}</b><i>{k.map(|t| format!("sent {}", fmt_ts(t))).unwrap_or_else(|| "not sent".into())}</i>
                </button>
                <button type="button" class="ad-tile" on:click=move |_| go(2, 0)><span>"flats"</span><b>{flats_n}</b><i>"on your shelves"</i></button>
            </div>
            <div class="dl-q">"next"</div>
            <button type="button" class="pd-cta cb-next" on:click=move |_| go(next_m, next_sub)><b>{next_t}</b><span>{format!("{next_s} →")}</span></button>
            <p class="dl-note">"your room lives in this browser: the neuron, the wallet's address, your flats and what you sent. the valley keeps nothing of yours on its servers."</p>
        }.into_any()
    };

    let p_identity = move || match sub.get() {
        1 => match eth.get() {
            Some(w) => view! {
                <div class="dl-rows">
                    <div><span class="dl-k">"address"</span><b class="mono">{w.address.clone()}</b></div>
                    <div><span class="dl-k">"wallet"</span><b>{if w.wallet.is_empty() { "the browser's wallet".to_string() } else { w.wallet.clone() }}</b></div>
                    <div><span class="dl-k">"chain"</span><b>{w.chain_name()}</b></div>
                    <div><span class="dl-k">"connected"</span><b>{fmt_ts(w.connected_ms)}</b></div>
                </div>
                <p class="dl-note">"this address receives your flat's token once the deed is signed, and signs for you on the market: an offer, a mandate to your agent."</p>
                <div class="dl-actions">
                    <button type="button" class="dl-mini" on:click=move |_| go(3, 0)>"YOUR MARKET →"</button>
                    <button type="button" class="dl-mini warn" on:click=move |_| { forget_eth(); market::sign_out(); eth.set(None); view.set(None); }>"FORGET THIS WALLET"</button>
                </div>
            }.into_any(),
            None => view! {
                <p class="dl-note">"a wallet holds your flat's token on Ethereum and is your signature on the market. paying by bank transfer needs none."</p>
                <crate::wallet_ui::WalletConnect on_connected=Callback::new(move |w| eth.set(Some(w))) />
            }.into_any(),
        },
        2 => {
            let sent = kyc_sent();
            view! {
                <p class="dl-note">{OrderKind::Kyc.line()}</p>
                {sent.map(|ts| view! {
                    <div class="dl-rows">
                        <div><span class="dl-k">"sent"</span><b>{format!("to @{SUPPORT_HANDLE} · {}", fmt_ts(ts))}</b></div>
                        <div><span class="dl-k">"state"</span><b class="dim">"support confirms in the same chat; a deal's track then shows it confirmed"</b></div>
                    </div>
                })}
                <a class="dl-go" href=order_href(OrderKind::Kyc, "you", "") target="_blank" rel="noopener" on:click=move |_| log.update(|l| record_order(l, OrderKind::Kyc, "you"))>
                    {if sent.is_some() { "SEND KYC AGAIN →" } else { "SEND KYC IN A DIRECT MESSAGE →" }}
                </a>
            }.into_any()
        }
        _ => view! {
            <div class="dl-rows">
                <div><span class="dl-k">"neuron"</span><b class="mono">{neuron_full.clone()}</b></div>
            </div>
            <p class="dl-note">"made in this browser when you first came. it signs your signals and the offer of a deal. it is not a wallet and holds no money; clearing this browser's data makes a new one."</p>
        }.into_any(),
    };

    let p_flats = move || view! {
        <div class="ad-tiles cb-tiles">
            <div class="ad-tile go"><span>"on your shelves"</span><b>{flats_n}</b><i>"held, in deal, watching"</i></div>
        </div>
        <a class="pd-cta cb-next" href=format!("{BASE}/my")><b>"OPEN MY PLOTS"</b><span>"your flats on the map, each with its deal and its offer →"</span></a>
        <div class="mp-links">
            <a href=format!("{BASE}/map")>"PICK A FLAT ON THE MAP"</a>
            <a href=format!("{BASE}/plots")>"ALL FLATS AS CARDS"</a>
            <a href=format!("{BASE}/sell")>"SELL OR SUBLEASE A FLAT"</a>
            <a href=format!("{BASE}/docs")>"THE DOCUMENTS"</a>
        </div>
    }.into_any();

    let p_market = move || {
        let Some(v) = view.get() else {
            return match eth.get() {
                Some(w) => view! {
                    <p class="dl-note">"the market knows a wallet by its signature. sign one line to see the flats the valley bound to this address, your offers and your mandates."</p>
                    <button type="button" class="dl-go" disabled=move || busy.get() on:click=move |_| enter(view, busy, err, w.clone())>"SIGN IN TO THE MARKET →"</button>
                }.into_any(),
                None => view! {
                    <p class="dl-note">"connect a wallet first — it is your signature on the market"</p>
                    <crate::wallet_ui::WalletConnect on_connected=Callback::new(move |w: crate::eth::EthWallet| { eth.set(Some(w.clone())); enter(view, busy, err, w); }) />
                }.into_any(),
            };
        };
        let live = v.listings.iter().filter(|l| l.status == "live").count();
        let pending = v.listings.iter().filter(|l| l.status == "pending").count();
        let mandates = v.mandates.iter().filter(|m| m.status == "live").count();
        view! {
            <div class="ad-tiles cb-tiles">
                <div class="ad-tile go"><span>"flats bound to your wallet"</span><b>{v.holders.len()}</b><i>"by the valley"</i></div>
                <div class="ad-tile"><span>"offers"</span><b>{live}</b><i>{format!("live · {pending} waiting for approval")}</i></div>
                <div class="ad-tile"><span>"mandates"</span><b>{mandates}</b><i>"in force"</i></div>
            </div>
            {(!v.listings.is_empty()).then(|| view! {
                <div class="sl-cards">
                    {v.listings.iter().rev().take(6).map(|l| view! {
                        <div class=format!("sl-card {}", l.status)>
                            <div class="sl-card-top"><b>{l.plot.to_uppercase()}</b><span class="sl-tag">{l.status.to_uppercase()}</span></div>
                            <div class="sl-card-line">{format!("{} · {}", if l.deal == "lease" { "sublease" } else { "resale" }, fmt_usd(l.price_usd))}</div>
                        </div>
                    }).collect_view()}
                </div>
            })}
            <a class="pd-cta cb-next" href=format!("{BASE}/sell")><b>"SELL OR SUBLEASE"</b><span>"offer a flat, give a mandate to your agent, withdraw →"</span></a>
            {v.is_admin().then(|| view! { <a class="dl-mini" href=format!("{BASE}/admin")>"YOU ARE AN ADMIN · OPEN THE DESK →"</a> })}
        }.into_any()
    };

    let p_papers = move || {
        let lib = library();
        view! {
            <div class="ad-tiles cb-tiles">
                {["sign", "land", "company"].into_iter().map(|k| {
                    let s = lib.iter().find(|s| s.key == k);
                    let n = s.map(|s| s.items.len()).unwrap_or(0);
                    let name = match k { "sign" => "instruments you sign", "land" => "papers of the land", _ => "papers of the Company" };
                    view! { <a class="ad-tile" href=format!("{BASE}/docs#{k}") style="text-decoration: none;"><span>{name}</span><b>{n}</b><i>"on IPFS by content hash →"</i></a> }
                }).collect_view()}
            </div>
            <p class="dl-note">"the papers of one deal — the boundary of the flat, the offer you were shown, the index your rent is counted in — are made on that deal's track, step PAPERS."</p>
            <a class="dl-mini" href=format!("{BASE}/docs")>"THE WHOLE LIBRARY →"</a>
        }.into_any()
    };

    let p_help = move || view! {
        <div class="sl-cards">
            <a class="sl-card live" href=support_href() target="_blank" rel="noopener" style="text-decoration: none;">
                <div class="sl-card-top"><b>{format!("Direct message · @{SUPPORT_HANDLE}")}</b><span class="sl-tag">"→"</span></div>
                <div class="sl-card-line dim">"the deal, KYC, documents, anything about a flat"</div>
            </a>
            <a class="sl-card" href=community_href() target="_blank" rel="noopener" style="text-decoration: none;">
                <div class="sl-card-top"><b>"Community group · cyberia"</b><span class="sl-tag">"→"</span></div>
                <div class="sl-card-line dim">"residents, visitors, news of the valley"</div>
            </a>
        </div>
    }.into_any();

    view! {
        <div class="page-shell cities-shell plot-shell">
            <div class="site-chrome cyberia-chrome">
                <div class="chrome-inner">
                    <div class="header-row1">
                        <div class="logo-zone">
                            <h1 class="logo">
                                <a href=format!("{BASE}/") class="brand-flag" title="the globe" inner_html=FLAG_SVG></a>
                                <span style="color: var(--cyber-green);">"cyber"</span>
                                <span style="color: var(--cyber-green); margin: 0 1px;">"•"</span>
                                <span style="color: #fff;">"ia"</span>
                            </h1>
                        </div>
                        <CyberiaNav active="cabinet" />
                    </div>
                </div>
            </div>
            <div class="dl ad" style="--ink: #00fe00;">
                <aside class="dl-track">
                    <a class="dl-back" href=format!("{BASE}/map")>"← the valley map"</a>
                    <div class="dl-flat">"Your room"</div>
                    <div class="dl-flat-s">{short_neuron.clone()}</div>
                    <div class="dl-steps" style="margin-top: 12px;">
                        {MENU.iter().enumerate().map(|(i, (title, line, _))| view! {
                            <button type="button" class=move || { let m = mark(i).1; if menu.get() == i { format!("dl-step on {m}") } else { format!("dl-step {m}") } } on:click=move |_| go(i, 0)>
                                <span class="dl-step-n">{move || mark(i).0}</span>
                                <span class="dl-step-c"><b>{*title}</b><i>{*line}</i></span>
                            </button>
                        }).collect_view()}
                    </div>
                    <a class="dl-help" href=support_href() target="_blank" rel="noopener">{format!("a person answers · @{SUPPORT_HANDLE}")}</a>
                </aside>
                <section class="dl-panel">
                    <div class="dl-panel-h">
                        <span class="dl-panel-n">{move || MENU[menu.get()].1.to_uppercase()}</span>
                        <h2>{move || MENU[menu.get()].0}</h2>
                        {move || {
                            let subs = MENU[menu.get()].2;
                            (!subs.is_empty()).then(|| view! {
                                <div class="ad-subs" role="tablist" style="--cyber-yellow: #00fe00;">
                                    {subs.iter().enumerate().map(|(i, s)| view! {
                                        <button type="button" role="tab" class=move || if sub.get() == i { "ad-sub on" } else { "ad-sub" } on:click=move |_| sub.set(i)>{*s}</button>
                                    }).collect_view()}
                                </div>
                            })
                        }}
                    </div>
                    <div class="dl-body">
                        {move || { let e = err.get(); (!e.is_empty()).then(|| view! { <p class="dl-note warn">{e}</p> }) }}
                        {move || match menu.get() {
                            0 => p_you(),
                            1 => p_identity(),
                            2 => p_flats(),
                            3 => p_market(),
                            4 => p_papers(),
                            _ => p_help(),
                        }}
                    </div>
                </section>
            </div>
        </div>
    }
}
