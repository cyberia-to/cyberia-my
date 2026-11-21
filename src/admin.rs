//! `/admin` — the desk of the company that coordinates the land: what it has
//! to answer, and what it has to keep true. Built from what the company
//! actually controls, one menu per thing:
//!
//!   NOW     what waits for the valley, oldest first — the only list that must reach zero
//!   FLATS   the register: every flat, where it stands, who holds it
//!   DEALS   first sales: terms, money received against money due, the seven steps
//!   MARKET  holders' offers, their agents' mandates, accredited agents
//!   PEOPLE  which wallet holds which flat, claims, who may sign for the valley
//!   TRAIL   every act with its signer, and whether the machine is well
//!
//! Each menu is one module over the same `Desk` — `admin_now.rs`,
//! `admin_flats.rs`, `admin_deals.rs`, `admin_market.rs`,
//! `admin_people.rs` — and what is left here is the page itself: the shell,
//! the menu, and the handful of parts every section shares.
//!
//! The page changes nothing by itself: every button is an act the admin's
//! wallet signs and the gate commits (`gate/acts.py`).

use crate::admin_deals::deals;
use crate::admin_flats::flats;
use crate::admin_market::market_menu;
use crate::admin_now::{now, waiting};
use crate::admin_people::{people, trail};
use crate::globe::BASE;
use crate::land::{load_map, FLAG_SVG};
use crate::market::{self, short, DealRec, View};
use crate::nav::CyberiaNav;
use crate::orders::PublishedDeal;
use crate::pay::{due_at_signing, Form};
use crate::sell::{enter, submit};
use crate::terms::{bare_name, terms};
use leptos::prelude::*;
use std::collections::HashMap;

/// The seven steps of a first sale, in the order they are confirmed.
pub const STEPS: &[(&str, &str)] = &[
    ("request", "TERMS"),
    ("kyc", "IDENTITY"),
    ("dd", "DOSSIER"),
    ("premium", "PAYMENT"),
    ("docs", "PAPERS"),
    ("notary", "NOTARY"),
    ("signed", "SIGNED"),
];

/// menu · its line · its submenus
const MENU: &[(&str, &str, &[&str])] = &[
    ("NOW", "what waits for the valley", &[]),
    ("FLATS", "the register of the land", &["ALL", "OPEN", "IN DEAL", "HELD", "ON REQUEST"]),
    ("DEALS", "first sales: terms, money, steps", &["PIPELINE", "MONEY", "OPEN A DEAL", "NOTARY SHEET"]),
    ("MARKET", "holders' offers and agents", &["OFFERS", "MANDATES", "AGENTS"]),
    ("PEOPLE", "who holds, who may sign", &["HOLDERS", "CLAIMS", "ADMINS"]),
    ("TRAIL", "every act, and the machine", &["ACTS", "HEALTH"]),
];

/// The desk's signals — copied into every section.
#[derive(Clone, Copy)]
pub struct Desk {
    pub view: RwSignal<Option<View>>,
    pub busy: RwSignal<bool>,
    pub err: RwSignal<String>,
    pub menu: RwSignal<usize>,
    pub sub: RwSignal<usize>,
    /// the flat or the deal opened in place
    pub focus: RwSignal<String>,
    /// the hard force bot's published deals, for the flats the desk has not taken yet
    pub bot: RwSignal<HashMap<String, PublishedDeal>>,
    pub q: RwSignal<String>,
    /// the titles opened in the register
    pub groups: RwSignal<Vec<String>>,
    // one set of fields, reused by the form in view
    pub a: RwSignal<String>,
    pub b: RwSignal<String>,
    pub c: RwSignal<String>,
    pub d: RwSignal<String>,
    pub e: RwSignal<String>,
}

impl Desk {
    pub fn sign(self, kind: &'static str, fields: Vec<(&'static str, String)>) {
        submit(self.view, self.busy, self.err, kind, fields);
    }

    pub fn go(self, menu: usize, sub: usize) {
        self.menu.set(menu);
        self.sub.set(sub);
        self.focus.set(String::new());
        self.clear();
    }

    /// Open a flat or a deal in place — or close it again, when it is the
    /// one already open.
    pub fn look(self, id: &str) {
        let same = self.focus.get_untracked() == id;
        self.focus.set(if same { String::new() } else { id.to_string() });
        self.clear();
    }

    /// The five shared fields belong to whatever is open: they are emptied
    /// whenever the focus moves, so an amount or an address typed against
    /// one flat is never signed against the next.
    pub fn clear(self) {
        for s in [self.a, self.b, self.c, self.d, self.e] {
            s.set(String::new());
        }
    }
}

