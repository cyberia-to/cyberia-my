//! The flat page's side column — what this flat is to the person looking at
//! it, and the one thing they can do next.
//!
//! Status decides everything. AVAILABLE: lease it — the premium flow opens.
//! ON REQUEST: ask. RESERVED / IN TRANSFER / OWNED: whose it is; the holder's
//! own browser keeps its payments and orders. NOT ON OFFER: why. Under the
//! status: the ground (survey, photos) — orderable by a prospective holder
//! or the holder — and the flat's papers.

use crate::deals::{
    cancel_deal, flat_status, fmt_ts, holder, load_jv, open_deal, request_jv, Deal, FlatStatus,
    STEPS,
};
use crate::orders::{
    load_orders_log, order_href, ordered, record_order, support_href, OrderKind, PlotData,
    SUPPORT_HANDLE,
};
use crate::pay::{load_sent, sent_for};
use crate::plot_pay::PayPanel;
use crate::terms::{fmt_usd, price_line, terms, LandUse, AVALON_TRACKS};
use leptos::prelude::*;
use std::collections::HashMap;

fn encode(s: &str) -> String {
    String::from(js_sys::encode_uri_component(s))
}

pub fn text_data_url(mime: &str, body: &str) -> String {
    format!("data:{mime};charset=utf-8,{}", String::from(js_sys::encode_uri_component(body)))
}

/// Is this browser party to the flat — a request, a payment, an order?
fn is_mine(id: &str, deals: &HashMap<String, Deal>) -> bool {
    deals.get(id).map(|d| d.published.is_none() && !d.cancelled).unwrap_or(false)
        || !sent_for(&load_sent(), id).is_empty()
        || load_orders_log().iter().any(|o| o.plot == id)
}

/// The status line: label, one line under it, the ink.
fn head(id: &str, st: FlatStatus, deal: &Option<Deal>) -> (String, String) {
    let lu = crate::terms::land_use(id);
    let book = terms(id);
    if let Some(pd) = deal.as_ref().and_then(|d| d.published.clone()) {
        let who = if pd.holder.is_empty() { String::new() } else { format!(" · {}", pd.holder) };
        return match pd.status.as_str() {
            "reserved" => ("RESERVED".into(), format!("premium received{who}")),
            "owned" => ("OWNED".into(), format!("held{}", if who.is_empty() { String::new() } else { format!(" by {}", pd.holder) })),
            _ => ("IN TRANSFER".into(), format!("papers in motion{who}")),
        };
    }
    match st {
        FlatStatus::Transfer => ("REQUESTED".into(), "your request · the premium reserves the flat".into()),
        FlatStatus::Owned => (
            "OWNED".into(),
            holder(id).map(|h| format!("held by {h}")).unwrap_or_else(|| "held by its holder".into()),
        ),
        FlatStatus::Available => ("AVAILABLE".into(), lu.line().into()),
        FlatStatus::OnRequest => (
            "ON REQUEST".into(),
            match lu {
                LandUse::Hgb => "the whole district goes as one HGB title".into(),
                _ if id.starts_with("avalon") => format!("joint venture · {}", AVALON_TRACKS.join(" · ")),
                _ => book.map(|t| t.kind_line()).unwrap_or_else(|| "business ground".into()),
            },
        ),
        FlatStatus::Closed => (
            "NOT ON OFFER".into(),
            match (lu, book) {
                (_, Some(t)) if t.in_reregistration() => "the title is being re-registered".into(),
                (LandUse::Commons, _) => "city land · shared by everyone".into(),
                (LandUse::Special, _) => "special place · held by the valley".into(),
                (_, Some(t)) if t.is_city_land() => "held by the valley".into(),
                _ => "opens with the next wave".into(),
            },
        ),
    }
}

