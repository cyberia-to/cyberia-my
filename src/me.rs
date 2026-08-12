//! Super dashboard — one screen for everything you hold in Cyberia.
//! Soft3 local: profile, CX/ETH balance, robots, leases, events, cities, intents.

use crate::cities::{load_found, CityCard};
use crate::economy::{fmt_qty, good};
use crate::events::{load_user_events, EventCard};
use crate::land::FLAG_SVG;
use crate::nav::CyberiaNav;
use crate::robots::{load_owned, OwnedRobot};
use crate::wallet::{
    ensure_economy_boot, load_balance, load_intents, load_leases, load_orders, load_profile,
    load_stocks, save_balance, save_profile, IntentRec, Lease, SoftBalance, StockLine,
};
use leptos::prelude::*;

fn fmt_cx(v: f64) -> String {
    if v >= 1000.0 {
        format!("{v:.0}")
    } else if v >= 10.0 {
        format!("{v:.1}")
    } else {
        format!("{v:.2}")
    }
}

fn fmt_eth(v: f64) -> String {
    if v >= 10.0 {
        format!("{v:.2}")
    } else {
        format!("{v:.3}")
    }
}

fn activity_score(
    robots: usize,
    leases: usize,
    events: usize,
    cities: usize,
    intents: usize,
    depth: f64,
) -> f64 {
    let raw = robots as f64 * 8.0
        + leases as f64 * 12.0
        + events as f64 * 6.0
        + cities as f64 * 15.0
        + intents as f64 * 2.0
        + depth * 0.4;
    raw.clamp(0.0, 99.0)
}

