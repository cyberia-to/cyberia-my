//! Land our friends sell beside the valley — their listing, as they wrote it.
//! `/friend/:slug` — the listing's own page, the same land board as a flat.

use crate::land::{area_m2, FLAG_SVG};
use crate::nav::CyberiaNav;
use crate::plot_board::LandBoard;
use crate::plot_land::{load_state, Frame};
use crate::terms::LandUse;
use leptos::prelude::*;
use leptos_router::hooks::use_params_map;
use serde::Deserialize;
use std::sync::Arc;

#[derive(Clone, Debug, Deserialize)]
pub struct FriendSale {
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub fields: Vec<(String, String)>,
    pub coords: Vec<[f64; 2]>,
}

impl FriendSale {
    pub fn slug(&self) -> String {
        self.name
            .to_lowercase()
            .chars()
            .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
            .collect()
    }
}

pub fn friend_sales() -> &'static [FriendSale] {
    static SALES: std::sync::OnceLock<Vec<FriendSale>> = std::sync::OnceLock::new();
    SALES.get_or_init(|| serde_json::from_str(include_str!("friend_sales.json")).unwrap_or_default())
}

#[component]
pub fn FriendPage() -> impl IntoView {
    let params = use_params_map();
    let slug = params.get_untracked().get("slug").unwrap_or_default();
    let Some(f) = friend_sales().iter().find(|f| f.slug() == slug) else {
        document().set_title("Cyberia — no such listing");
        return view! {
            <div class="page-shell" style="padding:40px;">
                <h1 style="color: var(--cyber-red);">"404"</h1>
                <p style="color:#777777; margin-top:12px;">{format!("no friends' listing called {slug}")}</p>
                <a href="/map" style="color: var(--cyber-green);">"← back to the map"</a>
            </div>
        }
        .into_any();
    };

    let ink = LandUse::Hgb.color();
    let name = f.name.to_uppercase();
    let plan = area_m2(&f.coords);
    let frame = Arc::new(Frame::new(&f.coords));
    let pid = format!("friend-{}", f.slug());
    let state = RwSignal::new(load_state(&pid));
    document().set_title(&format!("Cyberia — {}", f.name));

    view! {
        <div class="page-shell cities-shell plot-shell">
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
                            <span class="phase-dot" style:background=ink></span>
                            {format!("{name} · FOR SALE")}
                        </div>
                        <CyberiaNav active="map" />
                    </div>
                </div>
            </div>

            <div class="cities-stage plot-stage-wrap">
                <div class="cities-hero plot-hero">
                    <div>
                        <div class="cities-kicker" style:color=ink>"FRIENDS' SALE"</div>
                        <h2 class="cities-title">{name.clone()}</h2>
                        <p class="cities-lead">
                            {format!("sold by a friend of the valley · {plan:.0} m² in plan on the map · {} verts", f.coords.len())}
                        </p>
                    </div>
                    <a class="cta-btn cta-lease dock-found" href="/map" style="text-decoration:none; max-width: 220px;">
                        <span class="cta-copy">
                            <span class="cta-title">"← MAP"</span>
                            <span class="cta-sub">"back to the valley"</span>
                        </span>
                    </a>
                </div>

                <div class="plot-stage">
                    <div class="plot-col plot-col-land">
                        <LandBoard
                            plot_id=pid
                            frame=frame
                            state=state
                            district_color=ink
                            status_color=ink
                        />
                    </div>
                    <div class="plot-col plot-col-side">
                        <section class="plot-card plot-deal">
                            <div class="cyberia-panel-h">
                                <span class="panel-kicker">"LISTING"</span>
                                <span class="panel-sub">"as the seller wrote it"</span>
                            </div>
                            <div class="friend-fields">
                                {f.fields.iter().map(|(k, v)| view! {
                                    <div class="intent-row">
                                        <span class="intent-k">{k.clone()}</span>
                                        <span class="intent-v">{v.clone()}</span>
                                    </div>
                                }).collect_view()}
                            </div>
                            {(!f.description.is_empty()).then(|| view! {
                                <p class="friend-desc">{f.description.clone()}</p>
                            })}
                        </section>
                    </div>
                </div>
            </div>
        </div>
    }
    .into_any()
}
