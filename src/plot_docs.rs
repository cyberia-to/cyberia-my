//! Documents of a flat — upload to this browser, download the plot's papers.
//!
//! Uploads are kept as data URLs in localStorage (≤ 1 MB each, this is a
//! phase-0 surface, not an archive). The plot passport and the KML are
//! generated from the land data on the fly, so they are always current.

use crate::deals::{fmt_ts, now_ms};
use leptos::prelude::*;
use serde::{Deserialize, Serialize};
use wasm_bindgen::closure::Closure;
use wasm_bindgen::JsCast;

const MAX_BYTES: f64 = 1_048_576.0;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Doc {
    pub name: String,
    pub size: f64,
    pub mime: String,
    pub ts_ms: f64,
    pub data_url: String,
}

fn docs_key(plot_id: &str) -> String {
    format!("cyberia.plot.{plot_id}.docs.v1")
}

pub fn load_docs(plot_id: &str) -> Vec<Doc> {
    web_sys::window()
        .and_then(|w| w.local_storage().ok().flatten())
        .and_then(|ls| ls.get_item(&docs_key(plot_id)).ok().flatten())
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

pub fn save_docs(plot_id: &str, docs: &[Doc]) -> bool {
    let Some(ls) = web_sys::window().and_then(|w| w.local_storage().ok().flatten()) else {
        return false;
    };
    let Ok(s) = serde_json::to_string(docs) else {
        return false;
    };
    ls.set_item(&docs_key(plot_id), &s).is_ok()
}

pub fn text_data_url(mime: &str, body: &str) -> String {
    let enc = js_sys::encode_uri_component(body);
    format!("data:{mime};charset=utf-8,{}", String::from(enc))
}

pub fn fmt_size(b: f64) -> String {
    if b >= 1_048_576.0 {
        format!("{:.1} MB", b / 1_048_576.0)
    } else if b >= 1024.0 {
        format!("{:.0} KB", b / 1024.0)
    } else {
        format!("{b:.0} B")
    }
}

/// `passport` / `kml` are the flat's generated papers; `slug` names the files.
#[component]
pub fn DocsPanel(
    plot_id: String,
    slug: String,
    passport: String,
    kml: String,
) -> impl IntoView {
    let docs = RwSignal::new(load_docs(&plot_id));
    let note = RwSignal::new(String::new());
    let pid_up = plot_id.clone();
    let pid_rm = plot_id.clone();

    let on_pick = move |ev: leptos::ev::Event| {
        let Some(input) = ev
            .target()
            .and_then(|t| t.dyn_into::<web_sys::HtmlInputElement>().ok())
        else {
            return;
        };
        let Some(files) = input.files() else { return };
        let Some(file) = files.get(0) else { return };
        if file.size() > MAX_BYTES {
            note.set(format!(
                "{} is {} — the phase-0 shelf takes files up to 1 MB",
                file.name(),
                fmt_size(file.size())
            ));
            input.set_value("");
            return;
        }
        let Ok(reader) = web_sys::FileReader::new() else { return };
        let name = file.name();
        let size = file.size();
        let mime = file.type_();
        let pid = pid_up.clone();
        let reader2 = reader.clone();
        let onload = Closure::once(move |_ev: web_sys::Event| {
            let Some(url) = reader2.result().ok().and_then(|v| v.as_string()) else {
                note.set("could not read the file".into());
                return;
            };
            docs.update(|v| {
                v.insert(
                    0,
                    Doc {
                        name: name.clone(),
                        size,
                        mime,
                        ts_ms: now_ms(),
                        data_url: url,
                    },
                );
                if !save_docs(&pid, v) {
                    v.remove(0);
                    note.set("browser storage is full — remove a document first".into());
                } else {
                    note.set(format!("{name} is on the shelf"));
                }
            });
        });
        reader.set_onload(Some(onload.as_ref().unchecked_ref()));
        onload.forget();
        let _ = reader.read_as_data_url(&file);
        input.set_value("");
    };

    let passport_url = text_data_url("application/json", &passport);
    let kml_url = text_data_url("application/vnd.google-earth.kml+xml", &kml);
    let passport_name = format!("{slug}-passport.json");
    let kml_name = format!("{slug}.kml");

    view! {
        <section class="plot-card">
            <div class="cyberia-panel-h">
                <span class="panel-kicker">"DOCUMENTS"</span>
                <span class="panel-sub">"the flat's papers · this browser"</span>
            </div>
            <div class="plot-docs">
                <div class="docs-row docs-generated">
                    <a class="doc-dl" href=passport_url download=passport_name>
                        <span class="doc-name">"PLOT PASSPORT"</span>
                        <span class="doc-meta">"json · geometry, status, deal history"</span>
                    </a>
                    <a class="doc-dl" href=kml_url download=kml_name>
                        <span class="doc-name">"BOUNDARY KML"</span>
                        <span class="doc-meta">"opens in Google Earth / My Maps"</span>
                    </a>
                </div>
                <label class="doc-upload">
                    <input type="file" on:change=on_pick />
                    <span class="doc-name">"UPLOAD A DOCUMENT"</span>
                    <span class="doc-meta">"agreement, survey, receipt · up to 1 MB"</span>
                </label>
                <p class="docs-note">{move || note.get()}</p>
                {move || {
                    let list = docs.get();
                    if list.is_empty() {
                        return view! { <p class="docs-empty">"nothing uploaded yet"</p> }.into_any();
                    }
                    let pid = pid_rm.clone();
                    view! {
                        <div class="docs-list">
                            {list.into_iter().enumerate().map(|(i, d)| {
                                let pid = pid.clone();
                                let href = d.data_url.clone();
                                let dl = d.name.clone();
                                view! {
                                    <div class="docs-row">
                                        <a class="doc-dl" href=href download=dl>
                                            <span class="doc-name">{d.name.clone()}</span>
                                            <span class="doc-meta">
                                                {format!("{} · {} · {}", fmt_size(d.size), if d.mime.is_empty() { "file".to_string() } else { d.mime.clone() }, fmt_ts(d.ts_ms))}
                                            </span>
                                        </a>
                                        <button
                                            type="button"
                                            class="doc-x"
                                            title="remove from the shelf"
                                            on:click=move |_| {
                                                docs.update(|v| {
                                                    if i < v.len() {
                                                        v.remove(i);
                                                    }
                                                    save_docs(&pid, v);
                                                });
                                            }
                                        >"✕"</button>
                                    </div>
                                }
                            }).collect_view()}
                        </div>
                    }.into_any()
                }}
            </div>
        </section>
    }
}
