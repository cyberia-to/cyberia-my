//! `/cabinet` — a Holder's own room: every status of every flat they touched,
//! their identity on both chains, KYC, documents, and the door to support.
//! Everything here lives in this browser; the valley's files live on IPFS.

use crate::deals::{fmt_ts, load_dd, load_deals, load_jv, Deal, STEPS};
use crate::eth::{connect, forget_eth, has_provider, load_eth, EthWallet};
use crate::land::{load_map, FLAG_SVG};
use crate::nav::CyberiaNav;
use crate::orders::{
    community_href, fetch_index, load_orders_log, order_href, ordered, record_order, support_href,
    OrderKind, OrderRec, PublishedDeal, SUPPORT_HANDLE,
};
use crate::pay::{load_sent, sent_for};
use crate::terms::fmt_usd;
use crate::signal::neuron;
use crate::terms::{bare_name, terms};
use leptos::prelude::*;
use std::collections::{BTreeMap, HashMap};

/// One flat the Holder touched, with everything known about it here.
struct Row {
    id: String,
    name: String,
    deal: Option<Deal>,
    dd_ms: Option<f64>,
    jv_ms: Option<f64>,
    survey: String,
    media: String,
    paid: String,
}

fn rows(log: &[OrderRec], index: &HashMap<String, PublishedDeal>) -> Vec<Row> {
    let map = load_map();
    let mut deals = load_deals();
    crate::deals::merge_published(&mut deals, index);
    let sent = load_sent();
    let dd = load_dd();
    let jv = load_jv();
    let mut ids: BTreeMap<String, ()> = BTreeMap::new();
    for k in deals.keys().chain(dd.keys()).chain(jv.keys()) {
        ids.insert(k.clone(), ());
    }
    for p in &sent {
        ids.insert(p.plot.clone(), ());
    }
    for o in log {
        if o.kind != OrderKind::Kyc {
            ids.insert(o.plot.clone(), ());
        }
    }
    ids.keys()
        .map(|id| {
            let name = map
                .phase0
                .iter()
                .find(|f| &f.id == id)
                .map(|f| bare_name(&f.name))
                .unwrap_or_else(|| id.to_uppercase());
            let book = terms(id);
            let survey = match (book.and_then(|t| t.measured_m2), ordered(log, OrderKind::Survey, id)) {
                (Some(m2), _) => format!("{m2:.2} m² · in the lease book"),
                (None, Some(ts)) => format!("ordered {}", fmt_ts(ts)),
                (None, None) => "—".to_string(),
            };
            let media = match ordered(log, OrderKind::Media, id) {
                Some(ts) => format!("ordered {}", fmt_ts(ts)),
                None => "—".to_string(),
            };
            let paid = match (index.get(id).map(|d| d.paid_usd).filter(|p| *p > 0.0), sent_for(&sent, id).first()) {
                (Some(p), _) => format!("{} received", fmt_usd(p)),
                (None, Some(s)) => format!("{} sent · awaiting", fmt_usd(s.usd)),
                (None, None) => "—".to_string(),
            };
            Row {
                id: id.clone(),
                name,
                deal: deals.get(id).cloned(),
                dd_ms: dd.get(id).copied(),
                jv_ms: jv.get(id).copied(),
                survey,
                media,
                paid,
            }
        })
        .collect()
}

fn deal_line(d: &Option<Deal>) -> (String, &'static str) {
    match d {
        Some(d) if d.published.is_some() => {
            let pd = d.published.clone().unwrap_or_default();
            (pd.status.clone(), if pd.status == "owned" { "#00fe00" } else { "#fcf000" })
        }
        Some(d) if d.cancelled => ("cancelled".into(), "#777777"),
        Some(d) if d.is_signed() => ("signed".into(), "#00fe00"),
        Some(d) => (
            format!("{} · step {} of {}", d.step, d.step_index() + 1, STEPS.len()),
            "#fcf000",
        ),
        None => ("—".into(), "#777777"),
    }
}

