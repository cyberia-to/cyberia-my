//! MARKET — what the holders bring: their offers (to approve, to reject, to
//! take down), the mandates they have given their agents, and the agents the
//! valley accredits.

use crate::admin::{act, empty, field, name_of, Desk};
use crate::market::{self, View};
use crate::terms::fmt_usd;
use leptos::prelude::*;

pub fn market_menu(d: Desk, v: &View) -> AnyView {
    let vv = v.clone();
    match d.sub.get() {
        1 => {
            let list: Vec<_> = v.mandates.iter().rev().cloned().collect();
            if list.is_empty() { return empty("no holder has given a mandate"); }
            view! {
                <div class="ad-table">
                    <div class="ad-row head l5"><span>"flat"</span><span>"holder → agent"</span><span>"covers"</span><span>"until"</span><span>"state"</span></div>
                    {list.into_iter().map(|m| view! {
                        <div class="ad-row l5">
                            <b>{name_of(&m.plot)}</b>
                            <span>{format!("{} → {}", vv.name_of(&m.holder), vv.name_of(&m.agent))}</span>
                            <span>{format!("{} · {}%", m.scope, m.commission)}</span>
                            <span>{m.until.clone()}</span>
                            <span>{if m.status == "live" { act(d, "REVOKE", "dl-mini", "revoke", vec![("mandate", m.id.clone())]).into_any() } else { view! { <i>{m.status.clone()}</i> }.into_any() }}</span>
                        </div>
                    }).collect_view()}
                </div>
            }.into_any()
        }
        2 => {
            let agents: Vec<_> = v.roles.iter().filter(|r| r.role == "agent").cloned().collect();
            view! {
                <p class="dl-note">"an accredited agent shows as such to buyers. any address a holder gives a mandate may list; accreditation is the valley's word for it."</p>
                {if agents.is_empty() { empty("no agent is accredited yet") } else {
                    view! {
                        <div class="ad-table">
                            {agents.into_iter().map(|r| view! {
                                <div class="ad-row l3"><b>{r.name.clone()}</b><span class="mono">{r.address.clone()}</span>
                                    <span>{act(d, "REMOVE", "dl-mini", "role", vec![("address", r.address.clone()), ("role", "none".to_string())])}</span></div>
                            }).collect_view()}
                        </div>
                    }.into_any()
                }}
                <div class="dl-q">"accredit an agent"</div>
                <div class="sl-grid">{field("address", "0x…", d.a)}{field("name", "Bo of Bali Land", d.b)}</div>
                <button type="button" class="dl-go sm" disabled=move || d.busy.get() || d.a.get().trim().len() != 42
                    on:click=move |_| d.sign("role", vec![("address", d.a.get_untracked().trim().to_string()), ("role", "agent".to_string()), ("name", d.b.get_untracked())])>"SIGN · ACCREDIT"</button>
            }.into_any()
        }
        _ => {
            let pending: Vec<_> = v.listings.iter().filter(|l| l.status == "pending").cloned().collect();
            let live: Vec<_> = v.listings.iter().filter(|l| l.status == "live").cloned().collect();
            let past = v.listings.len() - pending.len() - live.len();
            let card = move |l: market::Listing, live: bool| {
                view! {
                    <div class=if live { "sl-card live" } else { "sl-card pending" }>
                        <div class="sl-card-top"><b>{name_of(&l.plot)}</b><span class="sl-tag">{format!("{} · {}", if l.deal == "lease" { "SUBLEASE" } else { "RESALE" }, l.status.to_uppercase())}</span></div>
                        <div class="sl-card-line">{format!("{}{} · by {} {} · {}", fmt_usd(l.price_usd), if l.deal == "lease" { " a year" } else { "" }, if l.role == "agent" { "the agent" } else { "the holder" }, vv.name_of(&l.by), l.contact)}</div>
                        {(!l.words.is_empty()).then(|| view! { <div class="sl-card-line dim">{l.words.clone()}</div> })}
                        <div class="dl-actions">
                            {if live {
                                act(d, "TAKE IT DOWN", "dl-mini", "withdraw", vec![("listing", l.id.clone())]).into_any()
                            } else {
                                view! {
                                    {act(d, "APPROVE", "dl-go sm", "approve", vec![("listing", l.id.clone())])}
                                    {act(d, "REJECT", "dl-mini", "reject", vec![("target", l.id.clone())])}
                                }.into_any()
                            }}
                        </div>
                    </div>
                }
            };
            let card2 = card.clone();
            view! {
                <div class="dl-q">{format!("waiting for approval · {}", pending.len())}</div>
                {if pending.is_empty() { empty("nothing waits") } else { view! { <div class="sl-cards">{pending.into_iter().map(|l| card(l, false)).collect_view()}</div> }.into_any() }}
                <div class="dl-q">{format!("live on the map · {}", live.len())}</div>
                {if live.is_empty() { empty("no offer is live") } else { view! { <div class="sl-cards">{live.into_iter().map(|l| card2(l, true)).collect_view()}</div> }.into_any() }}
                <p class="dl-note">{format!("{past} past offers — withdrawn, replaced or rejected — are in the trail")}</p>
            }.into_any()
        }
    }
}
