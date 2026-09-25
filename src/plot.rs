//! `/plot/:id` — a flat's own page: land board, lease deal, documents.
//! `/plot/:id/lease` — the same page entered from the map's LEASE press:
//! the deal panel comes first and the request is opened on arrival.

use crate::deals::{
    advance_deal, cancel_deal, flat_status, fmt_ts, holder, load_deals, open_deal, FlatStatus,
    STEPS,
};
use crate::terms::{bare_name, fmt_usd, land_use, price_line, size_line, terms, LandUse, AVALON_TRACKS};
use crate::land::{area_m2, load_map, LandFlat, FLAG_SVG};
use crate::nav::CyberiaNav;
use crate::plot_board::LandBoard;
use crate::plot_docs::DocsPanel;
use crate::plot_land::{load_state, Frame};
use crate::wallet::push_intent;
use leptos::prelude::*;
use leptos_router::hooks::{use_location, use_params_map};
use std::sync::Arc;

fn district_color(zone: &str) -> &'static str {
    let z = zone.to_lowercase();
    if z.contains("avalon") {
        "#ff6600"
    } else if z.contains("sinwood") {
        "#00ff41"
    } else if z.contains("bridge") {
        "#00e5ff"
    } else if z.contains("core") {
        "#ffd700"
    } else if z.contains("ether") {
        "#9945ff"
    } else if z.contains("asgard") {
        "#ff0040"
    } else if z.contains("edem") || z.contains("canyon") {
        "#00ffd0"
    } else {
        "#c8c8d2"
    }
}

fn passport_json(flat: &LandFlat, frame: &Frame, status: FlatStatus) -> String {
    let deals = load_deals();
    let deal = deals.get(&flat.id);
    let st = load_state(&flat.id);
    let doc = serde_json::json!({
        "id": flat.id,
        "name": bare_name(&flat.name),
        "holder": holder(&flat.id),
        "lease_book": terms(&flat.id),
        "zone": flat.zone,
        "phase": flat.phase,
        "status": status.label(),
        "area_m2": (area_m2(&flat.coords) * 10.0).round() / 10.0,
        "perimeter_m": (frame.perimeter_m() * 10.0).round() / 10.0,
        "vertices": flat.coords.len(),
        "coords_lonlat": flat.coords,
        "grid_cell_m": crate::plot_land::CELL_M,
        "pruned_cells": st.pruned,
        "placed": st.placed,
        "deal": deal,
        "generated_ms": crate::deals::now_ms(),
        "source": "cyberia.my · phase 0 · soft3 local",
    });
    serde_json::to_string_pretty(&doc).unwrap_or_default()
}

fn boundary_kml(flat: &LandFlat) -> String {
    let ring: Vec<String> = flat
        .coords
        .iter()
        .map(|c| format!("{},{},0", c[0], c[1]))
        .collect();
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<kml xmlns=\"http://www.opengis.net/kml/2.2\"><Document><name>{name}</name><Placemark><name>{name}</name><description>{zone} · phase {phase} · cyberia.my</description><Polygon><outerBoundaryIs><LinearRing><coordinates>{coords}</coordinates></LinearRing></outerBoundaryIs></Polygon></Placemark></Document></kml>\n",
        name = bare_name(&flat.name),
        zone = flat.zone,
        phase = flat.phase,
        coords = ring.join(" "),
    )
}