#[component]
pub fn CabinetPage() -> impl IntoView {
    let log = RwSignal::new(load_orders_log());
    let index = RwSignal::new(HashMap::<String, PublishedDeal>::new());
    leptos::task::spawn_local(async move {
        index.set(fetch_index().await);
    });
    let eth = RwSignal::new(load_eth());
    let eth_note = RwSignal::new(String::new());
    let n = neuron();

    Effect::new(move |_| {
        document().set_title("Cyberia — cabinet");
    });

    let kyc_href = order_href(OrderKind::Kyc, "you", "");
    let on_connect = move |_| {
        eth_note.set("asking the wallet…".into());
        leptos::task::spawn_local(async move {
            match connect().await {
                Ok(w) => {
                    eth_note.set(String::new());
                    eth.set(Some(w));
                }
                Err(e) => eth_note.set(e),
            }
        });
    };

    view! {
        <div class="page-shell cities-shell plot-shell">
            <div class="site-chrome cyberia-chrome">
                <div class="chrome-inner">
                    <div class="header-row1">
                        <div class="logo-zone">
                            <h1 class="logo">
                                <a href="/cities" class="brand-flag" title="home" inner_html=FLAG_SVG></a>
                                <span style="color: var(--cyber-green);">"cyber"</span>
                                <span style="color: var(--cyber-green); margin: 0 1px;">"•"</span>
                                <span style="color: #fff;">"ia"</span>
                            </h1>
                        </div>
                        <CyberiaNav active="cabinet" />
                    </div>
                </div>
            </div>
            <div class="cities-stage plot-stage-wrap">
                <div class="cities-hero plot-hero">
                    <div>
                        <div class="cities-kicker">"HOLDER · CABINET"</div>
                        <h2 class="cities-title">"Your room"</h2>
                        <p class="cities-lead">
                            "Every status in one place — the deal, due diligence, survey, media, KYC, the wallet. This page lives in your browser; the valley keeps nothing of yours on its servers."
                        </p>
                    </div>
                    <a class="cta-btn cta-lease dock-found" href="/docs" style="text-decoration:none; max-width: 220px;">
                        <span class="cta-copy">
                            <span class="cta-title">"DOCUMENTS →"</span>
                            <span class="cta-sub">"the pack, rules, papers"</span>
                        </span>
                    </a>
                </div>

                <div class="library">
                    <section class="plot-card">
                        <div class="cyberia-panel-h">
                            <span class="panel-kicker">"IDENTITY"</span>
                            <span class="panel-sub">"the neuron on cyber · the wallet on Ethereum · KYC"</span>
                        </div>
                        <div class="cabinet-grid">
                            <div class="cabinet-cell">
                                <div class="deal-now-step">"NEURON"</div>
                                <div class="cabinet-mono">{n.bech32.clone()}</div>
                                <div class="deal-now-since">"derived in this browser · signs your signals and requests"</div>
                            </div>
                            <div class="cabinet-cell">
                                <div class="deal-now-step">"ETHEREUM WALLET"</div>
                                {move || match eth.get() {
                                    Some(w) => view! {
                                        <div class="cabinet-mono" title=w.address.clone()>{w.short()}</div>
                                        <div class="deal-now-since">{format!("{} · connected {}", w.chain_name(), fmt_ts(w.connected_ms))}</div>
                                        <button type="button" class="gps-btn" on:click=move |_| { forget_eth(); eth.set(None); }>"FORGET"</button>
                                    }.into_any(),
                                    None => view! {
                                        <div class="deal-now-line">{if has_provider() { "a wallet is in this browser" } else { "no wallet in this browser yet" }}</div>
                                        <button type="button" class="intent-commit lease" on:click=on_connect>"CONNECT WALLET"</button>
                                        <div class="deal-now-since">{move || eth_note.get()}</div>
                                        {(!has_provider()).then(|| view! {
                                            <div class="wallet-guide">
                                                <div class="deal-now-step">"HOW TO GET ONE · 5 MINUTES"</div>
                                                <ol>
                                                    <li>"on a computer: add the "<a href="https://metamask.io/download" target="_blank" rel="noopener">"MetaMask"</a>" or "<a href="https://rabby.io" target="_blank" rel="noopener">"Rabby"</a>" extension to your browser"</li>
                                                    <li>"on a phone: install the MetaMask or Rabby app, then open cyberia.my inside the app's own browser"</li>
                                                    <li>"create a wallet · write the 12 words on paper · nobody at the valley will ever ask for them"</li>
                                                    <li>"come back to this page and press CONNECT WALLET · approve in the wallet"</li>
                                                </ol>
                                                <div class="deal-now-since">"the wallet holds your plot's token on Ethereum and signs for the DAO · paying by bank transfer needs no wallet at all"</div>
                                            </div>
                                        })}
                                    }.into_any(),
                                }}
                            </div>
                            <div class="cabinet-cell">
                                <div class="deal-now-step">"KYC"</div>
                                {move || match ordered(&log.get(), OrderKind::Kyc, "you") {
                                    Some(ts) => view! {
                                        <div class="deal-now-line">{format!("sent to @{SUPPORT_HANDLE} · {}", fmt_ts(ts))}</div>
                                        <div class="deal-now-since">"support confirms in the same chat"</div>
                                    }.into_any(),
                                    None => view! {
                                        <div class="deal-now-line">{OrderKind::Kyc.line()}</div>
                                    }.into_any(),
                                }}
                                <a class="intent-commit lease ground-order-btn" href=kyc_href.clone() target="_blank" rel="noopener"
                                    on:click=move |_| log.update(|l| record_order(l, OrderKind::Kyc, "you"))>
                                    {move || if ordered(&log.get(), OrderKind::Kyc, "you").is_some() { "SEND KYC AGAIN" } else { "SEND KYC" }}
                                </a>
                            </div>
                        </div>
                    </section>

                    <section class="plot-card">
                        <div class="cyberia-panel-h">
                            <span class="panel-kicker">"YOUR FLATS"</span>
                            <span class="panel-sub">"every flat you asked about · the deal track and the ground"</span>
                        </div>
                        {move || {
                            let list = rows(&log.get(), &index.get());
                            if list.is_empty() {
                                return view! {
                                    <p class="docs-empty">"no flat yet — pick one on the "<a href="/map" style="color: var(--cyber-green);">"map"</a>" and request a lease or its documents"</p>
                                }.into_any();
                            }
                            view! {
                                <div class="cabinet-table">
                                    <div class="cabinet-row cabinet-head">
                                        <span>"flat"</span><span>"deal"</span><span>"paid"</span><span>"due diligence"</span><span>"survey"</span><span>"media"</span>
                                    </div>
                                    {list.into_iter().map(|r| {
                                        let (dl, dc) = deal_line(&r.deal);
                                        let dd = match (r.dd_ms, r.jv_ms) {
                                            (Some(ts), _) => format!("requested {}", fmt_ts(ts)),
                                            (None, Some(ts)) => format!("on request · asked {}", fmt_ts(ts)),
                                            (None, None) => "—".to_string(),
                                        };
                                        let href = format!("/plot/{}", r.id);
                                        view! {
                                            <a class="cabinet-row" href=href>
                                                <span class="cabinet-flat">{r.name}</span>
                                                <span style:color=dc>{dl}</span>
                                                <span>{r.paid}</span>
                                                <span>{dd}</span>
                                                <span>{r.survey}</span>
                                                <span>{r.media}</span>
                                            </a>
                                        }
                                    }).collect_view()}
                                </div>
                            }.into_any()
                        }}
                    </section>

                    <section class="plot-card">
                        <div class="cyberia-panel-h">
                            <span class="panel-kicker">"SUPPORT"</span>
                            <span class="panel-sub">"a person answers · Telegram"</span>
                        </div>
                        <div class="docs-list">
                            <div class="docs-row">
                                <a class="doc-dl" href=support_href() target="_blank" rel="noopener">
                                    <span class="doc-name">{format!("DIRECT MESSAGE · @{SUPPORT_HANDLE}")}</span>
                                    <span class="doc-meta">"the deal, KYC, documents, anything about a flat"</span>
                                </a>
                            </div>
                            <div class="docs-row">
                                <a class="doc-dl" href=community_href() target="_blank" rel="noopener">
                                    <span class="doc-name">"COMMUNITY GROUP · cyberia"</span>
                                    <span class="doc-meta">"residents, visitors, news of the valley"</span>
                                </a>
                            </div>
                            <div class="docs-row">
                                <a class="doc-dl" href="/docs">
                                    <span class="doc-name">"DOCUMENT LIBRARY"</span>
                                    <span class="doc-meta">"the pack, the rules, the index, the land and company papers"</span>
                                </a>
                            </div>
                        </div>
                    </section>
                </div>
            </div>
        </div>
    }
}
