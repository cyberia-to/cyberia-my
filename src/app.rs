//! cyberia.my router — cities catalog + per-city consoles.

use crate::cities::CitiesPage;
use crate::console::ValleyConsole;
use leptos::prelude::*;
use leptos_router::components::*;
use leptos_router::path;

#[component]
pub fn App() -> impl IntoView {
    view! {
        <Router>
            <Routes fallback=|| {
                view! {
                    <div class="page-shell" style="padding:40px;">
                        <h1 style="color: var(--cyber-red);">"404"</h1>
                        <p style="color:#888; margin-top:12px;">"City or page not found."</p>
                        <a href="/cities" style="color: var(--cyber-green);">"← cities"</a>
                    </div>
                }
            }>
                <Route path=path!("/") view=CitiesPage />
                <Route path=path!("/cities") view=CitiesPage />
                <Route path=path!("/city/cyber-valley") view=ValleyConsole />
                <Route path=path!("/city/:slug") view=CityStub />
            </Routes>
        </Router>
    }
}

/// Placeholder for founding / not-yet-live cities.
#[component]
fn CityStub() -> impl IntoView {
    let params = leptos_router::hooks::use_params_map();
    let slug = move || params.get().get("slug").unwrap_or_else(|| "unknown".into());
    Effect::new(move |_| {
        document().set_title(&format!("Cyberia — {}", slug()));
    });
    view! {
        <div class="page-shell cities-shell">
            <div class="site-chrome cyberia-chrome">
                <div class="chrome-inner">
                    <div class="header-row1">
                        <div class="logo-zone">
                            <h1 class="logo">
                                <a href="/cities" style="color: var(--cyber-green); text-decoration:none;">
                                    "cyber•ia"
                                </a>
                            </h1>
                        </div>
                        <div class="map-zone">
                            <a class="nav-btn" href="/cities">"CITIES"</a>
                        </div>
                    </div>
                </div>
            </div>
            <div class="cities-stage" style="align-content: start;">
                <div class="cities-kicker">"CITY"</div>
                <h2 class="cities-title" style="text-transform: none;">
                    {move || slug().to_uppercase()}
                </h2>
                <p class="cities-lead">
                    "Console not live yet. Founding entries stay in the catalog until land + fleets ship."
                </p>
                <a class="cta-btn cta-lease cta-lg" href="/cities" style="max-width: 280px; margin-top: 16px; text-decoration: none; display: inline-flex;">
                    <span class="cta-title">"← BACK TO CITIES"</span>
                </a>
            </div>
        </div>
    }
}
