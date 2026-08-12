//! Products — produce · BOM transform chains. The derivative layer:
//! biological or simple chemistry, everything made FROM elements and
//! genomes. The fundamental table itself lives at /elements.

use crate::economy::{
    city_ask, elements, fmt_qty, good, product_buy, product_sell, products, run_bom, BomRecipe,
    GoodDef, GoodKind, BOMS, GOODS,
};
use crate::land::FLAG_SVG;
use crate::nav::CyberiaNav;
use crate::wallet::{ensure_economy_boot, load_balance, stock_qty};
use leptos::prelude::*;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Tab {
    Elements,
    Products,
    Bom,
}

#[component]
pub fn ProductsPage() -> impl IntoView {
    let tab = RwSignal::new(Tab::Bom);
    let msg = RwSignal::new(None::<(bool, String)>); // ok, text
    let tick = RwSignal::new(0u32);

    Effect::new(move |_| {
        document().set_title("Cyberia — products · BOM");
        ensure_economy_boot();
        tick.update(|n| *n += 1);
    });

    let refresh = move || tick.update(|n| *n += 1);

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
                                format!("{} GOODS · {} BOM", GOODS.len(), BOMS.len())
                            }}
                        </div>
                        <CyberiaNav active="products" />
                    </div>
                </div>
            </div>

            <div class="cities-stage">
                <div class="cities-hero">
                    <div>
                        <div class="cities-kicker">"PRODUCE"</div>
                        <h2 class="cities-title">"Products"</h2>
                        <p class="cities-lead">
                            "Derivatives — everything produced from elements and genomes. BOM chains transform stocks: inputs → outputs. Trade the result on Market."
                        </p>
                    </div>
                </div>

                <div class="list-toolbar bank-toolbar">
                    <div class="list-filters">
                        <button
                            class=move || if tab.get() == Tab::Bom { "chip chip-on" } else { "chip" }
                            on:click=move |_| tab.set(Tab::Bom)
                        >"BOM CHAINS"</button>
                        <button
                            class=move || if tab.get() == Tab::Elements { "chip chip-on" } else { "chip" }
                            on:click=move |_| tab.set(Tab::Elements)
                        >"ELEMENTS"</button>
                        <button
                            class=move || if tab.get() == Tab::Products { "chip chip-on" } else { "chip" }
                            on:click=move |_| tab.set(Tab::Products)
                        >"PRODUCTS"</button>
                        <a class="chip" href="/market">"MARKET →"</a>
                    </div>
                    <span class="bank-mock-tag">
                        {move || {
                            let _ = tick.get();
                            format!("{} CX", fmt_qty(load_balance().cx))
                        }}
                    </span>
                </div>

                {move || msg.get().map(|(ok, t)| view! {
                    <div class=if ok { "eco-msg ok" } else { "eco-msg err" }>{t}</div>
                })}

                // BOM
                {move || (tab.get() == Tab::Bom).then(|| {
                    let _ = tick.get();
                    view! {
                        <div class="bom-list">
                            {BOMS.iter().map(|r| {
                                let recipe: &BomRecipe = r;
                                let id = recipe.id;
                                let can = {
                                    let needs: Vec<(String, f64)> = recipe
                                        .inputs
                                        .iter()
                                        .map(|i| (i.id.to_string(), i.qty))
                                        .collect();
                                    crate::wallet::stock_has(&needs)
                                };
                                view! {
                                    <div class=if can { "bom-card ready" } else { "bom-card" }>
                                        <div class="bom-card-top">
                                            <div>
                                                <div class="bom-name">{recipe.name}</div>
                                                <div class="bom-blurb">{recipe.blurb}</div>
                                            </div>
                                            <button
                                                class=if can { "chip chip-on" } else { "chip" }
                                                disabled=!can
                                                on:click=move |_| {
                                                    match run_bom(id) {
                                                        Ok(s) => {
                                                            msg.set(Some((true, s)));
                                                            refresh();
                                                        }
                                                        Err(e) => msg.set(Some((false, e))),
                                                    }
                                                }
                                            >"RUN"</button>
                                        </div>
                                        <div class="bom-io">
                                            <div class="bom-col">
                                                <div class="bom-col-h">"IN"</div>
                                                {recipe.inputs.iter().map(|i| {
                                                    let have = stock_qty(i.id);
                                                    let short = have + 1e-9 < i.qty;
                                                    view! {
                                                        <div class=if short { "bom-line short" } else { "bom-line" }>
                                                            <span>{i.id}</span>
                                                            <span>{format!("{} {}", fmt_qty(i.qty), good(i.id).map(|g| g.unit).unwrap_or(""))}</span>
                                                            <span class="bom-have">{format!("have {}", fmt_qty(have))}</span>
                                                        </div>
                                                    }
                                                }).collect_view()}
                                            </div>
                                            <div class="bom-arrow">"→"</div>
                                            <div class="bom-col">
                                                <div class="bom-col-h">"OUT"</div>
                                                {recipe.outputs.iter().map(|o| {
                                                    view! {
                                                        <div class="bom-line out">
                                                            <span>{o.id}</span>
                                                            <span>{format!("+{} {}", fmt_qty(o.qty), good(o.id).map(|g| g.unit).unwrap_or(""))}</span>
                                                        </div>
                                                    }
                                                }).collect_view()}
                                            </div>
                                        </div>
                                        <div class="bom-note">{format!("· {}", recipe.labor_note)}</div>
                                    </div>
                                }
                            }).collect_view()}
                        </div>
                    }
                })}

                // Elements table
                {move || (tab.get() == Tab::Elements).then(|| {
                    let _ = tick.get();
                    goods_table(elements().collect(), true)
                })}

                // Products table
                {move || (tab.get() == Tab::Products).then(|| {
                    let _ = tick.get();
                    goods_table(products().collect(), false)
                })}
            </div>

            <div class="search-dock cyberia-dock cities-dock">
                <span class="dock-count">
                    "BOM closes the loop: stock → transform → product → market → stock"
                </span>
                <a class="cta-btn cta-lease cta-lg dock-found" href="/market" style="text-decoration:none; max-width: 280px;">
                    <span class="cta-copy">
                        <span class="cta-title">"MARKET"</span>
                        <span class="cta-sub">"buy · sell · city book"</span>
                    </span>
                </a>
                <a href="/me" class="dock-credit" style="color: var(--cyber-cyan);">"← YOU"</a>
            </div>
        </div>
    }
}