pub fn today() -> String {
    let d = js_sys::Date::new_0();
    format!("{:04}-{:02}-{:02}", d.get_utc_full_year(), d.get_utc_month() + 1, d.get_utc_date())
}

pub fn name_of(id: &str) -> String {
    load_map().phase0.iter().find(|f| f.id == id).map(|f| bare_name(&f.name)).unwrap_or_else(|| id.to_uppercase())
}

/// The money a deal is due at signing: the price it is counted at — the
/// deal's own, or the lease book's — under its form (`crate::pay`).
pub fn due_of(d: &DealRec) -> Option<f64> {
    let price = d.price_usd.or_else(|| terms(&d.plot).and_then(|t| t.price()))?;
    let form = if d.form == "index" { Form::Index } else { Form::Upfront };
    Some(due_at_signing(price, form, d.premium_pct.unwrap_or(100)))
}

/// A deal still in motion: neither signed nor closed. A closed deal keeps
/// its record and its ledger — the gate publishes it as `available` and the
/// flat is open again — but it waits for nothing.
pub fn in_motion(d: &DealRec) -> bool {
    d.status != "owned" && d.status != "available"
}

pub fn next_step(d: &DealRec) -> Option<(&'static str, &'static str)> {
    STEPS.iter().find(|(k, _)| !d.steps.contains_key(*k)).copied()
}

