//! Full-page ERP forms — URLs only, no overlay sheets.
//! Cyberlink is the core create path (draft = intent).

use crate::economy::fmt_qty;
use crate::erp::{
    commit_cyberlink, commit_intent, create_card, create_or_add_coin, create_schedule,
    create_template, cyberlinks_touching, delete_card, delete_cyberlink, delete_intent,
    delete_schedule, delete_template, draft_cyberlink, edit_card, format_io_blob, get_cyberlink,
    get_template, load_cards, load_cyberlinks, load_erp_intents, load_schedules, load_templates,
    run_template, set_coin_qty, set_intent_state, tick_schedules, update_cyberlink,
    update_schedule, update_template, UserTemplate,
};
use crate::wallet::{load_stocks, stock_qty, StockLine};
use crate::world::StudioShell;
use leptos::prelude::*;
use leptos_router::hooks::use_params_map;

fn param(key: &str) -> String {
    use_params_map()
        .get()
        .get(key)
        .map(|s| s.to_string())
        .unwrap_or_default()
}

// ─── CYBERLINKS ───────────────────────────────────────────────────────

#[component]
pub fn LinksListPage() -> impl IntoView {
    let tick = RwSignal::new(0u32);
    let filter = RwSignal::new(String::new()); // draft | linked | all
    Effect::new(move |_| {
        tick.set(1);
    });
    view! {
        <StudioShell title="Cyberlinks" kicker="GRAPH" lead="Everything is a cyberlink. Draft = intent. Commit = write into the graph.">
            <div class="studio-create-bar">
                <a class="cta-btn cta-lease cta-lg" href="/world/links/new" style="text-decoration:none;">
                    <span class="cta-copy"><span class="cta-title">"+ CYBERLINK"</span><span class="cta-sub">"draft intent first"</span></span>
                </a>
            </div>
            <div class="list-filters" style="margin-bottom:12px;">
                <button class=move || if filter.get().is_empty() { "chip chip-on" } else { "chip" } on:click=move |_| filter.set(String::new())>"ALL"</button>
                <button class=move || if filter.get() == "draft" { "chip chip-on" } else { "chip" } on:click=move |_| filter.set("draft".into())>"DRAFT (INTENTS)"</button>
                <button class=move || if filter.get() == "linked" { "chip chip-on" } else { "chip" } on:click=move |_| filter.set("linked".into())>"LINKED"</button>
            </div>
            <div class="studio-list">
                {move || {
                    let _ = tick.get();
                    let f = filter.get();
                    let mut list = load_cyberlinks();
                    if !f.is_empty() {
                        list.retain(|c| c.state == f);
                    }
                    if list.is_empty() {
                        return view! { <div class="me-empty">"No links in this filter. "<a href="/world/links/new">"Create one"</a></div> }.into_any();
                    }
                    view! {
                        {list.into_iter().map(|cl| {
                            let href = format!("/world/link/{}", cl.id);
                            view! {
                                <a class="studio-row studio-row-link" href=href>
                                    <div class="studio-row-main">
                                        <span class=if cl.state == "linked" { "studio-kind" } else { "studio-kind draft-kind" }>{cl.state.to_uppercase()}</span>
                                        <div>
                                            <div class="studio-title">{format!("{} —[{}]→ {}", cl.from, cl.rel, cl.to)}</div>
                                            <div class="studio-meta">{format!("{} · weight {} · {}", cl.id, cl.weight, cl.note)}</div>
                                        </div>
                                    </div>
                                    <span class="chip">"OPEN →"</span>
                                </a>
                            }
                        }).collect_view()}
                    }.into_any()
                }}
            </div>
        </StudioShell>
    }
}

#[component]
pub fn LinkNewPage() -> impl IntoView {
    let from = RwSignal::new(String::new());
    let to = RwSignal::new(String::new());
    let rel = RwSignal::new("knows".to_string());
    let note = RwSignal::new(String::new());
    let tpl = RwSignal::new(String::new());
    let err = RwSignal::new(None::<String>);
    let ok = RwSignal::new(None::<String>);

    // particle pick helpers
    let particles = move || {
        let mut p: Vec<String> = load_cards().into_iter().map(|c| c.id).collect();
        p.extend(load_templates().into_iter().map(|t| t.id));
        p.extend(load_stocks().into_iter().map(|s| s.id));
        p.sort();
        p.dedup();
        p
    };

    view! {
        <StudioShell title="New cyberlink" kicker="DRAFT INTENT" lead="Creating a cyberlink always starts as a draft intent. Commit on the next page to finalize.">
            <div class="studio-form-page">
                {move || err.get().map(|e| view! { <div class="eco-msg err">{e}</div> })}
                {move || ok.get().map(|s| view! { <div class="eco-msg ok">{s}</div> })}

                <label class="found-label">"FROM PARTICLE"</label>
                <input class="found-input" type="text" prop:value=move || from.get()
                    on:input=move |ev| from.set(event_target_value(&ev))
                    placeholder="card id · coin · template · free particle / CID" />
                <div class="list-filters" style="margin:8px 0;">
                    {move || particles().into_iter().take(24).map(|p| {
                        let p2 = p.clone();
                        view! {
                            <button class="chip" on:click=move |_| from.set(p2.clone())>{p}</button>
                        }
                    }).collect_view()}
                </div>

                <label class="found-label">"RELATION (predicate)"</label>
                <input class="found-input" type="text" prop:value=move || rel.get()
                    on:input=move |ev| rel.set(event_target_value(&ev))
                    placeholder="knows · owns · located_in · ran · mints · intends" />
                <div class="list-filters" style="margin:8px 0;">
                    {["knows","owns","located_in","works_on","ran","mints","burns","intends","schedules"].into_iter().map(|r| {
                        view! { <button class="chip" on:click=move |_| rel.set(r.into())>{r}</button> }
                    }).collect_view()}
                </div>

                <label class="found-label">"TO PARTICLE"</label>
                <input class="found-input" type="text" prop:value=move || to.get()
                    on:input=move |ev| to.set(event_target_value(&ev))
                    placeholder="particle id" />
                <div class="list-filters" style="margin:8px 0;">
                    {move || particles().into_iter().take(24).map(|p| {
                        let p2 = p.clone();
                        view! {
                            <button class="chip" on:click=move |_| to.set(p2.clone())>{p}</button>
                        }
                    }).collect_view()}
                </div>

                <label class="found-label">"TEMPLATE ID (optional · for ops intents)"</label>
                <input class="found-input" type="text" prop:value=move || tpl.get()
                    on:input=move |ev| tpl.set(event_target_value(&ev)) />

                <label class="found-label">"NOTE"</label>
                <input class="found-input" type="text" prop:value=move || note.get()
                    on:input=move |ev| note.set(event_target_value(&ev)) />

                <div class="studio-form-actions">
                    <button class="intent-commit" on:click=move |_| {
                        let template = {
                            let t = tpl.get().trim().to_string();
                            if t.is_empty() { None } else { Some(t) }
                        };
                        match draft_cyberlink(&from.get(), &to.get(), &rel.get(), &note.get(), template) {
                            Ok(cl) => {
                                ok.set(Some(format!("draft {} — open to COMMIT", cl.id)));
                                err.set(None);
                                // navigate via location
                                if let Some(w) = web_sys::window() {
                                    let _ = w.location().set_href(&format!("/world/link/{}", cl.id));
                                }
                            }
                            Err(e) => { err.set(Some(e)); ok.set(None); }
                        }
                    }>"CREATE DRAFT (INTENT)"</button>
                    <a class="chip" href="/world/links">"← CANCEL"</a>
                </div>
            </div>
        </StudioShell>
    }
}

