//! Events catalog — multi-event surface for cyberia.my
//! Seeded Cyber Valley events + Add Event (localStorage).

use leptos::prelude::*;
use serde::{Deserialize, Serialize};

const FLAG_SVG: &str = include_str!("../assets/cyberia-flag.svg");
const EVENTS_KEY: &str = "cyberia_events";

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct EventCard {
    pub id: String,
    pub title: String,
    pub city: String,
    pub when: String,
    pub where_: String,
    pub blurb: String,
    pub status: String, // live | upcoming | draft
}

fn seed_events() -> Vec<EventCard> {
    vec![
        EventCard {
            id: "cv-land-ops".into(),
            title: "Land Ops · Phase 0".into(),
            city: "Cyber Valley".into(),
            when: "ongoing".into(),
            where_: "Gesing · plots + fleets".into(),
            blurb: "Daily land work — survey, clear, plant. Open the console.".into(),
            status: "live".into(),
        },
        EventCard {
            id: "cv-soft-circle".into(),
            title: "Soft circle".into(),
            city: "Cyber Valley".into(),
            when: "weekly".into(),
            where_: "Soft · event space".into(),
            blurb: "Community gather — conferences, coworking, parties.".into(),
            status: "upcoming".into(),
        },
    ]
}

fn load_user_events() -> Vec<EventCard> {
    web_sys::window()
        .and_then(|w| w.local_storage().ok().flatten())
        .and_then(|ls| ls.get_item(EVENTS_KEY).ok().flatten())
        .and_then(|raw| serde_json::from_str(&raw).ok())
        .unwrap_or_default()
}

fn save_user_events(list: &[EventCard]) {
    if let Some(ls) = web_sys::window().and_then(|w| w.local_storage().ok().flatten()) {
        if let Ok(raw) = serde_json::to_string(list) {
            let _ = ls.set_item(EVENTS_KEY, &raw);
        }
    }
}

fn slugify(name: &str) -> String {
    let s: String = name
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect();
    let mut out = String::new();
    for ch in s.chars() {
        if ch == '-' {
            if !out.ends_with('-') && !out.is_empty() {
                out.push(ch);
            }
        } else {
            out.push(ch);
        }
    }
    out.trim_matches('-').to_string()
}

