//! `/admin` — the valley's and the notary's view of every deal the valley
//! published: holder, form, money received, steps, the ground, and the
//! recitals a notary fills from. Read-only on the web; the admin acts in
//! the hard force chat with the bot's commands listed at the bottom.

use crate::docs::{library, Item};
use crate::land::{area_m2, load_map, FLAG_SVG};
use crate::nav::CyberiaNav;
use crate::orders::{fetch_index, fetch_plot_data, PlotData, PublishedDeal};
use crate::terms::{bare_name, fmt_usd, terms};
use leptos::prelude::*;

/// Title data every lease recites — from the Company's documents.
const TITLE: &[(&str, &str)] = &[
    ("Landowner", "PT CYBER VALLEY ESTATE · PT PMA · Gesing, Banjar, Buleleng, Bali"),
    ("Akta Pendirian", "No. 01 of 04-01-2022 · AHU-0001412-AH.01.01 of 07-01-2022"),
    ("NIB", "2504220016017"),
    ("NPWP", "53.898.326.3-902.000"),
    ("Title", "HGB No. 00048/Gesing · 40,000 m² · parcel NIB 22040417.00276 · Surat Ukur No. 00086/2002"),
    ("Granted", "08-02-2023 under KEPMEN ATR/KBPN No. 1339/SK-HK.02/X/2022 · expiring 02-02-2053 · AJB No. 81/2023"),
    ("PKKPR", "No. 10082310215108014 · OSS 09-08-2023 · KBLI 68111 · 40,000 m²"),
    ("Notices", "pt.cybervalley.estate@gmail.com · 15:00 WITA"),
];

const COMMANDS: &[(&str, &str)] = &[
    ("/deals", "every published deal"),
    ("/hold <plot> <name>", "the name the Holder agreed to show"),
    ("/form <plot> upfront|index [premium %]", "the form of the lease"),
    ("/price <plot> <usd>", "the plot price when it differs from the book"),
    ("/paid <plot> <usd> bank|eth [reference]", "a payment received · the first one reserves the flat"),
    ("/status <plot> reserved|transfer|owned|available", "what the map paints"),
    ("/step <plot> kyc|dd|docs|notary|signed [YYYY-MM-DD]", "a step done · signed makes the flat owned"),
];

#[derive(Clone)]
struct DealRow {
    id: String,
    name: String,
    deal: PublishedDeal,
    data: Option<PlotData>,
}

fn steps_line(d: &PublishedDeal) -> String {
    let order = ["request", "kyc", "dd", "premium", "docs", "notary", "signed"];
    order
        .iter()
        .map(|s| match d.steps.get(*s) {
            Some(on) => format!("{s} {on}"),
            None => format!("{s} —"),
        })
        .collect::<Vec<_>>()
        .join(" · ")
}