#[component]
pub fn LinkViewPage() -> impl IntoView {
    let id = param("id");
    let id_for_get = id.clone();
    let tick = RwSignal::new(0u32);
    let msg = RwSignal::new(None::<(bool, String)>);

    let cl = move || {
        let _ = tick.get();
        get_cyberlink(&id_for_get)
    };

    view! {
        <StudioShell title="Cyberlink" kicker="LINK" lead="Draft = intent. Commit finalizes the cyberlink in the local cybergraph.">
            {move || msg.get().map(|(ok, t)| view! { <div class=if ok { "eco-msg ok" } else { "eco-msg err" }>{t}</div> })}
            {move || match cl() {
                None => view! {
                    <div class="me-empty">"Link not found. "<a href="/world/links">"← list"</a></div>
                }.into_any(),
                Some(c) => {
                    let cid = c.id.clone();
                    let cid2 = c.id.clone();
                    let cid3 = c.id.clone();
                    let is_draft = c.state == "draft";
                    view! {
                        <div class="studio-detail">
                            <div class="studio-kind-lg">{c.state.to_uppercase()}</div>
                            <h3 class="studio-title-lg">{format!("{} —[{}]→ {}", c.from, c.rel, c.to)}</h3>
                            <div class="studio-meta">{format!("id {} · owner {} · weight {}", c.id, c.owner, c.weight)}</div>
                            <div class="studio-meta">{c.note.clone()}</div>
                            {c.template_id.as_ref().map(|t| view! {
                                <div class="studio-meta">
                                    "template: "
                                    <a href=format!("/world/template/{t}")>{t.clone()}</a>
                                </div>
                            })}
                            <div class="studio-form-actions">
                                {is_draft.then(|| view! {
                                    <button class="intent-commit" on:click=move |_| {
                                        match commit_cyberlink(&cid) {
                                            Ok(_) => { msg.set(Some((true, "committed · linked".into()))); tick.update(|n| { *n += 1; }); }
                                            Err(e) => msg.set(Some((false, e))),
                                        }
                                    }>"COMMIT → LINKED"</button>
                                })}
                                <a class="chip" href=format!("/world/link/{}/edit", cid2)>"EDIT"</a>
                                <button class="chip" on:click=move |_| {
                                    match delete_cyberlink(&cid3) {
                                        Ok(()) => {
                                            if let Some(w) = web_sys::window() {
                                                let _ = w.location().set_href("/world/links");
                                            }
                                        }
                                        Err(e) => msg.set(Some((false, e))),
                                    }
                                }>"DELETE"</button>
                                <a class="chip" href="/world/links">"← LIST"</a>
                            </div>
                        </div>
                    }.into_any()
                }
            }}
        </StudioShell>
    }
}

#[component]
pub fn LinkEditPage() -> impl IntoView {
    let id = param("id");
    let id_for_effect = id.clone();
    let id_save = id.clone();
    let cancel_href = format!("/world/link/{id}");
    let from = RwSignal::new(String::new());
    let to = RwSignal::new(String::new());
    let rel = RwSignal::new(String::new());
    let note = RwSignal::new(String::new());
    let weight = RwSignal::new("1".to_string());
    let err = RwSignal::new(None::<String>);

    Effect::new(move |_| {
        if let Some(c) = get_cyberlink(&id_for_effect) {
            from.set(c.from);
            to.set(c.to);
            rel.set(c.rel);
            note.set(c.note);
            weight.set(c.weight.to_string());
        }
    });

    view! {
        <StudioShell title="Edit cyberlink" kicker="EDIT LINK" lead="">
            {move || err.get().map(|e| view!{ <div class="eco-msg err">{e}</div>})}
            <div class="studio-form-page">
                <label class="found-label">"FROM"</label>
                <input class="found-input" prop:value=move||from.get() on:input=move|ev| from.set(event_target_value(&ev)) />
                <label class="found-label">"REL"</label>
                <input class="found-input" prop:value=move||rel.get() on:input=move|ev| rel.set(event_target_value(&ev)) />
                <label class="found-label">"TO"</label>
                <input class="found-input" prop:value=move||to.get() on:input=move|ev| to.set(event_target_value(&ev)) />
                <label class="found-label">"NOTE"</label>
                <input class="found-input" prop:value=move||note.get() on:input=move|ev| note.set(event_target_value(&ev)) />
                <label class="found-label">"WEIGHT"</label>
                <input class="found-input" prop:value=move||weight.get() on:input=move|ev| weight.set(event_target_value(&ev)) />
                <div class="studio-form-actions">
                    <button class="intent-commit" on:click=move |_| {
                        let w: f64 = weight.get().parse().unwrap_or(1.0);
                        let id_nav = id_save.clone();
                        match update_cyberlink(&id_save, &from.get(), &to.get(), &rel.get(), &note.get(), w) {
                            Ok(()) => {
                                if let Some(win) = web_sys::window() {
                                    let _ = win.location().set_href(&format!("/world/link/{id_nav}"));
                                }
                            }
                            Err(e) => err.set(Some(e)),
                        }
                    }>"SAVE"</button>
                    <a class="chip" href=cancel_href>"← CANCEL"</a>
                </div>
            </div>
        </StudioShell>
    }
}

