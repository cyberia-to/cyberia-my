//! PAY — the card where a Holder chooses the form of the lease and pays the
//! premium or the whole price: to the Company's bank account, or on
//! Ethereum once the Company names its address. What the Company has
//! received comes from the published deal; what this browser sent is kept
//! here until the valley confirms it.

use crate::deals::fmt_ts;
use crate::orders::{support_href, PublishedDeal, SUPPORT_HANDLE};
use crate::pay::{
    load_sent, new_sent, record_sent, sent_for, sent_text, Form, BANK, ETH_ADDRESS, PREMIUM_STEPS,
};
use crate::terms::fmt_usd;
use leptos::prelude::*;

fn encode(s: &str) -> String {
    String::from(js_sys::encode_uri_component(s))
}

/// `price`: the book's plot price; `published`: the valley's deal, if any.
#[component]
pub fn PayPanel(
    plot_id: String,
    plot_name: String,
    price: Option<f64>,
    #[prop(into)] published: Signal<Option<PublishedDeal>>,
) -> impl IntoView {
    let form = RwSignal::new(Form::Upfront);
    let pct = RwSignal::new(30u32);
    let method = RwSignal::new("bank".to_string());
    let sent = RwSignal::new(load_sent());
    let note = RwSignal::new(String::new());

    let share = move || if form.get() == Form::Upfront { 100 } else { pct.get() };
    let due = move || price.map(|p| p * share() as f64 / 100.0);

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
        note.set("recorded here · tell support in the chat that opened".into());
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
                <span class="panel-kicker">"PAY"</span>
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
                                ("index", Some(p)) => format!(" · premium {p}% + century index"),
                                ("upfront", _) => " · upfront".to_string(),
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
                None => view! { <p class="docs-empty">"the price of this flat is set on request — write to support"</p> }.into_any(),
                Some(p) => view! {
                    <div class="pay-form">
                        <div class="pay-row">
                            <span class="pay-k">"plot price"</span>
                            <span class="pay-v">{fmt_usd(p)}</span>
                        </div>
                        <div class="pay-choice">
                            {[Form::Upfront, Form::Index].into_iter().map(|f| view! {
                                <button
                                    type="button"
                                    class=move || if form.get() == f { "pay-opt is-on" } else { "pay-opt" }
                                    on:click=move |_| form.set(f)
                                >{f.label()}</button>
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
                            <p class="docs-note">
                                "the rest is rent in century index quantities for the term, reset yearly within the collar, paid monthly — the figure for your premium is on "
                                <a href="https://cyberia.my/cx" target="_blank" rel="noopener" style="color: var(--cyber-green);">"cyberia.my/cx"</a>
                            </p>
                        })}
                        <div class="pay-row pay-due">
                            <span class="pay-k">{move || format!("due now · {}%", share())}</span>
                            <span class="pay-v">{move || due().map(fmt_usd).unwrap_or_default()}</span>
                        </div>
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
                                    <div class="pay-row"><span class="pay-k">"currencies"</span><span class="pay-v">{BANK.currencies}</span></div>
                                    <div class="pay-row"><span class="pay-k">"bank"</span><span class="pay-v">{format!("{} · {}", BANK.bank, BANK.branch)}</span></div>
                                    <div class="pay-row"><span class="pay-k">"bank address"</span><span class="pay-v">{BANK.bank_address}</span></div>
                                    <div class="pay-row"><span class="pay-k">"SWIFT"</span><span class="pay-v cabinet-mono">{BANK.swift}</span></div>
                                    <div class="pay-row"><span class="pay-k">"holder address"</span><span class="pay-v">{BANK.holder_address}</span></div>
                                    <div class="pay-row"><span class="pay-k">"reference"</span><span class="pay-v cabinet-mono">{move || crate::pay::reference(&pid_now, share())}</span></div>
                                    <p class="docs-note">"USD, EUR and AUD land as sent; IDR settles at Bank Indonesia's JISDOR rate on the invoice date"</p>
                                </div>
                            }.into_any()
                        } else {
                            match ETH_ADDRESS {
                                Some(a) => view! {
                                    <div class="pay-bank">
                                        <div class="pay-row"><span class="pay-k">"address"</span><span class="pay-v cabinet-mono">{a}</span></div>
                                        <p class="docs-note">"ETH or USDT on Ethereum mainnet · put the reference in the chat to support after sending"</p>
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
                        <button type="button" class="intent-commit lease" on:click=on_sent>"I HAVE SENT IT"</button>
                        <p class="docs-note">{move || note.get()}</p>
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
