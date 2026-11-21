//! PEOPLE and TRAIL — who holds which flat and who may sign for the valley,
//! and then every act with its signer: the log the gate folds, its verify
//! runs, the public files.

use crate::admin::{act, empty, field, name_of, reject, why, Desk};
use crate::market::{short, View};
use leptos::prelude::*;

pub fn people(d: Desk, v: &View) -> AnyView {
    match d.sub.get() {
        1 => {
            let list: Vec<_> = v.claims.iter().rev().cloned().collect();
            if list.is_empty() { return empty("nobody has claimed a flat"); }
            let pending = list.iter().any(|c| c.status == "pending");
            view! {
                {pending.then(|| why(d.e))}
                <div class="ad-table">
                    <div class="ad-row head l5"><span>"flat"</span><span>"who"</span><span>"contact"</span><span>"address"</span><span>"state"</span></div>
                    {list.into_iter().map(|c| view! {
                        <div class="ad-row l5"><b>{name_of(&c.plot)}</b><span>{c.name.clone()}</span><span>{c.contact.clone()}</span><span class="mono">{short(&c.from)}</span>
                            <span>{if c.status == "pending" {
                                view! {
                                    {act(d, "BIND", "dl-go sm", "holder", vec![("plot", c.plot.clone()), ("address", c.from.clone()), ("name", c.name.clone())])}
                                    {reject(d, c.id.clone(), d.e)}
                                }.into_any()
                            } else { view! { <i>{c.status.clone()}</i> }.into_any() }}</span>
                        </div>
                    }).collect_view()}
                </div>
            }.into_any()
        }
        2 => {
            let is_root = v.role == "root";
            let admins: Vec<_> = v.roles.iter().filter(|r| r.role == "admin").cloned().collect();
            view! {
                <p class="dl-note">"the root is the owner's wallet, pinned outside every repository; it alone names and removes admins. an admin approves, binds, records money and accredits agents."</p>
                <div class="ad-table">
                    <div class="ad-row l3"><b>"ROOT"</b><span>"the owner"</span><span></span></div>
                    {admins.into_iter().map(|r| view! {
                        <div class="ad-row l3"><b>{r.name.clone()}</b><span class="mono">{r.address.clone()}</span>
                            <span>{is_root.then(|| act(d, "REMOVE", "dl-mini", "role", vec![("address", r.address.clone()), ("role", "none".to_string())]))}</span></div>
                    }).collect_view()}
                </div>
                {if is_root {
                    view! {
                        <div class="dl-q">"name an admin"</div>
                        <div class="sl-grid">{field("address", "0x…", d.a)}{field("name", "valley desk", d.b)}</div>
                        <button type="button" class="dl-go sm" disabled=move || d.busy.get() || d.a.get().trim().len() != 42
                            on:click=move |_| d.sign("role", vec![("address", d.a.get_untracked().trim().to_string()), ("role", "admin".to_string()), ("name", d.b.get_untracked())])>"SIGN · NAME THE ADMIN"</button>
                    }.into_any()
                } else { empty("only the root names admins") }}
            }.into_any()
        }
        _ => view! {
            <p class="dl-note">"a flat is bound to one wallet: the holder's. the binding lets them offer the flat or give a mandate. binding it to another address ends what the old holder gave and listed."</p>
            {if v.holders.is_empty() { empty("no flat is bound to a wallet yet") } else {
                view! {
                    <div class="ad-table">
                        <div class="ad-row head l3"><span>"flat"</span><span>"holder"</span><span>"wallet"</span></div>
                        {v.holders.iter().map(|h| view! { <div class="ad-row l3"><b>{name_of(&h.plot)}</b><span>{h.name.clone()}</span><span class="mono">{h.address.clone()}</span></div> }).collect_view()}
                    </div>
                }.into_any()
            }}
            <div class="dl-q">"bind a flat"</div>
            <div class="sl-grid">{field("flat id", "sinwood-7", d.a)}{field("the holder's address", "0x…", d.b)}{field("the name shown", "Ana", d.c)}</div>
            <button type="button" class="dl-go sm" disabled=move || d.busy.get() || d.a.get().trim().is_empty() || d.b.get().trim().len() != 42
                on:click=move |_| d.sign("holder", vec![("plot", d.a.get_untracked().trim().to_lowercase()), ("address", d.b.get_untracked().trim().to_string()), ("name", d.c.get_untracked())])>"SIGN · BIND THE HOLDER"</button>
        }.into_any(),
    }
}

// ── TRAIL ──────────────────────────────────────────────────────

pub fn trail(d: Desk, v: &View) -> AnyView {
    if d.sub.get() == 1 {
        let last = v.log.first().map(|l| l.at.clone()).unwrap_or_else(|| "—".into());
        return view! {
            <div class="ad-tiles">
                <div class="ad-tile go"><span>"acts in the log"</span><b>{v.log.first().map(|l| l.n).unwrap_or(0)}</b><i>{format!("last {last}")}</i></div>
                <div class="ad-tile"><span>"the gate"</span><b>"answers"</b><i>"you are reading its fold"</i></div>
                <div class="ad-tile"><span>"deals published"</span><b>{v.deals.len()}</b><i>"by signed acts"</i></div>
            </div>
            <div class="dl-q">"where the truth lives"</div>
            <div class="mp-links">
                <a href="https://github.com/cyberia-to/cyberia-my-acts" target="_blank" rel="noopener">"THE LOG OF ACTS"</a>
                <a href="https://github.com/cyberia-to/cyberia-my-acts/actions" target="_blank" rel="noopener">"ITS VERIFY RUNS"</a>
                <a href="/api/acts/public" target="_blank">"PUBLIC OFFERS"</a>
                <a href="/api/acts/deals" target="_blank">"PUBLIC DEALS"</a>
            </div>
            <p class="dl-note">"the market and the deals are the fold of the log's files in the order they were committed. the build re-folds them on every push and fails on an act whose signature or right does not hold."</p>
        }.into_any();
    }
    if v.log.is_empty() { return empty("no act has been signed yet — the first is yours"); }
    let vv = v.clone();
    view! {
        <div class="ad-table">
            <div class="ad-row head l5"><span>"#"</span><span>"act"</span><span>"on"</span><span>"signed by"</span><span>"at"</span></div>
            {v.log.iter().map(|l| view! {
                <div class="ad-row l5"><span>{l.n}</span><b>{l.kind.to_uppercase()}</b>
                    <span>{format!("{}{}", if l.what.len() > 30 { short(&l.what) } else { l.what.clone() }, if l.detail.is_empty() { String::new() } else { format!(" · {}", l.detail) })}</span>
                    <span>{vv.name_of(&l.by)}</span><span>{l.at.replace('T', " ").replace('Z', "")}</span></div>
            }).collect_view()}
        </div>
    }.into_any()
}