/// A button that signs one act.
pub fn act(d: Desk, label: impl Into<String>, class: &'static str, kind: &'static str, fields: Vec<(&'static str, String)>) -> impl IntoView {
    let label = label.into();
    view! {
        <button type="button" class=class disabled=move || d.busy.get() on:click=move |_| d.sign(kind, fields.clone())>{label}</button>
    }
}

pub fn field(label: &'static str, hint: &'static str, sig: RwSignal<String>) -> impl IntoView {
    view! {
        <label class="sl-field">
            <span>{label}</span>
            <input type="text" placeholder=hint prop:value=move || sig.get() on:input=move |ev| sig.set(event_target_value(&ev)) />
        </label>
    }
}

pub fn empty(text: &'static str) -> AnyView {
    view! { <p class="ad-empty">{text}</p> }.into_any()
}

#[component]
pub fn AdminPage() -> impl IntoView {
    document().set_title("Cyberia — the desk");
    let d = Desk {
        view: RwSignal::new(None),
        busy: RwSignal::new(false),
        err: RwSignal::new(String::new()),
        menu: RwSignal::new(0),
        sub: RwSignal::new(0),
        focus: RwSignal::new(String::new()),
        bot: RwSignal::new(HashMap::new()),
        q: RwSignal::new(String::new()),
        groups: RwSignal::new(Vec::new()),
        a: RwSignal::new(String::new()),
        b: RwSignal::new(String::new()),
        c: RwSignal::new(String::new()),
        d: RwSignal::new(String::new()),
        e: RwSignal::new(String::new()),
    };
    leptos::task::spawn_local(async move {
        if let Some(v) = market::fetch_me().await {
            d.view.set(Some(v));
        }
    });
    leptos::task::spawn_local(async move {
        let bot: Option<HashMap<String, PublishedDeal>> = crate::orders::fetch_bot_index().await;
        d.bot.set(bot.unwrap_or_default());
    });

    let badge = move |i: usize| -> Option<usize> {
        let v = d.view.get()?;
        let n = match i {
            0 => waiting(&v),
            2 => v.deals.iter().filter(|x| in_motion(x)).count(),
            3 => v.listings.iter().filter(|l| l.status == "pending").count(),
            4 => v.claims.iter().filter(|c| c.status == "pending").count(),
            _ => 0,
        };
        (n > 0).then_some(n)
    };

    let body = move || {
        let Some(v) = d.view.get() else {
            return view! {
                <p class="dl-note">"the desk opens to the wallet of the root or of an admin. connect it and sign one line; what you may do follows from the address."</p>
                <crate::wallet_ui::WalletConnect on_connected=Callback::new(move |w| enter(d.view, d.busy, d.err, w)) />
            }.into_any();
        };
        if !v.is_admin() {
            return view! {
                <p class="dl-note warn">{format!("{} is not an admin of the valley. the root names admins.", short(&v.address))}</p>
                <button type="button" class="dl-mini" on:click=move |_| { market::sign_out(); crate::eth::forget_eth(); d.view.set(None); }>"SIGN OUT"</button>
            }.into_any();
        };
        match d.menu.get() {
            0 => now(d, &v),
            1 => flats(d, &v),
            2 => deals(d, &v),
            3 => market_menu(d, &v),
            4 => people(d, &v),
            _ => trail(d, &v),
        }
    };

    view! {
        <div class="page-shell cities-shell plot-shell">
            <div class="site-chrome cyberia-chrome">
                <div class="chrome-inner">
                    <div class="header-row1">
                        <div class="logo-zone">
                            <h1 class="logo">
                                <a href=format!("{BASE}/") class="brand-flag" title="home" inner_html=FLAG_SVG></a>
                                <span style="color: var(--cyber-green);">"cyber"</span>
                                <span style="color: var(--cyber-green); margin: 0 1px;">"•"</span>
                                <span style="color: #fff;">"ia"</span>
                            </h1>
                        </div>
                        <CyberiaNav active="cabinet" />
                    </div>
                </div>
            </div>
            <div class="dl ad" style="--ink: #fcf000;">
                <aside class="dl-track">
                    <a class="dl-back" href=format!("{BASE}/map")>"← the valley map"</a>
                    <div class="dl-flat">"The desk"</div>
                    <div class="dl-flat-s">{move || match d.view.get() {
                        Some(v) if v.is_admin() => format!("{} · {}", if v.role == "root" { "root" } else { "admin" }, short(&v.address)),
                        _ => "the company that coordinates the land · every answer is a signature".to_string(),
                    }}</div>
                    <div class="dl-steps" style="margin-top: 12px;">
                        {MENU.iter().enumerate().map(|(i, (title, line, _))| view! {
                            <button type="button" class=move || if d.menu.get() == i { "dl-step on" } else { "dl-step" } on:click=move |_| d.go(i, 0)>
                                <span class=move || if badge(i).is_some() { "dl-step-n hot" } else { "dl-step-n" }>{move || badge(i).map(|n| n.to_string()).unwrap_or_else(|| "·".to_string())}</span>
                                <span class="dl-step-c"><b>{*title}</b><i>{*line}</i></span>
                            </button>
                        }).collect_view()}
                    </div>
                    {move || d.view.get().is_some().then(|| view! {
                        <button type="button" class="dl-withdraw" style="margin-top: auto;" on:click=move |_| { market::sign_out(); crate::eth::forget_eth(); d.view.set(None); }>"sign out"</button>
                    })}
                </aside>
                <section class="dl-panel">
                    <div class="dl-panel-h">
                        <span class="dl-panel-n">{move || MENU[d.menu.get()].1.to_uppercase()}</span>
                        <h2>{move || MENU[d.menu.get()].0}</h2>
                        {move || {
                            let subs = MENU[d.menu.get()].2;
                            (!subs.is_empty()).then(|| view! {
                                <div class="ad-subs" role="tablist">
                                    {subs.iter().enumerate().map(|(i, s)| view! {
                                        <button type="button" role="tab" class=move || if d.sub.get() == i { "ad-sub on" } else { "ad-sub" }
                                            on:click=move |_| { let m = d.menu.get_untracked(); d.go(m, i); }>{*s}</button>
                                    }).collect_view()}
                                </div>
                            })
                        }}
                    </div>
                    <div class="dl-body ad-body">
                        {move || { let e = d.err.get(); (!e.is_empty()).then(|| view! { <p class="dl-note warn">{e}</p> }) }}
                        {move || d.busy.get().then(|| view! { <p class="dl-note">"waiting for your wallet…"</p> })}
                        {body}
                    </div>
                </section>
            </div>
        </div>
    }
}
