//! LAND board component — the flat on its 2 m grid with the three live tools.

use crate::console::client_to_world;
use crate::plot_land::{
    dist_m, figure, fits, footprint_cells, save_state, Frame, Placed, PlotState, CELL_M, FIGURES,
    PX_M,
};
use crate::wallet::push_intent;
use leptos::prelude::*;
use std::sync::Arc;
use wasm_bindgen::JsCast;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tool {
    Measure,
    Prune,
    Place(&'static str),
}

#[component]
pub fn LandBoard(
    plot_id: String,
    frame: Arc<Frame>,
    state: RwSignal<PlotState>,
    district_color: &'static str,
    status_color: &'static str,
) -> impl IntoView {
    let tool = RwSignal::new(Tool::Measure);
    let measure = RwSignal::new(Vec::<(f64, f64)>::new());
    let hint = RwSignal::new(String::from("tap two points on the flat to measure"));

    let w = frame.svg_w();
    let h = frame.svg_h();
    let outline = frame.outline_path();
    let cells = frame.cells();
    let cells_n = cells.len();
    let pid_click = plot_id.clone();
    let frame_click = frame.clone();
    let frame_grid = frame.clone();
    let frame_dots = frame.clone();
    let frame_placed = frame.clone();
    let frame_measure = frame.clone();

    let on_board_click = move |ev: leptos::ev::MouseEvent| {
        let Some(el) = ev
            .current_target()
            .and_then(|t| t.dyn_into::<web_sys::Element>().ok())
        else {
            return;
        };
        let rect = el.get_bounding_client_rect();
        let (sx, sy) = client_to_world(
            ev.client_x() as f64 - rect.left(),
            ev.client_y() as f64 - rect.top(),
            rect.width(),
            rect.height(),
            (w / 2.0, h / 2.0),
            1.0,
            w,
            h,
        );
        let (x, y) = frame_click.from_svg(sx, sy);
        let fr = &frame_click;
        match tool.get_untracked() {
            Tool::Measure => {
                measure.update(|m| {
                    if m.len() >= 2 {
                        m.clear();
                    }
                    m.push((x, y));
                });
                let m = measure.get_untracked();
                if m.len() == 2 {
                    hint.set(format!("{:.1} m between the two marks", dist_m(m[0], m[1])));
                } else {
                    hint.set("one mark down — tap the second point".into());
                }
            }
            Tool::Prune => {
                let (c, r) = fr.cell_at(x, y);
                if !fr.cell_in_plot(c, r) {
                    hint.set("that cell is off the flat".into());
                    return;
                }
                state.update(|st| {
                    if let Some(i) = st.pruned.iter().position(|p| *p == (c, r)) {
                        st.pruned.remove(i);
                    } else {
                        st.pruned.push((c, r));
                    }
                    save_state(&pid_click, st);
                });
                let n = state.get_untracked().pruned.len();
                hint.set(format!(
                    "{n} cells marked for pruning · {:.0} m²",
                    n as f64 * CELL_M * CELL_M
                ));
            }
            Tool::Place(fid) => {
                let Some(fig) = figure(fid) else { return };
                let (c, r) = fr.cell_at(x, y);
                // tapping a placed figure lifts it
                let hit = state
                    .get_untracked()
                    .placed
                    .iter()
                    .position(|p| {
                        figure(&p.fig)
                            .map(|f| footprint_cells(f, p.c, p.r).contains(&(c, r)))
                            .unwrap_or(false)
                    });
                if let Some(i) = hit {
                    state.update(|st| {
                        st.placed.remove(i);
                        save_state(&pid_click, st);
                    });
                    hint.set("lifted".into());
                    return;
                }
                let placed = state.get_untracked().placed;
                if !fits(fr, &placed, fig, c, r) {
                    hint.set(format!(
                        "{} needs {}×{} free cells on the flat — try another spot",
                        fig.name, fig.cols, fig.rows
                    ));
                    return;
                }
                state.update(|st| {
                    st.placed.push(Placed {
                        fig: fid.to_string(),
                        c,
                        r,
                        built: false,
                    });
                    save_state(&pid_click, st);
                });
                if fig.live {
                    push_intent("YOU", "build", &pid_click);
                    hint.set(format!("{} set at cell {c}·{r} — build intent committed", fig.name));
                } else {
                    hint.set(format!(
                        "{} set at cell {c}·{r} · placement only, build opens in phase 1",
                        fig.name
                    ));
                }
            }
        }
    };

    view! {
        <section class="plot-card plot-land">
            <div class="cyberia-panel-h">
                <span class="panel-kicker">"LAND"</span>
                <span class="panel-sub">{format!("2 m grid · {cells_n} cells on the flat")}</span>
            </div>
            <div class="land-tools">
                <button
                    type="button"
                    class=move || if tool.get() == Tool::Measure { "land-tool on" } else { "land-tool" }
                    on:click=move |_| { tool.set(Tool::Measure); hint.set("tap two points on the flat to measure".into()); }
                >"MEASURE"</button>
                <button
                    type="button"
                    class=move || if tool.get() == Tool::Prune { "land-tool on" } else { "land-tool" }
                    on:click=move |_| { tool.set(Tool::Prune); hint.set("tap cells to mark them for pruning".into()); }
                >"PRUNING"</button>
                <button
                    type="button"
                    class=move || if tool.get() == Tool::Place("unit") { "land-tool on" } else { "land-tool" }
                    on:click=move |_| { tool.set(Tool::Place("unit")); hint.set("tap a free cell — shower + toilet unit, 2×2×2 m".into()); }
                >"BUILD UNIT"</button>
                <span class="land-tools-sep"></span>
                {FIGURES.iter().filter(|f| !f.live).map(|f| {
                    let id = f.id;
                    let name = f.name;
                    let sub = f.sub;
                    view! {
                        <button
                            type="button"
                            class=move || if tool.get() == Tool::Place(id) { "land-tool ghost on" } else { "land-tool ghost" }
                            title=format!("{sub} · placement only")
                            on:click=move |_| { tool.set(Tool::Place(id)); hint.set(format!("{name} · place only, build locked until phase 1")); }
                        >{name}</button>
                    }
                }).collect_view()}
            </div>
            <svg
                class="land-svg"
                viewBox=format!("0 0 {w:.1} {h:.1}")
                on:click=on_board_click
            >
                // the flat, status-tinted, district-edged
                <path d=outline.clone() fill=status_color fill-opacity="0.16" stroke=district_color stroke-width="2" vector-effect="non-scaling-stroke" />
                // 2 m cells
                {cells.iter().map(|&(c, r)| {
                    let (cx, cy) = frame_grid.cell_center(c, r);
                    let (sx, sy) = frame_grid.to_svg(cx - CELL_M / 2.0, cy + CELL_M / 2.0);
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
                // pruning marks
                {move || state.get().pruned.iter().map(|&(c, r)| {
                    let (cx, cy) = frame_dots.cell_center(c, r);
                    let (sx, sy) = frame_dots.to_svg(cx - CELL_M / 2.0, cy + CELL_M / 2.0);
                    view! {
                        <rect
                            x=format!("{sx:.2}")
                            y=format!("{sy:.2}")
                            width=format!("{:.2}", CELL_M * PX_M)
                            height=format!("{:.2}", CELL_M * PX_M)
                            class="land-pruned"
                        />
                    }
                }).collect_view()}
                // figures — the live unit in cyan, the rest grey
                {move || state.get().placed.iter().map(|p| {
                    let fig = figure(&p.fig);
                    let (cols, rows, live, glyph) = fig
                        .map(|f| (f.cols, f.rows, f.live, &f.name[..1]))
                        .unwrap_or((1, 1, false, "?"));
                    let (x0, y0) = frame_placed.cell_center(p.c, p.r);
                    let (sx, sy) = frame_placed.to_svg(x0 - CELL_M / 2.0, y0 - CELL_M / 2.0 + rows as f64 * CELL_M);
                    let wpx = cols as f64 * CELL_M * PX_M;
                    let hpx = rows as f64 * CELL_M * PX_M;
                    let cls = if live { "land-fig live" } else { "land-fig ghost" };
                    view! {
                        <g class=cls>
                            <rect x=format!("{sx:.2}") y=format!("{sy:.2}") width=format!("{wpx:.2}") height=format!("{hpx:.2}") rx="2" />
                            <text x=format!("{:.2}", sx + wpx / 2.0) y=format!("{:.2}", sy + hpx / 2.0 + 4.0) text-anchor="middle">{glyph.to_string()}</text>
                        </g>
                    }
                }).collect_view()}
                // measure marks + the line
                {move || {
                    let m = measure.get();
                    let pts: Vec<(f64, f64)> = m.iter().map(|&(x, y)| frame_measure.to_svg(x, y)).collect();
                    view! {
                        {(pts.len() == 2).then(|| {
                            let mid = ((pts[0].0 + pts[1].0) / 2.0, (pts[0].1 + pts[1].1) / 2.0);
                            let d = dist_m(m[0], m[1]);
                            view! {
                                <line x1=format!("{:.2}", pts[0].0) y1=format!("{:.2}", pts[0].1) x2=format!("{:.2}", pts[1].0) y2=format!("{:.2}", pts[1].1) class="land-measure" />
                                <text x=format!("{:.2}", mid.0) y=format!("{:.2}", mid.1 - 6.0) text-anchor="middle" class="land-measure-label">{format!("{d:.1} m")}</text>
                            }
                        })}
                        {pts.iter().map(|&(x, y)| view! {
                            <circle cx=format!("{x:.2}") cy=format!("{y:.2}") r="3.5" class="land-mark" />
                        }).collect_view()}
                    }
                }}
            </svg>
            <div class="land-readout">
                <span class="land-hint">{move || hint.get()}</span>
                <span class="land-stats">{move || {
                    let st = state.get();
                    let live = st.placed.iter().filter(|p| figure(&p.fig).map(|f| f.live).unwrap_or(false)).count();
                    let ghost = st.placed.len() - live;
                    format!("{} pruned · {live} units · {ghost} ghosts", st.pruned.len())
                }}</span>
            </div>
        </section>
    }
}
