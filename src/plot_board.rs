//! PLAN — the flat drawn to scale on its 2 m grid, north up, every side wide
//! enough signed in metres. A buyer reads it; nothing on it is a tool.

use crate::plot_land::{Frame, Side, CELL_M, PX_M};
use leptos::prelude::*;
use std::sync::Arc;

#[component]
pub fn LandBoard(
    frame: Arc<Frame>,
    area_m2: f64,
    district_color: &'static str,
    status_color: &'static str,
) -> impl IntoView {
    let w = frame.svg_w();
    let h = frame.svg_h();
    let outline = frame.outline_path();
    let cells = frame.cells();
    let sides = frame.sides();
    let perimeter = frame.perimeter_m();
    let width_m = frame.max_x - frame.min_x;
    let depth_m = frame.max_y - frame.min_y;
    let font = frame.font();
    let off = frame.sign_off();

    view! {
        <section class="plot-card plot-land">
            <div class="cyberia-panel-h">
                <span class="panel-kicker">"PLAN"</span>
                <span class="panel-sub">{format!(
                    "{area_m2:.0} m² · {perimeter:.0} m around · {width_m:.0} × {depth_m:.0} m across · 2 m grid · north up"
                )}</span>
            </div>
            <svg class="land-svg" viewBox=format!("0 0 {w:.1} {h:.1}")>
                <path d=outline.clone() fill=status_color fill-opacity="0.16" stroke="none" />
                {cells.iter().map(|&(c, r)| {
                    let (cx, cy) = frame.cell_center(c, r);
                    let (sx, sy) = frame.to_svg(cx - CELL_M / 2.0, cy + CELL_M / 2.0);
                    view! {
                        <rect
                            x=format!("{sx:.2}")
                            y=format!("{sy:.2}")
                            width=format!("{:.2}", CELL_M * PX_M)
                            height=format!("{:.2}", CELL_M * PX_M)
                            class="land-cell"
                        />
                    }
                }).collect_view()}
                <path d=outline fill="none" stroke=district_color stroke-width="2" vector-effect="non-scaling-stroke" />
                // each side that can carry its length is signed just outside its
                // line; a side too short for its own sign is left bare
                {sides.into_iter().filter_map(|Side { mid, out, sign, .. }| {
                    sign.map(|text| {
                        let (x, y) = (mid.0 + out.0 * off, mid.1 + out.1 * off + font * 0.35);
                        view! {
                            <text
                                x=format!("{x:.1}")
                                y=format!("{y:.1}")
                                text-anchor="middle"
                                class="land-side"
                                font-size=format!("{font:.1}")
                            >{text}</text>
                        }
                    })
                }).collect_view()}
            </svg>
        </section>
    }
}
