//! `/cabinet` — a Holder's own room: every status of every flat they touched,
//! their identity on both chains, KYC, documents, and the door to support.
//! Everything here lives in this browser; the valley's files live on IPFS.

use crate::deals::{fmt_ts, load_dd, load_deals, load_jv, Deal, STEPS};
use crate::eth::{connect, forget_eth, has_provider, load_eth};
use crate::globe::BASE;
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

    let short_neuron = {
        let b = n.bech32.clone();
        if b.len() > 20 { format!("{}…{}", &b[..12], &b[b.len() - 6..]) } else { b }
    };

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
            <div class="cities-stage plot-stage-wrap cab">
                <header class="cab-hero">
                    <div class="cities-kicker">"HOLDER · CABINET"</div>
                    <h2 class="cab-title">"Your room"</h2>
                    <p class="cab-lead">"Your flats, your identity and the people who answer. It lives in this browser; the valley keeps nothing of yours on its servers."</p>
                </header>

                // where you stand, at a glance: four steps, each lit when done
                <div class="cab-steps">
                    <div class="cab-step done">
                        <span class="cs-mark">"✓"</span>
                        <span class="cs-copy"><span class="cs-name">"NEURON"</span><span class="cs-state">{short_neuron.clone()}</span></span>
                    </div>
                    {move || {
                        let w = eth.get();
                        let on = w.is_some();
                        let state = w.map(|w| w.short()).unwrap_or_else(|| "not connected".into());
                        view! {
                            <div class=if on { "cab-step done" } else { "cab-step" }>
                                <span class="cs-mark">{if on { "✓" } else { "2" }}</span>
                                <span class="cs-copy"><span class="cs-name">"WALLET"</span><span class="cs-state">{state}</span></span>
                            </div>
                        }
                    }}
                    {move || {
                        let sent = ordered(&log.get(), OrderKind::Kyc, "you");
                        view! {
                            <div class=if sent.is_some() { "cab-step sent" } else { "cab-step" }>
                                <span class="cs-mark">{if sent.is_some() { "…" } else { "3" }}</span>
                                <span class="cs-copy"><span class="cs-name">"KYC"</span><span class="cs-state">{sent.map(|ts| format!("sent {}", fmt_ts(ts))).unwrap_or_else(|| "not sent".into())}</span></span>
                            </div>
                        }
                    }}
                    {move || {
                        let count = rows(&log.get(), &index.get()).len();
                        let cls = if count > 0 { "cab-step done" } else { "cab-step" };
                        view! {
                            <div class=cls>
                                <span class="cs-mark">{if count > 0 { count.to_string() } else { "4".to_string() }}</span>
                                <span class="cs-copy"><span class="cs-name">"FLATS"</span><span class="cs-state">{if count > 0 { "in your room".to_string() } else { "none yet".to_string() }}</span></span>
                            </div>
                        }
                    }}
                </div>

                <div class="cab-grid">
                    // ── the flats: the reason the room exists ──
                    <section class="cab-card cab-flats">
                        <div class="cab-card-h">
                            <span class="panel-kicker">"YOUR FLATS"</span>
                            <span class="panel-sub">"the deal and the ground of every flat you asked about"</span>
                        </div>
                        {move || {
                            let list = rows(&log.get(), &index.get());
                            if list.is_empty() {
                                return view! {
                                    <div class="cab-empty">
                                        <div class="cab-empty-t">"No flat yet"</div>
                                        <div class="cab-empty-s">"pick one on the map and ask for a lease or for its documents — it appears here with its track"</div>
                                        <a class="cab-btn" href=format!("{BASE}/map")>"OPEN CYBER VALLEY →"</a>
                                    </div>
                                }.into_any();
                            }
                            view! {
                                <div class="cab-flat-list">
                                    {list.into_iter().map(|r| {
                                        let (dl, dc) = deal_line(&r.deal);
                                        let dd = match (r.dd_ms, r.jv_ms) {
                                            (Some(ts), _) => format!("requested {}", fmt_ts(ts)),
                                            (None, Some(ts)) => format!("on request · asked {}", fmt_ts(ts)),
                                            (None, None) => "—".to_string(),
                                        };
                                        let href = format!("{BASE}/plot/{}", r.id);
                                        view! {
                                            <a class="cab-flat" href=href>
                                                <div class="cf-top">
                                                    <span class="cf-name">{r.name}</span>
                                                    <span class="cf-deal" style:color=dc style:border-color=dc>{dl}</span>
                                                    <span class="cf-go">"→"</span>
                                                </div>
                                                <div class="cf-facts">
                                                    <div><span class="cf-k">"paid"</span><span class="cf-v">{r.paid}</span></div>
                                                    <div><span class="cf-k">"due diligence"</span><span class="cf-v">{dd}</span></div>
                                                    <div><span class="cf-k">"survey"</span><span class="cf-v">{r.survey}</span></div>
                                                    <div><span class="cf-k">"media"</span><span class="cf-v">{r.media}</span></div>
                                                </div>
                                            </a>
                                        }
                                    }).collect_view()}
                                    <a class="cab-more" href=format!("{BASE}/map")>"+ another flat on the map"</a>
                                </div>
                            }.into_any()
                        }}
                    </section>

                    <div class="cab-side">
                        // ── who you are: three rows, each with its state and its one action ──
                        <section class="cab-card">
                            <div class="cab-card-h">
                                <span class="panel-kicker">"IDENTITY"</span>
                                <span class="panel-sub">"cyber · Ethereum · KYC"</span>
                            </div>
                            <div class="cab-id">
                                <span class="ci-dot on"></span>
                                <div class="ci-body">
                                    <div class="ci-name">"NEURON"</div>
                                    <div class="cabinet-mono ci-val">{n.bech32.clone()}</div>
                                    <div class="ci-note">"derived in this browser · signs your signals and requests"</div>
                                </div>
                            </div>
                            {move || match eth.get() {
                                Some(w) => view! {
                                    <div class="cab-id">
                                        <span class="ci-dot on"></span>
                                        <div class="ci-body">
                                            <div class="ci-name">"ETHEREUM WALLET"</div>
                                            <div class="cabinet-mono ci-val" title=w.address.clone()>{w.short()}</div>
                                            <div class="ci-note">{format!("{} · connected {}", w.chain_name(), fmt_ts(w.connected_ms))}</div>
                                            <button type="button" class="cab-btn ghost" on:click=move |_| { forget_eth(); eth.set(None); }>"FORGET"</button>
                                        </div>
                                    </div>
                                }.into_any(),
                                None => view! {
                                    <div class="cab-id">
                                        <span class="ci-dot"></span>
                                        <div class="ci-body">
                                            <div class="ci-name">"ETHEREUM WALLET"</div>
                                            <div class="ci-note">{if has_provider() { "a wallet is in this browser" } else { "no wallet in this browser yet · bank transfer needs none" }}</div>
                                            <button type="button" class="cab-btn" on:click=on_connect>"CONNECT WALLET"</button>
                                            <div class="ci-note">{move || eth_note.get()}</div>
                                            {(!has_provider()).then(|| view! {
                                                <details class="cab-how">
                                                    <summary>"how to get a wallet · 5 minutes"</summary>
                                                    <ol>
                                                        <li>"on a computer: add the "<a href="https://metamask.io/download" target="_blank" rel="noopener">"MetaMask"</a>" or "<a href="https://rabby.io" target="_blank" rel="noopener">"Rabby"</a>" extension to your browser"</li>
                                                        <li>"on a phone: install the MetaMask or Rabby app, then open cyberia.my inside the app's own browser"</li>
                                                        <li>"create a wallet · write the 12 words on paper · nobody at the valley will ever ask for them"</li>
                                                        <li>"come back to this page and press CONNECT WALLET · approve in the wallet"</li>
                                                    </ol>
                                                    <div class="ci-note">"the wallet holds your plot's token on Ethereum and signs for the DAO"</div>
                                                </details>
                                            })}
                                        </div>
                                    </div>
                                }.into_any(),
                            }}
                            <div class="cab-id">
                                <span class=move || if ordered(&log.get(), OrderKind::Kyc, "you").is_some() { "ci-dot wait" } else { "ci-dot" }></span>
                                <div class="ci-body">
                                    <div class="ci-name">"KYC"</div>
                                    {move || match ordered(&log.get(), OrderKind::Kyc, "you") {
                                        Some(ts) => view! {
                                            <div class="ci-val">{format!("sent to @{SUPPORT_HANDLE} · {}", fmt_ts(ts))}</div>
                                            <div class="ci-note">"support confirms in the same chat"</div>
                                        }.into_any(),
                                        None => view! {
                                            <div class="ci-note">{OrderKind::Kyc.line()}</div>
                                        }.into_any(),
                                    }}
                                    <a class="cab-btn" href=kyc_href.clone() target="_blank" rel="noopener"
                                        on:click=move |_| log.update(|l| record_order(l, OrderKind::Kyc, "you"))>
                                        {move || if ordered(&log.get(), OrderKind::Kyc, "you").is_some() { "SEND KYC AGAIN" } else { "SEND KYC" }}
                                    </a>
                                </div>
                            </div>
                        </section>

                        // ── where to turn ──
                        <section class="cab-card">
                            <div class="cab-card-h">
                                <span class="panel-kicker">"HELP"</span>
                                <span class="panel-sub">"a person answers · Telegram"</span>
                            </div>
                            <a class="cab-link" href=support_href() target="_blank" rel="noopener">
                                <span class="cl-copy"><span class="cl-name">{format!("Direct message · @{SUPPORT_HANDLE}")}</span><span class="cl-sub">"the deal, KYC, documents, anything about a flat"</span></span>
                                <span class="cl-go">"→"</span>
                            </a>
                            <a class="cab-link" href=community_href() target="_blank" rel="noopener">
                                <span class="cl-copy"><span class="cl-name">"Community group · cyberia"</span><span class="cl-sub">"residents, visitors, news of the valley"</span></span>
                                <span class="cl-go">"→"</span>
                            </a>
                            <a class="cab-link" href=format!("{BASE}/docs")>
                                <span class="cl-copy"><span class="cl-name">"Documents"</span><span class="cl-sub">"the pack, the rules, the land and company papers"</span></span>
                                <span class="cl-go">"→"</span>
                            </a>
                        </section>
                    </div>
                </div>
            </div>
        </div>
    }
}