// ─── CARDS ────────────────────────────────────────────────────────────

#[component]
pub fn CardsListPage() -> impl IntoView {
    view! {
        <StudioShell title="Cards" kicker="PARTICLES · TSP-2" lead="Named entities in the graph. Link them with cyberlinks.">
            <div class="studio-create-bar">
                <a class="cta-btn cta-lease cta-lg" href="/world/cards/new" style="text-decoration:none;">
                    <span class="cta-copy"><span class="cta-title">"+ CARD"</span><span class="cta-sub">"new particle"</span></span>
                </a>
            </div>
            <div class="studio-list">
                {move || {
                    let cards = load_cards();
                    if cards.is_empty() {
                        return view!{ <div class="me-empty">"No cards."</div> }.into_any();
                    }
                    view! {
                        {cards.into_iter().map(|c| {
                            let href = format!("/world/card/{}", c.id);
                            let n = cyberlinks_touching(&c.id).len();
                            view! {
                                <a class="studio-row studio-row-link" href=href>
                                    <div class="studio-row-main">
                                        <span class="studio-kind">{c.kind.to_uppercase()}</span>
                                        <div>
                                            <div class="studio-title">{c.name.clone()}</div>
                                            <div class="studio-meta">{format!("{} · {} cyberlinks · parent {}", c.id, n, c.parent.clone().unwrap_or_else(||"—".into()))}</div>
                                        </div>
                                    </div>
                                    <span class="chip">"OPEN →"</span>
                                </a>
                            }
                        }).collect_view()}
                    }.into_any()
                }}
            </div>
        </StudioShell>
    }
}

#[component]
pub fn CardNewPage() -> impl IntoView {
    let kind = RwSignal::new("plot".to_string());
    let name = RwSignal::new(String::new());
    let class = RwSignal::new(String::new());
    let zone = RwSignal::new(String::new());
    let parent = RwSignal::new("city-cyber-valley".to_string());
    let note = RwSignal::new(String::new());
    let err = RwSignal::new(None::<String>);
    view! {
        <StudioShell title="New card" kicker="CREATE PARTICLE" lead="TSP-2 entity. After create, link it with cyberlinks.">
            <div class="studio-form-page">
                {move || err.get().map(|e| view!{<div class="eco-msg err">{e}</div>})}
                <label class="found-label">"KIND"</label>
                <select class="found-input" prop:value=move||kind.get() on:change=move|ev| kind.set(event_target_value(&ev))>
                    <option value="plot">"plot"</option>
                    <option value="building">"building"</option>
                    <option value="place">"place"</option>
                    <option value="city">"city"</option>
                    <option value="person">"person"</option>
                    <option value="project">"project"</option>
                    <option value="asset">"asset"</option>
                </select>
                <label class="found-label">"NAME"</label>
                <input class="found-input" prop:value=move||name.get() on:input=move|ev| name.set(event_target_value(&ev)) />
                <label class="found-label">"CLASS"</label>
                <input class="found-input" prop:value=move||class.get() on:input=move|ev| class.set(event_target_value(&ev)) />
                <label class="found-label">"ZONE"</label>
                <input class="found-input" prop:value=move||zone.get() on:input=move|ev| zone.set(event_target_value(&ev)) />
                <label class="found-label">"PARENT ID"</label>
                <input class="found-input" prop:value=move||parent.get() on:input=move|ev| parent.set(event_target_value(&ev)) />
                <label class="found-label">"NOTE"</label>
                <input class="found-input" prop:value=move||note.get() on:input=move|ev| note.set(event_target_value(&ev)) />
                <div class="studio-form-actions">
                    <button class="intent-commit" on:click=move |_| {
                        let p = { let x=parent.get().trim().to_string(); if x.is_empty(){None}else{Some(x)} };
                        match create_card(&kind.get(), &name.get(), &class.get(), p, &zone.get(), &note.get()) {
                            Ok(c) => {
                                if let Some(w)=web_sys::window(){ let _=w.location().set_href(&format!("/world/card/{}", c.id)); }
                            }
                            Err(e) => err.set(Some(e)),
                        }
                    }>"CREATE CARD"</button>
                    <a class="chip" href="/world/cards">"← CANCEL"</a>
                </div>
            </div>
        </StudioShell>
    }
}

#[component]
pub fn CardViewPage() -> impl IntoView {
    let id = param("id");
    let id_g = id.clone();
    let tick = RwSignal::new(0u32);
    view! {
        <StudioShell title="Card" kicker="PARTICLE" lead="">
            {move || {
                let _ = tick.get();
                let card = load_cards().into_iter().find(|c| c.id == id_g);
                match card {
                    None => view!{ <div class="me-empty">"Not found"</div> }.into_any(),
                    Some(c) => {
                        let links = cyberlinks_touching(&c.id);
                        let cid = c.id.clone();
                        let cid2 = c.id.clone();
                        view! {
                            <div class="studio-detail">
                                <div class="studio-kind-lg">{c.kind.to_uppercase()}</div>
                                <h3 class="studio-title-lg">{c.name.clone()}</h3>
                                <div class="studio-meta">{format!("{} · class {} · owner {}", c.id, c.class, c.owner)}</div>
                                <div class="studio-meta">{format!("parent {} · zone {} · {}", c.parent.clone().unwrap_or_else(||"—".into()), c.zone, c.note)}</div>
                                <div class="studio-form-actions">
                                    <a class="chip chip-on" href=format!("/world/card/{}/edit", cid)>"EDIT"</a>
                                    <a class="chip" href=format!("/world/links/new")>"+ LINK FROM HERE"</a>
                                    <button class="chip" on:click=move |_| {
                                        match delete_card(&cid2) {
                                            Ok(()) => { if let Some(w)=web_sys::window(){ let _=w.location().set_href("/world/cards"); } }
                                            Err(_) => {}
                                        }
                                    }>"DELETE"</button>
                                    <a class="chip" href="/world/cards">"← LIST"</a>
                                </div>
                                <div class="studio-section-h" style="margin-top:20px;"><span>"CYBERLINKS TOUCHING THIS PARTICLE"</span></div>
                                <div class="studio-list">
                                    {if links.is_empty() {
                                        view!{ <div class="me-empty">"No links yet."</div> }.into_any()
                                    } else {
                                        links.into_iter().map(|cl| {
                                            let href = format!("/world/link/{}", cl.id);
                                            view! {
                                                <a class="studio-row studio-row-link" href=href>
                                                    <div class="studio-row-main">
                                                        <span class="studio-kind">{cl.state.to_uppercase()}</span>
                                                        <div class="studio-title">{format!("{} —[{}]→ {}", cl.from, cl.rel, cl.to)}</div>
                                                    </div>
                                                </a>
                                            }
                                        }).collect_view().into_any()
                                    }}
                                </div>
                            </div>
                        }.into_any()
                    }
                }
            }}
        </StudioShell>
    }
}

