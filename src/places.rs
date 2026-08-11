//! Places catalog — estates, buildings, and land points on plots.

use crate::land::{
    centroid, load_map, place_kind, place_score, plot_containing, rating_tier, PlaceKind, FLAG_SVG,
};
use crate::nav::CyberiaNav;
use leptos::prelude::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SortKey {
    ScoreDesc,
    Name,
    Kind,
    Plot,
}

#[derive(Clone, Debug)]
struct PlaceRow {
    id: String,
    name: String,
    kind: PlaceKind,
    score: f64,
    plot_id: Option<String>,
    plot_name: Option<String>,
    plot_zone: Option<String>,
    lon: f64,
    lat: f64,
}

#[component]
pub fn PlacesPage() -> impl IntoView {
    let map = load_map();
    let rows: Vec<PlaceRow> = map
        .places
        .iter()
        .enumerate()
        .map(|(i, p)| {
            let (lon, lat) = centroid(&p.coords).unwrap_or((0.0, 0.0));
            let kind = place_kind(&p.name, &p.id);
            let score = place_score(&p.name, &p.id, kind);
            let host = plot_containing(&map.phase0, lon, lat);
            // unique key — ids can collide (multiple "wc")
            let id = if p.id.is_empty() {
                format!("place-{i}")
            } else {
                format!("{}-{}", p.id, i)
            };
            PlaceRow {
                id,
                name: p.name.clone(),
                kind,
                score,
                plot_id: host.map(|h| h.id.clone()),
                plot_name: host.map(|h| h.name.clone()),
                plot_zone: host.map(|h| {
                    if h.zone.is_empty() {
                        h.id.split('-').next().unwrap_or("—").to_string()
                    } else {
                        h.zone.clone()
                    }
                }),
                lon,
                lat,
            }
        })
        .collect();

    let rows_sig = RwSignal::new(rows);
    let sort = RwSignal::new(SortKey::ScoreDesc);
    let kind_filter = RwSignal::new(None::<PlaceKind>);
    let q = RwSignal::new(String::new());

    Effect::new(move |_| {
        document().set_title("Cyberia — places");
    });

    let filtered = move || {
        let kf = kind_filter.get();
        let query = q.get().trim().to_lowercase();
        let mut list: Vec<PlaceRow> = rows_sig
            .get()
            .into_iter()
            .filter(|r| kf.map(|k| r.kind == k).unwrap_or(true))
            .filter(|r| {
                if query.is_empty() {
                    true
                } else {
                    r.name.to_lowercase().contains(&query)
                        || r.id.to_lowercase().contains(&query)
                        || r.plot_name
                            .as_ref()
                            .map(|n| n.to_lowercase().contains(&query))
                            .unwrap_or(false)
                        || r.plot_zone
                            .as_ref()
                            .map(|n| n.to_lowercase().contains(&query))
                            .unwrap_or(false)
                }
            })
            .collect();
        match sort.get() {
            SortKey::ScoreDesc => list.sort_by(|a, b| {
                b.score
                    .partial_cmp(&a.score)
                    .unwrap_or(std::cmp::Ordering::Equal)
            }),
            SortKey::Name => list.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase())),
            SortKey::Kind => list.sort_by(|a, b| {
                a.kind
                    .label()
                    .cmp(b.kind.label())
                    .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
            }),
            SortKey::Plot => list.sort_by(|a, b| {
                a.plot_name
                    .as_deref()
                    .unwrap_or("~")
                    .cmp(b.plot_name.as_deref().unwrap_or("~"))
                    .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
            }),
        }
        list
    };

    let n_show = move || filtered().len();
    let n_estate = move || {
        filtered()
            .iter()
            .filter(|r| r.kind == PlaceKind::Estate)
            .count()
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
                            {move || format!("{} PLACES", n_show())}
                        </div>
                        <CyberiaNav active="places" />
                    </div>
                </div>
            </div>

            <div class="cities-stage">
                <div class="cities-hero">
                    <div>
                        <div class="cities-kicker">"ON LAND · CYBER VALLEY"</div>
                        <h2 class="cities-title">"Places"</h2>
                        <p class="cities-lead">
                            "Estates, buildings, and points on plots — houses, amenities, trails, nature."
                        </p>
                    </div>
                </div>

                <div class="list-toolbar">
                    <input
                        class="found-input list-search"
                        type="search"
                        placeholder="search place · plot · zone"
                        prop:value=move || q.get()
                        on:input=move |ev| q.set(event_target_value(&ev))
                    />
                    <div class="list-filters">
                        <button
                            class=move || if kind_filter.get().is_none() { "chip chip-on" } else { "chip" }
                            on:click=move |_| kind_filter.set(None)
                        >"ALL"</button>
                        <button
                            class=move || if kind_filter.get() == Some(PlaceKind::Estate) { "chip chip-on" } else { "chip" }
                            on:click=move |_| kind_filter.set(Some(PlaceKind::Estate))
                        >"ESTATE"</button>
                        <button
                            class=move || if kind_filter.get() == Some(PlaceKind::Landmark) { "chip chip-on" } else { "chip" }
                            on:click=move |_| kind_filter.set(Some(PlaceKind::Landmark))
                        >"LANDMARK"</button>
                        <button
                            class=move || if kind_filter.get() == Some(PlaceKind::Amenity) { "chip chip-on" } else { "chip" }
                            on:click=move |_| kind_filter.set(Some(PlaceKind::Amenity))
                        >"AMENITY"</button>
                        <button
                            class=move || if kind_filter.get() == Some(PlaceKind::Nature) { "chip chip-on" } else { "chip" }
                            on:click=move |_| kind_filter.set(Some(PlaceKind::Nature))
                        >"NATURE"</button>
                        <button
                            class=move || if kind_filter.get() == Some(PlaceKind::Trail) { "chip chip-on" } else { "chip" }
                            on:click=move |_| kind_filter.set(Some(PlaceKind::Trail))
                        >"TRAIL"</button>
                    </div>
                    <div class="list-sorts">
                        <span class="list-sort-label">"SORT"</span>
                        <button
                            class=move || if sort.get() == SortKey::ScoreDesc { "chip chip-on" } else { "chip" }
                            on:click=move |_| sort.set(SortKey::ScoreDesc)
                        >"RATING ↓"</button>
                        <button
                            class=move || if sort.get() == SortKey::Name { "chip chip-on" } else { "chip" }
                            on:click=move |_| sort.set(SortKey::Name)
                        >"NAME"</button>
                        <button
                            class=move || if sort.get() == SortKey::Kind { "chip chip-on" } else { "chip" }
                            on:click=move |_| sort.set(SortKey::Kind)
                        >"KIND"</button>
                        <button
                            class=move || if sort.get() == SortKey::Plot { "chip chip-on" } else { "chip" }
                            on:click=move |_| sort.set(SortKey::Plot)
                        >"PLOT"</button>
                    </div>
                </div>

                <div class="cities-grid places-grid">
                    {move || filtered().into_iter().enumerate().map(|(i, r)| {
                        let rank = i + 1;
                        let tier = rating_tier(r.score);
                        let score = r.score;
                        let name = r.name.to_uppercase();
                        let kind_l = r.kind.label();
                        let plot_line = match (&r.plot_name, &r.plot_zone) {
                            (Some(pn), Some(pz)) => format!("{} · {}", pn.to_uppercase(), pz.to_uppercase()),
                            (Some(pn), None) => pn.to_uppercase(),
                            _ => "off-plot / open land".into(),
                        };
                        let href = if let Some(pid) = &r.plot_id {
                            format!("/map?plot={pid}")
                        } else {
                            format!("/map?lat={:.6}&lon={:.6}", r.lat, r.lon)
                        };
                        let is_estate = r.kind == PlaceKind::Estate;
                        let kind_cls = match r.kind {
                            PlaceKind::Estate => "place-kind estate",
                            PlaceKind::Amenity => "place-kind amenity",
                            PlaceKind::Nature => "place-kind nature",
                            PlaceKind::Trail => "place-kind trail",
                            PlaceKind::Landmark => "place-kind landmark",
                        };
                        view! {
                            <a class=if is_estate { "city-card live place-card" } else { "city-card place-card" } href=href>
                                <div class="city-card-top">
                                    <span class="city-rank">{format!("#{rank:02}")}</span>
                                    <span class=format!("rating-badge tier-{}", tier.to_lowercase())>
                                        {format!("{tier} · {score:.0}")}
                                    </span>
                                </div>
                                <div class="city-name">{name}</div>
                                <div class="city-region">{plot_line}</div>
                                <p class="city-blurb">
                                    {match r.kind {
                                        PlaceKind::Estate => "Named hold / building on a plot.",
                                        PlaceKind::Amenity => "Site amenity — parking, WC, pad.",
                                        PlaceKind::Nature => "Natural feature on or near land.",
                                        PlaceKind::Trail => "Path, bridge, or trail node.",
                                        PlaceKind::Landmark => "Land point / marker.",
                                    }}
                                </p>
                                <div class="city-meta">
                                    <span class=kind_cls>{kind_l}</span>
                                    <span class="city-open">"MAP →"</span>
                                </div>
                            </a>
                        }
                    }).collect_view()}
                </div>
            </div>

            <div class="search-dock cyberia-dock cities-dock">
                <span class="dock-count">
                    {move || format!(
                        "{} places · {} estates · soft labels",
                        n_show(),
                        n_estate()
                    )}
                </span>
                <a class="cta-btn cta-lease cta-lg dock-found" href="/map" style="text-decoration:none; max-width: 280px;">
                    <span class="cta-copy">
                        <span class="cta-title">"MAP"</span>
                        <span class="cta-sub">"see on land"</span>
                    </span>
                </a>
                <a href="https://x.com/cyberiacap" target="_blank" rel="noopener" class="dock-credit">
                    "🏴 a "<span style="color: var(--cyber-green);">"cyberia"</span>" project"
                </a>
            </div>
        </div>
    }
}
