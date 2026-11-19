//! `/plot/:id` — a flat's own page, made to be wanted. One screen, no
//! scroll: a stage with four views of the flat (where it lies, its plan, its
//! money, its papers) and a rail that always carries the price and the one
//! thing to do next. The work that follows a yes — identity, payment,
//! papers, notary — lives on its own track at `/plot/:id/deal`.

use crate::aerial::{district_stock, nearby_places, Aerial};
use crate::deal::deal_started;
use crate::deals::{flat_status, holder, load_deals, load_jv, request_jv, FlatStatus};
use crate::docs::library;
use crate::globe::BASE;
use crate::land::{area_m2, load_map, LandFlat, FLAG_SVG};
use crate::nav::CyberiaNav;
use crate::orders::{
    load_orders_log, order_href, ordered, record_order, support_href, OrderKind, PlotData,
    SUPPORT_HANDLE,
};
use crate::pay::{yearly_rent, TERM_YEARS};
use crate::plot_board::LandBoard;
use crate::plot_land::Frame;
use crate::terms::{bare_name, fmt_usd, land_use, size_line, terms, LandUse, AVALON_TRACKS};
use leptos::prelude::*;
use leptos_router::hooks::use_params_map;
use std::sync::Arc;

fn district_color(zone: &str) -> &'static str {
    let z = zone.to_lowercase();
    if z.contains("avalon") {
        "#ff5b00"
    } else if z.contains("sinwood") {
        "#00fe00"
    } else if z.contains("bridge") {
        "#00acff"
    } else if z.contains("core") {
        "#fcf000"
    } else if z.contains("ether") {
        "#d500f9"
    } else if z.contains("asgard") {
        "#ff0000"
    } else if z.contains("edem") || z.contains("canyon") {
        "#304ffe"
    } else {
        "#ffffff"
    }
}

/// `sinwood` → `Sinwood`
fn title_case(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
        None => String::new(),
    }
}

fn encode(s: &str) -> String {
    String::from(js_sys::encode_uri_component(s))
}

pub fn text_data_url(mime: &str, body: &str) -> String {
    format!("data:{mime};charset=utf-8,{}", String::from(js_sys::encode_uri_component(body)))
}