#[component]
pub fn CardEditPage() -> impl IntoView {
    let id = param("id");
    let id_e = id.clone();
    let id_save = id.clone();
    let cancel_href = format!("/world/card/{id}");
    let name = RwSignal::new(String::new());
    let class = RwSignal::new(String::new());
    let zone = RwSignal::new(String::new());
    let parent = RwSignal::new(String::new());
    let note = RwSignal::new(String::new());
    let err = RwSignal::new(None::<String>);
    Effect::new(move |_| {
        if let Some(c) = load_cards().into_iter().find(|c| c.id == id_e) {
            name.set(c.name);
            class.set(c.class);
            zone.set(c.zone);
            parent.set(c.parent.unwrap_or_default());
            note.set(c.note);
        }
    });
    view! {
        <StudioShell title="Edit card" kicker="EDIT PARTICLE" lead="">
            <div class="studio-form-page">
                {move || err.get().map(|e| view!{<div class="eco-msg err">{e}</div>})}
                <label class="found-label">"NAME"</label>
                <input class="found-input" prop:value=move||name.get() on:input=move|ev| name.set(event_target_value(&ev)) />
                <label class="found-label">"CLASS"</label>
                <input class="found-input" prop:value=move||class.get() on:input=move|ev| class.set(event_target_value(&ev)) />
                <label class="found-label">"ZONE"</label>
                <input class="found-input" prop:value=move||zone.get() on:input=move|ev| zone.set(event_target_value(&ev)) />
                <label class="found-label">"PARENT"</label>
                <input class="found-input" prop:value=move||parent.get() on:input=move|ev| parent.set(event_target_value(&ev)) />
                <label class="found-label">"NOTE"</label>
                <input class="found-input" prop:value=move||note.get() on:input=move|ev| note.set(event_target_value(&ev)) />
                <div class="studio-form-actions">
                    <button class="intent-commit" on:click=move |_| {
                        let p = { let x=parent.get().trim().to_string(); if x.is_empty(){None}else{Some(x)} };
                        let id_nav = id_save.clone();
                        match edit_card(&id_save, &name.get(), &class.get(), &zone.get(), &note.get(), p) {
                            Ok(()) => { if let Some(w)=web_sys::window(){ let _=w.location().set_href(&format!("/world/card/{id_nav}")); } }
                            Err(e) => err.set(Some(e)),
                        }
                    }>"SAVE"</button>
                    <a class="chip" href=cancel_href>"← CANCEL"</a>
                </div>
            </div>
        </StudioShell>
    }
}

// ─── COINS ────────────────────────────────────────────────────────────

#[component]
pub fn CoinsListPage() -> impl IntoView {
    view! {
        <StudioShell title="Coins" kicker="TSP-1 STOCKS" lead="Fungible balances. Linked by cyberlinks (burns/mints from templates).">
            <div class="studio-create-bar">
                <a class="cta-btn cta-buy cta-lg" href="/world/coins/new" style="text-decoration:none;">
                    <span class="cta-copy"><span class="cta-title">"+ COIN"</span><span class="cta-sub">"mint qty"</span></span>
                </a>
                <a class="chip" href="/market">"MARKET →"</a>
            </div>
            <div class="studio-list">
                {move || {
                    let stocks = load_stocks();
                    if stocks.is_empty() {
                        return view!{ <div class="me-empty">"Empty. "<a href="/world/coins/new">"Mint"</a></div> }.into_any();
                    }
                    view! {
                        {stocks.into_iter().map(|s: StockLine| {
                            let href = format!("/world/coin/{}", s.id);
                            view! {
                                <a class="studio-row studio-row-link" href=href>
                                    <div class="studio-row-main">
                                        <span class="studio-kind">"COIN"</span>
                                        <div>
                                            <div class="studio-title">{s.id.to_uppercase()}</div>
                                            <div class="studio-meta">{format!("qty {}", fmt_qty(s.qty))}</div>
                                        </div>
                                    </div>
                                    <span class="chip">"EDIT →"</span>
                                </a>
                            }
                        }).collect_view()}
                    }.into_any()
                }}
            </div>
        </StudioShell>
    }
}

#[component]
pub fn CoinNewPage() -> impl IntoView {
    let id = RwSignal::new("energy".to_string());
    let qty = RwSignal::new("10".to_string());
    let err = RwSignal::new(None::<String>);
    view! {
        <StudioShell title="Mint coin" kicker="TSP-1" lead="">
            <div class="studio-form-page">
                {move || err.get().map(|e| view!{<div class="eco-msg err">{e}</div>})}
                <label class="found-label">"COIN ID"</label>
                <input class="found-input" prop:value=move||id.get() on:input=move|ev| id.set(event_target_value(&ev)) />
                <label class="found-label">"QTY (+)"</label>
                <input class="found-input" prop:value=move||qty.get() on:input=move|ev| qty.set(event_target_value(&ev)) />
                <div class="studio-form-actions">
                    <button class="intent-commit" on:click=move |_| {
                        let q: f64 = qty.get().parse().unwrap_or(0.0);
                        match create_or_add_coin(&id.get(), q) {
                            Ok(_) => { if let Some(w)=web_sys::window(){ let _=w.location().set_href(&format!("/world/coin/{}", id.get())); } }
                            Err(e) => err.set(Some(e)),
                        }
                    }>"MINT"</button>
                    <a class="chip" href="/world/coins">"← CANCEL"</a>
                </div>
            </div>
        </StudioShell>
    }
}

