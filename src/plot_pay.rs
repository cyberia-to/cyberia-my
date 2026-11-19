//! LEASE — the checkout of a flat, in three steps a buyer reads top to
//! bottom: how you pay (all now, or a premium and a yearly rent in CX), where
//! you send it (the Company's bank account or its Ethereum address), and
//! telling the valley it is sent — which is the moment the flat becomes
//! requested in this browser, never earlier. What the Company has received
//! comes from the published deal; what this browser sent is kept here until
//! the valley confirms it.

use crate::deals::{fmt_ts, open_deal, Deal};
use crate::orders::{support_href, PublishedDeal, SUPPORT_HANDLE};
use crate::pay::{
    load_sent, new_sent, record_sent, sent_for, sent_text, yearly_rent, Form, BANK, ETH_ADDRESS,
    PREMIUM_STEPS, TERM_YEARS,
};
use crate::terms::fmt_usd;
use leptos::prelude::*;
use std::collections::HashMap;

fn encode(s: &str) -> String {
    String::from(js_sys::encode_uri_component(s))
}

/// What happens after the money is sent, in the valley's order.
const AFTER: &[&str] = &[
    "the Company confirms the payment — the flat is reserved for you",
    "the lease agreement and the flat's survey come to you to read",
    "both sides sign — the leasehold is yours for the term",
];

