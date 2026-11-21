//! The map legend: every mark tells what it means. Point at a mark (tap it on a phone, tab to it from the
//! keyboard) and the legend reads it out on its second line, which holds nothing else, while the map lights
//! what wears the mark and dims the rest. Nothing floats over the map. The words come from where the marks are made:
//! land uses from `terms::LandUse::line`, the deal marks from `deals::flat_status`, the lines from the
//! trail sync in `scripts/sync-plot-terms.py`.

use crate::deals::FlatStatus;
use crate::terms::LandUse;
use leptos::prelude::*;
use wasm_bindgen::JsCast;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mark {
    Use(LandUse),
    Owned,
    Transfer,
    Road,
    Path,
}

impl Mark {
    pub fn name(self) -> &'static str {
        match self {
            Mark::Use(u) => u.short(),
            Mark::Owned => "OWNED",
            Mark::Transfer => "IN TRANSFER",
            Mark::Road => "ROAD",
            Mark::Path => "PATH",
        }
    }

    pub fn meaning(self) -> &'static str {
        match self {
            Mark::Use(u) => u.line(),
            // flat_status: a signed deal
            Mark::Owned => "held · its lease is signed",
            // flat_status: the valley's reserved and transfer deals, a request opened in this browser, and a
            // flat the lease book names a holder for
            Mark::Transfer => "a deal is open on this flat · reserved or rights in transfer",
            Mark::Road => "a road · concrete or gravel, 5 m wide, 3.5 m for small vehicles",
            Mark::Path => "a walking path · 2–3 m wide",
        }
    }

    /// the colour the readout names the mark in
    pub fn color(self) -> &'static str {
        match self {
            Mark::Use(u) => u.color(),
            _ => "#ffffff",
        }
    }

    /// does a valley plot of this land use and status wear the mark
    pub fn lights_plot(self, u: LandUse, s: FlatStatus) -> bool {
        match self {
            Mark::Use(m) => m == u,
            Mark::Owned => s == FlatStatus::Owned,
            Mark::Transfer => s == FlatStatus::Transfer,
            Mark::Road | Mark::Path => false,
        }
    }

    /// friends' sales wear the blue of ANY
    pub fn lights_friend_sale(self) -> bool {
        self == Mark::Use(LandUse::Hgb)
    }

    /// a road or a path dims only while the other kind of line is read
    pub fn dims_trail(self, kind: &str) -> bool {
        match self {
            Mark::Road => kind != "road",
            Mark::Path => kind != "path",
            _ => false,
        }
    }
}

/// One mark of the legend. `class` is its look (`swatch`, `hatch`, `dash`, `line`, `line thin`), `sw` its colour.
#[component]
pub fn LegendMark(
    mark: Mark,
    on: RwSignal<Option<Mark>>,
    class: &'static str,
    #[prop(default = "#ffffff")] sw: &'static str,
) -> impl IntoView {
    // a mouse reads by hovering; a finger or a pen taps the mark on and off. Whether it was on is taken at the
    // touch: focus and blur fire between the touch and the click and must not decide the toggle
    let tapped = StoredValue::new(false);
    let was_on = StoredValue::new(false);
    let off = move || on.update(|m| if *m == Some(mark) { *m = None });
    view! {
        <span
            class=move || format!("leg {class} mark{}", if on.get() == Some(mark) { " is-on" } else { "" })
            style:--sw=sw
            tabindex="0"
            role="button"
            aria-label=format!("{}: {}", mark.name(), mark.meaning())
            on:pointerdown=move |ev: web_sys::PointerEvent| {
                tapped.set_value(ev.pointer_type() != "mouse");
                was_on.set_value(on.get_untracked() == Some(mark));
            }
            on:pointerenter=move |ev: web_sys::PointerEvent| {
                if ev.pointer_type() == "mouse" {
                    on.set(Some(mark));
                }
            }
            on:pointerleave=move |ev: web_sys::PointerEvent| {
                if ev.pointer_type() == "mouse" {
                    off();
                }
            }
            on:focus=move |ev| {
                // the keyboard reads on focus; a tap's focus is left to the tap
                let keys = ev
                    .target()
                    .and_then(|t| t.dyn_into::<web_sys::Element>().ok())
                    .map(|e| e.matches(":focus-visible").unwrap_or(false))
                    .unwrap_or(false);
                if keys {
                    on.set(Some(mark));
                }
            }
            on:blur=move |_| off()
            on:click=move |_| {
                if tapped.get_value() {
                    on.set(if was_on.get_value() { None } else { Some(mark) });
                }
            }
        >
            {mark.name()}
        </span>
    }
}

/// What the mark in hand means, alone on the legend's second line.
#[component]
pub fn LegendReadout(on: RwSignal<Option<Mark>>) -> impl IntoView {
    move || {
        on.get().map(|m| {
            view! {
                <span class="leg-read" aria-live="polite">
                    <b style:color=m.color()>{m.name()}</b>
                    " · "
                    {m.meaning()}
                </span>
            }
        })
    }
}