#[component]
pub fn CoinEditPage() -> impl IntoView {
    let id = param("id");
    let id_e = id.clone();
    let id_show = id.clone();
    let id_btn = id.clone();
    let qty = RwSignal::new(String::new());
    let err = RwSignal::new(None::<String>);
    Effect::new(move |_| {
        qty.set(fmt_qty(stock_qty(&id_e)));
    });
    view! {
        <StudioShell title=format!("Coin · {id_show}") kicker="EDIT QTY" lead="">
            <div class="studio-form-page">
                {move || err.get().map(|e| view!{<div class="eco-msg err">{e}</div>})}
                <div class="studio-meta">{format!("particle id: {id_show}")}</div>
                <label class="found-label">"SET QTY"</label>
                <input class="found-input" prop:value=move||qty.get() on:input=move|ev| qty.set(event_target_value(&ev)) />
                <div class="studio-form-actions">
                    <button class="intent-commit" on:click=move |_| {
                        let q: f64 = qty.get().parse().unwrap_or(0.0);
                        match set_coin_qty(&id_btn, q) {
                            Ok(_) => { if let Some(w)=web_sys::window(){ let _=w.location().set_href("/world/coins"); } }
                            Err(e) => err.set(Some(e)),
                        }
                    }>"SAVE"</button>
                    <a class="chip" href="/world/coins">"← LIST"</a>
                    <a class="chip" href="/world/links/new">"+ CYBERLINK"</a>
                </div>
            </div>
        </StudioShell>
    }
}

// ─── TEMPLATES ────────────────────────────────────────────────────────

#[component]
pub fn TemplatesListPage() -> impl IntoView {
    view! {
        <StudioShell title="Templates" kicker="RECIPES" lead="Declared burns → mints. RUN writes cyberlinks (ran / mints / located_in).">
            <div class="studio-create-bar">
                <a class="cta-btn cta-found cta-lg" href="/world/templates/new" style="text-decoration:none;">
                    <span class="cta-copy"><span class="cta-title">"+ TEMPLATE"</span><span class="cta-sub">"new recipe"</span></span>
                </a>
            </div>
            <div class="studio-list">
                {move || {
                    let list = load_templates();
                    view! {
                        {list.into_iter().map(|t: UserTemplate| {
                            let href = format!("/world/template/{}", t.id);
                            view! {
                                <a class="studio-row studio-row-link" href=href>
                                    <div class="studio-row-main">
                                        <span class="studio-kind">{t.kind.to_uppercase()}</span>
                                        <div>
                                            <div class="studio-title">
                                                {t.name.clone()}
                                                {t.system.then(|| view!{ <span class="sys-tag">" SYS"</span> })}
                                            </div>
                                            <div class="studio-meta">{format!("{} · burn {} · mint {}", t.id, format_io_blob(&t.burns), format_io_blob(&t.mints_coin))}</div>
                                        </div>
                                    </div>
                                    <span class="chip">"OPEN →"</span>
                                </a>
                            }
                        }).collect_view()}
                    }
                }}
            </div>
        </StudioShell>
    }
}

#[component]
pub fn TemplateNewPage() -> impl IntoView {
    let id = RwSignal::new(String::new());
    let name = RwSignal::new(String::new());
    let blurb = RwSignal::new(String::new());
    let kind = RwSignal::new("transform".to_string());
    let burns = RwSignal::new("labor:1, energy:1".to_string());
    let mints = RwSignal::new("wood:1".to_string());
    let bldg = RwSignal::new(String::new());
    let needs_plot = RwSignal::new(false);
    let err = RwSignal::new(None::<String>);
    view! {
        <StudioShell title="New template" kicker="CREATE RECIPE" lead="IO: id:qty, id:qty">
            <div class="studio-form-page">
                {move || err.get().map(|e| view!{<div class="eco-msg err">{e}</div>})}
                <label class="found-label">"ID"</label>
                <input class="found-input" prop:value=move||id.get() on:input=move|ev| id.set(event_target_value(&ev)) />
                <label class="found-label">"NAME"</label>
                <input class="found-input" prop:value=move||name.get() on:input=move|ev| name.set(event_target_value(&ev)) />
                <label class="found-label">"KIND"</label>
                <select class="found-input" prop:value=move||kind.get() on:change=move|ev| kind.set(event_target_value(&ev))>
                    <option value="transform">"transform"</option>
                    <option value="construct">"construct"</option>
                    <option value="custom">"custom"</option>
                </select>
                <label class="found-label">"BLURB"</label>
                <input class="found-input" prop:value=move||blurb.get() on:input=move|ev| blurb.set(event_target_value(&ev)) />
                <label class="found-label">"BURNS"</label>
                <input class="found-input" prop:value=move||burns.get() on:input=move|ev| burns.set(event_target_value(&ev)) />
                <label class="found-label">"MINTS COIN"</label>
                <input class="found-input" prop:value=move||mints.get() on:input=move|ev| mints.set(event_target_value(&ev)) />
                <label class="found-label">"MINTS BUILDING"</label>
                <input class="found-input" prop:value=move||bldg.get() on:input=move|ev| bldg.set(event_target_value(&ev)) />
                <button class="chip" on:click=move |_| needs_plot.update(|v| *v=!*v)>
                    {move || if needs_plot.get() { "NEEDS PLOT: YES" } else { "NEEDS PLOT: NO" }}
                </button>
                <div class="studio-form-actions">
                    <button class="intent-commit" on:click=move |_| {
                        let tid = if id.get().trim().is_empty() { name.get() } else { id.get() };
                        match create_template(&tid, &name.get(), &blurb.get(), &kind.get(), &burns.get(), &mints.get(), &bldg.get(), needs_plot.get()) {
                            Ok(t) => { if let Some(w)=web_sys::window(){ let _=w.location().set_href(&format!("/world/template/{}", t.id)); } }
                            Err(e) => err.set(Some(e)),
                        }
                    }>"CREATE"</button>
                    <a class="chip" href="/world/templates">"← CANCEL"</a>
                </div>
            </div>
        </StudioShell>
    }
}