/// `price`: the book's plot price; `published`: the valley's deal, if any.
#[component]
pub fn PayPanel(
    plot_id: String,
    plot_name: String,
    price: Option<f64>,
    #[prop(into)] published: Signal<Option<PublishedDeal>>,
    deals: RwSignal<HashMap<String, Deal>>,
) -> impl IntoView {
    let form = RwSignal::new(Form::Upfront);
    let pct = RwSignal::new(30u32);
    let method = RwSignal::new("bank".to_string());
    let sent = RwSignal::new(load_sent());
    let note = RwSignal::new(String::new());
    let cx = RwSignal::new(None::<f64>);
    leptos::task::spawn_local(async move {
        cx.set(crate::orders::fetch_cx_level().await);
    });

    let share = move || if form.get() == Form::Upfront { 100 } else { pct.get() };
    let due = move || price.map(|p| p * share() as f64 / 100.0);
    let rent = move || price.map(|p| yearly_rent(p, share()));

    let received = move || {
        published
            .get()
            .map(|d| (d.paid_usd, d.payments.clone(), d.status.clone(), d.form.clone(), d.premium_pct))
            .unwrap_or((0.0, Vec::new(), String::new(), String::new(), None))
    };

    let pid_send = plot_id.clone();
    let pname_send = plot_name.clone();
    let on_sent = move |_| {
        let Some(usd) = due() else { return };
        let p = new_sent(&pid_send, usd, share(), form.get(), &method.get());
        let text = sent_text(&p, &pname_send);
        sent.update(|l| record_sent(l, p));
        // sending the money is the request: the flat is spoken for from here
        deals.update(|d| open_deal(d, &pid_send, &pname_send));
        note.set("recorded · the chat with the valley opened — send the message there".into());
        if let Some(w) = web_sys::window() {
            let _ = w.open_with_url_and_target(
                &format!("https://t.me/{SUPPORT_HANDLE}?text={}", encode(&text)),
                "_blank",
            );
        }
    };

    let pid_rows = plot_id.clone();
    let pid_ref = plot_id.clone();

    view! {
        <section class="plot-card plot-pay">
            <div class="cyberia-panel-h">
                <span class="panel-kicker">"LEASE"</span>
                <span class="panel-sub">{format!("{TERM_YEARS}-year leasehold · the right to use the flat, never its title")}</span>
            </div>
            {move || {
                let (paid, payments, status, pform, ppct) = received();
                if payments.is_empty() {
                    return view! { <span></span> }.into_any();
                }
                view! {
                    <div class="pay-received">
                        <div class="deal-now-price">{format!("{} received", fmt_usd(paid))}</div>
                        <div class="deal-now-line">{format!(
                            "{}{}",
                            status,
                            match (pform.as_str(), ppct) {
                                ("index", Some(p)) => format!(" · premium {p}% + yearly rent in CX"),
                                ("upfront", _) => " · paid in full".to_string(),
                                _ => String::new(),
                            }
                        )}</div>
                        {payments.into_iter().map(|p| view! {
                            <div class="deal-event">
                                <span class="deal-event-step">{p.method.to_uppercase()}</span>
                                <span class="deal-event-note">{format!("{} · {}{}", fmt_usd(p.usd), p.reference, if p.note.is_empty() { String::new() } else { format!(" · {}", p.note) })}</span>
                                <span class="deal-event-ts">{p.on}</span>
                            </div>
                        }).collect_view()}
                    </div>
                }.into_any()
            }}
            {match price {
                None => view! {
                    <p class="docs-empty">
                        "the price of this flat is set on request — "
                        <a href=support_href() target="_blank" rel="noopener" style="color: var(--cyber-green);">{format!("write to @{SUPPORT_HANDLE}")}</a>
                    </p>
                }.into_any(),
                Some(p) => view! {
                    <div class="pay-form">
                        <div class="pay-stepline"><span class="pay-n">"1"</span>"how you pay"</div>
                        <div class="pay-forms">
                            {[Form::Upfront, Form::Index].into_iter().map(|f| view! {
                                <button
                                    type="button"
                                    class=move || if form.get() == f { "pay-form-opt is-on" } else { "pay-form-opt" }
                                    on:click=move |_| form.set(f)
                                >
                                    <span class="pay-form-t">{f.label()}</span>
                                    <span class="pay-form-s">{f.sub()}</span>
                                </button>
                            }).collect_view()}
                        </div>
                        {move || (form.get() == Form::Index).then(|| view! {
                            <div class="pay-row">
                                <span class="pay-k">"premium"</span>
                                <span class="pay-steps">
                                    {PREMIUM_STEPS.iter().map(|&s| view! {
                                        <button type="button" class=move || if pct.get() == s { "pay-step is-on" } else { "pay-step" } on:click=move |_| pct.set(s)>{format!("{s}%")}</button>
                                    }).collect_view()}
                                </span>
                            </div>
                        })}
                        <div class="pay-sum">
                            <div class="pay-row">
                                <span class="pay-k">"plot price"</span>
                                <span class="pay-v">{fmt_usd(p)}</span>
                            </div>
                            <div class="pay-row pay-due">
                                <span class="pay-k">{move || if share() == 100 { "at signing".to_string() } else { format!("at signing · {}%", share()) }}</span>
                                <span class="pay-v">{move || due().map(fmt_usd).unwrap_or_default()}</span>
                            </div>
                            {move || (form.get() == Form::Index).then(|| {
                                let r = rent().unwrap_or(0.0);
                                view! {
                                    <div class="pay-row pay-rent">
                                        <span class="pay-k">"rent, each year"</span>
                                        <span class="pay-v">{move || match cx.get() {
                                            Some(level) => format!("{:.0} CX ≈ {} today", r / level, fmt_usd(r)),
                                            None => format!("≈ {} in today's dollars", fmt_usd(r)),
                                        }}</span>
                                    </div>
                                    <p class="docs-note">
                                        {format!("for {TERM_YEARS} years. the rent is a number of CX — the ")}
                                        <a href="https://cx.cyberia.my" target="_blank" rel="noopener" style="color: var(--cyber-green);">"century index"</a>
                                        " — fixed for the term and invoiced on each anniversary at that day's fix, so it keeps its value without a raise clause. a smaller premium carries a higher rate: 9% + 5% × the share left as rent."
                                    </p>
                                }
                            })}
                        </div>

                        <div class="pay-stepline"><span class="pay-n">"2"</span>"where you send it"</div>
                        <div class="pay-choice">
                            <button type="button" class=move || if method.get() == "bank" { "pay-opt is-on" } else { "pay-opt" } on:click=move |_| method.set("bank".into())>"BANK TRANSFER"</button>
                            <button type="button" class=move || if method.get() == "eth" { "pay-opt is-on" } else { "pay-opt" } on:click=move |_| method.set("eth".into())>"ETHEREUM"</button>
                        </div>
                        {move || if method.get() == "bank" {
                            let pid_now = pid_ref.clone();
                            view! {
                                <div class="pay-bank">
                                    <div class="pay-row"><span class="pay-k">"account name"</span><span class="pay-v">{BANK.holder}</span></div>
                                    <div class="pay-row"><span class="pay-k">"account number"</span><span class="pay-v cabinet-mono">{BANK.number}</span></div>
                                    <div class="pay-row"><span class="pay-k">"SWIFT"</span><span class="pay-v cabinet-mono">{BANK.swift}</span></div>
                                    <div class="pay-row"><span class="pay-k">"reference"</span><span class="pay-v cabinet-mono">{move || crate::pay::reference(&pid_now, share())}</span></div>
                                    <div class="pay-row"><span class="pay-k">"bank"</span><span class="pay-v">{format!("{} · {}", BANK.bank, BANK.branch)}</span></div>
                                    <div class="pay-row"><span class="pay-k">"bank address"</span><span class="pay-v">{BANK.bank_address}</span></div>
                                    <div class="pay-row"><span class="pay-k">"holder address"</span><span class="pay-v">{BANK.holder_address}</span></div>
                                    <div class="pay-row"><span class="pay-k">"currencies"</span><span class="pay-v">{BANK.currencies}</span></div>
                                    <p class="docs-note">"USD, EUR and AUD land as sent; IDR settles at Bank Indonesia's JISDOR rate on the invoice date"</p>
                                </div>
                            }.into_any()
                        } else {
                            match ETH_ADDRESS {
                                Some(a) => view! {
                                    <div class="pay-bank">
                                        <div class="pay-row"><span class="pay-k">"address"</span><span class="pay-v cabinet-mono">{a}</span></div>
                                        <p class="docs-note">"ETH or USDT on Ethereum mainnet"</p>
                                    </div>
                                }.into_any(),
                                None => view! {
                                    <div class="pay-bank">
                                        <p class="docs-note">"the Company's Ethereum address is given to you by "
                                            <a href=support_href() target="_blank" rel="noopener" style="color: var(--cyber-green);">{format!("@{SUPPORT_HANDLE}")}</a>
                                            " in a direct message · ETH or USDT on mainnet"
                                        </p>
                                    </div>
                                }.into_any(),
                            }
                        }}

                        <div class="pay-stepline"><span class="pay-n">"3"</span>"tell the valley"</div>
                        <button type="button" class="intent-commit lease" on:click=on_sent>
                            {move || format!("I HAVE SENT {}", due().map(fmt_usd).unwrap_or_default())}
                        </button>
                        <p class="docs-note">{move || {
                            let n = note.get();
                            if n.is_empty() { "this opens a chat with the valley with the payment filled in, and marks the flat as requested by you".to_string() } else { n }
                        }}</p>
                        <ol class="pay-after">
                            {AFTER.iter().map(|s| view! { <li>{*s}</li> }).collect_view()}
                        </ol>
                    </div>
                }.into_any(),
            }}
            {move || {
                let mine = sent_for(&sent.get(), &pid_rows);
                if mine.is_empty() {
                    return view! { <span></span> }.into_any();
                }
                view! {
                    <div class="deal-history">
                        {mine.into_iter().map(|p| view! {
                            <div class="deal-event">
                                <span class="deal-event-step">"SENT"</span>
                                <span class="deal-event-note">{format!("{} via {} · {} · awaiting the Company's confirmation", fmt_usd(p.usd), p.method, p.reference)}</span>
                                <span class="deal-event-ts">{fmt_ts(p.ts_ms)}</span>
                            </div>
                        }).collect_view()}
                    </div>
                }.into_any()
            }}
        </section>
    }
}
