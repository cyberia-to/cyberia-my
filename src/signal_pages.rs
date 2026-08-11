//! Signal Studio pages — word · link · signal. Full URLs, no overlays.
//! The create path: pick words → link them → links batch into the open
//! draft signal → COMMIT signs the batch (ADR-036) → the graph grows.

use crate::signal::{
    canonical_body, commit_signal, draft_link, find_signal, find_word, graph_links, lexicon,
    links_touching, load_signals, load_words, mint_word, neuron, remove_draft_link, resolve_word,
    sentence_run, verify_signal, word_focus, word_name, WORD_KINDS,
};
use crate::world::StudioShell;
use leptos::prelude::*;
use leptos_router::hooks::{use_params_map, use_query_map};

fn param(key: &str) -> String {
    use_params_map()
        .get()
        .get(key)
        .map(|s| s.to_string())
        .unwrap_or_default()
}

fn query(key: &str) -> String {
    use_query_map()
        .get()
        .get(key)
        .map(|s| s.to_string())
        .unwrap_or_default()
}

fn goto(href: &str) {
    if let Some(w) = web_sys::window() {
        let _ = w.location().set_href(href);
    }
}

fn short_particle(p: &str) -> String {
    if p.len() > 12 {
        format!("{}…{}", &p[..6], &p[p.len() - 4..])
    } else {
        p.to_string()
    }
}

/// Datalist of all word names (optionally one kind only).
#[component]
fn WordDatalist(id: &'static str, #[prop(optional)] kind: Option<&'static str>) -> impl IntoView {
    let words = load_words();
    view! {
        <datalist id=id>
            {words.into_iter()
                .filter(|w| kind.map(|k| w.kind == k).unwrap_or(true))
                .map(|w| view! { <option value=w.name.clone() label=format!("{} · {}", w.kind, short_particle(&w.particle))></option> })
                .collect_view()}
        </datalist>
    }
}

// ─── WORDS ────────────────────────────────────────────────────────────