#[component]
pub fn SideColumn(
    plot_id: String,
    plot_name: String,
    drawn_m2: f64,
    passport: String,
    kml: String,
    lease_mode: bool,
    deals: RwSignal<HashMap<String, Deal>>,
    pdata: RwSignal<Option<PlotData>>,
) -> impl IntoView {
    let id = plot_id.clone();
    let book = terms(&plot_id);
    let price = book.and_then(|t| t.price());
    let log = RwSignal::new(load_orders_log());
    let jv = RwSignal::new(load_jv());
    let pay_open = RwSignal::new(lease_mode);

    let status = Memo::new({
        let id = id.clone();
        move |_| flat_status(&id, &deals.get())
    });
    let deal = Memo::new({
        let id = id.clone();
        move |_| deals.get().get(&id).cloned()
    });
    let mine = Memo::new({
        let id = id.clone();
        move |_| {
            log.track();
            is_mine(&id, &deals.get())
        }
    });
    let published = Signal::derive({
        let id = id.clone();
        move || {
            pdata.get().and_then(|d| d.deal).filter(|d| !d.status.is_empty())
                .or_else(|| deals.get().get(&id).and_then(|d| d.published.clone()))
        }
    });
    // a prospective holder orders the survey before paying; the holder after
    let can_order = Memo::new(move |_| status.get() == FlatStatus::Available || mine.get());

    // arrived from the map's LEASE press: the request opens, the flow is up
    if lease_mode && status.get_untracked() == FlatStatus::Available {
        let (i, n) = (plot_id.clone(), plot_name.clone());
        deals.update(|d| open_deal(d, &i, &n));
    }

    let ink = crate::terms::land_use(&plot_id).color();
    let id_lease = plot_id.clone();
    let name_lease = plot_name.clone();
    let id_cancel = plot_id.clone();
    let id_ask = plot_id.clone();
    let ask_href = format!(
        "https://t.me/{SUPPORT_HANDLE}?text={}",
        encode(&format!("ASK {} — {}\nhttps://cyberia.my/plot/{}/", plot_id, plot_name, plot_id))
    );

    let status_card = move || {
        let st = status.get();
        let d = deal.get();
        let (label, line) = head(&id, st, &d);
        let on_offer = matches!(st, FlatStatus::Available | FlatStatus::OnRequest) && d.as_ref().map(|d| d.published.is_none()).unwrap_or(true);
        let price_text = match (st, crate::terms::land_use(&id), book) {
            (FlatStatus::OnRequest, LandUse::Hgb, Some(t)) => t.are_price_usd.map(|a| format!("{} / are · whole district", fmt_usd(a))),
            (FlatStatus::Available, _, Some(t)) => price_line(t),
            _ => None,
        };
        let live_local = d.as_ref().map(|d| d.published.is_none() && d.is_live()).unwrap_or(false);
        let id_l = id_lease.clone();
        let n_l = name_lease.clone();
        let id_c = id_cancel.clone();
        let id_a = id_ask.clone();
        let asked = jv.get().contains_key(&id);
        view! {
            <div class="deal-now side-status" style:--st=ink>
                <div class="deal-now-step">{label}</div>
                <div class="deal-now-line">{line}</div>
                {on_offer.then(|| price_text.map(|p| view! { <div class="deal-now-price">{p}</div> }))}
                {(st == FlatStatus::Available && !live_local && !pay_open.get()).then(|| view! {
                    <button type="button" class="intent-commit lease side-cta" on:click=move |_| {
                        deals.update(|dd| open_deal(dd, &id_l, &n_l));
                        pay_open.set(true);
                    }>"LEASE THIS FLAT"</button>
                })}
                {(st == FlatStatus::OnRequest).then(|| view! {
                    <a class="intent-commit lease side-cta" href=ask_href.clone() target="_blank" rel="noopener"
                        on:click=move |_| jv.update(|m| request_jv(m, &id_a))>
                        {if asked { "ASK AGAIN" } else { "ASK FOR IT" }}
                    </a>
                })}
                {d.as_ref().filter(|d| d.published.is_none() && !d.cancelled).map(|d| {
                    let idx = d.step_index();
                    let last = d.history.last().cloned();
                    view! {
                        <div class="deal-steps">
                            {STEPS.iter().enumerate().map(|(i, (s, _))| view! {
                                <span class=if i <= idx { "deal-dot on" } else { "deal-dot" } title=*s></span>
                            }).collect_view()}
                        </div>
                        {last.map(|e| view! { <div class="deal-now-since">{format!("{} · {}", e.note, fmt_ts(e.ts_ms))}</div> })}
                        {live_local.then(|| view! {
                            <button type="button" class="side-cancel" on:click=move |_| {
                                deals.update(|dd| cancel_deal(dd, &id_c, "withdrawn by the requester"));
                                pay_open.set(false);
                            }>"withdraw the request"</button>
                        })}
                    }
                })}
            </div>
        }
    };

    let id_pay = plot_id.clone();
    let name_pay = plot_name.clone();
    let pay_card = move || {
        let st = status.get();
        let show = pay_open.get()
            || (st == FlatStatus::Transfer && mine.get())
            || deal.get().as_ref().map(|d| d.published.is_none() && d.is_live()).unwrap_or(false);
        (show && st != FlatStatus::Owned && st != FlatStatus::Closed).then(|| view! {
            <PayPanel plot_id=id_pay.clone() plot_name=name_pay.clone() price=price published=published />
        })
    };

    let id_g = plot_id.clone();
    let name_g = plot_name.clone();
    let order_button = move |kind: OrderKind| {
        let href = order_href(kind, &id_g, &name_g);
        let id_o = id_g.clone();
        let when = Memo::new({
            let id_w = id_g.clone();
            move |_| ordered(&log.get(), kind, &id_w)
        });
        view! {
            <a class="intent-commit lease side-order" href=href target="_blank" rel="noopener"
                on:click=move |_| log.update(|l| record_order(l, kind, &id_o))>
                {move || if when.get().is_some() { format!("{} · AGAIN", kind.label()) } else { kind.label().to_string() }}
            </a>
            {move || when.get().map(|ts| view! { <span class="deal-now-since">{format!("ordered {}", fmt_ts(ts))}</span> })}
        }
    };

    let ground = {
        let ob = order_button.clone();
        move || {
            let d = pdata.get().unwrap_or_default();
            let survey = d.survey.clone().map(|s| (s.m2, s.on.clone(), s.file.clone()))
                .or_else(|| book.and_then(|t| t.measured_m2.map(|m| (m, t.measured_on.clone(), String::new()))));
            let media = d.media.clone();
            let can = can_order.get();
            let surveyed = survey.is_some();
            view! {
                <div class="ground">
                    <div class="ground-row">
                        {match survey {
                            Some((m2, on, file)) => view! {
                                <div class="ground-fig">{format!("{m2:.2} m²")}</div>
                                <div class="ground-what">{format!("surveyed on the slope · {on}")}
                                    {(!file.is_empty()).then(|| view! { " · "<a href=file target="_blank" rel="noopener">"sheet"</a> })}
                                </div>
                            }.into_any(),
                            None => view! {
                                <div class="ground-fig">{format!("{drawn_m2:.0} m²")}</div>
                                <div class="ground-what">"in plan on the map · no survey yet"</div>
                                {can.then(|| ob(OrderKind::Survey))}
                            }.into_any(),
                        }}
                        {surveyed.then(|| view! { <div class="ground-sub">{format!("{drawn_m2:.0} m² in plan on the map")}</div> })}
                    </div>
                    <div class="ground-row">
                        {if media.is_empty() {
                            view! {
                                <div class="ground-what">"no photos or drone video yet"</div>
                                {can.then(|| ob(OrderKind::Media))}
                            }.into_any()
                        } else {
                            view! {
                                <div class="plot-media-grid">
                                    {media.into_iter().map(|m| {
                                        let url = m.url.clone();
                                        let cap = if m.taken.is_empty() { m.note.clone() } else { m.taken.clone() };
                                        view! {
                                            <figure class="plot-media">
                                                {if m.kind == "video" {
                                                    view! { <video controls preload="metadata" src=url.clone()></video> }.into_any()
                                                } else {
                                                    view! { <a href=url.clone() target="_blank" rel="noopener"><img src=url.clone() loading="lazy" alt=cap.clone() /></a> }.into_any()
                                                }}
                                                <figcaption>{cap}</figcaption>
                                            </figure>
                                        }
                                    }).collect_view()}
                                </div>
                            }.into_any()
                        }}
                    </div>
                </div>
            }
        }
    };

    let passport_url = text_data_url("application/json", &passport);
    let kml_url = text_data_url("application/vnd.google-earth.kml+xml", &kml);
    let papers = view! {
        <div class="papers">
            <a href=passport_url download=format!("{plot_id}-passport.json")>"passport"</a>
            <a href=kml_url download=format!("{plot_id}.kml")>"boundary kml"</a>
            <a href=format!("/plot/{plot_id}/plan.svg") target="_blank" rel="noopener">"plan"</a>
            <a href=format!("/plot/{plot_id}/metadata.json") target="_blank" rel="noopener">"token"</a>
            <a href="/docs#land" class="papers-pack">"due diligence pack →"</a>
        </div>
    };

    view! {
        <section class="plot-card">{status_card}</section>
        {pay_card}
        <section class="plot-card">
            <div class="cyberia-panel-h"><span class="panel-kicker">"GROUND"</span></div>
            {ground}
        </section>
        <section class="plot-card">
            <div class="cyberia-panel-h"><span class="panel-kicker">"PAPERS"</span></div>
            {papers}
        </section>
        {move || (status.get() == FlatStatus::Owned && !mine.get()).then(|| view! {
            <a class="side-support" href=support_href() target="_blank" rel="noopener">"write to the valley about this flat"</a>
        })}
    }
}
