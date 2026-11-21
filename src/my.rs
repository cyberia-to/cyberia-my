//! `/my` — MY PLOTS: the flats of one person as a collection of tokens, on
//! one screen. A flat is a token (ERC-721 once its deed is signed); here it
//! is a card with its shape, lit on the map of the valley, with the one
//! thing to do next. Three shelves: HELD (bound to your wallet by the
//! valley, or signed), IN DEAL (a track you opened), WATCHING (a flat you
//! starred). The deck flips with the arrows; the map shows where each lies.
//!
//! What is yours is read from three places and said as such: the gate's
//! record of which wallet holds which flat (`market`), this browser's own
//! deals and payments, and the list of flats you starred.

use crate::aerial::{tile_nw, tile_of, TILE_Z};
use crate::deal::{deal_started, progress, step_titles};
use crate::deals::{cancel_deal, load_deals, Deal};
use crate::globe::{BASE, TILES};
use crate::land::{area_m2, load_map, LandFlat, FLAG_SVG};
use crate::market::{self, View};
use crate::nav::CyberiaNav;
use crate::orders::PublishedDeal;
use crate::pay::{load_sent, sent_for};
use crate::plot_land::Frame;
use crate::terms::{bare_name, fmt_usd, land_use, size_line, terms};
use leptos::prelude::*;
use std::collections::HashMap;

pub const WATCH_KEY: &str = "cyberia.watch.v1";