fn passport_json(flat: &LandFlat, frame: &Frame, status: FlatStatus) -> String {
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

/// The four views of a flat, in the order a buyer walks them.
const TABS: &[(&str, &str)] = &[
    ("PLACE", "where it lies"),
    ("PLAN", "its shape and size"),
    ("MONEY", "what it costs you"),
    ("PAPERS", "what stands behind it"),
];

/// One money path a buyer can pick with a press.
const PATHS: &[(u32, &str)] = &[(100, "ALL NOW"), (50, "HALF NOW"), (30, "A THIRD"), (10, "LIGHT START")];

#[component]
pub fn PlotPage() -> impl IntoView {
    let params = use_params_map();
    let map = load_map();
    let id = params.get_untracked().get("id").unwrap_or_default();
    let flat = map.phase0.iter().find(|f| f.id == id).cloned();

    let Some(flat) = flat else {
        document().set_title("Cyberia — no such flat");
        return view! {
            <div class="page-shell" style="padding:40px;">
                <h1 style="color: var(--cyber-red);">"404"</h1>
                <p style="color:#777777; margin-top:12px;">{format!("no flat called {id} on the Gesing map")}</p>
                <a href=format!("{BASE}/map") style="color: var(--cyber-green);">"← back to the map"</a>
            </div>
        }
        .into_any();
    };

    let frame = Arc::new(Frame::new(&flat.coords));
    let deals = RwSignal::new(load_deals());
    leptos::task::spawn_local(async move {
        let index = crate::orders::fetch_index().await;
        if !index.is_empty() {
            deals.update(|d| crate::deals::merge_published(d, &index));
        }
    });
    let pdata = RwSignal::new(None::<PlotData>);
    {
        let fid = flat.id.clone();
        leptos::task::spawn_local(async move {
            pdata.set(crate::orders::fetch_plot_data(&fid).await);
        });
    }
    let cx = RwSignal::new(None::<f64>);
    leptos::task::spawn_local(async move {
        cx.set(crate::orders::fetch_cx_level().await);
    });
    let log = RwSignal::new(load_orders_log());
    let jv = RwSignal::new(load_jv());

    let name = bare_name(&flat.name);
    let book = terms(&flat.id);
    let price = book.and_then(|t| t.price());
    let zone = if flat.zone.is_empty() { "gesing".to_string() } else { flat.zone.clone() };
    let area = area_m2(&flat.coords);
    let dcol = district_color(&zone);
    let lu = land_use(&flat.id);
    let ink = lu.color();
    let size = size_line(book, area);
    let near = nearby_places(&flat);

    let status = Memo::new({
        let fid = flat.id.clone();
        move |_| flat_status(&fid, &deals.get())
    });
    let mine = {
        let fid = flat.id.clone();
        Memo::new(move |_| {
            deals.get().get(&fid).map(|d| d.published.is_none() && !d.cancelled).unwrap_or(false) || deal_started(&fid)
        })
    };
    let stock = Memo::new({
        let z = flat.zone.clone();
        move |_| district_stock(&z, &deals.get())
    });

    {
        // the same title the flat's static page carries for crawlers
        let title = format!("{} · {} {} in {}, Cyber Valley, Bali", name, size, lu.label().to_lowercase(), title_case(&zone));
        Effect::new(move |_| document().set_title(&title));
    }

    // the flats before and after this one in its district — the deck you flip through
    let deck: Vec<&LandFlat> = map.phase0.iter().filter(|f| f.zone == flat.zone).collect();
    let pos = deck.iter().position(|f| f.id == flat.id).unwrap_or(0);
    let step_to = |d: isize| {
        let n = deck.len() as isize;
        let f = deck[((pos as isize + d).rem_euclid(n.max(1))) as usize];
        (format!("{BASE}/plot/{}", f.id), bare_name(&f.name))
    };
    let (prev_href, prev_name) = step_to(-1);
    let (next_href, next_name) = step_to(1);

    // the views: which one is up, which ones were seen
    let tab = RwSignal::new(0usize);
    let seen = RwSignal::new(vec![true, false, false, false]);
    let go = move |i: usize| {
        tab.set(i);
        seen.update(|s| s[i] = true);
    };
    let pct = RwSignal::new(100u32);
    {
        // 1–4 pick a view, ← → flip the deck
        let (ph, nh) = (prev_href.clone(), next_href.clone());
        let handle = window_event_listener(leptos::ev::keydown, move |ev| {
            let nav = |href: &str| {
                if let Some(w) = web_sys::window() {
                    let _ = w.location().set_href(href);
                }
            };
            match ev.key().as_str() {
                "1" => go(0),
                "2" => go(1),
                "3" => go(2),
                "4" => go(3),
                "ArrowLeft" => nav(&ph),
                "ArrowRight" => nav(&nh),
                _ => {}
            }
        });
        on_cleanup(move || handle.remove());
    }

    let passport_url = text_data_url("application/json", &passport_json(&flat, &frame, status.get_untracked()));
    let kml_url = text_data_url("application/vnd.google-earth.kml+xml", &boundary_kml(&flat));
    let deal_href = format!("{BASE}/plot/{}/deal", flat.id);
    let ask_href = format!(
        "https://t.me/{SUPPORT_HANDLE}?text={}",
        encode(&format!("ASK {} — {}\nhttps://cyberia.my/plot/{}/", flat.id, name, flat.id))
    );
    let per_day = price.map(|p| p / (TERM_YEARS as f64 * 365.0));
    let per_m2 = price.map(|p| p / area.max(1.0));

    // ── the four views ────────────────────────────────────────────────

    let v_place = {
        let flat = flat.clone();
        let near = near.clone();
        let zone_u = zone.to_uppercase();
        move || {
            let near = near.clone();
            view! {
                <div class="pd-view pd-place">
                    <Aerial flat=flat.clone() radius_m=210.0 deals=deals />
                    <div class="pd-over pd-over-tl">
                        <b>{zone_u.clone()}</b>
                        <span>{move || { let (o, n) = stock.get(); format!("{o} of {n} flats open") }}</span>
                    </div>
                    <div class="pd-over pd-near">
                        <i>"a walk away"</i>
                        {near.into_iter().take(5).map(|n| view! {
                            <span><b>{format!("{:.0} m", n.dist_m)}</b>{n.name}</span>
                        }).collect_view()}
                    </div>
                    <div class="pd-over pd-legend">
                        <span><i class="lg me" style:background=ink></i>"this flat"</span>
                        <span><i class="lg open"></i>"open to lease"</span>
                        <span><i class="lg held"></i>"held"</span>
                    </div>
                </div>
            }
        }
    };

    let v_plan = {
        let frame = frame.clone();
        move || view! {
            <div class="pd-view pd-plan">
                <LandBoard frame=frame.clone() area_m2=area district_color=dcol status_color=ink />
            </div>
        }
    };

    let v_money = move || {
        let Some(p) = price else {
            return view! {
                <div class="pd-view pd-money">
                    <div class="pm-none">"the price of this flat is named on request"</div>
                </div>
            }.into_any();
        };
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
                        <b>{per_day.map(|d| format!("${d:.2}")).unwrap_or_default()}</b>
                        <i>{format!("{} over {TERM_YEARS} years · {} a m²", fmt_usd(p), per_m2.map(|v| format!("${v:.0}")).unwrap_or_default())}</i>
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
    };

    let v_papers = {
        let (fid, fname, zone_k) = (flat.id.clone(), name.clone(), zone.clone());
        move || {
            let lib = library();
            let count = |k: &str| lib.iter().find(|s| s.key == k).map(|s| s.items.iter().filter(|i| !i.cid.is_empty()).count()).unwrap_or(0);
            // the first pages of a dossier, fanned — the papers are real and you can see them
            let covers = |k: &str| {
                let imgs: Vec<String> = lib
                    .iter()
                    .find(|s| s.key == k)
                    .map(|s| s.items.iter().filter(|i| !i.cover.is_empty()).take(4).map(|i| i.cover.clone()).collect())
                    .unwrap_or_default();
                view! {
                    <div class="pp-covers">
                        {imgs.into_iter().map(|src| view! { <img src=src loading="lazy" alt="" /> }).collect_view()}
                    </div>
                }
            };
            let d = pdata.get().unwrap_or_default();
            let survey = d.survey.clone().map(|s| format!("{:.2} m² · surveyed {}", s.m2, s.on))
                .or_else(|| book.and_then(|t| t.measured_m2.map(|m| format!("{m:.2} m² · surveyed {}", t.measured_on))));
            let media_n = d.media.len();
            let can_order = status.get() == FlatStatus::Available || mine.get();
            let order = |kind: OrderKind, label: &'static str| {
                let (fid, fname) = (fid.clone(), fname.clone());
                let when = ordered(&log.get(), kind, &fid);
                view! {
                    <a class="pp-btn" href=order_href(kind, &fid, &fname) target="_blank" rel="noopener"
                        on:click=move |_| log.update(|l| record_order(l, kind, &fid))>
                        {if when.is_some() { format!("{label} · AGAIN") } else { label.to_string() }}
                    </a>
                }
            };
            view! {
                <div class="pd-view pd-papers">
                    <div class="pp-grid">
                        <a class="pp-card" href=format!("{BASE}/docs#land")>
                            <b>{count("land")}</b><span>"papers of the land"</span>
                            <i>{if zone_k == "sinwood" { "HGB 00048/Gesing · 40,000 m² registered to PT Cyber Valley Estate" } else { "the Title and every paper behind it" }}</i>
                            {covers("land")}
                        </a>
                        <a class="pp-card" href=format!("{BASE}/docs#company")>
                            <b>{count("company")}</b><span>"papers of the Company"</span>
                            <i>"PT Cyber Valley Estate as it stands at the registry"</i>
                            {covers("company")}
                        </a>
                        <a class="pp-card" href=format!("{BASE}/docs#sign")>
                            <b>{count("sign")}</b><span>"instruments you sign"</span>
                            <i>"the lease agreement and annexes A–G, in fourteen languages"</i>
                            {covers("sign")}
                        </a>
                        <div class="pp-card">
                            <b>{if survey.is_some() { "✓" } else { "—" }}</b><span>"survey of this flat"</span>
                            <i>{survey.unwrap_or_else(|| format!("{area:.0} m² in plan on the map · no ground survey yet"))}</i>
                            {can_order.then(|| order(OrderKind::Survey, "ORDER A SURVEY"))}
                        </div>
                        <div class="pp-card">
                            <b>{media_n}</b><span>"photos and drone video"</span>
                            <i>{if media_n > 0 { "shot on this flat" } else { "none yet — the hard force flies it on order" }}</i>
                            {d.media.first().map(|m| view! { <a class="pp-btn" href=m.url.clone() target="_blank" rel="noopener">"OPEN THE FIRST"</a> })}
                            {can_order.then(|| order(OrderKind::Media, "ORDER PHOTO · VIDEO"))}
                        </div>
                        <div class="pp-card">
                            <b>"4"</b><span>"files of this flat"</span>
                            <i class="pp-files">
                                <a href=passport_url.clone() download=format!("{fid}-passport.json")>"passport"</a>
                                <a href=kml_url.clone() download=format!("{fid}.kml")>"boundary kml"</a>
                                <a href=format!("/plot/{fid}/plan.svg") target="_blank" rel="noopener">"plan"</a>
                                <a href=format!("/plot/{fid}/metadata.json") target="_blank" rel="noopener">"token"</a>
                            </i>
                        </div>
                    </div>
                    <p class="pp-line">"every paper is on IPFS by its content hash — what you read is what the notary reads"</p>
                </div>
            }
        }
    };

    // ── the rail: price, scarcity, the one thing to do ────────────────

    let fid_rail = flat.id.clone();
    let zone_rail = zone.to_uppercase();
    let rail_cta = move || {
        let st = status.get();
        let hot = seen.get().iter().all(|s| *s);
        match (st, mine.get()) {
            (_, true) => view! {
                <a class="pd-cta" href=deal_href.clone()><b>"CONTINUE YOUR DEAL"</b><span>"your track is open →"</span></a>
            }.into_any(),
            (FlatStatus::Available, _) => view! {
                <a class=if hot { "pd-cta hot" } else { "pd-cta" } href=deal_href.clone()>
                    <b>"MAKE IT YOURS"</b>
                    <span>{match price { Some(p) => format!("from {} at signing →", fmt_usd(p * 0.1)), None => "name your terms →".to_string() }}</span>
                </a>
            }.into_any(),
            (FlatStatus::OnRequest, _) => {
                let fid = fid_rail.clone();
                view! {
                    <a class="pd-cta" href=ask_href.clone() target="_blank" rel="noopener" on:click=move |_| jv.update(|m| request_jv(m, &fid))>
                        <b>"ASK FOR IT"</b><span>"the valley answers in the chat →"</span>
                    </a>
                }.into_any()
            }
            _ => view! {
                <a class="pd-cta quiet" href=format!("{BASE}/map")><b>"SEE WHAT IS OPEN"</b><span>"back to the valley →"</span></a>
            }.into_any(),
        }
    };
    let fid_head = flat.id.clone();
    let status_line = move || {
        let st = status.get();
        match st {
            FlatStatus::Available => lu.line().to_string(),
            FlatStatus::OnRequest => match lu {
                LandUse::Hgb => "the whole district goes as one HGB title".to_string(),
                _ if fid_head.starts_with("avalon") => format!("joint venture · {}", AVALON_TRACKS.join(" · ")),
                _ => book.map(|t| t.kind_line()).unwrap_or_else(|| "business ground".into()),
            },
            FlatStatus::Transfer => "a deal is open on this flat".to_string(),
            FlatStatus::Owned => holder(&fid_head).map(|h| format!("held by {h}")).unwrap_or_else(|| "held".into()),
            FlatStatus::Closed => "not on offer now".to_string(),
        }
    };

    view! {
        <div class="page-shell plot-shell pd-shell">
            <div class="site-chrome cyberia-chrome">
                <div class="chrome-inner">
                    <div class="header-row1">
                        <div class="logo-zone">
                            <h1 class="logo">
                                <a href=format!("{BASE}/cities") class="brand-flag" title="home" inner_html=FLAG_SVG></a>
                                <span style="color: var(--cyber-green);">"cyber"</span>
                                <span style="color: var(--cyber-green); margin: 0 1px;">"•"</span>
                                <span style="color: #fff;">"ia"</span>
                            </h1>
                        </div>
                        <div class="cyberia-phase-pill is-real">
                            <span class="phase-dot" style:background=ink></span>
                            {move || format!("{} · {}", name.clone(), status.get().label())}
                        </div>
                        <CyberiaNav active="map" />
                    </div>
                </div>
            </div>

            <div class="pd" style:--ink=ink>
                <div class="pd-stage">
                    <div class="pd-head">
                        <a class="pd-flip" href=prev_href title=format!("← {prev_name}")>"‹"</a>
                        <div class="pd-title">
                            <div class="pd-kicker" style:color=dcol>{format!("{} · FLAT {} OF {}", zone.to_uppercase(), pos + 1, deck.len())}</div>
                            <h2>{bare_name(&flat.name)}</h2>
                        </div>
                        <a class="pd-flip" href=next_href title=format!("{next_name} →")>"›"</a>
                        <div class="pd-tabs" role="tablist">
                            {TABS.iter().enumerate().map(|(i, (t, sub))| view! {
                                <button type="button" role="tab"
                                    class=move || { let s = seen.get()[i]; match (tab.get() == i, s) { (true, _) => "pd-tab on", (false, true) => "pd-tab seen", _ => "pd-tab" } }
                                    on:click=move |_| go(i)>
                                    <b>{*t}</b><span>{*sub}</span>
                                </button>
                            }).collect_view()}
                        </div>
                    </div>
                    <div class="pd-body">
                        {move || match tab.get() {
                            0 => v_place().into_any(),
                            1 => v_plan().into_any(),
                            2 => v_money().into_any(),
                            _ => v_papers().into_any(),
                        }}
                    </div>
                </div>

                <aside class="pd-rail">
                    <div class="pd-status">
                        <b>{move || status.get().label()}</b>
                        <span>{status_line}</span>
                    </div>
                    <div class="pd-price">
                        {match price {
                            Some(p) => view! {
                                <b>{fmt_usd(p)}</b>
                                <span>{book.and_then(|t| t.are_price_usd).map(|a| format!("{} an are · {size}", fmt_usd(a))).unwrap_or_else(|| size.clone())}</span>
                            }.into_any(),
                            None => view! { <b class="ask">"on request"</b><span>{size.clone()}</span> }.into_any(),
                        }}
                    </div>
                    {price.map(|p| view! {
                        <div class="pd-or">
                            <span>"or"</span>
                            <b>{fmt_usd(p * 0.1)}</b>
                            <span>{move || match cx.get() {
                                Some(l) => format!("now + {:.0} CX a year", yearly_rent(p, 10) / l),
                                None => format!("now + {} a year", fmt_usd(yearly_rent(p, 10))),
                            }}</span>
                        </div>
                    })}
                    <div class="pd-facts">
                        <div><span>"use"</span><b style:color=ink>{lu.short()}</b></div>
                        <div><span>"term"</span><b>{format!("{TERM_YEARS} years")}</b></div>
                        <div><span>"size"</span><b>{format!("{area:.0} m²")}</b></div>
                        {near.first().map(|n| view! { <div><span>"nearest"</span><b>{format!("{} · {:.0} m", n.name, n.dist_m)}</b></div> })}
                    </div>
                    <div class="pd-stock">
                        <div class="pd-stock-t">{move || { let (o, n) = stock.get(); format!("{o} of {n} open in {zone_rail}") }}</div>
                        <div class="pd-stock-bar"><i style:width=move || { let (o, n) = stock.get(); format!("{:.0}%", if n == 0 { 0.0 } else { (n - o) as f64 / n as f64 * 100.0 }) }></i></div>
                        <div class="pd-stock-s">{move || { let (o, n) = stock.get(); format!("{} taken or not on offer", n - o) }}</div>
                    </div>
                    <div class="pd-walk">
                        {near.iter().take(6).map(|n| view! { <span>{n.name.clone()}<b>{format!("{:.0} m", n.dist_m)}</b></span> }).collect_view()}
                    </div>
                    <div class="pd-seen">
                        {TABS.iter().enumerate().map(|(i, _)| view! { <i class=move || if seen.get()[i] { "on" } else { "" }></i> }).collect_view()}
                        <span>{move || { let n = seen.get().iter().filter(|s| **s).count(); if n == TABS.len() { "you have seen all of it".to_string() } else { format!("{n} of {} views seen · keys 1–4", TABS.len()) } }}</span>
                    </div>
                    {rail_cta}
                    <a class="pd-ask" href=support_href() target="_blank" rel="noopener">{format!("ask a person · @{SUPPORT_HANDLE}")}</a>
                    <a class="pd-back" href=format!("{BASE}/map?plot={}", flat.id)>"← the valley map"</a>
                </aside>
            </div>
        </div>
    }
    .into_any()
}