#[component]
pub fn EventsPage() -> impl IntoView {
    let user_events = RwSignal::new(load_user_events());
    let sheet_open = RwSignal::new(false);
    let draft_title = RwSignal::new(String::new());
    let draft_city = RwSignal::new("Cyber Valley".to_string());
    let draft_when = RwSignal::new(String::new());
    let draft_where = RwSignal::new(String::new());
    let draft_blurb = RwSignal::new(String::new());
    let err = RwSignal::new(None::<String>);

    Effect::new(move |_| {
        document().set_title("Cyberia — events");
    });

    let catalog = move || {
        let mut list = seed_events();
        list.extend(user_events.get());
        list
    };

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
                            {move || format!("{} EVENTS", catalog().len())}
                        </div>
                        <div class="map-zone">
                            <a class="nav-btn" href="/cities">"CITIES"</a>
                            <a class="nav-btn nav-here" href="/events">"EVENTS"</a>
                            <a class="nav-btn" href="/city/cyber-valley">"CONSOLE"</a>
                            <a class="nav-btn" href="https://cyberstates.net" target="_blank" rel="noopener">"STATES"</a>
                        </div>
                    </div>
                </div>
            </div>

            <div class="cities-stage">
                <div class="cities-hero">
                    <div>
                        <div class="cities-kicker">"CATALOG"</div>
                        <h2 class="cities-title">"Events"</h2>
                        <p class="cities-lead">
                            "Gatherings across Cyberia cities — land ops, soft circles, and what you add."
                        </p>
                    </div>
                    <button class="cta-btn cta-event cta-lg cta-bold" on:click=move |_| {
                        err.set(None);
                        sheet_open.set(true);
                    }>
                        <span class="cta-ico">"✚"</span>
                        <span class="cta-copy">
                            <span class="cta-title">"ADD EVENT"</span>
                            <span class="cta-sub">"post to the catalog"</span>
                        </span>
                    </button>
                </div>

                <div class="cities-grid">
                    {move || catalog().into_iter().enumerate().map(|(i, e)| {
                        let live = e.status == "live";
                        let upcoming = e.status == "upcoming";
                        let st_cls = if live {
                            "city-status live"
                        } else if upcoming {
                            "city-status upcoming"
                        } else {
                            "city-status founding"
                        };
                        let title = e.title.clone();
                        let city = e.city.clone();
                        let when = e.when.clone();
                        let where_ = e.where_.clone();
                        let blurb = e.blurb.clone();
                        let status = e.status.to_uppercase();
                        let rank = i + 1;
                        let href = if e.city == "Cyber Valley" {
                            "/city/cyber-valley".to_string()
                        } else {
                            "/events".to_string()
                        };
                        view! {
                            <a class=if live { "city-card live event-card" } else { "city-card founding event-card" } href=href>
                                <div class="city-card-top">
                                    <span class="city-rank">{format!("#{rank:02}")}</span>
                                    <span class=st_cls>{status}</span>
                                </div>
                                <div class="city-name">{title}</div>
                                <div class="city-region">{format!("{city} · {when}")}</div>
                                <p class="city-blurb">{blurb}</p>
                                <div class="city-meta">
                                    <span>{where_}</span>
                                    <span class="city-open">
                                        {if live { "OPEN CONSOLE →" } else { "LISTED →" }}
                                    </span>
                                </div>
                            </a>
                        }
                    }).collect_view()}

                    <button class="city-card ghost event-card" on:click=move |_| {
                        err.set(None);
                        sheet_open.set(true);
                    }>
                        <div class="city-card-top">
                            <span class="city-rank">"+"</span>
                            <span class="city-status founding">"NEW"</span>
                        </div>
                        <div class="city-name">"Add an event"</div>
                        <div class="city-region">"any cyberia city"</div>
                        <p class="city-blurb">
                            "Name it, place it, time it. Catalog stays local until the backend ships."
                        </p>
                        <div class="city-meta">
                            <span>"when · where"</span>
                            <span class="city-open">"ADD EVENT →"</span>
                        </div>
                    </button>
                </div>
            </div>

            {move || sheet_open.get().then(|| view! {
                <div class="cyberia-sheet-backdrop" on:click=move |_| sheet_open.set(false)>
                    <div class="cyberia-sheet" on:click=move |ev| ev.stop_propagation()>
                        <div class="sheet-h">
                            <span class="panel-kicker">"ADD EVENT"</span>
                            <button class="sheet-x" on:click=move |_| sheet_open.set(false)>"✕"</button>
                        </div>
                        <p class="sheet-note">
                            "Soft3 local catalog — no closed backend. Event is stored in this browser."
                        </p>
                        <label class="found-label">"TITLE"</label>
                        <input
                            class="found-input"
                            type="text"
                            placeholder="e.g. Full moon trail"
                            prop:value=move || draft_title.get()
                            on:input=move |ev| draft_title.set(event_target_value(&ev))
                        />
                        <label class="found-label">"CITY"</label>
                        <input
                            class="found-input"
                            type="text"
                            placeholder="Cyber Valley"
                            prop:value=move || draft_city.get()
                            on:input=move |ev| draft_city.set(event_target_value(&ev))
                        />
                        <label class="found-label">"WHEN"</label>
                        <input
                            class="found-input"
                            type="text"
                            placeholder="e.g. 2026-08-20 · sunset"
                            prop:value=move || draft_when.get()
                            on:input=move |ev| draft_when.set(event_target_value(&ev))
                        />
                        <label class="found-label">"WHERE"</label>
                        <input
                            class="found-input"
                            type="text"
                            placeholder="e.g. Soft · edge of Sinwood"
                            prop:value=move || draft_where.get()
                            on:input=move |ev| draft_where.set(event_target_value(&ev))
                        />
                        <label class="found-label">"BLURB"</label>
                        <input
                            class="found-input"
                            type="text"
                            placeholder="one line about the event"
                            prop:value=move || draft_blurb.get()
                            on:input=move |ev| draft_blurb.set(event_target_value(&ev))
                        />
                        {move || err.get().map(|e| view! { <div class="found-err">{e}</div> })}
                        <button
                            class="intent-commit event"
                            on:click=move |_| {
                                let title = draft_title.get().trim().to_string();
                                if title.len() < 2 {
                                    err.set(Some("title too short".into()));
                                    return;
                                }
                                let mut id = slugify(&title);
                                if id.is_empty() {
                                    err.set(Some("invalid title".into()));
                                    return;
                                }
                                let mut list = user_events.get_untracked();
                                if list.iter().any(|e| e.id == id) || seed_events().iter().any(|e| e.id == id) {
                                    id = format!("{id}-{}", list.len() + 2);
                                }
                                let city = {
                                    let c = draft_city.get().trim().to_string();
                                    if c.is_empty() { "Cyber Valley".into() } else { c }
                                };
                                let when = {
                                    let w = draft_when.get().trim().to_string();
                                    if w.is_empty() { "TBD".into() } else { w }
                                };
                                let where_ = {
                                    let w = draft_where.get().trim().to_string();
                                    if w.is_empty() { "TBD".into() } else { w }
                                };
                                let blurb = {
                                    let b = draft_blurb.get().trim().to_string();
                                    if b.is_empty() {
                                        "User-listed event — local catalog.".into()
                                    } else {
                                        b
                                    }
                                };
                                list.insert(0, EventCard {
                                    id,
                                    title,
                                    city,
                                    when,
                                    where_,
                                    blurb,
                                    status: "draft".into(),
                                });
                                save_user_events(&list);
                                user_events.set(list);
                                draft_title.set(String::new());
                                draft_when.set(String::new());
                                draft_where.set(String::new());
                                draft_blurb.set(String::new());
                                err.set(None);
                                sheet_open.set(false);
                            }
                        >
                            "CONFIRM ADD"
                        </button>
                    </div>
                </div>
            })}

            <div class="search-dock cyberia-dock cities-dock">
                <span class="dock-count">
                    {move || {
                        let n = catalog().len();
                        let u = user_events.get().len();
                        format!("{n} events · {u} added here")
                    }}
                </span>
                <button class="cta-btn cta-event cta-lg cta-bold dock-found" on:click=move |_| {
                    err.set(None);
                    sheet_open.set(true);
                }>
                    <span class="cta-ico">"✚"</span>
                    <span class="cta-copy">
                        <span class="cta-title">"ADD EVENT"</span>
                        <span class="cta-sub">"post to catalog"</span>
                    </span>
                </button>
                <a href="https://x.com/cyberiacap" target="_blank" rel="noopener" class="dock-credit">
                    "🏴 a "<span style="color: var(--cyber-green);">"cyberia"</span>" project"
                </a>
            </div>
        </div>
    }
}
