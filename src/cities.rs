//! Cities catalog — multi-city surface for cyberia.my
//! Cyber Valley is the first live entry; Found City seeds local catalog.

use crate::land::FLAG_SVG;
use crate::nav::CyberiaNav;
use leptos::prelude::*;
use serde::{Deserialize, Serialize};

const FOUND_KEY: &str = "cyberia_found_cities";

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CityCard {
    pub id: String,
    pub name: String,
    pub region: String,
    pub blurb: String,
    pub ha: f64,
    pub plots: u32,
    pub status: String, // live | founding
    pub href: String,
}

/// Built-in cities — Cyber Valley always #1.
fn seed_cities() -> Vec<CityCard> {
    vec![CityCard {
        id: "cyber-valley".into(),
        name: "Cyber Valley".into(),
        region: "Gesing · Bali · Indonesia".into(),
        blurb: "Phase 0 land map — 126 plots, hard-force fleets, intents.".into(),
        ha: 37.0,
        plots: 126,
        status: "live".into(),
        href: "/map".into(),
    }]
}

pub fn load_found() -> Vec<CityCard> {
    web_sys::window()
        .and_then(|w| w.local_storage().ok().flatten())
        .and_then(|ls| ls.get_item(FOUND_KEY).ok().flatten())
        .and_then(|raw| serde_json::from_str(&raw).ok())
        .unwrap_or_default()
}

