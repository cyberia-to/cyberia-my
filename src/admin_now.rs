//! NOW — the one list of the desk that must reach zero: the deals in motion
//! with the next step each of them needs, the offers waiting for the
//! valley's approval, the claims an address has made. Its count is the badge
//! on the menu.

use crate::admin::{act, due_of, in_motion, name_of, next_step, reject, today, why, Desk};
use crate::market::{short, View};
use crate::terms::{fmt_usd, terms};
use leptos::prelude::*;

pub fn waiting(v: &View) -> usize {
    v.listings.iter().filter(|l| l.status == "pending").count()
        + v.claims.iter().filter(|c| c.status == "pending").count()
        + v.deals.iter().filter(|d| in_motion(d)).count()
}

pub fn now(d: Desk, v: &View) -> AnyView {
    let offers: Vec<_> = v.listings.iter().filter(|l| l.status == "pending").cloned().collect();
    let claims: Vec<_> = v.claims.iter().filter(|c| c.status == "pending").cloned().collect();
    let deals: Vec<_> = v.deals.iter().filter(|x| in_motion(x)).cloned().collect();
    if offers.is_empty() && claims.is_empty() && deals.is_empty() {
        return view! {
            <div class="ad-zero"><b>"0"</b><span>"nothing waits for the valley"</span></div>
        }.into_any();
    }
    let vv = v.clone();
    view! {
        {(!deals.is_empty()).then(|| view! {
            <div class="dl-q">"deals in motion — the next thing each one needs"</div>
            <div class="sl-cards">
                {deals.into_iter().map(|x| {
                    let due = due_of(&x);
                    let short_usd = due.map(|v| (v - x.paid_usd).max(0.0)).unwrap_or(0.0);
                    let next = next_step(&x);
                    let plot = x.plot.clone();
                    view! {
                        <div class="sl-card pending">
                            <div class="sl-card-top"><b>{name_of(&x.plot)}</b><span class="sl-tag">{x.status.to_uppercase()}</span></div>
                            <div class="sl-card-line">{format!("{} · {} received{}", if x.holder.is_empty() { "holder not named" } else { x.holder.as_str() }, fmt_usd(x.paid_usd), due.map(|v| format!(" of {}", fmt_usd(v))).unwrap_or_default())}</div>
                            <div class="dl-actions">
                                {next.map(|(k, t)| act(d, format!("CONFIRM {t} · TODAY"), "dl-go sm", "step", vec![("plot", x.plot.clone()), ("step", k.to_string()), ("on", today())]))}
                                {(short_usd > 0.5).then(|| view! { <span class="ad-chip warn">{format!("{} still due", fmt_usd(short_usd))}</span> })}
                                <button type="button" class="dl-mini" on:click=move |_| { d.go(2, 0); d.focus.set(plot.clone()); }>"OPEN THE DEAL →"</button>
                            </div>
                        </div>
                    }
                }).collect_view()}
            </div>
        })}
        {(!offers.is_empty()).then(|| view! {
            <div class="dl-q">"offers to approve — a holder or an agent wants a flat on the map"</div>
            {why(d.d)}
            <div class="sl-cards">
                {offers.into_iter().map(|l| view! {
                    <div class="sl-card pending">
                        <div class="sl-card-top"><b>{name_of(&l.plot)}</b><span class="sl-tag">{if l.deal == "lease" { "SUBLEASE" } else { "RESALE" }}</span></div>
                        <div class="sl-card-line">{format!("{}{} · by {} {}", fmt_usd(l.price_usd), if l.deal == "lease" { " a year" } else { "" }, if l.role == "agent" { "the agent" } else { "the holder" }, vv.name_of(&l.by))}</div>
                        {(!l.words.is_empty()).then(|| view! { <div class="sl-card-line dim">{l.words.clone()}</div> })}
                        <div class="dl-actions">
                            {act(d, "APPROVE", "dl-go sm", "approve", vec![("listing", l.id.clone())])}
                            {reject(d, l.id.clone(), d.d)}
                        </div>
                    </div>
                }).collect_view()}
            </div>
        })}
        {(!claims.is_empty()).then(|| view! {
            <div class="dl-q">"claims — an address says a flat is theirs"</div>
            {why(d.e)}
            <div class="sl-cards">
                {claims.into_iter().map(|c| {
                    let book = terms(&c.plot).and_then(|t| t.holder()).map(|h| h.to_string());
                    view! {
                        <div class="sl-card pending">
                            <div class="sl-card-top"><b>{name_of(&c.plot)}</b><span class="sl-tag">"CLAIM"</span></div>
                            <div class="sl-card-line">{format!("{} · {} · {}", c.name, c.contact, short(&c.from))}</div>
                            <div class="sl-card-line dim">{match book { Some(h) => format!("the lease book names the holder: {h}"), None => "the lease book names no holder for this flat".to_string() }}</div>
                            <div class="dl-actions">
                                {act(d, "BIND THE FLAT TO THIS ADDRESS", "dl-go sm", "holder", vec![("plot", c.plot.clone()), ("address", c.from.clone()), ("name", c.name.clone())])}
                                {reject(d, c.id.clone(), d.e)}
                            </div>
                        </div>
                    }
                }).collect_view()}
            </div>
        })}
    }.into_any()
}