#[component]
pub fn WordsListPage() -> impl IntoView {
    let kind_filter = RwSignal::new(String::new());
    view! {
        <StudioShell title="Words" kicker="LEXICON" lead="A word is a typed particle — the unit of meaning. Ranked by focus: Σ weight of committed links touching it.">
            <div class="studio-create-bar">
                <a class="cta-btn cta-lease cta-lg" href="/world/words/new" style="text-decoration:none;">
                    <span class="cta-copy"><span class="cta-title">"+ WORD"</span><span class="cta-sub">"mint a typed particle"</span></span>
                </a>
            </div>
            <div class="list-filters" style="margin-bottom:12px;">
                <button class=move || if kind_filter.get().is_empty() { "chip chip-on" } else { "chip" } on:click=move |_| kind_filter.set(String::new())>"ALL"</button>
                {WORD_KINDS.iter().map(|k| {
                    let k2 = k.to_string();
                    let k3 = k.to_string();
                    view! {
                        <button class=move || if kind_filter.get() == k3 { "chip chip-on" } else { "chip" }
                            on:click=move |_| kind_filter.set(k2.clone())>{k.to_uppercase()}</button>
                    }
                }).collect_view()}
            </div>
            <div class="studio-list">
                {move || {
                    let f = kind_filter.get();
                    let mut lex = lexicon();
                    if !f.is_empty() {
                        lex.retain(|(w, _)| w.kind == f);
                    }
                    if lex.is_empty() {
                        return view! { <div class="me-empty">"No words in this kind yet. "<a href="/world/words/new">"Mint one"</a></div> }.into_any();
                    }
                    view! {
                        {lex.into_iter().map(|(w, focus)| {
                            let href = format!("/world/word/{}", w.particle);
                            view! {
                                <a class="studio-row studio-row-link" href=href>
                                    <div class="studio-row-main">
                                        <span class="studio-kind">{w.kind.to_uppercase()}</span>
                                        <div>
                                            <div class="studio-title">{w.name.clone()}</div>
                                            <div class="studio-meta">{format!("{} · φ {:.1}{}", short_particle(&w.particle), focus, if w.system { " · SYS" } else { "" })}</div>
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
pub fn WordNewPage() -> impl IntoView {
    let kind = RwSignal::new("concept".to_string());
    let name = RwSignal::new(String::new());
    let note = RwSignal::new(String::new());
    let err = RwSignal::new(None::<String>);

    let submit = move |_| {
        let n = name.get();
        let n = n.trim();
        if n.is_empty() {
            err.set(Some("name required — a word means something".into()));
            return;
        }
        let me = neuron();
        let particle = mint_word(&kind.get(), n, note.get().trim(), &me.bech32, false);
        goto(&format!("/world/word/{particle}"));
    };

    view! {
        <StudioShell title="New word" kicker="MINT" lead="form (kind:name) → particle (hemera hash) → a typed unit of meaning. Same form always yields the same particle — words don't duplicate.">
            <div class="studio-form-page">
                {move || err.get().map(|e| view! { <div class="eco-msg err">{e}</div> })}

                <label class="found-label">"KIND — the declared type face"</label>
                <div class="list-filters" style="margin:4px 0 12px;">
                    {WORD_KINDS.iter().map(|k| {
                        let k2 = k.to_string();
                        let k3 = k.to_string();
                        view! {
                            <button class=move || if kind.get() == k3 { "chip chip-on" } else { "chip" }
                                on:click=move |_| kind.set(k2.clone())>{k.to_uppercase()}</button>
                        }
                    }).collect_view()}
                </div>

                <label class="found-label">"NAME"</label>
                <input class="found-input" type="text" prop:value=move || name.get()
                    on:input=move |ev| name.set(event_target_value(&ev))
                    placeholder="alice · warung · solar-array …" />

                <label class="found-label" style="margin-top:10px;">"NOTE — annotation, not part of the form"</label>
                <input class="found-input" type="text" prop:value=move || note.get()
                    on:input=move |ev| note.set(event_target_value(&ev))
                    placeholder="optional" />

                {move || {
                    let n = name.get();
                    let n = n.trim().to_string();
                    (!n.is_empty()).then(|| {
                        let p = crate::signal::word_particle(&kind.get(), &n);
                        view! { <div class="studio-meta" style="margin-top:8px;">"particle preview: " <code>{short_particle(&p)}</code></div> }
                    })
                }}

                <div class="studio-form-actions">
                    <button class="cta-btn cta-lease" on:click=submit>
                        <span class="cta-copy"><span class="cta-title">"MINT WORD"</span></span>
                    </button>
                    <a class="chip" href="/world/words">"CANCEL"</a>
                </div>
            </div>
        </StudioShell>
    }
}

#[component]
pub fn WordViewPage() -> impl IntoView {
    view! {
        {move || {
            let particle = param("particle");
            let Some(w) = find_word(&particle) else {
                return view! {
                    <StudioShell title="Word not found" kicker="LEXICON" lead="">
                        <div class="me-empty">"No word holds this particle. "<a href="/world/words">"Back to the lexicon"</a></div>
                    </StudioShell>
                }.into_any();
            };
            let focus = word_focus(&w.particle);
            let touching = links_touching(&w.particle);
            let n_links = touching.len();
            let w2 = w.clone();
            view! {
                <StudioShell
                    title=w.name.clone()
                    kicker=format!("WORD · {}", w.kind.to_uppercase())
                    lead=if w.note.is_empty() { format!("φ {focus:.1} · {n_links} links") } else { format!("{} · φ {focus:.1} · {n_links} links", w.note) }
                >
                    // the four faces
                    <div class="bank-kpi-grid me-counts">
                        <div class="kpi me-kpi">
                            <div class="kpi-lab">"FORM"</div>
                            <div class="kpi-val" style="font-size:15px;">{format!("{}:{}", w2.kind, w2.name)}</div>
                            <div class="kpi-sub">"how it is spelled"</div>
                        </div>
                        <div class="kpi me-kpi">
                            <div class="kpi-lab">"PARTICLE"</div>
                            <div class="kpi-val" style="font-size:12px; word-break:break-all;">{w2.particle.clone()}</div>
                            <div class="kpi-sub">"hemera hash of the form"</div>
                        </div>
                        <div class="kpi me-kpi">
                            <div class="kpi-lab">"TYPE"</div>
                            <div class="kpi-val" style="font-size:15px;">{w2.kind.to_uppercase()}</div>
                            <div class="kpi-sub">"declared face"</div>
                        </div>
                        <div class="kpi me-kpi">
                            <div class="kpi-lab">"MEANING"</div>
                            <div class="kpi-val">{format!("φ {focus:.1}")}</div>
                            <div class="kpi-sub">"its position in the graph"</div>
                        </div>
                    </div>

                    <div class="studio-create-bar" style="margin-top:12px;">
                        <a class="cta-btn cta-lease" href=format!("/world/links/new?from={}", w.particle) style="text-decoration:none;">
                            <span class="cta-copy"><span class="cta-title">"LINK FROM THIS →"</span></span>
                        </a>
                        <a class="cta-btn cta-found" href=format!("/world/links/new?to={}", w.particle) style="text-decoration:none;">
                            <span class="cta-copy"><span class="cta-title">"→ LINK TO THIS"</span></span>
                        </a>
                        <a class="chip" href=format!("/world/word/{}/edit", w.particle)>"EDIT NOTE"</a>
                    </div>

                    <div class="studio-section-h" style="margin-top:16px;"><span>"MEANING — LINKS AROUND THIS WORD"</span></div>
                    <div class="studio-list">
                        {if touching.is_empty() {
                            view! { <div class="me-empty">"No committed links yet — meaning is position; link it."</div> }.into_any()
                        } else {
                            let p = w.particle.clone();
                            view! {
                                {touching.into_iter().map(|(sid, l)| {
                                    let role = if l.rel == p { "REL" } else if l.from == p { "FROM" } else { "TO" };
                                    let href = format!("/world/signal/{sid}");
                                    view! {
                                        <a class="studio-row studio-row-link" href=href>
                                            <div class="studio-row-main">
                                                <span class="studio-kind">{role}</span>
                                                <div>
                                                    <div class="studio-title">{format!("{} —[{}]→ {}", word_name(&l.from), word_name(&l.rel), word_name(&l.to))}</div>
                                                    <div class="studio-meta">{format!("{} · w={}", sid, l.weight)}</div>
                                                </div>
                                            </div>
                                            <span class="chip">"SIGNAL →"</span>
                                        </a>
                                    }
                                }).collect_view()}
                            }.into_any()
                        }}
                    </div>
                </StudioShell>
            }.into_any()
        }}
    }
}

#[component]
pub fn WordEditPage() -> impl IntoView {
    let note = RwSignal::new(None::<String>);
    view! {
        {move || {
            let particle = param("particle");
            let Some(w) = find_word(&particle) else {
                return view! {
                    <StudioShell title="Word not found" kicker="LEXICON" lead="">
                        <div class="me-empty"><a href="/world/words">"Back"</a></div>
                    </StudioShell>
                }.into_any();
            };
            if note.get_untracked().is_none() {
                note.set(Some(w.note.clone()));
            }
            let p2 = w.particle.clone();
            let save = move |_| {
                let mut words = load_words();
                if let Some(word) = words.iter_mut().find(|x| x.particle == p2) {
                    word.note = note.get_untracked().unwrap_or_default().trim().to_string();
                }
                crate::signal::save_words(&words);
                goto(&format!("/world/word/{p2}"));
            };
            view! {
                <StudioShell title=format!("Edit · {}", w.name) kicker="WORD" lead="Only the note is editable — the form is the particle. Renaming means minting a new word.">
                    <div class="studio-form-page">
                        <label class="found-label">"NOTE"</label>
                        <input class="found-input" type="text"
                            prop:value=move || note.get().unwrap_or_default()
                            on:input=move |ev| note.set(Some(event_target_value(&ev))) />
                        <div class="studio-form-actions">
                            <button class="cta-btn cta-lease" on:click=save>
                                <span class="cta-copy"><span class="cta-title">"SAVE"</span></span>
                            </button>
                            <a class="chip" href=format!("/world/word/{}", w.particle)>"CANCEL"</a>
                        </div>
                    </div>
                </StudioShell>
            }.into_any()
        }}
    }
}

// ─── LINKS ────────────────────────────────────────────────────────────

#[component]
pub fn LinksListPage() -> impl IntoView {
    let filter = RwSignal::new(String::new()); // "" | draft | linked
    view! {
        <StudioShell title="Links" kicker="GRAPH" lead="The atom of knowledge: word —relation→ word. Links land in signals; a committed signal writes them into the graph.">
            <div class="studio-create-bar">
                <a class="cta-btn cta-lease cta-lg" href="/world/links/new" style="text-decoration:none;">
                    <span class="cta-copy"><span class="cta-title">"+ LINK"</span><span class="cta-sub">"into the draft signal"</span></span>
                </a>
            </div>
            <div class="list-filters" style="margin-bottom:12px;">
                <button class=move || if filter.get().is_empty() { "chip chip-on" } else { "chip" } on:click=move |_| filter.set(String::new())>"ALL"</button>
                <button class=move || if filter.get() == "draft" { "chip chip-on" } else { "chip" } on:click=move |_| filter.set("draft".into())>"DRAFT"</button>
                <button class=move || if filter.get() == "linked" { "chip chip-on" } else { "chip" } on:click=move |_| filter.set("linked".into())>"LINKED"</button>
            </div>
            <div class="studio-list">
                {move || {
                    let f = filter.get();
                    let mut rows: Vec<(String, crate::signal::Link, bool)> = Vec::new();
                    if f.is_empty() || f == "draft" {
                        rows.extend(crate::signal::draft_links().into_iter().map(|(s, l)| (s, l, false)));
                    }
                    if f.is_empty() || f == "linked" {
                        rows.extend(graph_links().into_iter().map(|(s, l)| (s, l, true)));
                    }
                    if rows.is_empty() {
                        return view! { <div class="me-empty">"No links in this filter. "<a href="/world/links/new">"Create one"</a></div> }.into_any();
                    }
                    view! {
                        {rows.into_iter().map(|(sid, l, committed)| {
                            let href = format!("/world/signal/{sid}");
                            view! {
                                <a class="studio-row studio-row-link" href=href>
                                    <div class="studio-row-main">
                                        <span class=if committed { "studio-kind" } else { "studio-kind draft-kind" }>{if committed { "LINKED" } else { "DRAFT" }}</span>
                                        <div>
                                            <div class="studio-title">{format!("{} —[{}]→ {}", word_name(&l.from), word_name(&l.rel), word_name(&l.to))}</div>
                                            <div class="studio-meta">{format!("{} · w={} · {}", sid, l.weight, l.note)}</div>
                                        </div>
                                    </div>
                                    <span class="chip">"SIGNAL →"</span>
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
    let rel = RwSignal::new("knows".to_string());
    let to = RwSignal::new(String::new());
    let weight = RwSignal::new("1".to_string());
    let note = RwSignal::new(String::new());
    let err = RwSignal::new(None::<String>);

    // ?from= / ?to= prefill with a word particle → show its name
    Effect::new(move |_| {
        let qf = query("from");
        if !qf.is_empty() && from.get_untracked().is_empty() {
            from.set(word_name(&qf));
        }
        let qt = query("to");
        if !qt.is_empty() && to.get_untracked().is_empty() {
            to.set(word_name(&qt));
        }
    });

    let submit = move |_| {
        let f = from.get();
        let t = to.get();
        let r = rel.get();
        if f.trim().is_empty() || t.trim().is_empty() || r.trim().is_empty() {
            err.set(Some("from, relation and to are all required".into()));
            return;
        }
        let w: f64 = weight.get().trim().parse().unwrap_or(1.0);
        let me = neuron();
        let fp = resolve_word(&f, &me.bech32);
        let rp = mint_word("relation", r.trim(), "", &me.bech32, false);
        let tp = resolve_word(&t, &me.bech32);
        let sid = draft_link(&fp, &rp, &tp, w, note.get().trim());
        goto(&format!("/world/signal/{sid}"));
    };

    view! {
        <StudioShell title="New link" kicker="ATOM" lead="word —relation→ word. The link lands in the open draft signal; commit the signal to write it into the graph. Unknown names mint concept words on the fly.">
            <div class="studio-form-page">
                {move || err.get().map(|e| view! { <div class="eco-msg err">{e}</div> })}

                <label class="found-label">"FROM — word"</label>
                <input class="found-input" type="text" list="dl-words-from" prop:value=move || from.get()
                    on:input=move |ev| from.set(event_target_value(&ev))
                    placeholder="word name · or 64-hex particle" />
                <WordDatalist id="dl-words-from" />

                <label class="found-label" style="margin-top:10px;">"RELATION — itself a word"</label>
                <input class="found-input" type="text" list="dl-relations" prop:value=move || rel.get()
                    on:input=move |ev| rel.set(event_target_value(&ev))
                    placeholder="knows · owns · located_in · burns · mints …" />
                <WordDatalist id="dl-relations" kind="relation" />

                <label class="found-label" style="margin-top:10px;">"TO — word"</label>
                <input class="found-input" type="text" list="dl-words-to" prop:value=move || to.get()
                    on:input=move |ev| to.set(event_target_value(&ev))
                    placeholder="word name · or 64-hex particle" />
                <WordDatalist id="dl-words-to" />

                <label class="found-label" style="margin-top:10px;">"WEIGHT — soft focus stake"</label>
                <input class="found-input" type="text" prop:value=move || weight.get()
                    on:input=move |ev| weight.set(event_target_value(&ev)) />

                <label class="found-label" style="margin-top:10px;">"NOTE"</label>
                <input class="found-input" type="text" prop:value=move || note.get()
                    on:input=move |ev| note.set(event_target_value(&ev))
                    placeholder="optional" />

                <div class="studio-form-actions">
                    <button class="cta-btn cta-lease" on:click=submit>
                        <span class="cta-copy"><span class="cta-title">"ADD TO DRAFT SIGNAL"</span></span>
                    </button>
                    <a class="chip" href="/world/links">"CANCEL"</a>
                </div>
            </div>
        </StudioShell>
    }
}

// ─── SIGNALS ──────────────────────────────────────────────────────────

#[component]
pub fn SignalsListPage() -> impl IntoView {
    view! {
        <StudioShell title="Signals" kicker="SUBMISSION" lead="A signal is the unit of submission: one atomic batch of links, signed by your neuron. All of it lands together or none of it does.">
            <div class="studio-create-bar">
                <a class="cta-btn cta-lease cta-lg" href="/world/links/new" style="text-decoration:none;">
                    <span class="cta-copy"><span class="cta-title">"+ LINK"</span><span class="cta-sub">"grow the draft batch"</span></span>
                </a>
            </div>
            <div class="studio-list">
                {move || {
                    let mut signals = load_signals();
                    signals.reverse();
                    if signals.is_empty() {
                        return view! { <div class="me-empty">"No signals yet. "<a href="/world/links/new">"Draft the first link"</a></div> }.into_any();
                    }
                    view! {
                        {signals.into_iter().map(|s| {
                            let href = format!("/world/signal/{}", s.id);
                            let n = s.links.len();
                            let run = sentence_run(&s.links);
                            let sentence = n >= 2 && run == n;
                            let st = s.state.to_uppercase();
                            view! {
                                <a class="studio-row studio-row-link" href=href>
                                    <div class="studio-row-main">
                                        <span class=if s.state == "committed" { "studio-kind" } else { "studio-kind draft-kind" }>{st}</span>
                                        <div>
                                            <div class="studio-title">
                                                {format!("{} · {} link{}", s.id, n, if n == 1 { "" } else { "s" })}
                                                {sentence.then(|| view! { <span class="sys-tag" style="margin-left:8px;">"SENTENCE"</span> })}
                                            </div>
                                            <div class="studio-meta">{if s.note.is_empty() { format!("neuron {}", short_particle(&s.neuron)) } else { s.note.clone() }}</div>
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
pub fn SignalViewPage() -> impl IntoView {
    let msg = RwSignal::new(None::<Result<String, String>>);
    let tick = RwSignal::new(0u32);
    view! {
        {move || {
            let _ = tick.get();
            let id = param("id");
            let Some(s) = find_signal(&id) else {
                return view! {
                    <StudioShell title="Signal not found" kicker="SIGNALS" lead="">
                        <div class="me-empty"><a href="/world/signals">"Back to signals"</a></div>
                    </StudioShell>
                }.into_any();
            };
            let n = s.links.len();
            let run = sentence_run(&s.links);
            let is_sentence = n >= 2 && run == n;
            let draft = s.state == "draft";
            let id_commit = s.id.clone();
            let id_verify = s.id.clone();
            let commit = move |_| {
                match commit_signal(&id_commit) {
                    Ok(()) => msg.set(Some(Ok("committed — batch hashed and signed".into()))),
                    Err(e) => msg.set(Some(Err(e))),
                }
                tick.update(|t| *t += 1);
            };
            let verify = move |_| {
                match verify_signal(&id_verify) {
                    Ok(()) => msg.set(Some(Ok("signature VALID — pubkey matches neuron, body matches particle".into()))),
                    Err(e) => msg.set(Some(Err(e))),
                }
            };
            let body_preview = canonical_body(&s.links);
            view! {
                <StudioShell
                    title=format!("Signal {}", s.id)
                    kicker=if draft { "DRAFT — THE OPEN BATCH".to_string() } else { "COMMITTED".to_string() }
                    lead=format!("{} link{} · neuron {}", n, if n == 1 { "" } else { "s" }, s.neuron)
                >
                    {move || msg.get().map(|m| match m {
                        Ok(t) => view! { <div class="eco-msg ok">{t}</div> }.into_any(),
                        Err(t) => view! { <div class="eco-msg err">{t}</div> }.into_any(),
                    })}

                    {is_sentence.then(|| view! {
                        <div class="chain-hint" style="margin-bottom:10px;">
                            <span class="chain-step">"SENTENCE"</span>
                            <span class="chain-arr">"·"</span>
                            <span class="chain-step">{format!("CHAIN OF {n}")}</span>
                            <span class="chain-arr">"·"</span>
                            <span class="chain-step">"ORDER IS GRAMMAR"</span>
                        </div>
                    })}

                    <div class="studio-section-h"><span>"LINKS IN THIS BATCH"</span></div>
                    <div class="studio-list">
                        {if s.links.is_empty() {
                            view! { <div class="me-empty">"Empty batch. "<a href="/world/links/new">"Add a link"</a></div> }.into_any()
                        } else {
                            let sid = s.id.clone();
                            view! {
                                {s.links.iter().enumerate().map(|(i, l)| {
                                    let sid2 = sid.clone();
                                    let from_href = format!("/world/word/{}", l.from);
                                    let rel_href = format!("/world/word/{}", l.rel);
                                    let to_href = format!("/world/word/{}", l.to);
                                    let remove = move |_| {
                                        remove_draft_link(&sid2, i);
                                        tick.update(|t| *t += 1);
                                    };
                                    view! {
                                        <div class="studio-row">
                                            <div class="studio-row-main">
                                                <span class="studio-kind">{format!("{}", i + 1)}</span>
                                                <div>
                                                    <div class="studio-title">
                                                        <a href=from_href style="color:inherit;">{word_name(&l.from)}</a>
                                                        " —["
                                                        <a href=rel_href style="color:var(--cyber-cyan);">{word_name(&l.rel)}</a>
                                                        "]→ "
                                                        <a href=to_href style="color:inherit;">{word_name(&l.to)}</a>
                                                    </div>
                                                    <div class="studio-meta">{format!("w={} · {}", l.weight, l.note)}</div>
                                                </div>
                                            </div>
                                            {draft.then(|| view! {
                                                <button class="chip" on:click=remove>"REMOVE"</button>
                                            })}
                                        </div>
                                    }
                                }).collect_view()}
                            }.into_any()
                        }}
                    </div>

                    {if draft {
                        view! {
                            <div class="studio-create-bar" style="margin-top:14px;">
                                <a class="cta-btn cta-found" href="/world/links/new" style="text-decoration:none;">
                                    <span class="cta-copy"><span class="cta-title">"+ LINK"</span></span>
                                </a>
                                <button class="cta-btn cta-lease cta-lg" on:click=commit>
                                    <span class="cta-copy">
                                        <span class="cta-title">"COMMIT · SIGN"</span>
                                        <span class="cta-sub">"hemera hash + ADR-036"</span>
                                    </span>
                                </button>
                            </div>
                        }.into_any()
                    } else {
                        view! {
                            <div class="studio-section-h" style="margin-top:16px;"><span>"PROOF"</span></div>
                            <div class="bank-kpi-grid me-counts">
                                <div class="kpi me-kpi">
                                    <div class="kpi-lab">"BODY PARTICLE"</div>
                                    <div class="kpi-val" style="font-size:11px; word-break:break-all;">{s.body_particle.clone()}</div>
                                    <div class="kpi-sub">"hemera(canonical body)"</div>
                                </div>
                                <div class="kpi me-kpi">
                                    <div class="kpi-lab">"NEURON"</div>
                                    <div class="kpi-val" style="font-size:11px; word-break:break-all;">{s.neuron.clone()}</div>
                                    <div class="kpi-sub">"bech32 of pubkey hash"</div>
                                </div>
                                <div class="kpi me-kpi">
                                    <div class="kpi-lab">"PUBKEY"</div>
                                    <div class="kpi-val" style="font-size:11px; word-break:break-all;">{s.pubkey_hex.clone()}</div>
                                    <div class="kpi-sub">"secp256k1 compressed"</div>
                                </div>
                                <div class="kpi me-kpi">
                                    <div class="kpi-lab">"SIGNATURE"</div>
                                    <div class="kpi-val" style="font-size:11px; word-break:break-all;">{s.sig_hex.clone()}</div>
                                    <div class="kpi-sub">"ADR-036 over the body"</div>
                                </div>
                            </div>
                            <div class="studio-create-bar" style="margin-top:12px;">
                                <button class="cta-btn cta-found cta-lg" on:click=verify>
                                    <span class="cta-copy">
                                        <span class="cta-title">"VERIFY"</span>
                                        <span class="cta-sub">"re-check hash + signature"</span>
                                    </span>
                                </button>
                            </div>
                        }.into_any()
                    }}

                    <div class="studio-section-h" style="margin-top:16px;"><span>"CANONICAL BODY"</span></div>
                    <pre style="font-size:10px; color:#667; overflow-x:auto; background:rgba(255,255,255,0.02); padding:10px; border-radius:4px;">{body_preview}</pre>
                </StudioShell>
            }.into_any()
        }}
    }
}
