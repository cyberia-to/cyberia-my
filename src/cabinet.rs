//! `/cabinet` — a Holder's own room: every status of every flat they touched,
//! their identity on both chains, KYC, documents, and the door to support.
//! Everything here lives in this browser; the valley's files live on IPFS.

use crate::deals::{fmt_ts, load_deals};
use crate::eth::{forget_eth, load_eth};
use crate::globe::BASE;
use crate::land::FLAG_SVG;
use crate::nav::CyberiaNav;
use crate::orders::{
    community_href, fetch_index, load_orders_log, order_href, ordered, record_order, support_href,
    OrderKind, PublishedDeal, SUPPORT_HANDLE,
};
use crate::pay::load_sent;
use crate::signal::neuron;
use leptos::prelude::*;
use std::collections::HashMap;

/// How many flats this browser has on its shelves — a deal, a payment, a
/// star. The flats themselves live on `/my`.
fn shelved() -> usize {
    let mut ids: std::collections::BTreeSet<String> = load_deals().into_iter().filter(|(_, d)| !d.cancelled).map(|(k, _)| k).collect();
    ids.extend(load_sent().into_iter().map(|p| p.plot));
    ids.extend(crate::my::load_watch());
    ids.len()
}

#[component]
pub fn CabinetPage() -> impl IntoView {
    let log = RwSignal::new(load_orders_log());
    let index = RwSignal::new(HashMap::<String, PublishedDeal>::new());
    leptos::task::spawn_local(async move {
        index.set(fetch_index().await);
    });
    let eth = RwSignal::new(load_eth());
    let n = neuron();

    Effect::new(move |_| {
        document().set_title("Cyberia — cabinet");
    });

    let kyc_href = order_href(OrderKind::Kyc, "you", "");

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
                        let count = shelved();
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
                    // ── the flats live on their own screen now; the room keeps the way to it ──
                    <section class="cab-card cab-flats">
                        <div class="cab-card-h">
                            <span class="panel-kicker">"YOUR FLATS"</span>
                            <span class="panel-sub">"held, in deal, watching — on their own screen"</span>
                        </div>
                        {move || {
                            let count = shelved();
                            view! {
                                <div class="cab-empty">
                                    <div class="cab-empty-t">{if count > 0 { format!("{count} flats on your shelves") } else { "No flat yet".to_string() }}</div>
                                    <div class="cab-empty-s">"every flat is a token: its shape, where it lies in the valley, its deal, its offer on the market"</div>
                                    <a class="cab-btn" href=format!("{BASE}/my")>"OPEN MY PLOTS →"</a>
                                    <a class="cab-more" href=format!("{BASE}/map")>"+ pick a flat on the map"</a>
                                    <a class="cab-more" href=format!("{BASE}/sell")>"put a flat you hold on the map — resale or sublease →"</a>
                                </div>
                            }
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
                                            <div class="ci-note">{format!("{}{} · connected {}", if w.wallet.is_empty() { String::new() } else { format!("{} · ", w.wallet) }, w.chain_name(), fmt_ts(w.connected_ms))}</div>
                                            <button type="button" class="cab-btn ghost" on:click=move |_| { forget_eth(); eth.set(None); }>"FORGET"</button>
                                        </div>
                                    </div>
                                }.into_any(),
                                None => view! {
                                    <div class="cab-id">
                                        <span class="ci-dot"></span>
                                        <div class="ci-body">
                                            <div class="ci-name">"ETHEREUM WALLET"</div>
                                            <div class="ci-note">"holds your plot's token on Ethereum and signs for the DAO · bank transfer needs none"</div>
                                            <crate::wallet_ui::WalletConnect on_connected=Callback::new(move |w| eth.set(Some(w))) />
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
