//! Plots catalog — all Gesing land holds with soft ratings + sort/filter.

use crate::land::{
    area_m2, fmt_area_m2, load_map, plot_rating, rating_tier, LandFlat, PlotRating, FLAG_SVG,
};
use crate::nav::CyberiaNav;
use leptos::prelude::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SortKey {
    ScoreDesc,
    ScoreAsc,
    AreaDesc,
    AreaAsc,
    Name,
    Zone,
}

#[derive(Clone, Debug)]
struct PlotRow {
    id: String,
    name: String,
    zone: String,
    m2: f64,
    rating: PlotRating,
    named: bool,
}

fn build_rows(plots: &[LandFlat]) -> Vec<PlotRow> {
    plots
        .iter()
        .map(|p| {
            let m2 = area_m2(&p.coords);
            let zone = if p.zone.is_empty() {
                p.id.split('-').next().unwrap_or("—").to_string()
            } else {
                p.zone.clone()
            };
            let rating = plot_rating(&p.id, &p.name, &zone, m2);
            let named = p.name.contains(':') || p.name.contains('@');
            PlotRow {
                id: p.id.clone(),
                name: p.name.clone(),
                zone,
                m2,
                rating,
                named,
            }
        })
        .collect()
}

fn zones_from(rows: &[PlotRow]) -> Vec<String> {
    let mut z: Vec<String> = rows.iter().map(|r| r.zone.clone()).collect();
    z.sort();
    z.dedup();
    z
}

