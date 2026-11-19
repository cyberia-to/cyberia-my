//! PLAN — the flat drawn to scale on its 2 m grid, north up, each side
//! signed in metres. A buyer reads it; nothing on it is a tool.

use crate::plot_land::{Frame, CELL_M, PX_M};
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
    let edges = frame.edges();
    let perimeter = frame.perimeter_m();
    let width_m = frame.max_x - frame.min_x;
    let depth_m = frame.max_y - frame.min_y;
    // label size follows the drawing so a 5 are flat and a 50 are flat read alike
    let font = (w.max(h) / 34.0).clamp(9.0, 40.0);

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
                // every side signed with its length, set just outside the line
                {edges.into_iter().filter(|e| e.len_m >= 1.0).map(|e| {
                    let off = font * 0.9;
                    let (x, y) = (e.mid.0 + e.out.0 * off, e.mid.1 + e.out.1 * off + font * 0.35);
                    view! {
                        <text
                            x=format!("{x:.1}")
                            y=format!("{y:.1}")
                            text-anchor="middle"
                            class="land-edge"
                            font-size=format!("{font:.1}")
                        >{format!("{:.1} m", e.len_m)}</text>
                    }
                }).collect_view()}
            </svg>
        </section>
    }
}
