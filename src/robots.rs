//! Robots catalog — dedicated fleets surface for cyberia.my
//! Hard-force workers + machines roster, market buy (localStorage).

use crate::land::FLAG_SVG;
use crate::nav::CyberiaNav;
use crate::wallet::{debit_cx, load_intents, save_intents, IntentRec};
use leptos::prelude::*;
use serde::{Deserialize, Serialize};

pub const OWNED_KEY: &str = "cyberia_owned_robots";

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct OwnedRobot {
    pub id: String,
    pub name: String,
    pub kind: String,
    pub role: String,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct RosterUnit {
    id: &'static str,
    name: &'static str,
    kind: &'static str, // worker | machine
    role: &'static str,
    status: &'static str, // idle | busy | offline | live
    crew: &'static str,
}

/// Gesing hard-force workers — same roster as console.
const WORKERS: &[RosterUnit] = &[
    RosterUnit {
        id: "w-sutar",
        name: "SUTAR",
        kind: "worker",
        role: "repair lead · energy/water",
        status: "idle",
        crew: "repair",
    },
    RosterUnit {
        id: "w-witaya",
        name: "WITAYA",
        kind: "worker",
        role: "repair · electronics",
        status: "idle",
        crew: "repair",
    },
    RosterUnit {
        id: "w-lupus",
        name: "LUPUS",
        kind: "worker",
        role: "repair · mechanical",
        status: "idle",
        crew: "repair",
    },
    RosterUnit {
        id: "w-sudi",
        name: "SUDI",
        kind: "worker",
        role: "cube lead · build",
        status: "idle",
        crew: "cube",
    },
    RosterUnit {
        id: "w-budi",
        name: "BUDI",
        kind: "worker",
        role: "cube · build",
        status: "idle",
        crew: "cube",
    },
    RosterUnit {
        id: "w-tika",
        name: "TIKA",
        kind: "worker",
        role: "cube · stove",
        status: "idle",
        crew: "cube",
    },
    RosterUnit {
        id: "w-sastra",
        name: "SASTRA",
        kind: "worker",
        role: "cube · build",
        status: "idle",
        crew: "cube",
    },
    RosterUnit {
        id: "w-angga",
        name: "ANGGA",
        kind: "worker",
        role: "base lead · road/trail",
        status: "idle",
        crew: "base",
    },
    RosterUnit {
        id: "w-darma",
        name: "DARMA",
        kind: "worker",
        role: "base · mason",
        status: "idle",
        crew: "base",
    },
    RosterUnit {
        id: "w-darsana",
        name: "DARSANA",
        kind: "worker",
        role: "base · terrace",
        status: "idle",
        crew: "base",
    },
    RosterUnit {
        id: "w-arima",
        name: "ARIMA",
        kind: "worker",
        role: "pruning lead · land",
        status: "idle",
        crew: "pruning",
    },
    RosterUnit {
        id: "w-doplang",
        name: "DOPLANG",
        kind: "worker",
        role: "pruning · firewood",
        status: "idle",
        crew: "pruning",
    },
    RosterUnit {
        id: "w-surya",
        name: "SURYA",
        kind: "worker",
        role: "pruning · fodder",
        status: "idle",
        crew: "pruning",
    },
    RosterUnit {
        id: "w-suardita",
        name: "SUARDITA",
        kind: "worker",
        role: "pruning · compost",
        status: "idle",
        crew: "pruning",
    },
    RosterUnit {
        id: "w-pande",
        name: "PANDE",
        kind: "worker",
        role: "delivery lead · haul",
        status: "idle",
        crew: "delivery",
    },
];

const MACHINES: &[RosterUnit] = &[
    RosterUnit {
        id: "f-eye",
        name: "EYE-01",
        kind: "machine",
        role: "survey drone",
        status: "idle",
        crew: "survey",
    },
    RosterUnit {
        id: "f-haul",
        name: "HAUL-01",
        kind: "machine",
        role: "ground rover",
        status: "idle",
        crew: "haul",
    },
    RosterUnit {
        id: "f-cut",
        name: "CUT-01",
        kind: "machine",
        role: "clearing arm",
        status: "offline",
        crew: "clear",
    },
    RosterUnit {
        id: "f-build",
        name: "CUBE-01",
        kind: "machine",
        role: "build stack",
        status: "offline",
        crew: "build",
    },
];

#[derive(Clone, Copy, Debug, PartialEq)]
struct MarketUnit {
    id: &'static str,
    name: &'static str,
    kind: &'static str,
    role: &'static str,
    blurb: &'static str,
}

const MARKET: &[MarketUnit] = &[
    MarketUnit {
        id: "cat-eye",
        name: "EYE",
        kind: "machine",
        role: "survey drone",
        blurb: "Aerial survey — map flats, track growth.",
    },
    MarketUnit {
        id: "cat-hand",
        name: "HAND",
        kind: "worker",
        role: "field hand",
        blurb: "General land work — plant, clear, carry.",
    },
    MarketUnit {
        id: "cat-haul",
        name: "HAUL",
        kind: "machine",
        role: "ground rover",
        blurb: "Heavy haul across trails and flats.",
    },
    MarketUnit {
        id: "cat-cut",
        name: "CUT",
        kind: "machine",
        role: "clearing arm",
        blurb: "Brush and edge clear for new holds.",
    },
];

pub fn load_owned() -> Vec<OwnedRobot> {
    web_sys::window()
        .and_then(|w| w.local_storage().ok().flatten())
        .and_then(|ls| ls.get_item(OWNED_KEY).ok().flatten())
        .and_then(|raw| serde_json::from_str(&raw).ok())
        .unwrap_or_default()
}

pub fn save_owned(list: &[OwnedRobot]) {
    if let Some(ls) = web_sys::window().and_then(|w| w.local_storage().ok().flatten()) {
        if let Ok(raw) = serde_json::to_string(list) {
            let _ = ls.set_item(OWNED_KEY, &raw);
        }
    }
}

fn status_cls(status: &str) -> &'static str {
    match status {
        "idle" => "city-status live",
        "busy" => "city-status upcoming",
        "offline" => "city-status founding",
        _ => "city-status founding",
    }
}

