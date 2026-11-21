//! Holders' offers as the public sees them: a flat that is held can still be
//! had — resold or subleased by its holder, or through the holder's agent.
//! The offers come from the gate's public file; these two pieces show them
//! on a flat and as a list on the map.

use crate::land::load_map;
use crate::market::Offer;
use crate::terms::{bare_name, fmt_usd};
use leptos::prelude::*;

fn plot_name(id: &str) -> String {
    load_map().phase0.iter().find(|f| f.id == id).map(|f| bare_name(&f.name)).unwrap_or_else(|| id.to_uppercase())
}

fn price_line(o: &Offer) -> String {
    match o.deal.as_str() {
        "lease" => format!(
            "{} a year{}",
            fmt_usd(o.rent_usd_year.unwrap_or(o.price_usd)),
            o.term_years.map(|t| format!(" · {t} years")).unwrap_or_default()
        ),
        _ => fmt_usd(o.price_usd),
    }
}

/// The offers on one flat, each with the way to its seller.
#[component]
pub fn FlatOffers(plot: String, #[prop(into)] offers: Signal<Vec<Offer>>) -> impl IntoView {
    let name = plot_name(&plot);
    view! {
        {move || {
            let mine: Vec<Offer> = offers.get().into_iter().filter(|o| o.plot == plot).collect();
            let name = name.clone();
            (!mine.is_empty()).then(|| view! {
                <div class="of">
                    {mine.into_iter().map(|o| view! {
                        <div class="of-card">
                            <div class="of-top"><span class="of-tag">{o.deal_label()}</span><b>{price_line(&o)}</b></div>
                            <div class="of-by">{o.by_line()}</div>
                            {(!o.words.is_empty()).then(|| view! { <div class="of-words">{o.words.clone()}</div> })}
                            <a class="of-ask" href=o.ask_href(&name) target="_blank" rel="noopener">
                                {if o.seller.role == "agent" { "WRITE TO THE AGENT →" } else { "WRITE TO THE HOLDER →" }}
                            </a>
                        </div>
                    }).collect_view()}
                </div>
            })
        }}
    }
}

/// Every live offer as a list; a press picks the flat.
#[component]
pub fn OffersList(#[prop(into)] offers: Signal<Vec<Offer>>, #[prop(into)] on_pick: Callback<String>) -> impl IntoView {
    view! {
        {move || {
            let all = offers.get();
            (!all.is_empty()).then(|| view! {
                <div class="open-now">
                    <div class="open-now-h">
                        <span style="color: var(--cyber-yellow);">"FROM HOLDERS"</span>
                        <span class="dim">{format!("{} resale and sublease offers", all.len())}</span>
                    </div>
                    {all.into_iter().take(8).map(|o| {
                        let pick = o.plot.clone();
                        view! {
                            <button type="button" class="open-now-row" on:click=move |_| on_pick.run(pick.clone())>
                                <span class="open-now-dot" style="background: var(--cyber-yellow);"></span>
                                <span class="open-now-name">{plot_name(&o.plot)}</span>
                                <span class="open-now-size">{o.deal_label().to_lowercase()}</span>
                                <span class="open-now-price" style="color: var(--cyber-yellow);">{price_line(&o)}</span>
                            </button>
                        }
                    }).collect_view()}
                </div>
            })
        }}
    }
}
