//! ERP Studio hub — cyberlink-native. Sub-routes are full pages (no overlays).

use crate::economy::fmt_qty;
use crate::erp::{ensure_erp_boot, load_cyberlinks, sync_plot_cards_from_leases, world_view};
use crate::land::FLAG_SVG;
use crate::nav::CyberiaNav;
use crate::wallet::load_balance;
use leptos::prelude::*;

/// Shared chrome for all /world/* pages.
#[component]
pub fn StudioShell(
    #[prop(into)] title: String,
    #[prop(into)] kicker: String,
    #[prop(optional, into)] lead: String,
    children: Children,
) -> impl IntoView {
    let t = title.clone();
    Effect::new(move |_| {
        document().set_title(&format!("Cyberia — {t}"));
        ensure_erp_boot();
        sync_plot_cards_from_leases();
    });
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
                            "ERP · CYBERLINKS"
                        </div>
                        <CyberiaNav active="world" />
                    </div>
                </div>
            </div>
            <div class="cities-stage">
                <div class="studio-breadcrumb">
                    <a href="/world">"ERP"</a>
                    <span>" / "</span>
                    <span>{kicker.clone()}</span>
                </div>
                <div class="cities-hero">
                    <div>
                        <div class="cities-kicker">{kicker}</div>
                        <h2 class="cities-title">{title}</h2>
                        {(!lead.is_empty()).then(|| view! {
                            <p class="cities-lead">{lead}</p>
                        })}
                    </div>
                </div>
                {children()}
            </div>
            <div class="search-dock cyberia-dock cities-dock">
                <span class="dock-count">
                    {move || {
                        let v = world_view();
                        format!(
                            "{} links ({} draft · {} linked) · {} cards · {} CX",
                            v.cyberlinks, v.drafts, v.linked, v.cards, fmt_qty(load_balance().cx)
                        )
                    }}
                </span>
                <a class="cta-btn cta-lease cta-lg dock-found" href="/world/links/new" style="text-decoration:none;max-width:280px;">
                    <span class="cta-copy">
                        <span class="cta-title">"+ CYBERLINK"</span>
                        <span class="cta-sub">"intent first"</span>
                    </span>
                </a>
                <a href="/me" class="dock-credit" style="color: var(--cyber-cyan);">"YOU"</a>
            </div>
        </div>
    }
}

