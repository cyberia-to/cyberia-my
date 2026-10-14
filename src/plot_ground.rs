//! What the ground says about a flat — the triangulation survey and the
//! photos and drone video — or the button that orders them.

use crate::deals::fmt_ts;
use crate::orders::{
    fetch_plot_data, load_orders_log, order_href, ordered, record_order, OrderKind, PlotData,
};
use leptos::prelude::*;

/// `book_m2` / `book_on`: a survey already in the lease book (terms.rs).
#[component]
pub fn GroundPanels(
    plot_id: String,
    plot_name: String,
    book_m2: Option<f64>,
    book_on: String,
    drawn_m2: f64,
) -> impl IntoView {
    let data = LocalResource::new({
        let id = plot_id.clone();
        move || {
            let id = id.clone();
            async move { fetch_plot_data(&id).await.unwrap_or_default() }
        }
    });
    let log = RwSignal::new(load_orders_log());

    let order_button = {
        let pid = plot_id.clone();
        let pname = plot_name.clone();
        move |kind: OrderKind| {
            let href = order_href(kind, &pid, &pname);
            let pid_c = pid.clone();
            let when = Memo::new(move |_| ordered(&log.get(), kind, &pid_c));
            view! {
                <div class="ground-order">
                    <p class="docs-note">{kind.line()}</p>
                    <a
                        class="intent-commit lease ground-order-btn"
                        href=href
                        target="_blank"
                        rel="noopener"
                        on:click={
                            let pid = pid.clone();
                            move |_| log.update(|l| record_order(l, kind, &pid))
                        }
                    >
                        {move || if when.get().is_some() { format!("{} AGAIN", kind.label()) } else { kind.label().to_string() }}
                    </a>
                    {move || when.get().map(|ts| view! {
                        <p class="deal-now-since">{format!("ordered {} · the hard force answers in Telegram", fmt_ts(ts))}</p>
                    })}
                </div>
            }
        }
    };

    let survey_view = {
        let ob = order_button.clone();
        move |d: PlotData| {
            if let Some(s) = d.survey {
                let line = [
                    (!s.on.is_empty()).then(|| s.on.clone()),
                    (!s.method.is_empty()).then(|| s.method.clone()),
                    (!s.by.is_empty()).then(|| format!("by {}", s.by)),
                ]
                .into_iter()
                .flatten()
                .collect::<Vec<_>>()
                .join(" · ");
                return view! {
                    <div class="ground-fact">
                        <div class="deal-now-price">{format!("{:.2} m² on the slope", s.m2)}</div>
                        <div class="deal-now-line">{line}</div>
                        <div class="deal-now-since">{format!("{drawn_m2:.0} m² in plan on the map — the slope reads larger, the survey governs")}</div>
                        {(!s.file.is_empty()).then(|| view! {
                            <a class="doc-dl" href=s.file.clone() target="_blank" rel="noopener">
                                <span class="doc-name">"SURVEY SHEET"</span>
                                <span class="doc-meta">"on IPFS"</span>
                            </a>
                        })}
                    </div>
                }
                .into_any();
            }
            if let Some(m2) = book_m2 {
                return view! {
                    <div class="ground-fact">
                        <div class="deal-now-price">{format!("{m2:.2} m² on the slope")}</div>
                        <div class="deal-now-line">{format!("triangulation · {book_on} · in the lease book")}</div>
                        <div class="deal-now-since">{format!("{drawn_m2:.0} m² in plan on the map — the slope reads larger, the survey governs")}</div>
                    </div>
                }
                .into_any();
            }
            view! {
                <div class="ground-fact">
                    <div class="deal-now-line">{format!("no triangulation survey yet · {drawn_m2:.0} m² in plan on the map, the horizontal projection")}</div>
                    {ob(OrderKind::Survey)}
                </div>
            }
            .into_any()
        }
    };

    let media_view = {
        let ob = order_button.clone();
        move |d: PlotData| {
            if d.media.is_empty() {
                return view! {
                    <div class="ground-fact">
                        <div class="deal-now-line">"no photos or drone video yet"</div>
                        {ob(OrderKind::Media)}
                    </div>
                }
                .into_any();
            }
            view! {
                <div class="plot-media-grid">
                    {d.media.into_iter().map(|m| {
                        let caption = [
                            (!m.taken.is_empty()).then(|| m.taken.clone()),
                            (!m.note.is_empty()).then(|| m.note.clone()),
                        ].into_iter().flatten().collect::<Vec<_>>().join(" · ");
                        let url = m.url.clone();
                        view! {
                            <figure class="plot-media">
                                {if m.kind == "video" {
                                    view! { <video controls preload="metadata" src=url.clone()></video> }.into_any()
                                } else {
                                    view! { <a href=url.clone() target="_blank" rel="noopener"><img src=url.clone() loading="lazy" alt=caption.clone() /></a> }.into_any()
                                }}
                                <figcaption>{caption}</figcaption>
                            </figure>
                        }
                    }).collect_view()}
                </div>
            }
            .into_any()
        }
    };

    view! {
        <section class="plot-card plot-survey">
            <div class="cyberia-panel-h">
                <span class="panel-kicker">"SURVEY"</span>
                <span class="panel-sub">"triangulation on the slope · outranks the map"</span>
            </div>
            {move || match data.get() {
                Some(d) => survey_view(d),
                None => view! { <p class="docs-empty">"reading the ground…"</p> }.into_any(),
            }}
        </section>
        <section class="plot-card plot-media-card">
            <div class="cyberia-panel-h">
                <span class="panel-kicker">"MEDIA"</span>
                <span class="panel-sub">"photos · drone video"</span>
            </div>
            {move || match data.get() {
                Some(d) => media_view(d),
                None => view! { <p class="docs-empty">"reading the ground…"</p> }.into_any(),
            }}
        </section>
    }
}