#[component]
pub fn PlotsPage() -> impl IntoView {
    let map = load_map();
    let rows = build_rows(&map.phase0);
    let all_zones = zones_from(&rows);
    let rows_sig = RwSignal::new(rows);
    let sort = RwSignal::new(SortKey::ScoreDesc);
    let zone_filter = RwSignal::new(String::new()); // empty = all
    let q = RwSignal::new(String::new());

    Effect::new(move |_| {
        document().set_title("Cyberia — plots");
    });

    let filtered = move || {
        let zf = zone_filter.get();
        let query = q.get().trim().to_lowercase();
        let mut list: Vec<PlotRow> = rows_sig
            .get()
            .into_iter()
            .filter(|r| zf.is_empty() || r.zone == zf)
            .filter(|r| {
                if query.is_empty() {
                    true
                } else {
                    r.name.to_lowercase().contains(&query)
                        || r.id.to_lowercase().contains(&query)
                        || r.zone.to_lowercase().contains(&query)
                }
            })
            .collect();
        match sort.get() {
            SortKey::ScoreDesc => list.sort_by(|a, b| {
                b.rating
                    .score
                    .partial_cmp(&a.rating.score)
                    .unwrap_or(std::cmp::Ordering::Equal)
            }),
            SortKey::ScoreAsc => list.sort_by(|a, b| {
                a.rating
                    .score
                    .partial_cmp(&b.rating.score)
                    .unwrap_or(std::cmp::Ordering::Equal)
            }),
            SortKey::AreaDesc => {
                list.sort_by(|a, b| b.m2.partial_cmp(&a.m2).unwrap_or(std::cmp::Ordering::Equal))
            }
            SortKey::AreaAsc => {
                list.sort_by(|a, b| a.m2.partial_cmp(&b.m2).unwrap_or(std::cmp::Ordering::Equal))
            }
            SortKey::Name => list.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase())),
            SortKey::Zone => list.sort_by(|a, b| {
                a.zone
                    .cmp(&b.zone)
                    .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
            }),
        }
        list
    };

    let total_m2 = move || filtered().iter().map(|r| r.m2).sum::<f64>();
    let n_show = move || filtered().len();

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
                            {move || format!("{} PLOTS", n_show())}
                        </div>
                        <CyberiaNav active="plots" />
                    </div>
                </div>
            </div>

            <div class="cities-stage">
                <div class="cities-hero">
                    <div>
                        <div class="cities-kicker">"LAND · CYBER VALLEY"</div>
                        <h2 class="cities-title">"Plots"</h2>
                        <p class="cities-lead">
                            "All phase-0 holds — area, zone, soft ratings. Sort and filter, then open on the map."
                        </p>
                    </div>
                </div>

                <div class="list-toolbar">
                    <input
                        class="found-input list-search"
                        type="search"
                        placeholder="search name · id · zone"
                        prop:value=move || q.get()
                        on:input=move |ev| q.set(event_target_value(&ev))
                    />
                    <div class="list-filters">
                        <button
                            class=move || if zone_filter.get().is_empty() { "chip chip-on" } else { "chip" }
                            on:click=move |_| zone_filter.set(String::new())
                        >"ALL"</button>
                        {all_zones.into_iter().map(|z| {
                            let z2 = z.clone();
                            let z3 = z.clone();
                            view! {
                                <button
                                    class=move || if zone_filter.get() == z2 { "chip chip-on" } else { "chip" }
                                    on:click=move |_| zone_filter.set(z3.clone())
                                >{z.to_uppercase()}</button>
                            }
                        }).collect_view()}
                    </div>
                    <div class="list-sorts">
                        <span class="list-sort-label">"SORT"</span>
                        <button
                            class=move || if sort.get() == SortKey::ScoreDesc { "chip chip-on" } else { "chip" }
                            on:click=move |_| sort.set(SortKey::ScoreDesc)
                        >"RATING ↓"</button>
                        <button
                            class=move || if sort.get() == SortKey::ScoreAsc { "chip chip-on" } else { "chip" }
                            on:click=move |_| sort.set(SortKey::ScoreAsc)
                        >"RATING ↑"</button>
                        <button
                            class=move || if sort.get() == SortKey::AreaDesc { "chip chip-on" } else { "chip" }
                            on:click=move |_| sort.set(SortKey::AreaDesc)
                        >"AREA ↓"</button>
                        <button
                            class=move || if sort.get() == SortKey::AreaAsc { "chip chip-on" } else { "chip" }
                            on:click=move |_| sort.set(SortKey::AreaAsc)
                        >"AREA ↑"</button>
                        <button
                            class=move || if sort.get() == SortKey::Name { "chip chip-on" } else { "chip" }
                            on:click=move |_| sort.set(SortKey::Name)
                        >"NAME"</button>
                        <button
                            class=move || if sort.get() == SortKey::Zone { "chip chip-on" } else { "chip" }
                            on:click=move |_| sort.set(SortKey::Zone)
                        >"ZONE"</button>
                    </div>
                </div>

                <div class="cities-grid plots-grid">
                    {move || filtered().into_iter().enumerate().map(|(i, r)| {
                        let rank = i + 1;
                        let tier = rating_tier(r.rating.score);
                        let score = r.rating.score;
                        let size_r = r.rating.size;
                        let zone_r = r.rating.zone;
                        let depth_r = r.rating.depth;
                        let area = fmt_area_m2(r.m2);
                        let name = r.name.to_uppercase();
                        let zone = r.zone.to_uppercase();
                        let href = format!("/map?plot={}", r.id);
                        let live = score >= 65.0 || r.named;
                        view! {
                            <a class=if live { "city-card live plot-card" } else { "city-card plot-card" } href=href>
                                <div class="city-card-top">
                                    <span class="city-rank">{format!("#{rank:02}")}</span>
                                    <span class=format!("rating-badge tier-{}", tier.to_lowercase())>
                                        {format!("{tier} · {score:.0}")}
                                    </span>
                                </div>
                                <div class="city-name">{name}</div>
                                <div class="city-region">{format!("{zone} · {area}")}</div>
                                <div class="rating-bars">
                                    <div class="rating-row">
                                        <span>"SIZE"</span>
                                        <div class="rating-track"><div class="rating-fill size" style=format!("width:{size_r:.0}%")></div></div>
                                        <span class="rating-n">{format!("{size_r:.0}")}</span>
                                    </div>
                                    <div class="rating-row">
                                        <span>"ZONE"</span>
                                        <div class="rating-track"><div class="rating-fill zone" style=format!("width:{zone_r:.0}%")></div></div>
                                        <span class="rating-n">{format!("{zone_r:.0}")}</span>
                                    </div>
                                    <div class="rating-row">
                                        <span>"DEPTH"</span>
                                        <div class="rating-track"><div class="rating-fill depth" style=format!("width:{depth_r:.0}%")></div></div>
                                        <span class="rating-n">{format!("{depth_r:.0}")}</span>
                                    </div>
                                </div>
                                <div class="city-meta">
                                    <span>{format!("{:.0} m²", r.m2)}</span>
                                    <span class="city-open">"MAP →"</span>
                                </div>
                            </a>
                        }
                    }).collect_view()}
                </div>
            </div>

            <div class="search-dock cyberia-dock cities-dock">
                <span class="dock-count">
                    {move || {
                        let n = n_show();
                        let ha = total_m2() / 10_000.0;
                        format!("{n} plots · {ha:.2} ha shown · soft ratings")
                    }}
                </span>
                <a class="cta-btn cta-lease cta-lg dock-found" href="/map" style="text-decoration:none; max-width: 280px;">
                    <span class="cta-copy">
                        <span class="cta-title">"MAP"</span>
                        <span class="cta-sub">"land work"</span>
                    </span>
                </a>
                <a href="https://x.com/cyberiacap" target="_blank" rel="noopener" class="dock-credit">
                    "🏴 a "<span style="color: var(--cyber-green);">"cyberia"</span>" project"
                </a>
            </div>
        </div>
    }
}
