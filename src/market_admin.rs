//! The valley's desk over the land market, on `/admin`: what waits for an
//! answer first (offers to approve, claims to bind), then the books — who
//! holds which flat, who is an admin or an accredited agent, the mandates,
//! what is live. Every button is an act the admin's wallet signs and the
//! gate commits; the page itself can change nothing.

use crate::land::load_map;
use crate::market::{self, short, View};
use crate::sell::{enter, submit};
use crate::terms::{bare_name, fmt_usd};
use leptos::prelude::*;

fn plot_name(id: &str) -> String {
    load_map().phase0.iter().find(|f| f.id == id).map(|f| bare_name(&f.name)).unwrap_or_else(|| id.to_uppercase())
}

fn input(label: &'static str, hint: &'static str, sig: RwSignal<String>) -> impl IntoView {
    view! {
        <label class="sl-field">
            <span>{label}</span>
            <input type="text" placeholder=hint prop:value=move || sig.get() on:input=move |ev| sig.set(event_target_value(&ev)) />
        </label>
    }
}

#[component]
pub fn MarketDesk() -> impl IntoView {
    let view = RwSignal::new(None::<View>);
    let busy = RwSignal::new(false);
    let err = RwSignal::new(String::new());
    leptos::task::spawn_local(async move {
        if let Some(v) = market::fetch_me().await {
            view.set(Some(v));
        }
    });
    let h_plot = RwSignal::new(String::new());
    let h_addr = RwSignal::new(String::new());
    let h_name = RwSignal::new(String::new());
    let r_addr = RwSignal::new(String::new());
    let r_name = RwSignal::new(String::new());
    let r_role = RwSignal::new("agent".to_string());
    let reason = RwSignal::new(String::new());

    let body = move || {
        let Some(v) = view.get() else {
            return view! {
                <p class="dl-note">"the desk opens to an admin's wallet. sign in; what you may do follows from the address."</p>
                <crate::wallet_ui::WalletConnect on_connected=Callback::new(move |w| enter(view, busy, err, w)) />
            }.into_any();
        };
        if !v.is_admin() {
            return view! {
                <p class="dl-note warn">{format!("{} is not an admin of the valley. the root names admins; ask the owner.", short(&v.address))}</p>
                <button type="button" class="dl-mini" on:click=move |_| { market::sign_out(); crate::eth::forget_eth(); view.set(None); }>"SIGN OUT"</button>
            }.into_any();
        };
        let pending: Vec<_> = v.listings.iter().filter(|l| l.status == "pending").cloned().collect();
        let claims: Vec<_> = v.claims.iter().filter(|c| c.status == "pending").cloned().collect();
        let live: Vec<_> = v.listings.iter().filter(|l| l.status == "live").cloned().collect();
        let mandates: Vec<_> = v.mandates.iter().filter(|m| m.status == "live").cloned().collect();
        let (va, vb, vc, vd) = (v.clone(), v.clone(), v.clone(), v.clone());
        let is_root = v.role == "root";
        view! {
            <div class="md-head">
                <span>{format!("{} · {}", if is_root { "ROOT" } else { "ADMIN" }, short(&v.address))}</span>
                <span>{format!("{} to approve · {} claims · {} live · {} mandates", pending.len(), claims.len(), live.len(), mandates.len())}</span>
                <button type="button" class="dl-mini" on:click=move |_| { market::sign_out(); crate::eth::forget_eth(); view.set(None); }>"SIGN OUT"</button>
            </div>

            <div class="dl-q">"offers waiting for you"</div>
            {if pending.is_empty() {
                view! { <p class="dl-note">"nothing waits"</p> }.into_any()
            } else {
                view! {
                    <div class="sl-cards">
                        {pending.into_iter().map(|l| {
                            let (ida, idr) = (l.id.clone(), l.id.clone());
                            view! {
                                <div class="sl-card pending">
                                    <div class="sl-card-top">
                                        <a href=format!("/plot/{}", l.plot) target="_blank"><b>{plot_name(&l.plot)}</b></a>
                                        <span class="sl-tag">{if l.deal == "lease" { "SUBLEASE" } else { "RESALE" }}</span>
                                    </div>
                                    <div class="sl-card-line">{format!(
                                        "{}{}{}",
                                        fmt_usd(l.price_usd),
                                        if l.deal == "lease" { " a year" } else { "" },
                                        l.term_years.map(|t| format!(" · {t} years")).unwrap_or_default()
                                    )}</div>
                                    <div class="sl-card-line">{format!(
                                        "signed by {} · {} · {}",
                                        if l.role == "agent" { "the holder's agent" } else { "the holder" },
                                        va.name_of(&l.by),
                                        short(&l.by)
                                    )}</div>
                                    {(!l.words.is_empty()).then(|| view! { <div class="sl-card-line dim">{l.words.clone()}</div> })}
                                    {(!l.contact.is_empty()).then(|| view! { <div class="sl-card-line dim">{format!("contact {}", l.contact)}</div> })}
                                    <div class="dl-actions">
                                        <button type="button" class="dl-go sm" disabled=move || busy.get()
                                            on:click=move |_| submit(view, busy, err, "approve", vec![("listing", ida.clone())])>"APPROVE"</button>
                                        <button type="button" class="dl-mini" disabled=move || busy.get()
                                            on:click=move |_| submit(view, busy, err, "reject", vec![("target", idr.clone()), ("reason", reason.get_untracked())])>"REJECT"</button>
                                    </div>
                                </div>
                            }
                        }).collect_view()}
                    </div>
                }.into_any()
            }}

            <div class="dl-q">"claims — an address says a flat is theirs"</div>
            {if claims.is_empty() {
                view! { <p class="dl-note">"nothing waits"</p> }.into_any()
            } else {
                view! {
                    <div class="sl-cards">
                        {claims.into_iter().map(|c| {
                            let (plot, addr, name, idr) = (c.plot.clone(), c.from.clone(), c.name.clone(), c.id.clone());
                            let book = crate::terms::terms(&c.plot).and_then(|t| t.holder()).map(|h| h.to_string());
                            view! {
                                <div class="sl-card pending">
                                    <div class="sl-card-top">
                                        <a href=format!("/plot/{}", c.plot) target="_blank"><b>{plot_name(&c.plot)}</b></a>
                                        <span class="sl-tag">"CLAIM"</span>
                                    </div>
                                    <div class="sl-card-line">{format!("{} · {} · {}", c.name, c.contact, c.from)}</div>
                                    <div class="sl-card-line dim">{match book {
                                        Some(h) => format!("the lease book names the holder: {h}"),
                                        None => "the lease book names no holder for this flat".to_string(),
                                    }}</div>
                                    <div class="dl-actions">
                                        <button type="button" class="dl-go sm" disabled=move || busy.get()
                                            on:click=move |_| submit(view, busy, err, "holder", vec![("plot", plot.clone()), ("address", addr.clone()), ("name", name.clone())])>"BIND THE FLAT TO THIS ADDRESS"</button>
                                        <button type="button" class="dl-mini" disabled=move || busy.get()
                                            on:click=move |_| submit(view, busy, err, "reject", vec![("target", idr.clone()), ("reason", reason.get_untracked())])>"REJECT"</button>
                                    </div>
                                </div>
                            }
                        }).collect_view()}
                    </div>
                }.into_any()
            }}
            <div class="sl-grid">{input("the reason you give when rejecting", "not in the lease book", reason)}</div>

            <div class="dl-q">"live on the map"</div>
            {if live.is_empty() {
                view! { <p class="dl-note">"no offer is live"</p> }.into_any()
            } else {
                view! {
                    <div class="md-table">
                        {live.into_iter().map(|l| {
                            let id = l.id.clone();
                            view! {
                                <div>
                                    <b>{plot_name(&l.plot)}</b>
                                    <span>{format!("{} · {}", if l.deal == "lease" { "sublease" } else { "resale" }, fmt_usd(l.price_usd))}</span>
                                    <span>{format!("{} {}", if l.role == "agent" { "agent" } else { "holder" }, vb.name_of(&l.by))}</span>
                                    <button type="button" class="dl-mini" disabled=move || busy.get()
                                        on:click=move |_| submit(view, busy, err, "withdraw", vec![("listing", id.clone()), ("reason", reason.get_untracked())])>"TAKE DOWN"</button>
                                </div>
                            }
                        }).collect_view()}
                    </div>
                }.into_any()
            }}

            <div class="dl-q">"holders — a flat and the wallet that holds it"</div>
            <div class="md-table">
                {v.holders.iter().map(|h| view! {
                    <div><b>{plot_name(&h.plot)}</b><span>{h.name.clone()}</span><span class="mono">{h.address.clone()}</span><i></i></div>
                }).collect_view()}
            </div>
            <div class="sl-grid">
                {input("flat id", "sinwood-7", h_plot)}
                {input("the holder's address", "0x…", h_addr)}
                {input("the name shown", "Ana", h_name)}
            </div>
            <button type="button" class="dl-go sm" disabled=move || busy.get() || h_plot.get().trim().is_empty() || h_addr.get().trim().len() != 42
                on:click=move |_| submit(view, busy, err, "holder", vec![
                    ("plot", h_plot.get_untracked().trim().to_lowercase()),
                    ("address", h_addr.get_untracked().trim().to_string()),
                    ("name", h_name.get_untracked()),
                ])>"SIGN · BIND THE HOLDER"</button>

            <div class="dl-q">"mandates in force"</div>
            {if mandates.is_empty() {
                view! { <p class="dl-note">"no holder has given a mandate"</p> }.into_any()
            } else {
                view! {
                    <div class="md-table">
                        {mandates.into_iter().map(|m| {
                            let id = m.id.clone();
                            view! {
                                <div>
                                    <b>{plot_name(&m.plot)}</b>
                                    <span>{format!("{} → {}", vc.name_of(&m.holder), vc.name_of(&m.agent))}</span>
                                    <span>{format!("{} · {}% · until {}", m.scope, m.commission, m.until)}</span>
                                    <button type="button" class="dl-mini" disabled=move || busy.get()
                                        on:click=move |_| submit(view, busy, err, "revoke", vec![("mandate", id.clone())])>"REVOKE"</button>
                                </div>
                            }
                        }).collect_view()}
                    </div>
                }.into_any()
            }}

            <div class="dl-q">"admins and accredited agents"</div>
            <div class="md-table">
                {vd.roles.iter().filter(|r| r.role != "none").map(|r| view! {
                    <div><b>{r.role.to_uppercase()}</b><span>{r.name.clone()}</span><span class="mono">{r.address.clone()}</span><i></i></div>
                }).collect_view()}
            </div>
            <div class="sl-grid">
                {input("address", "0x…", r_addr)}
                {input("name", "Bo of Bali Land", r_name)}
                <label class="sl-field">
                    <span>"role"</span>
                    <select on:change=move |ev| r_role.set(event_target_value(&ev))>
                        <option value="agent">"accredited agent"</option>
                        {is_root.then(|| view! { <option value="admin">"admin"</option> })}
                        <option value="none">"none — remove"</option>
                    </select>
                </label>
            </div>
            <button type="button" class="dl-go sm" disabled=move || busy.get() || r_addr.get().trim().len() != 42
                on:click=move |_| submit(view, busy, err, "role", vec![
                    ("address", r_addr.get_untracked().trim().to_string()),
                    ("role", r_role.get_untracked()),
                    ("name", r_name.get_untracked()),
                ])>"SIGN · SET THE ROLE"</button>
        }.into_any()
    };

    view! {
        <section class="cab-card md">
            <div class="cab-card-h">
                <span class="panel-kicker">"LAND MARKET · THE DESK"</span>
                <span class="panel-sub">"holders, agents, offers — every answer is a signed act, committed"</span>
            </div>
            <div class="md-body">
                {move || { let e = err.get(); (!e.is_empty()).then(|| view! { <p class="dl-note warn">{e}</p> }) }}
                {move || busy.get().then(|| view! { <p class="dl-note">"waiting for your wallet…"</p> })}
                {body}
            </div>
        </section>
    }
}