#[component]
pub fn RobotsPage() -> impl IntoView {
    let owned = RwSignal::new(load_owned());
    let sheet_open = RwSignal::new(false);
    let buy_pick = RwSignal::new("cat-eye".to_string());
    let robot_serial = RwSignal::new({
        let max = load_owned()
            .iter()
            .filter_map(|r| {
                r.name
                    .rsplit('-')
                    .next()
                    .and_then(|s| s.parse::<u32>().ok())
            })
            .max()
            .unwrap_or(0);
        max + 1
    });

    Effect::new(move |_| {
        document().set_title("Cyberia — robots");
    });

    let total = move || WORKERS.len() + MACHINES.len() + owned.get().len();

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
                            {move || format!(
                                "{} UNITS · {} YOURS",
                                total(),
                                owned.get().len()
                            )}
                        </div>
                        <CyberiaNav active="robots" />
                    </div>
                </div>
            </div>

            <div class="cities-stage">
                <div class="cities-hero">
                    <div>
                        <div class="cities-kicker">"FLEETS"</div>
                        <h2 class="cities-title">"Robots"</h2>
                        <p class="cities-lead">
                            "Hard-force workers and machines on Cyber Valley land. Buy a unit, then assign it on the map."
                        </p>
                    </div>
                </div>

                <crate::portal::PortalNav active="fleet" />

                // ── YOUR FLEET ──
                {move || {
                    let list = owned.get();
                    if list.is_empty() {
                        return view! { <div></div> }.into_any();
                    }
                    view! {
                        <div class="robots-section">
                            <div class="robots-section-h">
                                <span class="fleet-section">"YOUR FLEET"</span>
                                <span class="robots-section-n">{format!("{} owned", list.len())}</span>
                            </div>
                            <div class="cities-grid robots-grid">
                                {list.into_iter().enumerate().map(|(i, r)| {
                                    let kind = r.kind.clone();
                                    let is_worker = kind == "worker";
                                    let name = r.name.clone();
                                    let role = r.role.clone();
                                    let rank = i + 1;
                                    view! {
                                        <a class="city-card live robot-card" href="/map">
                                            <div class="city-card-top">
                                                <span class="city-rank">{format!("#{rank:02}")}</span>
                                                <span class="city-status live">"YOURS"</span>
                                            </div>
                                            <div class=if is_worker { "city-name robot-name worker" } else { "city-name robot-name machine" }>
                                                {name}
                                            </div>
                                            <div class="city-region">{role}</div>
                                            <p class="city-blurb">"Purchased — open map to assign to a flat."</p>
                                            <div class="city-meta">
                                                <span>{kind.to_uppercase()}</span>
                                                <span class="city-open">"ASSIGN →"</span>
                                            </div>
                                        </a>
                                    }
                                }).collect_view()}
                            </div>
                        </div>
                    }.into_any()
                }}

                // ── HARD FORCE ──
                <div class="robots-section">
                    <div class="robots-section-h">
                        <span class="fleet-section">"WORKERS · HARD FORCE"</span>
                        <span class="robots-section-n">{format!("{} on site", WORKERS.len())}</span>
                    </div>
                    <div class="cities-grid robots-grid">
                        {WORKERS.iter().enumerate().map(|(i, w)| {
                            let rank = i + 1;
                            let st = w.status.to_uppercase();
                            let offline = w.status == "offline";
                            view! {
                                <a
                                    class=if offline { "city-card founding robot-card" } else { "city-card live robot-card" }
                                    href="/map"
                                >
                                    <div class="city-card-top">
                                        <span class="city-rank">{format!("#{rank:02}")}</span>
                                        <span class=status_cls(w.status)>{st}</span>
                                    </div>
                                    <div class="city-name robot-name worker">{w.name}</div>
                                    <div class="city-region">{format!("{} · {}", w.crew.to_uppercase(), w.role)}</div>
                                    <p class="city-blurb">"Gesing hard force — assign land work on the map."</p>
                                    <div class="city-meta">
                                        <span>"WORKER"</span>
                                        <span>{w.crew.to_uppercase()}</span>
                                        <span class="city-open">"MAP →"</span>
                                    </div>
                                </a>
                            }
                        }).collect_view()}
                    </div>
                </div>

                // ── MACHINES ──
                <div class="robots-section">
                    <div class="robots-section-h">
                        <span class="fleet-section">"MACHINES"</span>
                        <span class="robots-section-n">{format!("{} units", MACHINES.len())}</span>
                    </div>
                    <div class="cities-grid robots-grid">
                        {MACHINES.iter().enumerate().map(|(i, m)| {
                            let rank = i + 1;
                            let st = m.status.to_uppercase();
                            let offline = m.status == "offline";
                            view! {
                                <a
                                    class=if offline { "city-card founding robot-card" } else { "city-card live robot-card" }
                                    href="/map"
                                >
                                    <div class="city-card-top">
                                        <span class="city-rank">{format!("#{rank:02}")}</span>
                                        <span class=status_cls(m.status)>{st}</span>
                                    </div>
                                    <div class="city-name robot-name machine">{m.name}</div>
                                    <div class="city-region">{m.role}</div>
                                    <p class="city-blurb">
                                        {if offline {
                                            "Offline — phase gate or maintenance."
                                        } else {
                                            "Phase-0 hardware fleet on site."
                                        }}
                                    </p>
                                    <div class="city-meta">
                                        <span>"MACHINE"</span>
                                        <span>{m.crew.to_uppercase()}</span>
                                        <span class="city-open">"MAP →"</span>
                                    </div>
                                </a>
                            }
                        }).collect_view()}
                    </div>
                </div>

                // ── MARKET ──
                <div class="robots-section">
                    <div class="robots-section-h">
                        <span class="fleet-section">"MARKET · FOR SALE"</span>
                        <span class="robots-section-n">{format!("{} models", MARKET.len())}</span>
                    </div>
                    <div class="cities-grid robots-grid">
                        {MARKET.iter().map(|r| {
                            let id = r.id.to_string();
                            let is_worker = r.kind == "worker";
                            view! {
                                <button
                                    class="city-card ghost robot-card market-card"
                                    on:click=move |_| {
                                        buy_pick.set(id.clone());
                                        sheet_open.set(true);
                                    }
                                >
                                    <div class="city-card-top">
                                        <span class="city-rank">"BUY"</span>
                                        <span class="city-status founding">"FOR SALE"</span>
                                    </div>
                                    <div class=if is_worker { "city-name robot-name worker" } else { "city-name robot-name machine" }>
                                        {r.name}
                                    </div>
                                    <div class="city-region">{r.role}</div>
                                    <p class="city-blurb">{r.blurb}</p>
                                    <div class="city-meta">
                                        <span>{r.kind.to_uppercase()}</span>
                                        <span class="city-open">"BUY →"</span>
                                    </div>
                                </button>
                            }
                        }).collect_view()}
                    </div>
                </div>
            </div>

            // BUY ROBOT sheet
            {move || sheet_open.get().then(|| view! {
                <div class="cyberia-sheet-backdrop" on:click=move |_| sheet_open.set(false)>
                    <div class="cyberia-sheet" on:click=move |ev| ev.stop_propagation()>
                        <div class="sheet-h">
                            <span class="panel-kicker">"BUY ROBOT"</span>
                            <button class="sheet-x" on:click=move |_| sheet_open.set(false)>"✕"</button>
                        </div>
                        <p class="sheet-note">
                            "Phase 0 catalog — soft3 local intent. No payment rail yet; purchase adds the unit to your fleet (this browser) and is available on the map."
                        </p>
                        <div class="sheet-catalog">
                            {MARKET.iter().map(|r| {
                                let id = r.id.to_string();
                                let id2 = id.clone();
                                let kind_worker = r.kind == "worker";
                                view! {
                                    <button
                                        class=move || {
                                            let sel = buy_pick.get() == id;
                                            format!(
                                                "fleet-card catalog{}{}",
                                                if sel { " sel" } else { "" },
                                                if kind_worker { " worker" } else { " machine" },
                                            )
                                        }
                                        on:click=move |_| buy_pick.set(id2.clone())
                                    >
                                        <div class="fleet-top">
                                            <span class="fleet-name">{r.name}</span>
                                            <span class="fleet-status" style:color="var(--cyber-yellow)">"FOR SALE"</span>
                                        </div>
                                        <div class="fleet-role">{r.role}</div>
                                        <div class="fleet-meta">
                                            <span>{r.kind.to_uppercase()}</span>
                                            <span>"P0"</span>
                                        </div>
                                    </button>
                                }
                            }).collect_view()}
                        </div>
                        <button
                            class="intent-commit buy"
                            on:click=move |_| {
                                let pick = buy_pick.get();
                                let Some(cat) = MARKET.iter().find(|r| r.id == pick) else { return };
                                let n = robot_serial.get();
                                robot_serial.set(n + 1);
                                let unit_id = format!("own-{}-{}", cat.name.to_lowercase(), n);
                                let unit_name = format!("{}-{:02}", cat.name, n);
                                owned.update(|v| {
                                    v.insert(
                                        0,
                                        OwnedRobot {
                                            id: unit_id,
                                            name: unit_name.clone(),
                                            kind: cat.kind.to_string(),
                                            role: cat.role.to_string(),
                                        },
                                    );
                                    save_owned(v);
                                });
                                let mut q = load_intents();
                                let id = q.iter().map(|i| i.id).max().unwrap_or(0) + 1;
                                q.insert(
                                    0,
                                    IntentRec {
                                        id,
                                        fleet: unit_name,
                                        action: "buy".into(),
                                        flat: "market".into(),
                                    },
                                );
                                save_intents(&q);
                                debit_cx(15.0);
                                sheet_open.set(false);
                            }
                        >
                            "CONFIRM BUY"
                        </button>
                    </div>
                </div>
            })}

            <div class="search-dock cyberia-dock cities-dock">
                <span class="dock-count">
                    {move || format!(
                        "{} workers · {} machines · {} yours",
                        WORKERS.len(),
                        MACHINES.len(),
                        owned.get().len()
                    )}
                </span>
                <button class="cta-btn cta-buy cta-lg cta-bold dock-found" on:click=move |_| {
                    sheet_open.set(true);
                }>
                    <span class="cta-ico">"🤖"</span>
                    <span class="cta-copy">
                        <span class="cta-title">"BUY ROBOT"</span>
                        <span class="cta-sub">"add to your fleet"</span>
                    </span>
                </button>
                <a class="cta-btn cta-lease cta-lg dock-found" href="/map" style="text-decoration:none; max-width: 220px;">
                    <span class="cta-copy">
                        <span class="cta-title">"MAP"</span>
                        <span class="cta-sub">"assign on land"</span>
                    </span>
                </a>
                <a href="https://x.com/cyberiacap" target="_blank" rel="noopener" class="dock-credit">
                    "🏴 a "<span style="color: var(--cyber-green);">"cyberia"</span>" project"
                </a>
            </div>
        </div>
    }
}
