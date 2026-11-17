//! Shared top nav for cyberia.my catalog surfaces.
//!
//! Only EARTH · CYBER VALLEY · CABINET stay in view. Every other surface is parked under
//! `/menu/<name>` — still routed, no longer competing for the eye.

use crate::globe::BASE;
use crate::land::FLAG_SVG;
use leptos::prelude::*;

/// Surfaces hidden from the top bar: (slug, label, one-line purpose).
pub const HIDDEN: &[(&str, &str, &str)] = &[
    ("me", "YOU", "your dashboard — balance, leases, requests"),
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

/// The top bar: two places of the one map, and the cabinet.
/// `active`: earth | map | cabinet. On the map itself `on_go` is given: EARTH and
/// CYBER VALLEY then fly the camera ("earth" / "map") instead of loading a page.
#[component]
pub fn CyberiaNav(#[prop(into)] active: String, #[prop(optional, into)] on_go: Option<Callback<&'static str>>) -> impl IntoView {
    let a = active;
    let go = move |place: &'static str| {
        move |ev: leptos::ev::MouseEvent| {
            if let Some(cb) = on_go {
                ev.prevent_default();
                cb.run(place);
            }
        }
    };
    view! {
        <div class="map-zone cyberia-nav">
            <a class=if a == "earth" { "nav-btn nav-here" } else { "nav-btn" } href=format!("{BASE}/") title="the whole earth" on:click=go("earth")>"EARTH"</a>
            <a class=if a == "map" { "nav-btn nav-here" } else { "nav-btn" } href=format!("{BASE}/map") title="the land of Cyber Valley" on:click=go("map")>"CYBER VALLEY"</a>
            <a class=if a == "cabinet" { "nav-btn nav-here" } else { "nav-btn" } href=format!("{BASE}/cabinet")>"CABINET"</a>
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
                                <a href=format!("{BASE}/") class="brand-flag" title="the globe" inner_html=FLAG_SVG></a>
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