/// /world — hub
#[component]
pub fn WorldPage() -> impl IntoView {
    Effect::new(move |_| {
        document().set_title("Cyberia — ERP · cyberlinks");
        ensure_erp_boot();
        sync_plot_cards_from_leases();
    });

    let v = move || world_view();

    view! {
        <StudioShell
            title="ERP Studio"
            kicker="CYBERGRAPH"
            lead="In cyber everything is a cyberlink. Intent is the first step (draft). Commit writes the link into the graph. Cards, coins, templates and schedules are particles you link."
        >
            <div class="chain-hint">
                <span class="chain-step">"DRAFT LINK"</span>
                <span class="chain-arr">"→"</span>
                <span class="chain-step">"INTENT"</span>
                <span class="chain-arr">"→"</span>
                <span class="chain-step">"COMMIT"</span>
                <span class="chain-arr">"→"</span>
                <span class="chain-step">"CYBERLINK"</span>
                <span class="chain-arr">"→"</span>
                <span class="chain-step">"GRAPH / RANK"</span>
            </div>

            <div class="studio-create-bar">
                <a class="cta-btn cta-lease cta-lg" href="/world/links/new" style="text-decoration:none;">
                    <span class="cta-copy">
                        <span class="cta-title">"+ CYBERLINK"</span>
                        <span class="cta-sub">"core · from → to"</span>
                    </span>
                </a>
                <a class="cta-btn cta-event cta-lg" href="/world/cards/new" style="text-decoration:none;">
                    <span class="cta-copy">
                        <span class="cta-title">"+ CARD"</span>
                        <span class="cta-sub">"particle · TSP-2"</span>
                    </span>
                </a>
                <a class="cta-btn cta-buy cta-lg" href="/world/coins/new" style="text-decoration:none;">
                    <span class="cta-copy">
                        <span class="cta-title">"+ COIN"</span>
                        <span class="cta-sub">"stock · TSP-1"</span>
                    </span>
                </a>
                <a class="cta-btn cta-found cta-lg" href="/world/templates/new" style="text-decoration:none;">
                    <span class="cta-copy">
                        <span class="cta-title">"+ TEMPLATE"</span>
                        <span class="cta-sub">"recipe"</span>
                    </span>
                </a>
                <a class="cta-btn cta-lease cta-lg" href="/world/schedules/new" style="text-decoration:none;">
                    <span class="cta-copy">
                        <span class="cta-title">"+ SCHEDULE"</span>
                        <span class="cta-sub">"fire links"</span>
                    </span>
                </a>
            </div>

            <div class="bank-kpi-grid me-counts">
                <a class="kpi me-kpi" href="/world/links">
                    <div class="kpi-lab">"CYBERLINKS"</div>
                    <div class="kpi-val">{move || v().cyberlinks.to_string()}</div>
                    <div class="kpi-sub">{move || format!("{} draft · {} linked", v().drafts, v().linked)}</div>
                </a>
                <a class="kpi me-kpi" href="/world/intents">
                    <div class="kpi-lab">"INTENTS"</div>
                    <div class="kpi-val">{move || v().intents.to_string()}</div>
                    <div class="kpi-sub">"first step of links →"</div>
                </a>
                <a class="kpi me-kpi" href="/world/cards">
                    <div class="kpi-lab">"CARDS"</div>
                    <div class="kpi-val">{move || v().cards.to_string()}</div>
                    <div class="kpi-sub">"particles →"</div>
                </a>
                <a class="kpi me-kpi" href="/world/coins">
                    <div class="kpi-lab">"COINS"</div>
                    <div class="kpi-val">{move || v().coins.to_string()}</div>
                    <div class="kpi-sub">"balances →"</div>
                </a>
                <a class="kpi me-kpi" href="/world/templates">
                    <div class="kpi-lab">"TEMPLATES"</div>
                    <div class="kpi-val">{move || v().templates.to_string()}</div>
                    <div class="kpi-sub">"ops recipes →"</div>
                </a>
                <a class="kpi me-kpi" href="/world/schedules">
                    <div class="kpi-lab">"SCHEDULES"</div>
                    <div class="kpi-val">{move || v().schedules.to_string()}</div>
                    <div class="kpi-sub">"recurring →"</div>
                </a>
            </div>

            <div class="studio-section-h" style="margin-top:8px;">
                <span>"RECENT CYBERLINKS"</span>
                <a class="chip chip-on" href="/world/links/new">"+ NEW"</a>
            </div>
            <div class="studio-list">
                {move || {
                    let mut links = load_cyberlinks();
                    links.truncate(12);
                    if links.is_empty() {
                        return view! {
                            <div class="me-empty">
                                "No cyberlinks yet. "
                                <a href="/world/links/new">"Create a draft link"</a>
                                " (intent) then commit it into the graph."
                            </div>
                        }.into_any();
                    }
                    view! {
                        {links.into_iter().map(|cl| {
                            let href = format!("/world/link/{}", cl.id);
                            let st = cl.state.to_uppercase();
                            view! {
                                <a class="studio-row studio-row-link" href=href>
                                    <div class="studio-row-main">
                                        <span class=if cl.state == "linked" { "studio-kind" } else { "studio-kind draft-kind" }>{st}</span>
                                        <div>
                                            <div class="studio-title">{format!("{} —[{}]→ {}", cl.from, cl.rel, cl.to)}</div>
                                            <div class="studio-meta">{format!("{} · w={} · {}", cl.id, cl.weight, cl.note)}</div>
                                        </div>
                                    </div>
                                    <div class="studio-row-acts">
                                        <span class="chip">"OPEN →"</span>
                                    </div>
                                </a>
                            }
                        }).collect_view()}
                    }.into_any()
                }}
            </div>

            <p class="bank-footnote">
                "cyber: every assertion is a cyberlink. ERP studio is a local cybergraph console — particles (cards/coins/templates) + directed links. Full pages under /world/* · no overlay dialogs."
            </p>
        </StudioShell>
    }
}
