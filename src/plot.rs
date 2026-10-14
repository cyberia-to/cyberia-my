//! `/plot/:id` — a flat's own page: land board, lease deal, documents.
//! `/plot/:id/lease` — the same page entered from the map's LEASE press:
//! the deal panel comes first and the request is opened on arrival.

use crate::deals::{flat_status, holder, load_deals, FlatStatus};
use crate::terms::{bare_name, land_use, size_line, terms};
use crate::land::{area_m2, load_map, LandFlat, FLAG_SVG};
use crate::nav::CyberiaNav;
use crate::plot_board::LandBoard;
use crate::plot_side::SideColumn;
use crate::plot_land::{load_state, Frame};
use leptos::prelude::*;
use leptos_router::hooks::{use_location, use_params_map};
use std::sync::Arc;

fn district_color(zone: &str) -> &'static str {
    let z = zone.to_lowercase();
    if z.contains("avalon") {
        "#ff5b00"
    } else if z.contains("sinwood") {
        "#00fe00"
    } else if z.contains("bridge") {
        "#00acff"
    } else if z.contains("core") {
        "#fcf000"
    } else if z.contains("ether") {
        "#d500f9"
    } else if z.contains("asgard") {
        "#ff0000"
    } else if z.contains("edem") || z.contains("canyon") {
        "#304ffe"
    } else {
        "#ffffff"
    }
}

/// `sinwood` → `Sinwood`
fn title_case(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
        None => String::new(),
    }
}

fn passport_json(flat: &LandFlat, frame: &Frame, status: FlatStatus) -> String {
    let deals = load_deals();
    let deal = deals.get(&flat.id);
    let st = load_state(&flat.id);
    let doc = serde_json::json!({
        "id": flat.id,
        "name": bare_name(&flat.name),
        "holder": holder(&flat.id),
        "lease_book": terms(&flat.id),
        "zone": flat.zone,
        "phase": flat.phase,
        "status": status.label(),
        "area_m2": (area_m2(&flat.coords) * 10.0).round() / 10.0,
        "perimeter_m": (frame.perimeter_m() * 10.0).round() / 10.0,
        "vertices": flat.coords.len(),
        "coords_lonlat": flat.coords,
        "grid_cell_m": crate::plot_land::CELL_M,
        "pruned_cells": st.pruned,
        "placed": st.placed,
        "deal": deal,
        "generated_ms": crate::deals::now_ms(),
        "source": "cyberia.my · phase 0 · soft3 local",
    });
    serde_json::to_string_pretty(&doc).unwrap_or_default()
}

fn boundary_kml(flat: &LandFlat) -> String {
    let ring: Vec<String> = flat
        .coords
        .iter()
        .map(|c| format!("{},{},0", c[0], c[1]))
        .collect();
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<kml xmlns=\"http://www.opengis.net/kml/2.2\"><Document><name>{name}</name><Placemark><name>{name}</name><description>{zone} · phase {phase} · cyberia.my</description><Polygon><outerBoundaryIs><LinearRing><coordinates>{coords}</coordinates></LinearRing></outerBoundaryIs></Polygon></Placemark></Document></kml>\n",
        name = bare_name(&flat.name),
        zone = flat.zone,
        phase = flat.phase,
        coords = ring.join(" "),
    )
}

