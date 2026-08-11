//! Full soft market — elements & products. No orgs; city seed + your lots.

use crate::economy::{
    fmt_qty, good, market_buy, market_cancel, market_list_sell, market_sell_to_city, GOODS,
};
use crate::land::FLAG_SVG;
use crate::nav::CyberiaNav;
use crate::wallet::{
    ensure_economy_boot, load_balance, load_orders, load_profile, stock_qty, MarketOrder,
};
use leptos::prelude::*;

#[derive(Clone, Copy, PartialEq, Eq)]
enum SideFilter {
    All,
    Seed,
    Mine,
}

#[component]
pub fn MarketPage() -> impl IntoView {
    let filter_good = RwSignal::new(String::new());
    let side_f = RwSignal::new(SideFilter::All);
    let tick = RwSignal::new(0u32);
    let msg = RwSignal::new(None::<(bool, String)>);

    let list_good = RwSignal::new("energy".to_string());
    let list_qty = RwSignal::new("1".to_string());
    let list_px = RwSignal::new("2".to_string());
    let buy_qty = RwSignal::new("1".to_string());

    Effect::new(move |_| {
        document().set_title("Cyberia — market");
        ensure_economy_boot();
        tick.update(|n| *n += 1);
    });

    let refresh = move || tick.update(|n| *n += 1);

    let book = move || {
        let _ = tick.get();
        let g = filter_good.get();
        let sf = side_f.get();
        let me = load_profile().handle;
        let mut o = load_orders();
        o.retain(|x| g.is_empty() || x.good_id == g);
        o.retain(|x| match sf {
            SideFilter::All => true,
            SideFilter::Seed => x.owner == "cyber-valley",
            SideFilter::Mine => x.owner == me,
        });
        o.sort_by(|a, b| {
            a.good_id.cmp(&b.good_id).then_with(|| {
                a.price_cx
                    .partial_cmp(&b.price_cx)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
        });
        o
    };

    view! {
        <div class="page-shell cities-shell">
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
                        <div class="cyberia-phase-pill">
                            <span class="phase-dot"></span>
                            {move || {
                                let _ = tick.get();
                                format!("{} LOTS · {} CX", load_orders().len(), fmt_qty(load_balance().cx))
                            }}
                        </div>
                        <CyberiaNav active="market" />
                    </div>
                </div>
            </div>

            <div class="cities-stage">
                <div class="cities-hero">
                    <div>
                        <div class="cities-kicker">"CLEARING · ELEMENTS & PRODUCTS"</div>
                        <h2 class="cities-title">"Market"</h2>
                        <p class="cities-lead">
                            "Full book of primitives and products in CX. City seed keeps depth; you list from stock. No organizations — just lots."
                        </p>
                    </div>
                </div>

                <div class="list-toolbar">
                    <div class="list-filters">
                        <button
                            class=move || if filter_good.get().is_empty() { "chip chip-on" } else { "chip" }
                            on:click=move |_| filter_good.set(String::new())
                        >"ALL GOODS"</button>
                        {GOODS.iter().map(|g| {
                            let id = g.id.to_string();
                            let id2 = id.clone();
                            let name = g.name;
                            view! {
                                <button
                                    class=move || if filter_good.get() == id { "chip chip-on" } else { "chip" }
                                    on:click=move |_| filter_good.set(id2.clone())
                                >{name}</button>
                            }
                        }).collect_view()}
                    </div>
                    <div class="list-sorts">
                        <span class="list-sort-label">"BOOK"</span>
                        <button
                            class=move || if side_f.get() == SideFilter::All { "chip chip-on" } else { "chip" }
                            on:click=move |_| side_f.set(SideFilter::All)
                        >"ALL"</button>
                        <button
                            class=move || if side_f.get() == SideFilter::Seed { "chip chip-on" } else { "chip" }
                            on:click=move |_| side_f.set(SideFilter::Seed)
                        >"CITY SEED"</button>
                        <button
                            class=move || if side_f.get() == SideFilter::Mine { "chip chip-on" } else { "chip" }
                            on:click=move |_| side_f.set(SideFilter::Mine)
                        >"MY LOTS"</button>
                        <a class="chip" href="/elements">"BOM →"</a>
                    </div>
                </div>

                {move || msg.get().map(|(ok, t)| view! {
                    <div class=if ok { "eco-msg ok" } else { "eco-msg err" }>{t}</div>
                })}

                // list sell panel
                <div class="market-panel">
                    <div class="bank-section-h">"LIST SELL · FROM YOUR STOCK"</div>
                    <div class="market-form">
                        <select
                            class="found-input market-select"
                            on:change=move |ev| list_good.set(event_target_value(&ev))
                        >
                            {GOODS.iter().map(|g| {
                                let id = g.id;
                                let label = format!("{} ({})", g.name, g.unit);
                                view! {
                                    <option value=id selected=id == "energy">{label}</option>
                                }
                            }).collect_view()}
                        </select>
                        <input
                            class="found-input"
                            type="text"
                            placeholder="qty"
                            prop:value=move || list_qty.get()
                            on:input=move |ev| list_qty.set(event_target_value(&ev))
                        />
                        <input
                            class="found-input"
                            type="text"
                            placeholder="CX / unit"
                            prop:value=move || list_px.get()
                            on:input=move |ev| list_px.set(event_target_value(&ev))
                        />
                        <button class="chip chip-on" on:click=move |_| {
                            let g = list_good.get();
                            let q: f64 = list_qty.get().parse().unwrap_or(0.0);
                            let p: f64 = list_px.get().parse().unwrap_or(0.0);
                            match market_list_sell(&g, q, p) {
                                Ok(s) => { msg.set(Some((true, s))); refresh(); }
                                Err(e) => msg.set(Some((false, e))),
                            }
                        }>"LIST"</button>
                        <span class="market-have">
                            {move || {
                                let _ = tick.get();
                                let g = list_good.get();
                                format!("you hold {} {}", fmt_qty(stock_qty(&g)), g)
                            }}
                        </span>
                    </div>
                    <div class="market-form" style="margin-top: 8px;">
                        <span class="list-sort-label">"SELL TO CITY"</span>
                        <button class="chip" on:click=move |_| {
                            let g = list_good.get();
                            let q: f64 = list_qty.get().parse().unwrap_or(0.0);
                            match market_sell_to_city(&g, q) {
                                Ok(s) => { msg.set(Some((true, s))); refresh(); }
                                Err(e) => msg.set(Some((false, e))),
                            }
                        }>"SELL @ 70% SEED"</button>
                        <span class="me-chip-meta">"instant buy-back · no counterparty hunt"</span>
                    </div>
                </div>

                // order book
                <div class="bank-section-h" style="margin-top: 18px;">"ORDER BOOK · SELL LOTS"</div>
                <div class="states-table-wrap">
                    <div class="states-table-h market-table-h">
                        <span class="st-rank">"#"</span>
                        <span class="st-name">"GOOD"</span>
                        <span class="st-token">"QTY"</span>
                        <span class="st-cap">"CX/U"</span>
                        <span class="st-delta">"OWNER"</span>
                        <span class="st-region">"ACTION"</span>
                    </div>
                    <div class="states-table-body">
                        {move || {
                            let me = load_profile().handle;
                            let rows = book();
                            if rows.is_empty() {
                                return view! {
                                    <div class="me-empty" style="margin: 12px;">"No lots in this filter."</div>
                                }.into_any();
                            }
                            view! {
                                {rows.into_iter().map(|o: MarketOrder| {
                                    let oid = o.id;
                                    let is_mine = o.owner == me;
                                    let is_seed = o.owner == "cyber-valley";
                                    let gname = good(&o.good_id).map(|g| g.name).unwrap_or("?");
                                    let unit = good(&o.good_id).map(|g| g.unit).unwrap_or("");
                                    let qty_s = fmt_qty(o.qty);
                                    let px_s = format!("{:.2}", o.price_cx);
                                    let owner = o.owner.clone();
                                    view! {
                                        <div class="states-row market-row">
                                            <span class="st-rank">{oid}</span>
                                            <span class="st-name">
                                                <span class="st-name-text">{gname}</span>
                                                <span class="st-code">{o.good_id.clone()}</span>
                                            </span>
                                            <span class="st-token">{format!("{qty_s} {unit}")}</span>
                                            <span class="st-cap">{px_s}</span>
                                            <span class="st-delta">{if is_seed { "CITY".into() } else { owner }}</span>
                                            <span class="st-region market-acts">
                                                {if is_mine {
                                                    view! {
                                                        <button class="chip" on:click=move |_| {
                                                            match market_cancel(oid) {
                                                                Ok(s) => { msg.set(Some((true, s))); refresh(); }
                                                                Err(e) => msg.set(Some((false, e))),
                                                            }
                                                        }>"CANCEL"</button>
                                                    }.into_any()
                                                } else {
                                                    view! {
                                                        <input
                                                            class="found-input market-qty-in"
                                                            type="text"
                                                            prop:value=move || buy_qty.get()
                                                            on:input=move |ev| buy_qty.set(event_target_value(&ev))
                                                        />
                                                        <button class="chip chip-on" on:click=move |_| {
                                                            let q: f64 = buy_qty.get().parse().unwrap_or(1.0);
                                                            match market_buy(oid, q) {
                                                                Ok(s) => { msg.set(Some((true, s))); refresh(); }
                                                                Err(e) => msg.set(Some((false, e))),
                                                            }
                                                        }>"BUY"</button>
                                                    }.into_any()
                                                }}
                                            </span>
                                        </div>
                                    }
                                }).collect_view()}
                            }.into_any()
                        }}
                    </div>
                </div>

                <p class="bank-footnote">
                    "Soft3 local market — single browser agent + cyber-valley seed book. No org registry. BOM products and elements share this book. Multi-wallet settlement later."
                </p>
            </div>

            <div class="search-dock cyberia-dock cities-dock">
                <span class="dock-count">
                    {move || {
                        let _ = tick.get();
                        format!(
                            "{} CX · {} open lots · seed keeps depth",
                            fmt_qty(load_balance().cx),
                            load_orders().len()
                        )
                    }}
                </span>
                <a class="cta-btn cta-buy cta-lg dock-found" href="/elements" style="text-decoration:none; max-width: 280px;">
                    <span class="cta-copy">
                        <span class="cta-title">"BOM"</span>
                        <span class="cta-sub">"transform stocks"</span>
                    </span>
                </a>
                <a href="/me" class="dock-credit" style="color: var(--cyber-cyan);">"← YOU"</a>
            </div>
        </div>
    }
}