#[component]
pub fn MePage() -> impl IntoView {
    let profile = RwSignal::new(load_profile());
    let balance = RwSignal::new(load_balance());
    let robots = RwSignal::new(load_owned());
    let leases = RwSignal::new(load_leases());
    let events = RwSignal::new(load_user_events());
    let cities = RwSignal::new(load_found());
    let intents = RwSignal::new(load_intents());
    let stocks = RwSignal::new(load_stocks());
    let orders = RwSignal::new(load_orders());

    let edit_open = RwSignal::new(false);
    let draft_handle = RwSignal::new(String::new());
    let draft_name = RwSignal::new(String::new());

    // refresh from storage when landing (other tabs / map session)
    Effect::new(move |_| {
        document().set_title("Cyberia — you");
        ensure_economy_boot();
        profile.set(load_profile());
        balance.set(load_balance());
        robots.set(load_owned());
        leases.set(load_leases());
        events.set(load_user_events());
        cities.set(load_found());
        intents.set(load_intents());
        stocks.set(load_stocks());
        orders.set(load_orders());
    });

    let score = move || {
        activity_score(
            robots.get().len(),
            leases.get().len(),
            events.get().len(),
            cities.get().len(),
            intents.get().len(),
            balance.get().depth,
        )
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
                            {move || format!("@{} · YOU", profile.get().handle)}
                        </div>
                        <CyberiaNav active="you" />
                    </div>
                </div>
            </div>

            <div class="cities-stage me-stage">
                <div class="cities-hero">
                    <div>
                        <div class="cities-kicker">"SUPER DASHBOARD · THIS BROWSER"</div>
                        <h2 class="cities-title">
                            {move || {
                                let p = profile.get();
                                if p.name.is_empty() || p.name == "Anonymous" {
                                    format!("@{}", p.handle)
                                } else {
                                    p.name
                                }
                            }}
                        </h2>
                        <p class="cities-lead">
                            "Everything you hold in Cyberia — fleet, land, events, cities, intents, soft balance. Soft3 local until the backend ships."
                        </p>
                    </div>
                </div>

                // identity + balance strip
                <div class="me-id-row">
                    <div class="me-id-card">
                        <div class="me-avatar">{move || {
                            let h = profile.get().handle;
                            h.chars().next().unwrap_or('?').to_ascii_uppercase().to_string()
                        }}</div>
                        <div class="me-id-copy">
                            <div class="me-handle">{move || format!("@{}", profile.get().handle)}</div>
                            <div class="me-since">{move || format!("since · {}", profile.get().since)}</div>
                        </div>
                        <button class="chip" on:click=move |_| {
                            let p = profile.get();
                            draft_handle.set(p.handle);
                            draft_name.set(p.name);
                            edit_open.set(true);
                        }>"EDIT"</button>
                    </div>
                    <div class="me-bal-card">
                        <div class="kpi-lab">"CX BALANCE"</div>
                        <div class="kpi-val pos">{move || format!("{} CX", fmt_cx(balance.get().cx))}</div>
                        <div class="kpi-sub">"soft century-index units"</div>
                    </div>
                    <div class="me-bal-card">
                        <div class="kpi-lab">"ETH FLOAT"</div>
                        <div class="kpi-val">{move || format!("{} ETH", fmt_eth(balance.get().eth))}</div>
                        <div class="kpi-sub">"mock bank participation"</div>
                    </div>
                    <div class="me-bal-card">
                        <div class="kpi-lab">"ACTIVITY"</div>
                        <div class="kpi-val">{move || format!("{:.0}", score())}</div>
                        <div class="kpi-sub">"depth from ops · 0–99"</div>
                    </div>
                </div>

                // summary counts
                <div class="bank-kpi-grid me-counts">
                    <a class="kpi me-kpi" href="/robots">
                        <div class="kpi-lab">"ROBOTS"</div>
                        <div class="kpi-val">{move || robots.get().len().to_string()}</div>
                        <div class="kpi-sub">"owned fleet →"</div>
                    </a>
                    <a class="kpi me-kpi" href="/map">
                        <div class="kpi-lab">"LEASES"</div>
                        <div class="kpi-val">{move || leases.get().len().to_string()}</div>
                        <div class="kpi-sub">"plots you hold →"</div>
                    </a>
                    <a class="kpi me-kpi" href="/calendar">
                        <div class="kpi-lab">"EVENTS"</div>
                        <div class="kpi-val">{move || events.get().len().to_string()}</div>
                        <div class="kpi-sub">"you posted →"</div>
                    </a>
                    <a class="kpi me-kpi" href="/cities">
                        <div class="kpi-lab">"CITIES"</div>
                        <div class="kpi-val">{move || cities.get().len().to_string()}</div>
                        <div class="kpi-sub">"you founded →"</div>
                    </a>
                    <a class="kpi me-kpi" href="/map">
                        <div class="kpi-lab">"INTENTS"</div>
                        <div class="kpi-val">{move || intents.get().len().to_string()}</div>
                        <div class="kpi-sub">"ops queue →"</div>
                    </a>
                    <a class="kpi me-kpi" href="/products">
                        <div class="kpi-lab">"STOCKS"</div>
                        <div class="kpi-val">{move || stocks.get().len().to_string()}</div>
                        <div class="kpi-sub">"elements · products →"</div>
                    </a>
                    <a class="kpi me-kpi" href="/market">
                        <div class="kpi-lab">"LOTS"</div>
                        <div class="kpi-val">{move || {
                            let me = profile.get().handle;
                            orders.get().iter().filter(|o| o.owner == me).count().to_string()
                        }}</div>
                        <div class="kpi-sub">"your market lots →"</div>
                    </a>
                </div>

                // quick actions
                <div class="me-actions">
                    <a class="cta-btn cta-lease cta-lg" href="/products" style="text-decoration:none;">
                        <span class="cta-copy">
                            <span class="cta-title">"BOM"</span>
                            <span class="cta-sub">"transform stocks"</span>
                        </span>
                    </a>
                    <a class="cta-btn cta-buy cta-lg" href="/market" style="text-decoration:none;">
                        <span class="cta-copy">
                            <span class="cta-title">"MARKET"</span>
                            <span class="cta-sub">"buy · sell"</span>
                        </span>
                    </a>
                    <a class="cta-btn cta-event cta-lg" href="/robots" style="text-decoration:none;">
                        <span class="cta-copy">
                            <span class="cta-title">"ROBOTS"</span>
                            <span class="cta-sub">"fleet"</span>
                        </span>
                    </a>
                    <a class="cta-btn cta-found cta-lg" href="/world" style="text-decoration:none;">
                        <span class="cta-copy">
                            <span class="cta-title">"WORLD"</span>
                            <span class="cta-sub">"my presence"</span>
                        </span>
                    </a>
                </div>

                // ── STOCKS ──
                <section class="me-section">
                    <div class="me-section-h">
                        <span>"YOUR STOCKS · ELEMENTS & PRODUCTS"</span>
                        <a href="/products">"BOM →"</a>
                    </div>
                    {move || {
                        let list = stocks.get();
                        if list.is_empty() {
                            return view! {
                                <div class="me-empty">
                                    "Empty book. "
                                    <a href="/market">"Buy on market"</a>
                                    " or wait for starter pack boot."
                                </div>
                            }.into_any();
                        }
                        view! {
                            <div class="me-chip-grid">
                                {list.into_iter().map(|s: StockLine| {
                                    let name = good(&s.id).map(|g| g.name).unwrap_or("?");
                                    let unit = good(&s.id).map(|g| g.unit).unwrap_or("");
                                    view! {
                                        <a class="me-chip" href="/market">
                                            <span class="me-chip-name">{name}</span>
                                            <span class="me-chip-meta">{format!("{} {} · {}", fmt_qty(s.qty), unit, s.id)}</span>
                                        </a>
                                    }
                                }).collect_view()}
                            </div>
                        }.into_any()
                    }}
                </section>

                // ── FLEET ──
                <section class="me-section">
                    <div class="me-section-h">
                        <span>"YOUR FLEET"</span>
                        <a href="/robots">"all robots →"</a>
                    </div>
                    {move || {
                        let list = robots.get();
                        if list.is_empty() {
                            return view! {
                                <div class="me-empty">
                                    "No robots yet. "
                                    <a href="/robots">"Buy one"</a>
                                    " — soft3 local catalog."
                                </div>
                            }.into_any();
                        }
                        view! {
                            <div class="me-chip-grid">
                                {list.into_iter().map(|r: OwnedRobot| {
                                    let kind = r.kind.to_uppercase();
                                    let is_w = r.kind == "worker";
                                    view! {
                                        <a class="me-chip" href="/map">
                                            <span class=if is_w { "me-chip-name worker" } else { "me-chip-name machine" }>{r.name}</span>
                                            <span class="me-chip-meta">{format!("{kind} · {}", r.role)}</span>
                                        </a>
                                    }
                                }).collect_view()}
                            </div>
                        }.into_any()
                    }}
                </section>

                // ── LAND ──
                <section class="me-section">
                    <div class="me-section-h">
                        <span>"YOUR LAND"</span>
                        <a href="/map">"map →"</a>
                    </div>
                    {move || {
                        let list = leases.get();
                        if list.is_empty() {
                            return view! {
                                <div class="me-empty">
                                    "No leases. "
                                    <a href="/map">"Lease a plot"</a>
                                    " on the Gesing map."
                                </div>
                            }.into_any();
                        }
                        view! {
                            <div class="me-chip-grid">
                                {list.into_iter().map(|l: Lease| {
                                    let href = format!("/map?plot={}", l.flat_id);
                                    let zone = if l.zone.is_empty() { "phase 0".into() } else { l.zone };
                                    view! {
                                        <a class="me-chip land" href=href>
                                            <span class="me-chip-name land">{l.flat_name.to_uppercase()}</span>
                                            <span class="me-chip-meta">{format!("{} · {}", l.flat_id, zone)}</span>
                                        </a>
                                    }
                                }).collect_view()}
                            </div>
                        }.into_any()
                    }}
                </section>

                // ── EVENTS ──
                <section class="me-section">
                    <div class="me-section-h">
                        <span>"YOUR EVENTS"</span>
                        <a href="/calendar">"calendar →"</a>
                    </div>
                    {move || {
                        let list = events.get();
                        if list.is_empty() {
                            return view! {
                                <div class="me-empty">
                                    "No events posted. "
                                    <a href="/calendar">"Add an event"</a>
                                    "."
                                </div>
                            }.into_any();
                        }
                        view! {
                            <div class="me-chip-grid">
                                {list.into_iter().map(|e: EventCard| {
                                    view! {
                                        <a class="me-chip event" href="/calendar">
                                            <span class="me-chip-name event">{e.title}</span>
                                            <span class="me-chip-meta">{format!("{} · {} · {}", e.city, e.when, e.status)}</span>
                                        </a>
                                    }
                                }).collect_view()}
                            </div>
                        }.into_any()
                    }}
                </section>

                // ── CITIES ──
                <section class="me-section">
                    <div class="me-section-h">
                        <span>"CITIES YOU FOUNDED"</span>
                        <a href="/cities">"catalog →"</a>
                    </div>
                    {move || {
                        let list = cities.get();
                        if list.is_empty() {
                            return view! {
                                <div class="me-empty">
                                    "No founding entries. "
                                    <a href="/cities">"Found a city"</a>
                                    "."
                                </div>
                            }.into_any();
                        }
                        view! {
                            <div class="me-chip-grid">
                                {list.into_iter().map(|c: CityCard| {
                                    let href = c.href.clone();
                                    view! {
                                        <a class="me-chip city" href=href>
                                            <span class="me-chip-name city">{c.name}</span>
                                            <span class="me-chip-meta">{format!("{} · {}", c.region, c.status)}</span>
                                        </a>
                                    }
                                }).collect_view()}
                            </div>
                        }.into_any()
                    }}
                </section>

                // ── INTENTS ──
                <section class="me-section">
                    <div class="me-section-h">
                        <span>"INTENT TAPE"</span>
                        <a href="/map">"console →"</a>
                    </div>
                    {move || {
                        let list = intents.get();
                        if list.is_empty() {
                            return view! {
                                <div class="me-empty">
                                    "No intents yet — buy, lease, split, merge, or assign on the "
                                    <a href="/map">"map"</a>
                                    "."
                                </div>
                            }.into_any();
                        }
                        view! {
                            <div class="me-intent-list">
                                {list.into_iter().take(24).map(|it: IntentRec| {
                                    let act = it.action.to_uppercase();
                                    view! {
                                        <div class="me-intent">
                                            <span class="ii-id">{format!("#{:03}", it.id)}</span>
                                            <span class="ii-fleet">{it.fleet}</span>
                                            <span class="ii-act">{act}</span>
                                            <span class="ii-flat">{it.flat.to_uppercase()}</span>
                                        </div>
                                    }
                                }).collect_view()}
                            </div>
                        }.into_any()
                    }}
                </section>

                <p class="bank-footnote">
                    "Soft3 local dashboard — data stays in this browser (localStorage). No closed backend. Profile, CX, leases and intents sync with map ops."
                </p>
            </div>

            // edit profile sheet
            {move || edit_open.get().then(|| view! {
                <div class="cyberia-sheet-backdrop" on:click=move |_| edit_open.set(false)>
                    <div class="cyberia-sheet" on:click=move |ev| ev.stop_propagation()>
                        <div class="sheet-h">
                            <span class="panel-kicker">"EDIT PROFILE"</span>
                            <button class="sheet-x" on:click=move |_| edit_open.set(false)>"✕"</button>
                        </div>
                        <p class="sheet-note">"Handle and display name — local only."</p>
                        <label class="found-label">"HANDLE"</label>
                        <input
                            class="found-input"
                            type="text"
                            placeholder="anon"
                            prop:value=move || draft_handle.get()
                            on:input=move |ev| draft_handle.set(event_target_value(&ev))
                        />
                        <label class="found-label">"DISPLAY NAME"</label>
                        <input
                            class="found-input"
                            type="text"
                            placeholder="your name"
                            prop:value=move || draft_name.get()
                            on:input=move |ev| draft_name.set(event_target_value(&ev))
                        />
                        <button
                            class="intent-commit"
                            on:click=move |_| {
                                let mut h = draft_handle.get().trim().to_lowercase();
                                h = h.chars().filter(|c| c.is_ascii_alphanumeric() || *c == '_' || *c == '-').collect();
                                if h.is_empty() { h = "anon".into(); }
                                let name = {
                                    let n = draft_name.get().trim().to_string();
                                    if n.is_empty() { "Anonymous".into() } else { n }
                                };
                                let mut p = profile.get_untracked();
                                p.handle = h;
                                p.name = name;
                                save_profile(&p);
                                profile.set(p);
                                edit_open.set(false);
                            }
                        >
                            "SAVE"
                        </button>
                    </div>
                </div>
            })}

            <div class="search-dock cyberia-dock cities-dock">
                <span class="dock-count">
                    {move || {
                        let b: SoftBalance = balance.get();
                        format!(
                            "@{} · {} CX · {}r {}l {}e · activity {:.0}",
                            profile.get().handle,
                            fmt_cx(b.cx),
                            robots.get().len(),
                            leases.get().len(),
                            events.get().len(),
                            score(),
                        )
                    }}
                </span>
                <button class="cta-btn cta-lease cta-lg dock-found" on:click=move |_| {
                    // soft top-up for demo
                    let mut b = balance.get_untracked();
                    b.cx += 25.0;
                    save_balance(&b);
                    balance.set(b);
                }>
                    <span class="cta-copy">
                        <span class="cta-title">"+25 CX"</span>
                        <span class="cta-sub">"mock top-up"</span>
                    </span>
                </button>
                <a href="https://x.com/cyberiacap" target="_blank" rel="noopener" class="dock-credit">
                    "🏴 a "<span style="color: var(--cyber-green);">"cyberia"</span>" project"
                </a>
            </div>
        </div>
    }
}