#[component]
pub fn PlotPage() -> impl IntoView {
    let params = use_params_map();
    let location = use_location();
    let map = load_map();
    let id = params.get_untracked().get("id").unwrap_or_default();
    let lease_mode = location.pathname.get_untracked().trim_end_matches('/').ends_with("/lease");
    let flat = map.phase0.iter().find(|f| f.id == id).cloned();

    let Some(flat) = flat else {
        document().set_title("Cyberia — no such flat");
        return view! {
            <div class="page-shell" style="padding:40px;">
                <h1 style="color: var(--cyber-red);">"404"</h1>
                <p style="color:#777777; margin-top:12px;">{format!("no flat called {id} on the Gesing map")}</p>
                <a href="/map" style="color: var(--cyber-green);">"← back to the map"</a>
            </div>
        }
        .into_any();
    };

    let frame = Arc::new(Frame::new(&flat.coords));
    let deals = RwSignal::new(load_deals());
    leptos::task::spawn_local(async move {
        let index = crate::orders::fetch_index().await;
        if !index.is_empty() {
            deals.update(|d| crate::deals::merge_published(d, &index));
        }
    });
    // the flat's own published file carries the payments the index summary lacks
    let pdata = RwSignal::new(None::<crate::orders::PlotData>);
    {
        let fid_data = flat.id.clone();
        leptos::task::spawn_local(async move {
            pdata.set(crate::orders::fetch_plot_data(&fid_data).await);
        });
    }
    let state = RwSignal::new(load_state(&flat.id));
    let name = bare_name(&flat.name);
    let owner = holder(&flat.id).map(|s| s.to_string());
    let book = terms(&flat.id);
    let zone = if flat.zone.is_empty() { "gesing".to_string() } else { flat.zone.clone() };
    let area = area_m2(&flat.coords);
    let dcol = district_color(&zone);
    let fid = flat.id.clone();

    // arrived from the map's LEASE press: the deal panel leads, the request
    // opens only when you press REQUEST LEASE

    let status = Memo::new({
        let fid = fid.clone();
        move |_| flat_status(&fid, &deals.get())
    });

    {
        // the same title the flat's static page carries for crawlers
        let title = format!(
            "{} · {} {} in {}, Cyber Valley, Bali",
            name,
            size_line(book, area),
            land_use(&flat.id).label().to_lowercase(),
            title_case(&zone),
        );
        Effect::new(move |_| {
            document().set_title(&title);
        });
    }

    let passport = passport_json(&flat, &frame, status.get_untracked());
    let kml = boundary_kml(&flat);
    let map_href = format!("/map?plot={}", flat.id);
    let owner_line = owner.clone().map(|h| format!(" · held by {h}")).unwrap_or_default();
    let fid_pill = flat.id.clone();
    let lu_page = land_use(&flat.id);

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
                        <div class="cyberia-phase-pill is-real">
                            <span class="phase-dot" style:background=land_use(&fid_pill).color()></span>
                            {move || format!("{} · {}", name.clone(), status.get().label())}
                        </div>
                        <CyberiaNav active="map" />
                    </div>
                </div>
            </div>

            <div class="cities-stage plot-stage-wrap">
                <div class="cities-hero plot-hero">
                    <div>
                        <div class="cities-kicker" style:color=dcol>{format!("FLAT · {}", zone.to_uppercase())}</div>
                        <h2 class="cities-title">{bare_name(&flat.name)}</h2>
                        <p class="cities-lead">
                            {format!("{} · {}{owner_line}", lu_page.label(), size_line(book, area))}
                        </p>
                    </div>
                    <a class="cta-btn cta-lease dock-found" href=map_href style="text-decoration:none; max-width: 220px;">
                        <span class="cta-copy">
                            <span class="cta-title">"← MAP"</span>
                            <span class="cta-sub">"back to the valley"</span>
                        </span>
                    </a>
                </div>

                <div class=if lease_mode { "plot-stage lease-first" } else { "plot-stage" }>
                    <div class="plot-col plot-col-land">
                        <LandBoard
                            plot_id=flat.id.clone()
                            frame=frame.clone()
                            state=state
                            district_color=dcol
                            status_color=land_use(&flat.id).color()
                        />
                    </div>
                    <div class="plot-col plot-col-side">
                        <SideColumn
                            plot_id=flat.id.clone()
                            plot_name=bare_name(&flat.name)
                            drawn_m2=area
                            passport=passport
                            kml=kml
                            lease_mode=lease_mode
                            deals=deals
                            pdata=pdata
                        />
                    </div>
                </div>
            </div>
        </div>
    }
    .into_any()
}