#[component]
pub fn TemplateViewPage() -> impl IntoView {
    let id = param("id");
    let id_view = id.clone();
    let plot = RwSignal::new(String::new());
    let msg = RwSignal::new(None::<(bool, String)>);
    Effect::new(move |_| {
        if let Some(p) = load_cards().into_iter().find(|c| c.kind == "plot") {
            plot.set(p.id.strip_prefix("plot-").unwrap_or(&p.id).to_string());
        }
    });
    view! {
        <StudioShell title="Template" kicker="RECIPE" lead="">
            {move || msg.get().map(|(ok,t)| view!{ <div class=if ok{"eco-msg ok"}else{"eco-msg err"}>{t}</div>})}
            {move || match get_template(&id_view) {
                None => view!{ <div class="me-empty">"Not found"</div> }.into_any(),
                Some(t) => {
                    let tid = t.id.clone();
                    let tid2 = t.id.clone();
                    let tid3 = t.id.clone();
                    view! {
                        <div class="studio-detail">
                            <div class="studio-kind-lg">{t.kind.to_uppercase()}</div>
                            <h3 class="studio-title-lg">{t.name.clone()}</h3>
                            <div class="studio-meta">{t.blurb.clone()}</div>
                            <div class="studio-meta">{format!("id {} · needs_plot={} · system={}", t.id, t.needs_plot, t.system)}</div>
                            <div class="studio-meta">{format!("BURN  {}", format_io_blob(&t.burns))}</div>
                            <div class="studio-meta">{format!("MINT  {} {:?}", format_io_blob(&t.mints_coin), t.mints_building)}</div>
                            {t.needs_plot.then(|| view! {
                                <div>
                                    <label class="found-label">"PLOT TARGET (flat id)"</label>
                                    <input class="found-input" prop:value=move||plot.get() on:input=move|ev| plot.set(event_target_value(&ev)) />
                                </div>
                            })}
                            <div class="studio-form-actions">
                                <button class="intent-commit" on:click=move |_| {
                                    match run_template(&tid, &plot.get(), None) {
                                        Ok(s) => msg.set(Some((true, s))),
                                        Err(e) => msg.set(Some((false, e))),
                                    }
                                }>"RUN (writes cyberlinks)"</button>
                                <a class="chip" href=format!("/world/template/{}/edit", tid2)>"EDIT"</a>
                                <button class="chip" on:click=move |_| {
                                    match delete_template(&tid3) {
                                        Ok(()) => { if let Some(w)=web_sys::window(){ let _=w.location().set_href("/world/templates"); } }
                                        Err(e) => msg.set(Some((false, e))),
                                    }
                                }>"DELETE"</button>
                                <a class="chip" href="/world/templates">"← LIST"</a>
                            </div>
                        </div>
                    }.into_any()
                }
            }}
        </StudioShell>
    }
}

#[component]
pub fn TemplateEditPage() -> impl IntoView {
    let id = param("id");
    let id_e = id.clone();
    let id_save = id.clone();
    let cancel_href = format!("/world/template/{id}");
    let name = RwSignal::new(String::new());
    let blurb = RwSignal::new(String::new());
    let kind = RwSignal::new(String::new());
    let burns = RwSignal::new(String::new());
    let mints = RwSignal::new(String::new());
    let bldg = RwSignal::new(String::new());
    let needs_plot = RwSignal::new(false);
    let err = RwSignal::new(None::<String>);
    Effect::new(move |_| {
        if let Some(t) = get_template(&id_e) {
            name.set(t.name);
            blurb.set(t.blurb);
            kind.set(t.kind);
            burns.set(format_io_blob(&t.burns));
            mints.set(format_io_blob(&t.mints_coin));
            bldg.set(t.mints_building.unwrap_or_default());
            needs_plot.set(t.needs_plot);
        }
    });
    view! {
        <StudioShell title="Edit template" kicker="EDIT RECIPE" lead="">
            <div class="studio-form-page">
                {move || err.get().map(|e| view!{<div class="eco-msg err">{e}</div>})}
                <label class="found-label">"NAME"</label>
                <input class="found-input" prop:value=move||name.get() on:input=move|ev| name.set(event_target_value(&ev)) />
                <label class="found-label">"KIND"</label>
                <input class="found-input" prop:value=move||kind.get() on:input=move|ev| kind.set(event_target_value(&ev)) />
                <label class="found-label">"BLURB"</label>
                <input class="found-input" prop:value=move||blurb.get() on:input=move|ev| blurb.set(event_target_value(&ev)) />
                <label class="found-label">"BURNS"</label>
                <input class="found-input" prop:value=move||burns.get() on:input=move|ev| burns.set(event_target_value(&ev)) />
                <label class="found-label">"MINTS"</label>
                <input class="found-input" prop:value=move||mints.get() on:input=move|ev| mints.set(event_target_value(&ev)) />
                <label class="found-label">"BUILDING"</label>
                <input class="found-input" prop:value=move||bldg.get() on:input=move|ev| bldg.set(event_target_value(&ev)) />
                <button class="chip" on:click=move |_| needs_plot.update(|v| *v=!*v)>
                    {move || if needs_plot.get() { "NEEDS PLOT: YES" } else { "NEEDS PLOT: NO" }}
                </button>
                <div class="studio-form-actions">
                    <button class="intent-commit" on:click=move |_| {
                        let id_nav = id_save.clone();
                        match update_template(&id_save, &name.get(), &blurb.get(), &kind.get(), &burns.get(), &mints.get(), &bldg.get(), needs_plot.get()) {
                            Ok(()) => { if let Some(w)=web_sys::window(){ let _=w.location().set_href(&format!("/world/template/{id_nav}")); } }
                            Err(e) => err.set(Some(e)),
                        }
                    }>"SAVE"</button>
                    <a class="chip" href=cancel_href>"← CANCEL"</a>
                </div>
            </div>
        </StudioShell>
    }
}

// ─── INTENTS ──────────────────────────────────────────────────────────

#[component]
pub fn IntentsListPage() -> impl IntoView {
    view! {
        <StudioShell title="Intents" kicker="FIRST STEP" lead="Intents are draft cyberlinks / work orders before commit.">
            <div class="studio-create-bar">
                <a class="cta-btn cta-lease cta-lg" href="/world/links/new" style="text-decoration:none;">
                    <span class="cta-copy"><span class="cta-title">"+ CYBERLINK DRAFT"</span><span class="cta-sub">"preferred"</span></span>
                </a>
            </div>
            <div class="studio-list">
                {move || {
                    let list = load_erp_intents();
                    if list.is_empty() {
                        return view!{ <div class="me-empty">"No intents."</div> }.into_any();
                    }
                    view! {
                        {list.into_iter().map(|it| {
                            let href = format!("/world/intent/{}", it.id);
                            view! {
                                <a class="studio-row studio-row-link" href=href>
                                    <div class="studio-row-main">
                                        <span class="studio-kind">{it.state.to_uppercase()}</span>
                                        <div>
                                            <div class="studio-title">{format!("#{} · {}", it.id, it.template_id)}</div>
                                            <div class="studio-meta">{format!("{} · {}", it.kind, it.note)}</div>
                                        </div>
                                    </div>
                                    <span class="chip">"OPEN →"</span>
                                </a>
                            }
                        }).collect_view()}
                    }.into_any()
                }}
            </div>
        </StudioShell>
    }
}