fn save_found(list: &[CityCard]) {
    if let Some(ls) = web_sys::window().and_then(|w| w.local_storage().ok().flatten()) {
        if let Ok(raw) = serde_json::to_string(list) {
            let _ = ls.set_item(FOUND_KEY, &raw);
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
pub fn CitiesPage() -> impl IntoView {
    let found = RwSignal::new(load_found());
    let sheet_open = RwSignal::new(false);
    let draft_name = RwSignal::new(String::new());
    let draft_region = RwSignal::new(String::new());
    let draft_blurb = RwSignal::new(String::new());
    let err = RwSignal::new(None::<String>);

    Effect::new(move |_| {
        document().set_title("Cyberia — cities");
    });

    let catalog = move || {
        let mut list = seed_cities();
        list.extend(found.get());
        list
    };

    view! {
        <div class="page-shell cities-shell">
            <div class="site-chrome cyberia-chrome">
                <div class="chrome-inner">
                    <div class="header-row1">
                        <div class="logo-zone">
                            <h1 class="logo">
                                <a href="/cities" class="brand-flag" title="cities" inner_html=FLAG_SVG></a>
                                <span style="color: var(--cyber-green);">"cyber"</span>
                                <span style="color: var(--cyber-green); margin: 0 1px;">"•"</span>
                                <span style="color: #fff;">"ia"</span>
                            </h1>
                        </div>
                        <div class="cyberia-phase-pill">
                            <span class="phase-dot"></span>
                            {move || format!("{} CITIES", catalog().len())}
                        </div>
                        <CyberiaNav active="cities" />
                    </div>
                </div>
            </div>

            <div class="cities-stage">
                <div class="cities-hero">
                    <div>
                        <div class="cities-kicker">"CATALOG"</div>
                        <h2 class="cities-title">"Cities of Cyberia"</h2>
                        <p class="cities-lead">
                            "Land-first network cities. Open a live map, or found a new one."
                        </p>
                    </div>
                    <a class="cta-btn cta-lease cta-lg" href="/world" style="text-decoration:none;">
                        <span class="cta-copy">
                            <span class="cta-title">"ERP STUDIO"</span>
                            <span class="cta-sub">"create cards · coins · build"</span>
                        </span>
                    </a>
                </div>

                <div class="cities-grid">
                    {move || catalog().into_iter().enumerate().map(|(i, c)| {
                        let live = c.status == "live";
                        let href = c.href.clone();
                        let name = c.name.clone();
                        let region = c.region.clone();
                        let blurb = c.blurb.clone();
                        let ha = c.ha;
                        let plots = c.plots;
                        let status = c.status.to_uppercase();
                        let rank = i + 1;
                        view! {
                            <a class=if live { "city-card live" } else { "city-card founding" } href=href>
                                <div class="city-card-top">
                                    <span class="city-rank">{format!("#{rank:02}")}</span>
                                    <span class=if live { "city-status live" } else { "city-status founding" }>
                                        {status}
                                    </span>
                                </div>
                                <div class="city-name">{name}</div>
                                <div class="city-region">{region}</div>
                                <p class="city-blurb">{blurb}</p>
                                <div class="city-meta">
                                    <span>{if ha > 0.0 { format!("{ha:.0} ha") } else { "— ha".into() }}</span>
                                    <span>{if plots > 0 { format!("{plots} plots") } else { "plots TBD".into() }}</span>
                                    <span class="city-open">{if live { "OPEN MAP →" } else { "FOUNDING →" }}</span>
                                </div>
                            </a>
                        }
                    }).collect_view()}

                    // ghost slot — invites founding
                    <button class="city-card ghost" on:click=move |_| {
                        err.set(None);
                        sheet_open.set(true);
                    }>
                        <div class="city-card-top">
                            <span class="city-rank">"+"</span>
                            <span class="city-status founding">"EMPTY"</span>
                        </div>
                        <div class="city-name">"Found a city"</div>
                        <div class="city-region">"next cyberia node"</div>
                        <p class="city-blurb">
                            "Claim a region, seed the catalog. Map + fleets land later."
                        </p>
                        <div class="city-meta">
                            <span>"— ha"</span>
                            <span>"plots TBD"</span>
                            <span class="city-open">"FOUND CITY →"</span>
                        </div>
                    </button>
                </div>
            </div>

            // Found City sheet
            {move || sheet_open.get().then(|| view! {
                <div class="cyberia-sheet-backdrop" on:click=move |_| sheet_open.set(false)>
                    <div class="cyberia-sheet" on:click=move |ev| ev.stop_propagation()>
                        <div class="sheet-h">
                            <span class="panel-kicker">"FOUND CITY"</span>
                            <button class="sheet-x" on:click=move |_| sheet_open.set(false)>"✕"</button>
                        </div>
                        <p class="sheet-note">
                            "Soft3 local catalog — no closed backend yet. City appears in your browser; land map ships later."
                        </p>
                        <label class="found-label">"NAME"</label>
                        <input
                            class="found-input"
                            type="text"
                            placeholder="e.g. Night Harbor"
                            prop:value=move || draft_name.get()
                            on:input=move |ev| {
                                let v = event_target_value(&ev);
                                draft_name.set(v);
                            }
                        />
                        <label class="found-label">"REGION"</label>
                        <input
                            class="found-input"
                            type="text"
                            placeholder="e.g. Yucatán · Mexico"
                            prop:value=move || draft_region.get()
                            on:input=move |ev| {
                                draft_region.set(event_target_value(&ev));
                            }
                        />
                        <label class="found-label">"BLURB"</label>
                        <input
                            class="found-input"
                            type="text"
                            placeholder="one line about the city"
                            prop:value=move || draft_blurb.get()
                            on:input=move |ev| {
                                draft_blurb.set(event_target_value(&ev));
                            }
                        />
                        {move || err.get().map(|e| view! {
                            <div class="found-err">{e}</div>
                        })}
                        <button
                            class="intent-commit found"
                            on:click=move |_| {
                                let name = draft_name.get().trim().to_string();
                                if name.len() < 2 {
                                    err.set(Some("name too short".into()));
                                    return;
                                }
                                let mut id = slugify(&name);
                                if id.is_empty() {
                                    err.set(Some("invalid name".into()));
                                    return;
                                }
                                if id == "cyber-valley" {
                                    err.set(Some("Cyber Valley already exists".into()));
                                    return;
                                }
                                // unique among found
                                let mut list = found.get_untracked();
                                if list.iter().any(|c| c.id == id) || seed_cities().iter().any(|c| c.id == id) {
                                    id = format!("{id}-{}", list.len() + 2);
                                }
                                let region = {
                                    let r = draft_region.get().trim().to_string();
                                    if r.is_empty() { "unspecified region".into() } else { r }
                                };
                                let blurb = {
                                    let b = draft_blurb.get().trim().to_string();
                                    if b.is_empty() {
                                        "Founding city — catalog entry only for now.".into()
                                    } else {
                                        b
                                    }
                                };
                                let card = CityCard {
                                    id: id.clone(),
                                    name,
                                    region,
                                    blurb,
                                    ha: 0.0,
                                    plots: 0,
                                    status: "founding".into(),
                                    href: format!("/cities#{id}"),
                                };
                                list.push(card);
                                save_found(&list);
                                found.set(list);
                                draft_name.set(String::new());
                                draft_region.set(String::new());
                                draft_blurb.set(String::new());
                                err.set(None);
                                sheet_open.set(false);
                            }
                        >
                            "CONFIRM FOUND"
                        </button>
                    </div>
                </div>
            })}

            <div class="search-dock cyberia-dock cities-dock">
                <span class="dock-count">
                    {move || {
                        let n = catalog().len();
                        let f = found.get().len();
                        format!("{n} cities · {f} founded here · 1 live map")
                    }}
                </span>
                <button class="cta-btn cta-found cta-lg cta-bold dock-found" on:click=move |_| {
                    err.set(None);
                    sheet_open.set(true);
                }>
                    <span class="cta-ico">"⚑"</span>
                    <span class="cta-copy">
                        <span class="cta-title">"FOUND CITY"</span>
                        <span class="cta-sub">"add to catalog"</span>
                    </span>
                </button>
                <a href="https://x.com/cyberiacap" target="_blank" rel="noopener" class="dock-credit">
                    "🏴 a "<span style="color: var(--cyber-green);">"cyberia"</span>" project"
                </a>
            </div>
        </div>
    }
}