#[component]
pub fn PlotPage() -> impl IntoView {
    let params = use_params_map();
    let location = use_location();
    let map = load_map();
    let id = params.get_untracked().get("id").unwrap_or_default();
    let lease_mode = location.pathname.get_untracked().trim_end_matches('/').ends_with("/lease");
    let flat = map.phase0.iter().find(|f| f.id == id).cloned();

    let Some(flat) = flat else {
        return view! {
            <div class="page-shell" style="padding:40px;">
                <h1 style="color: var(--cyber-red);">"404"</h1>
                <p style="color:#888; margin-top:12px;">{format!("no flat called {id} on the Gesing map")}</p>
                <a href="/map" style="color: var(--cyber-green);">"← back to the map"</a>
            </div>
        }
        .into_any();
    };

    let frame = Arc::new(Frame::new(&flat.coords));
    let deals = RwSignal::new(load_deals());
    let state = RwSignal::new(load_state(&flat.id));
    let name = bare_name(&flat.name);
    let owner = holder(&flat.id).map(|s| s.to_string());
    let book = terms(&flat.id);
    let zone = if flat.zone.is_empty() { "gesing".to_string() } else { flat.zone.clone() };
    let area = area_m2(&flat.coords);
    let perimeter = frame.perimeter_m();
    let dcol = district_color(&zone);
    let fid = flat.id.clone();
    let fname = flat.name.clone();

    // arrived from the map's LEASE press: open the request right away
    if lease_mode && flat_status(&fid, &deals.get_untracked()) == FlatStatus::Available {
        deals.update(|d| open_deal(d, &fid, &fname));
        push_intent("YOU", "lease", &fid);
    }

    let status = Memo::new({
        let fid = fid.clone();
        move |_| flat_status(&fid, &deals.get())
    });

    {
        let title = name.clone();
        Effect::new(move |_| {
            document().set_title(&format!("Cyberia — {title}"));
        });
    }

    let deal_panel = {
        let fid = fid.clone();
        let fname = fname.clone();
        move || {
            let all = deals.get();
            let deal = all.get(&fid).cloned();
            let st = status.get();
            let lu = land_use(&fid);
            let ink = lu.color();
            let fid_a = fid.clone();
            let fid_c = fid.clone();
            let fid_o = fid.clone();
            let fname_o = fname.clone();
            let head = match &deal {
                Some(d) if d.cancelled => ("CANCELLED".to_string(), "the request was withdrawn — the flat is open again".to_string(), "#777777"),
                Some(d) if d.is_signed() => ("SIGNED".to_string(), d.step_line().to_string(), ink),
                Some(d) => (
                    format!("{} · step {} of {}", d.step.to_uppercase(), d.step_index() + 1, STEPS.len()),
                    d.step_line().to_string(),
                    ink,
                ),
                None => (
                    st.label().to_string(),
                    match st {
                        FlatStatus::Owned => "held by its owner — no deal open".to_string(),
                        FlatStatus::OnRequest if lu == LandUse::Hgb => format!(
                            "{} — the whole district goes as one HGB title; ask for it from the map",
                            LandUse::Hgb.label().to_lowercase()
                        ),
                        FlatStatus::OnRequest if terms(&fid).map(|t| t.is_construction()).unwrap_or(false) => {
                            terms(&fid).map(|t| t.kind_line()).unwrap_or_default() + " · on request, a company or a joint venture"
                        }
                        FlatStatus::OnRequest if fid.starts_with("avalon") => format!(
                            "Avalon special project — on request, as a joint venture: {}",
                            AVALON_TRACKS.join(" · ")
                        ),
                        FlatStatus::OnRequest => "business ground — a business or a joint venture, on request".to_string(),
                        FlatStatus::Closed => match (lu, terms(&fid)) {
                            (_, Some(t)) if t.in_reregistration() => {
                                "the title is being re-registered · off sale until it settles".to_string()
                            }
                            (LandUse::Commons | LandUse::Special | LandUse::Unclassified, _) => lu.line().to_string(),
                            (LandUse::Lease | LandUse::Dual, Some(t)) if !t.is_city_land() && t.price().is_none() => {
                                "opens with the second wave".to_string()
                            }
                            (_, Some(t)) if t.is_city_land() => format!("{} — the valley keeps it open for everyone", t.kind_line()),
                            (_, Some(_)) => "the plot sheet has no price for it yet".to_string(),
                            (_, None) => "this flat is not in the plot sheet yet".to_string(),
                        },
                        _ => "no deal yet — request a lease to start the track".to_string(),
                    },
                    ink,
                ),
            };
            let price = match lu {
                LandUse::Venture => Some("business / joint-venture terms".to_string()),
                LandUse::Hgb => terms(&fid)
                    .and_then(|t| t.are_price_usd)
                    .map(|a| format!("{} / are · whole district", fmt_usd(a))),
                _ if st == FlatStatus::Closed => None,
                _ => terms(&fid).and_then(price_line),
            };
            let since = deal
                .as_ref()
                .and_then(|d| d.history.last())
                .map(|e| format!("since {}", fmt_ts(e.ts_ms)))
                .unwrap_or_default();
            let can_request = deal.as_ref().map(|d| d.cancelled).unwrap_or(true) && st == FlatStatus::Available;
            let live = deal.as_ref().map(|d| d.is_live()).unwrap_or(false);
            let next_label = deal
                .as_ref()
                .filter(|d| d.is_live())
                .and_then(|d| STEPS.get(d.step_index() + 1))
                .map(|(s, _)| format!("NEXT · {}", s.to_uppercase()))
                .unwrap_or_default();
            let history: Vec<_> = deal
                .as_ref()
                .map(|d| d.history.iter().rev().cloned().collect())
                .unwrap_or_default();
            view! {
                <div class="deal-now" style:--st=head.2>
                    <div class="deal-now-step">{head.0}</div>
                    <div class="deal-now-line">{head.1}</div>
                    {price.map(|p| view! { <div class="deal-now-price">{p}</div> })}
                    <div class="deal-now-since">{since}</div>
                    <div class="deal-steps">
                        {STEPS.iter().enumerate().map(|(i, (s, _))| {
                            let done = deal.as_ref().map(|d| !d.cancelled && i <= d.step_index()).unwrap_or(false);
                            view! { <span class=if done { "deal-dot on" } else { "deal-dot" } title=*s></span> }
                        }).collect_view()}
                    </div>
                    <div class="deal-actions">
                        {can_request.then(move || view! {
                            <button type="button" class="intent-commit lease" on:click=move |_| {
                                deals.update(|d| open_deal(d, &fid_o, &fname_o));
                                push_intent("YOU", "lease", &fid_o);
                            }>"REQUEST LEASE"</button>
                        })}
                        {live.then(move || view! {
                            <button type="button" class="intent-commit" on:click=move |_| {
                                deals.update(|d| advance_deal(d, &fid_a, "step confirmed on the flat page"));
                            }>{next_label.clone()}</button>
                            <button type="button" class="gps-btn" on:click=move |_| {
                                deals.update(|d| cancel_deal(d, &fid_c, "withdrawn by the requester"));
                            }>"CANCEL"</button>
                        })}
                    </div>
                </div>
                <div class="deal-history">
                    {if history.is_empty() {
                        view! { <p class="docs-empty">"no history yet"</p> }.into_any()
                    } else {
                        view! {
                            {history.into_iter().map(|e| view! {
                                <div class="deal-event">
                                    <span class="deal-event-step">{e.step.to_uppercase()}</span>
                                    <span class="deal-event-note">{e.note}</span>
                                    <span class="deal-event-ts">{fmt_ts(e.ts_ms)}</span>
                                </div>
                            }).collect_view()}
                        }.into_any()
                    }}
                </div>
            }
        }
    };

    let passport = passport_json(&flat, &frame, status.get_untracked());
    let kml = boundary_kml(&flat);
    let map_href = format!("/map?plot={}", flat.id);
    let owner_line = owner.clone().map(|h| format!(" · held by {h}")).unwrap_or_default();
    let fid_pill = flat.id.clone();
    let lu_page = land_use(&flat.id);

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
                        <div class="cyberia-phase-pill">
                            <span class="phase-dot" style:background=land_use(&fid_pill).color()></span>
                            {move || format!("{} · {}", name.clone(), status.get().label())}
                        </div>
                        <CyberiaNav active="map" />
                    </div>
                </div>
            </div>

            <div class="cities-stage plot-stage-wrap">
                <div class="cities-hero plot-hero">
                    <div>
                        <div class="cities-kicker" style:color=dcol>{format!("FLAT · {}", zone.to_uppercase())}</div>
                        <h2 class="cities-title">{bare_name(&flat.name)}</h2>
                        <p class="cities-lead">
                            {format!(
                                "{} · {} · sheet: {} · {:.0} m perimeter · {} verts{owner_line}",
                                lu_page.label(),
                                size_line(book, area),
                                book.map(|t| t.kind_line()).unwrap_or_else(|| "not in the plot sheet".into()),
                                perimeter,
                                flat.coords.len(),
                            )}
                        </p>
                    </div>
                    <a class="cta-btn cta-lease dock-found" href=map_href style="text-decoration:none; max-width: 220px;">
                        <span class="cta-copy">
                            <span class="cta-title">"← MAP"</span>
                            <span class="cta-sub">"back to the valley"</span>
                        </span>
                    </a>
                </div>

                <div class=if lease_mode { "plot-stage lease-first" } else { "plot-stage" }>
                    <div class="plot-col plot-col-land">
                        <LandBoard
                            plot_id=flat.id.clone()
                            frame=frame.clone()
                            state=state
                            district_color=dcol
                            status_color=land_use(&flat.id).color()
                        />
                    </div>
                    <div class="plot-col plot-col-side">
                        <section class=if lease_mode { "plot-card plot-deal is-lease" } else { "plot-card plot-deal" }>
                            <div class="cyberia-panel-h">
                                <span class="panel-kicker">"DEAL"</span>
                                <span class="panel-sub">"lease track · current first, history below"</span>
                            </div>
                            {deal_panel}
                        </section>
                        <DocsPanel
                            plot_id=flat.id.clone()
                            slug=flat.id.clone()
                            passport=passport
                            kml=kml
                        />
                    </div>
                </div>
            </div>
        </div>
    }
    .into_any()
}