pub fn load_watch() -> Vec<String> {
    web_sys::window()
        .and_then(|w| w.local_storage().ok().flatten())
        .and_then(|ls| ls.get_item(WATCH_KEY).ok().flatten())
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

fn save_watch(list: &[String]) {
    if let Some(ls) = web_sys::window().and_then(|w| w.local_storage().ok().flatten()) {
        if let Ok(s) = serde_json::to_string(list) {
            let _ = ls.set_item(WATCH_KEY, &s);
        }
    }
}

/// Star or unstar a flat; returns whether it is starred now.
pub fn toggle_watch(id: &str) -> bool {
    let mut list = load_watch();
    let on = if let Some(i) = list.iter().position(|x| x == id) {
        list.remove(i);
        false
    } else {
        list.insert(0, id.to_string());
        true
    };
    save_watch(&list);
    on
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Shelf {
    Held,
    Deal,
    Watch,
}

impl Shelf {
    fn label(self) -> &'static str {
        match self {
            Shelf::Held => "HELD",
            Shelf::Deal => "IN DEAL",
            Shelf::Watch => "WATCHING",
        }
    }

    fn color(self) -> &'static str {
        match self {
            Shelf::Held => "#00fe00",
            Shelf::Deal => "#fcf000",
            Shelf::Watch => "#00acff",
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
struct Mine {
    flat: LandFlat,
    shelf: Shelf,
    /// bound to the signed-in wallet by the valley
    bound: bool,
}

/// The person's flats, by shelf: held first, then deals, then the starred.
fn collect(view: &Option<View>, deals: &HashMap<String, Deal>, index: &HashMap<String, PublishedDeal>, watch: &[String]) -> Vec<Mine> {
    let map = load_map();
    let sent = load_sent();
    let mut out: Vec<Mine> = Vec::new();
    let mut push = |id: &str, shelf: Shelf, bound: bool| {
        if out.iter().any(|m| m.flat.id == id) {
            return;
        }
        if let Some(f) = map.phase0.iter().find(|f| f.id == id) {
            out.push(Mine { flat: f.clone(), shelf, bound });
        }
    };
    if let Some(v) = view {
        for h in &v.holders {
            push(&h.plot, Shelf::Held, true);
        }
    }
    for (id, d) in deals {
        let owned = index.get(id).map(|p| p.status == "owned").unwrap_or(false);
        if d.published.is_none() && (d.is_signed() || (owned && !d.cancelled)) {
            push(id, Shelf::Held, false);
        }
    }
    for (id, d) in deals {
        if d.published.is_none() && !d.cancelled && (deal_started(id) || !sent_for(&sent, id).is_empty()) {
            push(id, Shelf::Deal, false);
        }
    }
    for id in watch {
        push(id, Shelf::Watch, false);
    }
    // a request this browser opened by merely visiting a page, before the track existed: a flat you looked at
    for (id, d) in deals {
        if d.published.is_none() && d.is_live() {
            push(id, Shelf::Watch, false);
        }
    }
    out
}

const TABS: &[&str] = &["TOKEN", "DEAL", "MARKET"];

#[component]
pub fn MyPlotsPage() -> impl IntoView {
    document().set_title("Cyberia — my plots");
    let map = load_map();
    let view = RwSignal::new(None::<View>);
    let busy = RwSignal::new(false);
    let err = RwSignal::new(String::new());
    leptos::task::spawn_local(async move {
        if let Some(v) = market::fetch_me().await {
            view.set(Some(v));
        }
    });
    let deals = RwSignal::new(load_deals());
    let index = RwSignal::new(HashMap::<String, PublishedDeal>::new());
    leptos::task::spawn_local(async move {
        index.set(crate::orders::fetch_index().await);
    });
    let offers = RwSignal::new(Vec::<market::Offer>::new());
    leptos::task::spawn_local(async move {
        offers.set(market::fetch_offers().await);
    });
    let watch = RwSignal::new(load_watch());

    let all = Memo::new(move |_| collect(&view.get(), &deals.get(), &index.get(), &watch.get()));
    let shelf = RwSignal::new(None::<Shelf>);
    let shown = Memo::new(move |_| {
        let s = shelf.get();
        all.get().into_iter().filter(|m| s.map(|s| m.shelf == s).unwrap_or(true)).collect::<Vec<_>>()
    });
    let picked = RwSignal::new(None::<String>);
    // the card in hand: the one picked, or the first of the shelf
    let hand = Memo::new(move |_| {
        let list = shown.get();
        let p = picked.get();
        list.iter().find(|m| Some(&m.flat.id) == p.as_ref()).cloned().or_else(|| list.first().cloned())
    });
    let tab = RwSignal::new(0usize);
    let close = RwSignal::new(false);

    {
        // ← → flip the deck, 1–3 the card's sides, Z the camera
        let handle = window_event_listener(leptos::ev::keydown, move |ev| {
            let list = shown.get_untracked();
            let at = hand.get_untracked().and_then(|h| list.iter().position(|m| m.flat.id == h.flat.id));
            let step = |d: isize| {
                if let (Some(a), n) = (at, list.len() as isize) {
                    if n > 0 {
                        picked.set(Some(list[((a as isize + d).rem_euclid(n)) as usize].flat.id.clone()));
                    }
                }
            };
            match ev.key().as_str() {
                "ArrowLeft" => step(-1),
                "ArrowRight" => step(1),
                "1" => tab.set(0),
                "2" => tab.set(1),
                "3" => tab.set(2),
                "z" | "Z" => close.update(|c| *c = !*c),
                _ => {}
            }
        });
        on_cleanup(move || handle.remove());
    }

    // ── the valley, with what is yours lit ────────────────────────────
    let all_coords: Vec<[f64; 2]> = map.phase0.iter().flat_map(|f| f.coords.iter().copied()).collect();
    let site = Frame::new(&all_coords);
    let (w_m, h_m) = (site.max_x - site.min_x, site.max_y - site.min_y);
    let pad = 40.0;
    let site_c = site.clone();
    let to_v = move |x: f64, y: f64| (x - site_c.min_x + pad, site_c.max_y - y + pad);
    let path_of = {
        let site = site.clone();
        let to_v = to_v.clone();
        move |coords: &[[f64; 2]]| {
            let mut s = String::new();
            for (i, c) in coords.iter().enumerate() {
                let (x, y) = site.to_xy(c[0], c[1]);
                let (vx, vy) = to_v(x, y);
                s.push_str(if i == 0 { "M" } else { " L" });
                s.push_str(&format!("{vx:.1},{vy:.1}"));
            }
            s.push('Z');
            s
        }
    };
    let centre_of = {
        let site = site.clone();
        let to_v = to_v.clone();
        move |f: &LandFlat| {
            let n = f.coords.len().max(1) as f64;
            let (lon, lat) = (f.coords.iter().map(|c| c[0]).sum::<f64>() / n, f.coords.iter().map(|c| c[1]).sum::<f64>() / n);
            let (x, y) = site.to_xy(lon, lat);
            to_v(x, y)
        }
    };
    let mut tiles = Vec::new();
    {
        // the ground reaches past the flats on every side, so a wide stage has no bare edges
        let reach = w_m.max(h_m);
        let (lon_w, lat_n) = site.to_lonlat(site.min_x - reach, site.max_y + reach * 0.4);
        let (lon_e, lat_s) = site.to_lonlat(site.max_x + reach, site.min_y - reach * 0.4);
        let (tx0, ty0) = tile_of(lon_w, lat_n, TILE_Z);
        let (tx1, ty1) = tile_of(lon_e, lat_s, TILE_Z);
        for ty in ty0..=ty1 {
            for tx in tx0..=tx1 {
                let (lw, ln) = tile_nw(tx, ty, TILE_Z);
                let (le, ls) = tile_nw(tx + 1, ty + 1, TILE_Z);
                let (x0, y0) = site.to_xy(lw, ln);
                let (x1, y1) = site.to_xy(le, ls);
                let (vx, vy) = to_v(x0, y0);
                tiles.push((format!("{TILES}/{TILE_Z}/{tx}/{ty}.jpg"), vx, vy, x1 - x0, y0 - y1));
            }
        }
    }
    let quiet: Vec<(String, String)> = map.phase0.iter().map(|f| (f.id.clone(), path_of(&f.coords))).collect();
    let (full_w, full_h) = (w_m + 2.0 * pad, h_m + 2.0 * pad);
    let centre_c = centre_of.clone();
    let view_box = move || match (close.get(), hand.get()) {
        (true, Some(h)) => {
            let (cx, cy) = centre_c(&h.flat);
            format!("{:.0} {:.0} 360 360", cx - 180.0, cy - 180.0)
        }
        _ => format!("0 0 {full_w:.0} {full_h:.0}"),
    };
    let path_lit = path_of.clone();
    let centre_l = centre_of.clone();

    let stage = view! {
        <svg class="mp-map" viewBox=view_box preserveAspectRatio="xMidYMid meet">
            <g class="aerial-tiles">
                {tiles.into_iter().map(|(href, x, y, w, h)| view! {
                    <image href=href x=format!("{x:.1}") y=format!("{y:.1}") width=format!("{:.1}", w + 0.6) height=format!("{:.1}", h + 0.6) preserveAspectRatio="none" />
                }).collect_view()}
            </g>
            <rect x="-9000" y="-9000" width="18000" height="18000" fill="#000" fill-opacity="0.35" />
            {quiet.into_iter().map(|(id, d)| {
                let href = format!("{BASE}/plot/{id}");
                view! {
                    <path d=d class="mp-quiet" vector-effect="non-scaling-stroke" on:click=move |_| {
                        if let Some(w) = web_sys::window() {
                            let _ = w.location().set_href(&href);
                        }
                    } />
                }
            }).collect_view()}
            {move || {
                let h = hand.get().map(|h| h.flat.id);
                all.get().into_iter().map(|m| {
                    let on = h.as_deref() == Some(m.flat.id.as_str());
                    let (cx, cy) = centre_l(&m.flat);
                    let id = m.flat.id.clone();
                    let dim = shelf.get().map(|s| s != m.shelf).unwrap_or(false);
                    view! {
                        <g class=if on { "mp-lit on" } else if dim { "mp-lit dim" } else { "mp-lit" } style:--c=m.shelf.color() on:click=move |_| { shelf.set(None); picked.set(Some(id.clone())); }>
                            {on.then(|| view! { <circle cx=format!("{cx:.1}") cy=format!("{cy:.1}") r="14" class="mp-ping" vector-effect="non-scaling-stroke" /> })}
                            <path d=path_lit(&m.flat.coords) vector-effect="non-scaling-stroke" />
                        </g>
                    }
                }).collect_view()
            }}
        </svg>
    };

    // ── the card in hand ──────────────────────────────────────────────
    let card = move || {
        let Some(m) = hand.get() else {
            return view! {
                <div class="mp-empty">
                    <b>"no flat here yet"</b>
                    <span>"star a flat on its page, open a deal, or connect the wallet the valley bound your flat to — it lands on this shelf"</span>
                    <a class="pd-cta" href=format!("{BASE}/map")><b>"PICK A FLAT"</b><span>"on the valley map →"</span></a>
                </div>
            }.into_any();
        };
        let id = m.flat.id.clone();
        let name = bare_name(&m.flat.name);
        let lu = land_use(&id);
        let book = terms(&id);
        let area = area_m2(&m.flat.coords);
        let frame = Frame::new(&m.flat.coords);
        let (aw, ah) = (frame.svg_w(), frame.svg_h());
        let price = book.and_then(|t| t.price());
        let pr = progress(&id, index.get().get(&id));
        let titles = step_titles();
        let my_listing = view.get().and_then(|v| v.listings.iter().rev().find(|l| l.plot == id && (l.status == "live" || l.status == "pending")).cloned());
        let offer = offers.get().into_iter().find(|o| o.plot == id);
        let (id_w, id_c) = (id.clone(), id.clone());
        let shelf_now = m.shelf;
        let bound = m.bound;
        view! {
            <div class="mp-token" style:--ink=lu.color() style:--c=shelf_now.color()>
                <div class="mp-art">
                    <svg viewBox=format!("0 0 {aw:.0} {ah:.0}")><path d=frame.outline_path() vector-effect="non-scaling-stroke" /></svg>
                    <span class="mp-badge">{shelf_now.label()}</span>
                    <span class="mp-id">{format!("#{id}")}</span>
                </div>
                <div class="mp-name">{name.clone()}</div>
                <div class="mp-sub">{format!("{} · {} · {}", m.flat.zone.to_uppercase(), size_line(book, area), lu.short())}</div>
            </div>
            <div class="mp-tabs" role="tablist">
                {TABS.iter().enumerate().map(|(i, t)| view! {
                    <button type="button" role="tab" class=move || if tab.get() == i { "mp-tab on" } else { "mp-tab" } on:click=move |_| tab.set(i)>{*t}</button>
                }).collect_view()}
            </div>
            <div class="mp-side">
                {match tab.get() {
                    0 => view! {
                        <div class="mp-rows">
                            <div><span>"token"</span><b>"ERC-721 · one flat, one token"</b></div>
                            <div><span>"state"</span><b>{match (shelf_now, bound) {
                                (Shelf::Held, true) => "bound to your wallet by the valley",
                                (Shelf::Held, false) => "signed in this browser · bind your wallet to manage it",
                                (Shelf::Deal, _) => "minted to your wallet when the deed is signed",
                                (Shelf::Watch, _) => "not yours yet · you are watching it",
                            }}</b></div>
                            <div><span>"size"</span><b>{format!("{area:.0} m² · {:.0} m around", frame.perimeter_m())}</b></div>
                            {price.map(|p| view! { <div><span>"price in the book"</span><b>{fmt_usd(p)}</b></div> })}
                        </div>
                        <div class="mp-links">
                            <a href=format!("{BASE}/plot/{id}")>"THE FLAT'S PAGE"</a>
                            <a href=format!("{BASE}/map?plot={id}")>"ON THE VALLEY MAP"</a>
                            <a href=format!("/plot/{id}/metadata.json") target="_blank" rel="noopener">"TOKEN METADATA"</a>
                            <a href=format!("/plot/{id}/plan.svg") target="_blank" rel="noopener">"PLAN"</a>
                        </div>
                    }.into_any(),
                    1 => view! {
                        <div class="mp-track">
                            {pr.marks.iter().zip(titles.iter()).map(|(m, t)| view! {
                                <div class=match m { 2 => "mp-step done", 1 => "mp-step wait", _ => "mp-step" }><i></i><span>{*t}</span></div>
                            }).collect_view()}
                        </div>
                        {match shelf_now {
                            Shelf::Held => view! { <p class="dl-note ok">"the deal is closed — the flat is held"</p> }.into_any(),
                            Shelf::Deal => view! {
                                <p class="dl-note">{format!("{} of {} steps confirmed · next: {}", pr.done, pr.marks.len(), pr.next)}</p>
                                <a class="pd-cta" href=format!("{BASE}/plot/{id}/deal")><b>"CONTINUE THE DEAL"</b><span>{format!("next: {} →", pr.next.to_lowercase())}</span></a>
                            }.into_any(),
                            Shelf::Watch => view! {
                                <p class="dl-note">"no deal is open on this flat from this browser"</p>
                                <a class="pd-cta" href=format!("{BASE}/plot/{id}")><b>"MAKE IT YOURS"</b><span>{price.map(|p| format!("{} · see the flat →", fmt_usd(p))).unwrap_or_else(|| "see the flat →".into())}</span></a>
                            }.into_any(),
                        }}
                    }.into_any(),
                    _ => view! {
                        {match (&my_listing, bound) {
                            (Some(l), _) => view! {
                                <div class=format!("sl-card {}", l.status)>
                                    <div class="sl-card-top"><b>{if l.deal == "lease" { "SUBLEASE" } else { "RESALE" }}</b><span class="sl-tag">{l.status.to_uppercase()}</span></div>
                                    <div class="sl-card-line">{format!("{}{}", fmt_usd(l.price_usd), if l.deal == "lease" { " a year" } else { "" })}</div>
                                    <div class="sl-card-line dim">{if l.status == "live" { "live on the map" } else { "waits for the valley's approval" }}</div>
                                </div>
                                <a class="pd-cta quiet" href=format!("{BASE}/sell")><b>"MANAGE THE OFFER"</b><span>"change the price, withdraw, your agent →"</span></a>
                            }.into_any(),
                            (None, true) => view! {
                                <p class="dl-note">"this flat is bound to your wallet. put it on the map for resale or sublease — yourself, or through an agent you give a mandate."</p>
                                <a class="pd-cta" href=format!("{BASE}/sell")><b>"PUT IT ON THE MAP"</b><span>"resale or sublease →"</span></a>
                            }.into_any(),
                            (None, false) => view! {
                                {offer.map(|o| view! {
                                    <div class="of-card">
                                        <div class="of-top"><span class="of-tag">{o.deal_label()}</span><b>{fmt_usd(o.price_usd)}</b></div>
                                        <div class="of-by">{o.by_line()}</div>
                                        <a class="of-ask" href=o.ask_href(&name) target="_blank" rel="noopener">"WRITE TO THE SELLER →"</a>
                                    </div>
                                })}
                                <p class="dl-note">{if shelf_now == Shelf::Held {
                                    "to offer this flat, the valley binds it to your wallet once — claim it and sign"
                                } else {
                                    "offering a flat is for its holder: once it is yours and bound to your wallet, it goes on the map from here"
                                }}</p>
                                {(shelf_now == Shelf::Held).then(|| view! { <a class="pd-cta quiet" href=format!("{BASE}/sell")><b>"CLAIM IT WITH YOUR WALLET"</b><span>"then offer it →"</span></a> })}
                            }.into_any(),
                        }}
                    }.into_any(),
                }}
            </div>
            <div class="mp-foot">
                {(shelf_now != Shelf::Held).then(|| view! {
                    <button type="button" class="dl-mini" on:click=move |_| {
                        // off the shelf: unstar it, and close a request this browser never sent
                        let mut w = load_watch();
                        w.retain(|x| x != &id_w);
                        save_watch(&w);
                        watch.set(w);
                        if !deal_started(&id_c) {
                            deals.update(|d| cancel_deal(d, &id_c, "taken off the shelf"));
                        }
                        picked.set(None);
                    }>"TAKE IT OFF THE SHELF"</button>
                })}
                <button type="button" class="dl-mini" on:click=move |_| close.update(|c| *c = !*c)>{move || if close.get() { "SHOW THE VALLEY · Z" } else { "FLY TO IT · Z" }}</button>
            </div>
        }.into_any()
    };

    let count = move |s: Option<Shelf>| all.get().iter().filter(|m| s.map(|s| m.shelf == s).unwrap_or(true)).count();
    let stats = move || {
        let list = all.get();
        let held: Vec<&Mine> = list.iter().filter(|m| m.shelf == Shelf::Held).collect();
        let ares: f64 = held.iter().map(|m| area_m2(&m.flat.coords) / 100.0).sum();
        let value: f64 = held.iter().filter_map(|m| terms(&m.flat.id).and_then(|t| t.price())).sum();
        let mut zones: Vec<&str> = list.iter().filter(|m| m.shelf != Shelf::Watch).map(|m| m.flat.zone.as_str()).collect();
        zones.sort();
        zones.dedup();
        let all_zones = {
            let m = load_map();
            let mut z: Vec<String> = m.phase0.iter().map(|f| f.zone.clone()).collect();
            z.sort();
            z.dedup();
            z.len()
        };
        view! {
            <div class="mp-stat"><b>{held.len()}</b><span>"held"</span></div>
            <div class="mp-stat"><b>{if ares > 0.0 { format!("{ares:.1}") } else { "0".to_string() }}</b><span>"ares held"</span></div>
            <div class="mp-stat"><b>{if value > 0.0 { fmt_usd(value) } else { "—".to_string() }}</b><span>"in the book"</span></div>
            <div class="mp-stat"><b>{format!("{} / {all_zones}", zones.len())}</b><span>"districts"</span></div>
        }
    };

    view! {
        <div class="page-shell plot-shell pd-shell">
            <div class="site-chrome cyberia-chrome">
                <div class="chrome-inner">
                    <div class="header-row1">
                        <div class="logo-zone">
                            <h1 class="logo">
                                <a href=format!("{BASE}/") class="brand-flag" title="home" inner_html=FLAG_SVG></a>
                                <span style="color: var(--cyber-green);">"cyber"</span>
                                <span style="color: var(--cyber-green); margin: 0 1px;">"•"</span>
                                <span style="color: #fff;">"ia"</span>
                            </h1>
                        </div>
                        <CyberiaNav active="my" />
                    </div>
                </div>
            </div>
            <div class="pd mp">
                <div class="pd-stage">
                    <div class="pd-head">
                        <div class="pd-title">
                            <div class="pd-kicker" style="color: var(--cyber-green);">"YOUR COLLECTION"</div>
                            <h2>"MY PLOTS"</h2>
                        </div>
                        <div class="mp-stats">{stats}</div>
                        <div class="pd-tabs" role="tablist">
                            {[None, Some(Shelf::Held), Some(Shelf::Deal), Some(Shelf::Watch)].into_iter().map(|s| view! {
                                <button type="button" role="tab"
                                    class=move || if shelf.get() == s { "pd-tab on" } else { "pd-tab seen" }
                                    style:--ink=s.map(|s| s.color()).unwrap_or("#ffffff")
                                    on:click=move |_| { shelf.set(s); picked.set(None); }>
                                    <b>{s.map(|s| s.label()).unwrap_or("ALL")}</b>
                                    <span>{move || format!("{} flats", count(s))}</span>
                                </button>
                            }).collect_view()}
                        </div>
                    </div>
                    <div class="pd-body mp-body">
                        {stage}
                        <div class="mp-deck">
                            {move || shown.get().into_iter().map(|m| {
                                let id = m.flat.id.clone();
                                let id_on = id.clone();
                                let f = Frame::new(&m.flat.coords);
                                let pr = (m.shelf == Shelf::Deal).then(|| progress(&id, index.get().get(&id)));
                                view! {
                                    <button type="button"
                                        class=move || if hand.get().map(|h| h.flat.id == id_on).unwrap_or(false) { "mp-card on" } else { "mp-card" }
                                        style:--c=m.shelf.color() style:--ink=land_use(&id).color()
                                        on:click=move |_| picked.set(Some(id.clone()))>
                                        <svg viewBox=format!("0 0 {:.0} {:.0}", f.svg_w(), f.svg_h())><path d=f.outline_path() vector-effect="non-scaling-stroke" /></svg>
                                        <b>{bare_name(&m.flat.name)}</b>
                                        <span>{match pr { Some(p) => format!("{} · {} of {}", m.shelf.label(), p.done, p.marks.len()), None => m.shelf.label().to_string() }}</span>
                                    </button>
                                }
                            }).collect_view()}
                        </div>
                    </div>
                </div>
                <aside class="pd-rail mp-rail">
                    {card}
                    {move || view.get().is_none().then(|| view! {
                        <div class="mp-wallet">
                            <span>"flats the valley bound to your wallet show here once it is connected"</span>
                            <crate::wallet_ui::WalletConnect on_connected=Callback::new(move |w| crate::sell::enter(view, busy, err, w)) />
                            {move || { let e = err.get(); (!e.is_empty()).then(|| view! { <span class="wc-note">{e}</span> }) }}
                        </div>
                    })}
                </aside>
            </div>
        </div>
    }
}
