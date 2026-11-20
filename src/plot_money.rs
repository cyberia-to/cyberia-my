//! The MONEY view of a flat page: the four paths a buyer can press, the
//! slider between them, what each one costs at signing and every year after,
//! and the whole term drawn as bars. It reads the price and the share the
//! buyer is holding, nothing else.

use crate::pay::{yearly_rent, TERM_YEARS};
use crate::terms::fmt_usd;
use leptos::prelude::*;

/// One money path a buyer can pick with a press.
const PATHS: &[(u32, &str)] = &[(100, "ALL NOW"), (50, "HALF NOW"), (30, "A THIRD"), (10, "LIGHT START")];

/// `price` is the whole price of the flat, `area` its size in m², `pct` the
/// share the buyer is holding at signing, `cx` the day's index fix.
pub fn money_view(
    price: Option<f64>,
    area: f64,
    pct: RwSignal<u32>,
    cx: RwSignal<Option<f64>>,
) -> AnyView {
    let Some(p) = price else {
        return view! {
            <div class="pd-view pd-money">
                <div class="pm-none">"the price of this flat is named on request"</div>
            </div>
        }.into_any();
    };
    let (per_day, per_m2) = (p / (TERM_YEARS as f64 * 365.0), p / area.max(1.0));
    view! {
        <div class="pd-view pd-money">
            <div class="pm-paths">
                {PATHS.iter().map(|&(s, label)| view! {
                    <button type="button" class=move || if pct.get() == s { "pm-path is-on" } else { "pm-path" } on:click=move |_| pct.set(s)>
                        <b>{label}</b><span>{format!("{s}% at signing")}</span>
                    </button>
                }).collect_view()}
            </div>
            <input type="range" class="pm-slider" min="10" max="100" step="10"
                prop:value=move || pct.get().to_string()
                on:input=move |ev| pct.set(event_target_value(&ev).parse().unwrap_or(100)) />
            <div class="pm-tiles">
                <div class="pm-tile go">
                    <span>"at signing"</span>
                    <b>{move || fmt_usd(p * pct.get() as f64 / 100.0)}</b>
                    <i>{move || format!("{}% of {}", pct.get(), fmt_usd(p))}</i>
                </div>
                <div class="pm-tile cx">
                    <span>"each year after"</span>
                    <b>{move || { let r = yearly_rent(p, pct.get()); if r <= 0.0 { "nothing".to_string() } else { fmt_usd(r) } }}</b>
                    <i>{move || {
                        let r = yearly_rent(p, pct.get());
                        match (r > 0.0, cx.get()) {
                            (false, _) => "no rent — it is paid".to_string(),
                            (true, Some(l)) => format!("{:.0} CX · the century index keeps its value", r / l),
                            (true, None) => "written in CX, the century index".to_string(),
                        }
                    }}</i>
                </div>
                <div class="pm-tile">
                    <span>"the whole price, per day of the term"</span>
                    <b>{format!("${per_day:.2}")}</b>
                    <i>{format!("{} over {TERM_YEARS} years · ${:.0} a m²", fmt_usd(p), per_m2)}</i>
                </div>
            </div>
            // the 25 years as bars: what leaves your pocket, year by year
            <div class="pm-years">
                {move || {
                    let first = p * pct.get() as f64 / 100.0;
                    let r = yearly_rent(p, pct.get());
                    let top = first.max(r).max(1.0);
                    let mut bars = vec![view! {
                        <i class="pm-bar go" style:height=format!("{:.1}%", first / top * 100.0) title=format!("at signing · {}", fmt_usd(first))></i>
                    }.into_any()];
                    for y in 1..=TERM_YEARS {
                        bars.push(view! {
                            <i class="pm-bar" style:height=format!("{:.1}%", (r / top * 100.0).max(if r > 0.0 { 1.5 } else { 0.0 })) title=format!("year {y} · {}", fmt_usd(r))></i>
                        }.into_any());
                    }
                    bars
                }}
            </div>
            <div class="pm-axis"><span>"signing"</span><span>{format!("year {TERM_YEARS}")}</span></div>
        </div>
    }.into_any()
}
