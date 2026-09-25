//! Shared top nav for cyberia.my catalog surfaces.
//!
//! Only YOU · CITIES · MAP stay in view. Every other surface is parked under
//! `/menu/<name>` — still routed, no longer competing for the eye.

use crate::land::FLAG_SVG;
use leptos::prelude::*;

/// Surfaces hidden from the top bar: (slug, label, one-line purpose).
pub const HIDDEN: &[(&str, &str, &str)] = &[
    ("world", "WORLD", "who am I — words, links, signals"),
    ("studio", "STUDIO", "the constructor hub"),
    ("elements", "ELEMENTS", "periodic table of the stack"),
    ("products", "PRODUCTS", "market · goods"),
    ("genetics", "GENETICS", "seed lines"),
    ("services", "SERVICES", "what the valley offers"),
    ("orgs", "ORGS", "organisations"),
    ("calendar", "CALENDAR", "events"),
    ("robots", "ROBOTS", "fleets · hard force"),
    ("plots", "PLOTS", "126 flats as cards"),
    ("places", "PLACES", "named points on the land"),
    ("domains", "DOMAINS", "21 cybics domains on the map"),
    ("states", "STATES", "earth states by capital"),
];

/// `active`: you | cities | map
#[component]
pub fn CyberiaNav(#[prop(into)] active: String) -> impl IntoView {
    let a = active;
    view! {
        <div class="map-zone cyberia-nav">
            <a class=if a == "you" { "nav-btn nav-here nav-you" } else { "nav-btn nav-you" } href="/me">"YOU"</a>
            <a class=if a == "cities" { "nav-btn nav-here" } else { "nav-btn" } href="/cities">"CITIES"</a>
            <a class=if a == "map" { "nav-btn nav-here" } else { "nav-btn" } href="/map">"MAP"</a>
        </div>
    }
}

/// `/menu` — the parked surfaces, one row each.
#[component]
pub fn MenuPage() -> impl IntoView {
    Effect::new(move |_| {
        document().set_title("Cyberia — menu");
    });
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
                        <CyberiaNav active="" />
                    </div>
                </div>
            </div>
            <div class="cities-stage">
                <div class="cities-hero">
                    <div>
                        <div class="cities-kicker">"MENU"</div>
                        <h2 class="cities-title">"Parked surfaces"</h2>
                        <p class="cities-lead">"Everything that left the top bar lives here."</p>
                    </div>
                </div>
                <div class="cities-grid">
                    {HIDDEN.iter().map(|(slug, label, blurb)| {
                        let href = format!("/menu/{slug}");
                        view! {
                            <a class="city-card live" href=href>
                                <div class="city-name">{*label}</div>
                                <p class="city-blurb">{*blurb}</p>
                                <div class="city-meta">
                                    <span>{format!("/menu/{slug}")}</span>
                                    <span class="city-open">"OPEN →"</span>
                                </div>
                            </a>
                        }
                    }).collect_view()}
                </div>
            </div>
        </div>
    }
}
