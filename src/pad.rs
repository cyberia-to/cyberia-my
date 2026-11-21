//! The pad: every way to move the camera, each key signed with its letter. It is the one panel that
//! sits over the globe, so it can be put away: the first key collapses the pad to a single chip in the
//! top-left corner, and the choice is kept in the browser under `cyberia.pad_hidden.v1` — the pad opens
//! the way the visitor left it. The task is `tasks/v2-pad-collapses.md`; the shape is `style.css`
//! § the pad.

use crate::globe::{cam_hold, cam_north, cam_world, Cam};
use leptos::prelude::*;

/// The pad's own state, kept in the browser: "1" when the keys are hidden.
const PAD_HIDDEN_KEY: &str = "cyberia.pad_hidden.v1";

fn load_pad_hidden() -> bool {
    web_sys::window()
        .and_then(|w| w.local_storage().ok().flatten())
        .and_then(|ls| ls.get_item(PAD_HIDDEN_KEY).ok().flatten())
        .map(|s| s == "1")
        .unwrap_or(false)
}

fn save_pad_hidden(hidden: bool) {
    if let Some(ls) = web_sys::window().and_then(|w| w.local_storage().ok().flatten()) {
        let _ = ls.set_item(PAD_HIDDEN_KEY, if hidden { "1" } else { "0" });
    }
}

/// The pad over the map: the camera's keys, the needle, the earth, the home and the layers switch,
/// with the toggle that collapses them all to one chip.
#[component]
pub fn NavPad(
    cam: RwSignal<Cam>,
    picked: RwSignal<Option<usize>>,
    layers_open: RwSignal<bool>,
    /// the home key: back to where the console opened, the same move the H key makes
    #[prop(into)]
    on_home: Callback<()>,
) -> impl IntoView {
    let pad_hidden = RwSignal::new(load_pad_hidden());
    view! {
        <div class="nav-pad globe-ui" class:np-hidden=move || pad_hidden.get() aria-label="map navigation">
            <button type="button" class="np-toggle"
                title=move || if pad_hidden.get() { "show the pad" } else { "hide the pad" }
                aria-expanded=move || (!pad_hidden.get()).to_string()
                on:click=move |_| {
                    let h = !pad_hidden.get_untracked();
                    pad_hidden.set(h);
                    save_pad_hidden(h);
                }
            >{move || if pad_hidden.get() { "▸ PAD" } else { "◂" }}</button>
            <button type="button" class="np-key" title="turn the view left · hold Q"
                on:pointerdown=move |_| cam_hold(3, -1.0)
                on:pointerup=move |_| cam_hold(3, 0.0)
                on:pointerleave=move |_| cam_hold(3, 0.0)
                on:pointercancel=move |_| cam_hold(3, 0.0)
            >"⟲"<kbd>"Q"</kbd></button>
            <button type="button" class="np-key" title="up · hold W or ↑ (shift: fast)"
                on:pointerdown=move |_| cam_hold(1, 1.0)
                on:pointerup=move |_| cam_hold(1, 0.0)
                on:pointerleave=move |_| cam_hold(1, 0.0)
                on:pointercancel=move |_| cam_hold(1, 0.0)
            >"↑"<kbd>"W"</kbd></button>
            <button type="button" class="np-key" title="turn the view right · hold E"
                on:pointerdown=move |_| cam_hold(3, 1.0)
                on:pointerup=move |_| cam_hold(3, 0.0)
                on:pointerleave=move |_| cam_hold(3, 0.0)
                on:pointercancel=move |_| cam_hold(3, 0.0)
            >"⟳"<kbd>"E"</kbd></button>
            <button type="button" class="np-key" title="left · hold A or ← (shift: fast)"
                on:pointerdown=move |_| cam_hold(0, 1.0)
                on:pointerup=move |_| cam_hold(0, 0.0)
                on:pointerleave=move |_| cam_hold(0, 0.0)
                on:pointercancel=move |_| cam_hold(0, 0.0)
            >"←"<kbd>"A"</kbd></button>
            <button type="button" class="np-north" title="north up · N" on:click=move |_| cam_north()>
                <span class="np-needle" style=move || format!("transform: rotate({:.3}rad)", cam.get().5)>"▲"</span>
                <kbd>"N"</kbd>
            </button>
            <button type="button" class="np-key" title="right · hold D or → (shift: fast)"
                on:pointerdown=move |_| cam_hold(0, -1.0)
                on:pointerup=move |_| cam_hold(0, 0.0)
                on:pointerleave=move |_| cam_hold(0, 0.0)
                on:pointercancel=move |_| cam_hold(0, 0.0)
            >"→"<kbd>"D"</kbd></button>
            <button type="button" class="np-key" title="zoom out · hold −"
                on:pointerdown=move |_| cam_hold(2, -1.0)
                on:pointerup=move |_| cam_hold(2, 0.0)
                on:pointerleave=move |_| cam_hold(2, 0.0)
                on:pointercancel=move |_| cam_hold(2, 0.0)
            >"−"<kbd>"−"</kbd></button>
            <button type="button" class="np-key" title="down · hold S or ↓ (shift: fast)"
                on:pointerdown=move |_| cam_hold(1, -1.0)
                on:pointerup=move |_| cam_hold(1, 0.0)
                on:pointerleave=move |_| cam_hold(1, 0.0)
                on:pointercancel=move |_| cam_hold(1, 0.0)
            >"↓"<kbd>"S"</kbd></button>
            <button type="button" class="np-key" title="zoom in · hold +"
                on:pointerdown=move |_| cam_hold(2, 1.0)
                on:pointerup=move |_| cam_hold(2, 0.0)
                on:pointerleave=move |_| cam_hold(2, 0.0)
                on:pointercancel=move |_| cam_hold(2, 0.0)
            >"+"<kbd>"+"</kbd></button>
            <button type="button" class="np-wide" title="the whole earth · 0" on:click=move |_| { picked.set(None); cam_world(); }>"◯"<span class="np-label">" EARTH"</span><kbd>"0"</kbd></button>
            <button type="button" class="np-wide2" title="Cyber Valley · H" on:click=move |_| on_home.run(())>"⌂"<kbd>"H"</kbd></button>
            <button type="button" class="np-layers" class:on=move || layers_open.get() title="layers · L" on:click=move |_| layers_open.update(|o| *o = !*o)>"▤"<span class="np-label">" LAYERS"</span><kbd>"L"</kbd></button>
        </div>
    }
}