fn goods_table(list: Vec<&'static GoodDef>, _is_el: bool) -> impl IntoView {
    let msg = RwSignal::new(None::<(bool, String)>);
    let tick = RwSignal::new(0u32);
    view! {
        {move || msg.get().map(|(ok, t)| view! {
            <div class=if ok { "eco-msg ok" } else { "eco-msg err" }>{t}</div>
        })}
        <div class="states-table-wrap">
            <div class="states-table-h eco-table-h">
                <span class="st-rank">"#"</span>
                <span class="st-name">"GOOD"</span>
                <span class="st-token">"UNIT"</span>
                <span class="st-cap">"YOU"</span>
                <span class="st-delta">"PRICE"</span>
                <span class="st-region">"TRADE · buy at ask, sell at 70%"</span>
            </div>
            <div class="states-table-body">
                {move || {
                    let _ = tick.get();
                    list.iter().enumerate().map(|(i, g)| {
                        let q = stock_qty(g.id);
                        let has = q > 0.0;
                        let kind = match g.kind {
                            GoodKind::Element => "EL",
                            GoodKind::Product => "PR",
                        };
                        let qty_cls = if has { "st-cap" } else { "st-cap soon" };
                        let ask = city_ask(g.id);
                        let gid = g.id;
                        let buy = move |_| {
                            msg.set(Some(match product_buy(gid, 1.0) {
                                Ok(t) => (true, t),
                                Err(t) => (false, t),
                            }));
                            tick.update(|n| *n += 1);
                        };
                        let sell = move |_| {
                            msg.set(Some(match product_sell(gid, 1.0) {
                                Ok(t) => (true, t),
                                Err(t) => (false, t),
                            }));
                            tick.update(|n| *n += 1);
                        };
                        view! {
                            <div class="states-row eco-row">
                                <span class="st-rank">{i + 1}</span>
                                <span class="st-name">
                                    <span class="st-name-text">{g.name}</span>
                                    <span class="st-code">{kind}</span>
                                </span>
                                <span class="st-token">{g.unit}</span>
                                <span class=qty_cls>{fmt_qty(q)}</span>
                                <span class="st-delta">{ask.map(|p| format!("{p:.2} CX")).unwrap_or_else(|| "—".into())}</span>
                                <span class="st-region" style="display:flex; gap:6px; align-items:center;">
                                    {ask.is_some().then(|| view! {
                                        <button class="chip chip-on" on:click=buy>"BUY 1"</button>
                                        <button class="chip" on:click=sell disabled=!has>"SELL 1"</button>
                                    })}
                                </span>
                            </div>
                        }
                    }).collect_view()
                }}
            </div>
        </div>
    }
}