#[component]
pub fn AdminPage() -> impl IntoView {
    Effect::new(move |_| {
        document().set_title("Cyberia — admin");
    });
    let rows = LocalResource::new(|| async move {
        let index = fetch_index().await;
        let map = load_map();
        let mut out = Vec::new();
        let mut ids: Vec<_> = index.keys().cloned().collect();
        ids.sort();
        for id in ids {
            let deal = index[&id].clone();
            let data = fetch_plot_data(&id).await;
            let name = map
                .phase0
                .iter()
                .find(|f| f.id == id)
                .map(|f| bare_name(&f.name))
                .unwrap_or_else(|| id.to_uppercase());
            out.push(DealRow { id, name, deal, data });
        }
        out
    });
    let map = load_map();
    let inventory: Vec<Item> = library()
        .iter()
        .filter(|s| s.section.starts_with("land and company"))
        .flat_map(|s| s.items.clone())
        .collect();

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
                        <CyberiaNav active="" />
                    </div>
                </div>
            </div>
            <div class="cities-stage plot-stage-wrap">
                <div class="cities-hero plot-hero">
                    <div>
                        <div class="cities-kicker">"VALLEY · NOTARY · ADMIN"</div>
                        <h2 class="cities-title">"Deals"</h2>
                        <p class="cities-lead">
                            "What the valley published for every flat with a deal: holder, form, money received, steps, the ground — and the recitals the notary fills from. Actions go through the hard force bot."
                        </p>
                    </div>
                </div>
                <div class="library">
                    {move || match rows.get() {
                        None => view! { <p class="docs-empty">"reading the register…"</p> }.into_any(),
                        Some(list) if list.is_empty() => view! { <p class="docs-empty">"no published deal yet — the first /paid or /status in the hard force chat opens the register"</p> }.into_any(),
                        Some(list) => list.into_iter().map(|r| {
                            let book = terms(&r.id);
                            let drawn = map.phase0.iter().find(|f| f.id == r.id).map(|f| area_m2(&f.coords)).unwrap_or(0.0);
                            let survey = r.data.as_ref().and_then(|d| d.survey.as_ref()).map(|s| format!("{:.2} m² surveyed {}", s.m2, s.on))
                                .or_else(|| book.and_then(|t| t.measured_m2).map(|m| format!("{m:.2} m² surveyed · lease book")))
                                .unwrap_or_else(|| format!("{:.0} m² in plan · no survey", drawn));
                            let media = r.data.as_ref().map(|d| d.media.len()).unwrap_or(0);
                            let price = r.deal.price_usd.or_else(|| book.and_then(|t| t.price()));
                            let premium = match (r.deal.form.as_str(), r.deal.premium_pct) {
                                ("upfront", _) => "100% · upfront".to_string(),
                                ("index", Some(p)) => format!("{p}% · the rest by the century index"),
                                _ => "—".to_string(),
                            };
                            let href = format!("/plot/{}", r.id);
                            view! {
                                <section class="plot-card">
                                    <div class="cyberia-panel-h">
                                        <span class="panel-kicker"><a href=href style="color: inherit; text-decoration: none;">{r.name.clone()}</a></span>
                                        <span class="panel-sub">{format!("{} · {}", r.deal.status.to_uppercase(), r.deal.line())}</span>
                                    </div>
                                    <div class="cabinet-grid">
                                        <div class="cabinet-cell">
                                            <div class="deal-now-step">"DEAL"</div>
                                            <div class="pay-row"><span class="pay-k">"holder"</span><span class="pay-v">{if r.deal.holder.is_empty() { "—".to_string() } else { r.deal.holder.clone() }}</span></div>
                                            <div class="pay-row"><span class="pay-k">"price"</span><span class="pay-v">{price.map(fmt_usd).unwrap_or_else(|| "—".into())}</span></div>
                                            <div class="pay-row"><span class="pay-k">"premium"</span><span class="pay-v">{premium}</span></div>
                                            <div class="pay-row"><span class="pay-k">"received"</span><span class="pay-v">{fmt_usd(r.deal.paid_usd)}</span></div>
                                            <div class="deal-now-since">{steps_line(&r.deal)}</div>
                                        </div>
                                        <div class="cabinet-cell">
                                            <div class="deal-now-step">"PAYMENTS"</div>
                                            {if r.deal.payments.is_empty() {
                                                view! { <p class="docs-empty">"none received"</p> }.into_any()
                                            } else {
                                                r.deal.payments.iter().map(|p| view! {
                                                    <div class="deal-event">
                                                        <span class="deal-event-step">{p.method.to_uppercase()}</span>
                                                        <span class="deal-event-note">{format!("{} · {}", fmt_usd(p.usd), p.reference)}</span>
                                                        <span class="deal-event-ts">{p.on.clone()}</span>
                                                    </div>
                                                }).collect_view().into_any()
                                            }}
                                        </div>
                                        <div class="cabinet-cell">
                                            <div class="deal-now-step">"RECITALS · FOR THE NOTARY"</div>
                                            <div class="pay-row"><span class="pay-k">"plot"</span><span class="pay-v">{format!("{} on the estate map · Annex B", r.name.clone())}</span></div>
                                            <div class="pay-row"><span class="pay-k">"area"</span><span class="pay-v">{survey}</span></div>
                                            <div class="pay-row"><span class="pay-k">"form and price"</span><span class="pay-v">{format!("{} · premium {}", price.map(fmt_usd).unwrap_or_else(|| "—".into()), r.deal.premium_pct.map(|p| format!("{p}%")).unwrap_or_else(|| "100%".into()))}</span></div>
                                            <div class="pay-row"><span class="pay-k">"term"</span><span class="pay-v">"25 years from signing · within the Title's 02-02-2052"</span></div>
                                            <div class="pay-row"><span class="pay-k">"media"</span><span class="pay-v">{format!("{media} files on the page")}</span></div>
                                            {TITLE.iter().map(|(k, v)| view! {
                                                <div class="pay-row"><span class="pay-k">{*k}</span><span class="pay-v">{*v}</span></div>
                                            }).collect_view()}
                                        </div>
                                    </div>
                                </section>
                            }
                        }).collect_view().into_any(),
                    }}

                    <section class="plot-card">
                        <div class="cyberia-panel-h">
                            <span class="panel-kicker">"DUE DILIGENCE PACK · ANNEX C"</span>
                            <span class="panel-sub">"the inventory, with the file on IPFS where pinned"</span>
                        </div>
                        <div class="docs-list">
                            {inventory.into_iter().map(|it| {
                                let meta = it.meta();
                                match it.href() {
                                    Some(h) => view! {
                                        <div class="docs-row"><a class="doc-dl" href=h target="_blank" rel="noopener"><span class="doc-name">{it.title.clone()}</span><span class="doc-meta">{meta}</span></a></div>
                                    }.into_any(),
                                    None => view! {
                                        <div class="docs-row"><span class="doc-dl doc-pending"><span class="doc-name">{it.title.clone()}</span><span class="doc-meta">{meta}</span></span></div>
                                    }.into_any(),
                                }
                            }).collect_view()}
                        </div>
                    </section>

                    <section class="plot-card">
                        <div class="cyberia-panel-h">
                            <span class="panel-kicker">"COMMANDS"</span>
                            <span class="panel-sub">"in the hard force chat · the bot writes the register"</span>
                        </div>
                        <div class="docs-list">
                            {COMMANDS.iter().map(|(c, what)| view! {
                                <div class="docs-row"><span class="doc-dl"><span class="doc-name cabinet-mono">{*c}</span><span class="doc-meta">{*what}</span></span></div>
                            }).collect_view()}
                        </div>
                    </section>
                </div>
            </div>
        </div>
    }
}