#[component]
pub fn IntentViewPage() -> impl IntoView {
    let id_s = param("id");
    let id: u64 = id_s.parse().unwrap_or(0);
    let _id_keep = id;
    let msg = RwSignal::new(None::<(bool, String)>);
    let tick = RwSignal::new(0u32);
    view! {
        <StudioShell title="Intent" kicker="DRAFT STEP" lead="">
            {move || msg.get().map(|(ok,t)| view!{ <div class=if ok{"eco-msg ok"}else{"eco-msg err"}>{t}</div>})}
            {move || {
                let _ = tick.get();
                let it = load_erp_intents().into_iter().find(|i| i.id == id);
                match it {
                    None => view!{ <div class="me-empty">"Not found"</div> }.into_any(),
                    Some(it) => {
                        let link_href = it.target.clone().map(|t| {
                            if t.starts_with("cl-") { format!("/world/link/{t}") } else { format!("/world/card/{t}") }
                        });
                        view! {
                            <div class="studio-detail">
                                <div class="studio-kind-lg">{it.state.to_uppercase()}</div>
                                <h3 class="studio-title-lg">{format!("#{} · {}", it.id, it.template_id)}</h3>
                                <div class="studio-meta">{format!("{} · {}", it.kind, it.note)}</div>
                                {link_href.map(|h| view!{ <div class="studio-meta"><a href=h>"open target →"</a></div> })}
                                <div class="studio-form-actions">
                                    <button class="chip" on:click=move |_| { let _=set_intent_state(id,"reserved"); tick.update(|n| { *n += 1; }); }>"RESERVE"</button>
                                    <button class="intent-commit" on:click=move |_| {
                                        match commit_intent(id) {
                                            Ok(s) => { msg.set(Some((true,s))); tick.update(|n| { *n += 1; }); }
                                            Err(e) => msg.set(Some((false,e))),
                                        }
                                    }>"COMMIT / RUN"</button>
                                    <button class="chip" on:click=move |_| {
                                        let _ = delete_intent(id);
                                        if let Some(w)=web_sys::window(){ let _=w.location().set_href("/world/intents"); }
                                    }>"DELETE"</button>
                                    <a class="chip" href="/world/intents">"← LIST"</a>
                                </div>
                            </div>
                        }.into_any()
                    }
                }
            }}
        </StudioShell>
    }
}

// ─── SCHEDULES ────────────────────────────────────────────────────────

#[component]
pub fn SchedulesListPage() -> impl IntoView {
    let msg = RwSignal::new(None::<(bool, String)>);
    view! {
        <StudioShell title="Schedules" kicker="CLOCK" lead="TICK advances counters; fire creates cyberlink drafts or runs templates.">
            {move || msg.get().map(|(ok,t)| view!{ <div class=if ok{"eco-msg ok"}else{"eco-msg err"}>{t}</div>})}
            <div class="studio-create-bar">
                <a class="cta-btn cta-lease cta-lg" href="/world/schedules/new" style="text-decoration:none;">
                    <span class="cta-copy"><span class="cta-title">"+ SCHEDULE"</span><span class="cta-sub">"link template"</span></span>
                </a>
                <button class="cta-btn cta-buy cta-lg" on:click=move |_| {
                    match tick_schedules() {
                        Ok(s) => msg.set(Some((true,s))),
                        Err(e) => msg.set(Some((false,e))),
                    }
                }>
                    <span class="cta-copy"><span class="cta-title">"TICK ALL"</span><span class="cta-sub">"advance clock"</span></span>
                </button>
            </div>
            <div class="studio-list">
                {move || {
                    let _ = msg.get();
                    let list = load_schedules();
                    if list.is_empty() {
                        return view!{ <div class="me-empty">"No schedules. "<a href="/world/schedules/new">"Create"</a></div> }.into_any();
                    }
                    view! {
                        {list.into_iter().map(|s| {
                            let href = format!("/world/schedule/{}", s.id);
                            view! {
                                <a class="studio-row studio-row-link" href=href>
                                    <div class="studio-row-main">
                                        <span class="studio-kind">{if s.enabled {"ON"} else {"OFF"}}</span>
                                        <div>
                                            <div class="studio-title">{s.name.clone()}</div>
                                            <div class="studio-meta">{format!("{} · tpl {} · {}/{} ticks", s.id, s.template_id, s.tick_count, s.every_ticks)}</div>
                                        </div>
                                    </div>
                                    <span class="chip">"OPEN →"</span>
                                </a>
                            }
                        }).collect_view()}
                    }.into_any()
                }}
            </div>
        </StudioShell>
    }
}

#[component]
pub fn ScheduleNewPage() -> impl IntoView {
    let name = RwSignal::new(String::new());
    let tpl = RwSignal::new(String::new());
    let target = RwSignal::new(String::new());
    let every = RwSignal::new("3".to_string());
    let auto = RwSignal::new(true);
    let note = RwSignal::new(String::new());
    let err = RwSignal::new(None::<String>);
    view! {
        <StudioShell title="New schedule" kicker="CREATE CLOCK" lead="Points at a template id. TICK fires when counter hits N.">
            <div class="studio-form-page">
                {move || err.get().map(|e| view!{<div class="eco-msg err">{e}</div>})}
                <label class="found-label">"NAME"</label>
                <input class="found-input" prop:value=move||name.get() on:input=move|ev| name.set(event_target_value(&ev)) />
                <label class="found-label">"TEMPLATE ID"</label>
                <input class="found-input" prop:value=move||tpl.get() on:input=move|ev| tpl.set(event_target_value(&ev)) />
                <div class="list-filters">
                    {load_templates().into_iter().take(12).map(|t| {
                        let id = t.id.clone();
                        view!{ <button class="chip" on:click=move |_| tpl.set(id.clone())>{t.id}</button> }
                    }).collect_view()}
                </div>
                <label class="found-label">"TARGET"</label>
                <input class="found-input" prop:value=move||target.get() on:input=move|ev| target.set(event_target_value(&ev)) />
                <label class="found-label">"EVERY N TICKS"</label>
                <input class="found-input" prop:value=move||every.get() on:input=move|ev| every.set(event_target_value(&ev)) />
                <button class="chip" on:click=move |_| auto.update(|v| *v=!*v)>
                    {move || if auto.get() { "AUTO RUN: YES" } else { "AUTO RUN: NO" }}
                </button>
                <label class="found-label">"NOTE"</label>
                <input class="found-input" prop:value=move||note.get() on:input=move|ev| note.set(event_target_value(&ev)) />
                <div class="studio-form-actions">
                    <button class="intent-commit" on:click=move |_| {
                        let every_n: u32 = every.get().parse().unwrap_or(3);
                        let tgt = { let t=target.get().trim().to_string(); if t.is_empty(){None}else{Some(t)} };
                        match create_schedule(&name.get(), &tpl.get(), tgt, every_n, auto.get(), &note.get()) {
                            Ok(s) => { if let Some(w)=web_sys::window(){ let _=w.location().set_href(&format!("/world/schedule/{}", s.id)); } }
                            Err(e) => err.set(Some(e)),
                        }
                    }>"CREATE"</button>
                    <a class="chip" href="/world/schedules">"← CANCEL"</a>
                </div>
            </div>
        </StudioShell>
    }
}

