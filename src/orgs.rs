//! Orgs — organizations as graph citizens. An org is a word (kind `org`);
//! founding commits you —founded→ org + you —member_of→ org in one signed
//! signal; joining adds your membership link. Members are read from the
//! graph, not a table.

use crate::land::FLAG_SVG;
use crate::nav::CyberiaNav;
use crate::signal::{
    emit_signal, graph_links, load_words, mint_word, neuron, word_name, word_particle, Link, Word,
};
use crate::wallet::load_profile;
use leptos::prelude::*;

pub const ORG_CLASSES: &[&str] = &["coop", "dao", "guild", "company"];

/// All membership links pointing at an org word.
fn members_of(org_particle: &str) -> Vec<String> {
    let member_of = word_particle("relation", "member_of");
    graph_links()
        .into_iter()
        .filter(|(_, l)| l.rel == member_of && l.to == org_particle)
        .map(|(_, l)| l.from)
        .collect()
}

fn is_member(org_particle: &str, person_particle: &str) -> bool {
    members_of(org_particle).iter().any(|m| m == person_particle)
}

fn found_org(name: &str, class: &str, note: &str) -> Result<String, String> {
    let n = name.trim();
    if n.is_empty() {
        return Err("name required".into());
    }
    let me = neuron().bech32;
    let handle = load_profile().handle;
    let org = mint_word("org", n, &format!("{class} · {note}"), &me, false);
    let you = mint_word("person", &handle, "YOU", &me, false);
    let founded = mint_word("relation", "founded", "organization genesis", "", true);
    let member_of = mint_word("relation", "member_of", "membership relation", "", true);
    emit_signal(
        vec![
            Link {
                from: you.clone(),
                rel: founded,
                to: org.clone(),
                weight: 1.0,
                note: class.into(),
            },
            Link {
                from: you,
                rel: member_of,
                to: org.clone(),
                weight: 1.0,
                note: "founder".into(),
            },
        ],
        &format!("found org · {n}"),
    )?;
    Ok(org)
}

fn join_org(org_particle: &str) -> Result<String, String> {
    let me = neuron().bech32;
    let handle = load_profile().handle;
    let you = mint_word("person", &handle, "YOU", &me, false);
    if is_member(org_particle, &you) {
        return Err("already a member".into());
    }
    let member_of = mint_word("relation", "member_of", "membership relation", "", true);
    let org_n = word_name(org_particle);
    emit_signal(
        vec![Link {
            from: you,
            rel: member_of,
            to: org_particle.to_string(),
            weight: 1.0,
            note: "joined".into(),
        }],
        &format!("join org · {org_n}"),
    )?;
    Ok(format!("joined {org_n}"))
}

