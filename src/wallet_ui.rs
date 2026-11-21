//! CONNECT A WALLET, as one piece every page uses: the wallets found in this
//! browser by name, one button each; what the wallet answered, in words a
//! person can act on; and, when there is none, how to get one.

use crate::eth::{connect_with, discover, wallets, EthWallet, WalletInfo};
use leptos::prelude::*;
use std::time::Duration;

/// `on_connected` receives the wallet once its account is given.
#[component]
pub fn WalletConnect(#[prop(into)] on_connected: Callback<EthWallet>) -> impl IntoView {
    let found = RwSignal::new(Vec::<WalletInfo>::new());
    let note = RwSignal::new(String::new());
    let busy = RwSignal::new(false);
    // wallets announce themselves a moment after they are asked, and some only put themselves on the
    // page later without announcing: look now, soon after, and then keep looking while none is found
    discover();
    found.set(wallets());
    set_timeout(move || { let _ = found.try_set(wallets()); }, Duration::from_millis(350));
    if let Ok(tick) = set_interval_with_handle(
        move || {
            if found.try_get_untracked().map(|f| f.is_empty()).unwrap_or(false) {
                let now = wallets();
                if !now.is_empty() {
                    let _ = found.try_set(now);
                }
            }
        },
        Duration::from_millis(1200),
    ) {
        on_cleanup(move || tick.clear());
    }
    // and whenever one announces itself later — an extension that wakes up after the page
    let late = window_event_listener_untyped("eip6963:announceProvider", move |_| {
        let _ = found.try_set(wallets());
    });
    on_cleanup(move || late.remove());

    let press = move |key: String, name: String| {
        busy.set(true);
        note.set(format!("{name} is asked — answer in its window"));
        leptos::task::spawn_local(async move {
            match connect_with(Some(key)).await {
                Ok(w) => {
                    note.set(String::new());
                    on_connected.run(w);
                }
                Err(e) => note.set(e),
            }
            busy.set(false);
        });
    };

    view! {
        <div class="wc">
            {move || {
                let list = found.get();
                if list.is_empty() {
                    return view! {
                        <div class="wc-none">"no wallet in this browser yet"</div>
                        <details class="cab-how">
                            <summary>"how to get a wallet · 5 minutes"</summary>
                            <ol>
                                <li>"on a computer: add the "<a href="https://metamask.io/download" target="_blank" rel="noopener">"MetaMask"</a>" or "<a href="https://rabby.io" target="_blank" rel="noopener">"Rabby"</a>" extension to your browser, then reload this page"</li>
                                <li>"on a phone: install the MetaMask or Rabby app, then open cyberia.my inside the app's own browser"</li>
                                <li>"create a wallet · write the 12 words on paper · nobody at the valley will ever ask for them"</li>
                            </ol>
                        </details>
                    }.into_any();
                }
                let many = list.len() > 1;
                view! {
                    {many.then(|| view! { <div class="wc-none">"several wallets are in this browser — pick the one you use"</div> })}
                    <div class="wc-list">
                        {list.into_iter().map(|w| {
                            let (key, name) = (w.key.clone(), w.name.clone());
                            view! {
                                <button type="button" class="wc-btn" disabled=move || busy.get() on:click=move |_| press(key.clone(), name.clone())>
                                    {(!w.icon.is_empty()).then(|| view! { <img src=w.icon.clone() alt="" /> })}
                                    <span>{format!("CONNECT {}", w.name.to_uppercase())}</span>
                                </button>
                            }
                        }).collect_view()}
                    </div>
                }.into_any()
            }}
            {move || { let n = note.get(); (!n.is_empty()).then(|| view! { <div class="wc-note">{n}</div> }) }}
        </div>
    }
}