#[component]
pub fn ScheduleViewPage() -> impl IntoView {
    let id = param("id");
    let id_g = id.clone();
    let msg = RwSignal::new(None::<(bool, String)>);
    view! {
        <StudioShell title="Schedule" kicker="CLOCK" lead="">
            {move || msg.get().map(|(ok,t)| view!{ <div class=if ok{"eco-msg ok"}else{"eco-msg err"}>{t}</div>})}
            {move || {
                let s = load_schedules().into_iter().find(|s| s.id == id_g);
                match s {
                    None => view!{ <div class="me-empty">"Not found"</div> }.into_any(),
                    Some(s) => {
                        let sid = s.id.clone();
                        let sid2 = s.id.clone();
                        let sid3 = s.id.clone();
                        view! {
                            <div class="studio-detail">
                                <h3 class="studio-title-lg">{s.name.clone()}</h3>
                                <div class="studio-meta">{format!("{} · template {} · {}/{} · auto={}", s.id, s.template_id, s.tick_count, s.every_ticks, s.auto_run)}</div>
                                <div class="studio-meta">
                                    <a href=format!("/world/template/{}", s.template_id)>"open template →"</a>
                                </div>
                                <div class="studio-form-actions">
                                    <button class="intent-commit" on:click=move |_| {
                                        match crate::erp::fire_schedule_now(&sid) {
                                            Ok(m) => msg.set(Some((true,m))),
                                            Err(e) => msg.set(Some((false,e))),
                                        }
                                    }>"FIRE NOW"</button>
                                    <a class="chip" href=format!("/world/schedule/{}/edit", sid2)>"EDIT"</a>
                                    <button class="chip" on:click=move |_| {
                                        let _ = delete_schedule(&sid3);
                                        if let Some(w)=web_sys::window(){ let _=w.location().set_href("/world/schedules"); }
                                    }>"DELETE"</button>
                                    <a class="chip" href="/world/schedules">"← LIST"</a>
                                </div>
                            </div>
                        }.into_any()
                    }
                }
            }}
        </StudioShell>
    }
}

#[component]
pub fn ScheduleEditPage() -> impl IntoView {
    let id = param("id");
    let id_e = id.clone();
    let id_save = id.clone();
    let cancel_href = format!("/world/schedule/{id}");
    let name = RwSignal::new(String::new());
    let tpl = RwSignal::new(String::new());
    let target = RwSignal::new(String::new());
    let every = RwSignal::new("3".to_string());
    let auto = RwSignal::new(true);
    let enabled = RwSignal::new(true);
    let note = RwSignal::new(String::new());
    let err = RwSignal::new(None::<String>);
    Effect::new(move |_| {
        if let Some(s) = load_schedules().into_iter().find(|s| s.id == id_e) {
            name.set(s.name);
            tpl.set(s.template_id);
            target.set(s.target.unwrap_or_default());
            every.set(s.every_ticks.to_string());
            auto.set(s.auto_run);
            enabled.set(s.enabled);
            note.set(s.note);
        }
    });
    view! {
        <StudioShell title="Edit schedule" kicker="EDIT CLOCK" lead="">
            <div class="studio-form-page">
                {move || err.get().map(|e| view!{<div class="eco-msg err">{e}</div>})}
                <label class="found-label">"NAME"</label>
                <input class="found-input" prop:value=move||name.get() on:input=move|ev| name.set(event_target_value(&ev)) />
                <label class="found-label">"TEMPLATE"</label>
                <input class="found-input" prop:value=move||tpl.get() on:input=move|ev| tpl.set(event_target_value(&ev)) />
                <label class="found-label">"TARGET"</label>
                <input class="found-input" prop:value=move||target.get() on:input=move|ev| target.set(event_target_value(&ev)) />
                <label class="found-label">"EVERY"</label>
                <input class="found-input" prop:value=move||every.get() on:input=move|ev| every.set(event_target_value(&ev)) />
                <button class="chip" on:click=move |_| auto.update(|v| *v=!*v)>{move || if auto.get(){"AUTO: YES"}else{"AUTO: NO"}}</button>
                <button class="chip" on:click=move |_| enabled.update(|v| *v=!*v)>{move || if enabled.get(){"ENABLED: YES"}else{"ENABLED: NO"}}</button>
                <label class="found-label">"NOTE"</label>
                <input class="found-input" prop:value=move||note.get() on:input=move|ev| note.set(event_target_value(&ev)) />
                <div class="studio-form-actions">
                    <button class="intent-commit" on:click=move |_| {
                        let every_n: u32 = every.get().parse().unwrap_or(3);
                        let tgt = { let t=target.get().trim().to_string(); if t.is_empty(){None}else{Some(t)} };
                        let id_nav = id_save.clone();
                        match update_schedule(&id_save, &name.get(), &tpl.get(), tgt, every_n, enabled.get(), auto.get(), &note.get()) {
                            Ok(()) => { if let Some(w)=web_sys::window(){ let _=w.location().set_href(&format!("/world/schedule/{id_nav}")); } }
                            Err(e) => err.set(Some(e)),
                        }
                    }>"SAVE"</button>
                    <a class="chip" href=cancel_href>"← CANCEL"</a>
                </div>
            </div>
        </StudioShell>
    }
}