#[component]
pub fn OrgsPage() -> impl IntoView {
    let msg = RwSignal::new(None::<(bool, String)>);
    let tick = RwSignal::new(0u32);
    let name = RwSignal::new(String::new());
    let class = RwSignal::new("coop".to_string());
    let note = RwSignal::new(String::new());

    Effect::new(move |_| {
        document().set_title("Cyberia — orgs");
        crate::erp::ensure_erp_boot();
        crate::signal::ensure_graph_boot();
    });

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
                            {move || {
                                let _ = tick.get();
                                let n = load_words().iter().filter(|w| w.kind == "org").count();
                                format!("{n} ORGS")
                            }}
                        </div>
                        <CyberiaNav active="orgs" />
                    </div>
                </div>
            </div>

            <div class="cities-stage">
                <div class="cities-hero">
                    <div>
                        <div class="cities-kicker">"ORGANIZATIONS"</div>
                        <h2 class="cities-title">"Orgs"</h2>
                        <p class="cities-lead">
                            "An org is a word in the graph; membership is a link, not a row in someone's database. Found one with a signed signal — the graph remembers who founded what and who stands with whom."
                        </p>
                    </div>
                </div>

                {move || msg.get().map(|(ok, t)| view! {
                    <div class=if ok { "eco-msg ok" } else { "eco-msg err" }>{t}</div>
                })}

                // found an org
                <div class="studio-section-h"><span>"FOUND AN ORG"</span></div>
                <div class="studio-form-page" style="margin-bottom:16px;">
                    <div class="list-filters" style="margin-bottom:8px;">
                        {ORG_CLASSES.iter().map(|c| {
                            let c2 = c.to_string();
                            let c3 = c.to_string();
                            view! {
                                <button class=move || if class.get() == c3 { "chip chip-on" } else { "chip" }
                                    on:click=move |_| class.set(c2.clone())>{c.to_uppercase()}</button>
                            }
                        }).collect_view()}
                    </div>
                    <label class="found-label">"NAME"</label>
                    <input class="found-input" type="text" prop:value=move || name.get()
                        on:input=move |ev| name.set(event_target_value(&ev))
                        placeholder="gesing growers · valley builders …" />
                    <label class="found-label" style="margin-top:8px;">"NOTE"</label>
                    <input class="found-input" type="text" prop:value=move || note.get()
                        on:input=move |ev| note.set(event_target_value(&ev))
                        placeholder="what it is for" />
                    <div class="studio-form-actions">
                        <button class="cta-btn cta-lease" on:click=move |_| {
                            match found_org(&name.get(), &class.get(), note.get().trim()) {
                                Ok(_) => {
                                    msg.set(Some((true, format!("{} founded — you are member #1", name.get()))));
                                    name.set(String::new());
                                    note.set(String::new());
                                }
                                Err(e) => msg.set(Some((false, e))),
                            }
                            tick.update(|n| *n += 1);
                        }>
                            <span class="cta-copy"><span class="cta-title">"FOUND · SIGN"</span></span>
                        </button>
                    </div>
                </div>

                // registry
                <div class="studio-section-h"><span>"REGISTRY"</span></div>
                <div class="studio-list">
                    {move || {
                        let _ = tick.get();
                        let orgs: Vec<Word> = load_words().into_iter().filter(|w| w.kind == "org").collect();
                        if orgs.is_empty() {
                            return view! { <div class="me-empty">"No orgs yet — found the first one."</div> }.into_any();
                        }
                        let handle = load_profile().handle;
                        let my_word = word_particle("person", &handle);
                        view! {
                            {orgs.into_iter().map(|o| {
                                let members = members_of(&o.particle);
                                let n = members.len();
                                let member_names: Vec<String> = members.iter().take(6).map(|m| word_name(m)).collect();
                                let mine = is_member(&o.particle, &my_word);
                                let word_href = format!("/world/word/{}", o.particle);
                                let op = o.particle.clone();
                                let join = move |_| {
                                    match join_org(&op) {
                                        Ok(t) => msg.set(Some((true, t))),
                                        Err(e) => msg.set(Some((false, e))),
                                    }
                                    tick.update(|n| *n += 1);
                                };
                                view! {
                                    <div class="studio-row">
                                        <div class="studio-row-main">
                                            <span class="studio-kind">"ORG"</span>
                                            <div>
                                                <div class="studio-title">{o.name.clone()}</div>
                                                <div class="studio-meta">{o.note.clone()}</div>
                                                <div class="studio-meta" style="color:var(--cyber-cyan);">
                                                    {format!("{n} member{} · {}", if n == 1 { "" } else { "s" }, member_names.join(" · "))}
                                                </div>
                                            </div>
                                        </div>
                                        <div class="studio-row-acts">
                                            {mine.then(|| view! { <span class="sys-tag">"MEMBER"</span> })}
                                            {(!mine).then(|| view! { <button class="chip chip-on" on:click=join>"JOIN"</button> })}
                                            <a class="chip" href=word_href>"WORD →"</a>
                                        </div>
                                    </div>
                                }
                            }).collect_view()}
                        }.into_any()
                    }}
                </div>

                <p class="bank-footnote">
                    "Founding writes two links in one signal: you —founded→ org, you —member_of→ org. Joining adds yours. Everything an org is, the graph carries — check any org's word page for its full meaning."
                </p>
            </div>
        </div>
    }
}
