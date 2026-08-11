//! cyberia.my router — ERP studio pages are full URLs (no overlays).

use crate::cities::CitiesPage;
use crate::console::ValleyConsole;
use crate::elements::ElementsPage;
use crate::events::EventsPage;
use crate::market::MarketPage;
use crate::me::MePage;
use crate::nav::CyberiaNav;
use crate::places::PlacesPage;
use crate::plots::PlotsPage;
use crate::robots::RobotsPage;
use crate::states::StatesPage;
use crate::studio::*;
use crate::world::WorldPage;
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
                        <a href="/world" style="color: var(--cyber-green);">"← ERP studio"</a>
                    </div>
                }
            }>
                <Route path=path!("/") view=CitiesPage />
                <Route path=path!("/cities") view=CitiesPage />
                <Route path=path!("/me") view=MePage />
                <Route path=path!("/you") view=MePage />

                // ERP hub + full-page forms
                <Route path=path!("/world") view=WorldPage />
                <Route path=path!("/erp") view=WorldPage />
                <Route path=path!("/world/links") view=LinksListPage />
                <Route path=path!("/world/links/new") view=LinkNewPage />
                <Route path=path!("/world/link/:id") view=LinkViewPage />
                <Route path=path!("/world/link/:id/edit") view=LinkEditPage />
                <Route path=path!("/world/cards") view=CardsListPage />
                <Route path=path!("/world/cards/new") view=CardNewPage />
                <Route path=path!("/world/card/:id") view=CardViewPage />
                <Route path=path!("/world/card/:id/edit") view=CardEditPage />
                <Route path=path!("/world/coins") view=CoinsListPage />
                <Route path=path!("/world/coins/new") view=CoinNewPage />
                <Route path=path!("/world/coin/:id") view=CoinEditPage />
                <Route path=path!("/world/templates") view=TemplatesListPage />
                <Route path=path!("/world/templates/new") view=TemplateNewPage />
                <Route path=path!("/world/template/:id") view=TemplateViewPage />
                <Route path=path!("/world/template/:id/edit") view=TemplateEditPage />
                <Route path=path!("/world/intents") view=IntentsListPage />
                <Route path=path!("/world/intent/:id") view=IntentViewPage />
                <Route path=path!("/world/schedules") view=SchedulesListPage />
                <Route path=path!("/world/schedules/new") view=ScheduleNewPage />
                <Route path=path!("/world/schedule/:id") view=ScheduleViewPage />
                <Route path=path!("/world/schedule/:id/edit") view=ScheduleEditPage />

                <Route path=path!("/elements") view=ElementsPage />
                <Route path=path!("/market") view=MarketPage />
                <Route path=path!("/calendar") view=EventsPage />
                <Route path=path!("/events") view=EventsPage />
                <Route path=path!("/robots") view=RobotsPage />
                <Route path=path!("/plots") view=PlotsPage />
                <Route path=path!("/places") view=PlacesPage />
                <Route path=path!("/states") view=StatesPage />
                <Route path=path!("/map") view=ValleyConsole />
                <Route path=path!("/city/cyber-valley") view=ValleyConsole />
                <Route path=path!("/city/:slug") view=CityStub />
            </Routes>
        </Router>
    }
}

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
                                <a href="/cities" style="color: var(--cyber-green); text-decoration:none;">"cyber•ia"</a>
                            </h1>
                        </div>
                        <CyberiaNav active="cities" />
                    </div>
                </div>
            </div>
            <div class="cities-stage" style="align-content: start;">
                <div class="cities-kicker">"CITY"</div>
                <h2 class="cities-title" style="text-transform: none;">{move || slug().to_uppercase()}</h2>
                <a class="cta-btn cta-lease cta-lg" href="/cities" style="max-width: 280px; margin-top: 16px; text-decoration: none; display: inline-flex;">
                    <span class="cta-title">"← BACK TO CITIES"</span>
                </a>
            </div>
        </div>
    }
}
