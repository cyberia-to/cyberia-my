//! `/sell` — the way a flat gets onto the map for resale or sublease. One
//! page for the two people who may put it there: the flat's holder, and an
//! agent the holder gave a mandate. Five steps, each closed by a signature
//! of the wallet: who you are, which flat, the offer, who sells it, and the
//! track of what was signed. Every signature is an act the gate commits
//! (`gate/acts.py`); the valley approves an offer before the map shows it.

use crate::globe::BASE;
use crate::land::{load_map, FLAG_SVG};
use crate::market::{self, short, View};
use crate::nav::CyberiaNav;
use crate::orders::{support_href, SUPPORT_HANDLE};
use crate::terms::{bare_name, fmt_usd};
use leptos::prelude::*;

/// Run one signed act; the signer's view comes back on success.
pub fn submit(
    view: RwSignal<Option<View>>,
    busy: RwSignal<bool>,
    err: RwSignal<String>,
    kind: &'static str,
    fields: Vec<(&'static str, String)>,
) {
    busy.set(true);
    err.set(String::new());
    leptos::task::spawn_local(async move {
        match market::act(kind, &fields).await {
            Ok(v) => view.set(Some(v)),
            Err(e) => err.set(e),
        }
        busy.set(false);
    });
}

/// Connect and prove the wallet; keep the session.
pub fn enter(view: RwSignal<Option<View>>, busy: RwSignal<bool>, err: RwSignal<String>) {
    busy.set(true);
    err.set(String::new());
    leptos::task::spawn_local(async move {
        match market::sign_in().await {
            Ok(v) => view.set(Some(v)),
            Err(e) => err.set(e),
        }
        busy.set(false);
    });
}

fn plot_name(id: &str) -> String {
    load_map().phase0.iter().find(|f| f.id == id).map(|f| bare_name(&f.name)).unwrap_or_else(|| id.to_uppercase())
}

/// A text field bound to a signal.
fn field(label: &'static str, hint: &'static str, sig: RwSignal<String>, kind: &'static str) -> impl IntoView {
    view! {
        <label class="sl-field">
            <span>{label}</span>
            <input type=kind placeholder=hint prop:value=move || sig.get() on:input=move |ev| sig.set(event_target_value(&ev)) />
        </label>
    }
}

const STEPS: &[(&str, &str)] = &[
    ("WALLET", "who you are"),
    ("THE FLAT", "yours, or one you represent"),
    ("THE OFFER", "resale or sublease, the price"),
    ("WHO SELLS", "you or your agent"),
    ("TRACK", "signed, approved, on the map"),
];

#[component]
pub fn SellPage() -> impl IntoView {
    document().set_title("Cyberia — put your flat on the map");
    let view = RwSignal::new(None::<View>);
    let busy = RwSignal::new(false);
    let err = RwSignal::new(String::new());
    leptos::task::spawn_local(async move {
        if let Some(v) = market::fetch_me().await {
            view.set(Some(v));
        }
    });

    let step = RwSignal::new(0usize);
    let plot = RwSignal::new(String::new());
    let deal = RwSignal::new("sale".to_string());
    let price = RwSignal::new(String::new());
    let term = RwSignal::new(String::new());
    let words = RwSignal::new(String::new());
    let contact = RwSignal::new(String::new());
    let p_name = RwSignal::new(String::new());
    let p_contact = RwSignal::new(String::new());
    let c_plot = RwSignal::new(String::new());
    let agent = RwSignal::new(String::new());
    let commission = RwSignal::new("5".to_string());
    let until = RwSignal::new(String::new());
    let scope = RwSignal::new("both".to_string());
    let who = RwSignal::new("me".to_string());

    // a signed-in visitor starts at the flat; the profile fills from what the gate knows
    Effect::new(move |seen: Option<bool>| {
        let Some(v) = view.get() else { return false };
        if seen != Some(true) {
            if let Some(p) = &v.profile {
                p_name.set(p.name.clone());
                p_contact.set(p.contact.clone());
                contact.set(p.contact.clone());
            }
            if step.get_untracked() == 0 {
                step.set(1);
            }
        }
        true
    });

    // the flats this address may offer: its own, and those it holds a live mandate for
    let mine = Memo::new(move |_| {
        view.get()
            .map(|v| {
                let mut out: Vec<(String, &'static str)> = v.holders.iter().map(|h| (h.plot.clone(), "you hold it")).collect();
                for m in v.mandates.iter().filter(|m| m.status == "live" && v.me(&m.agent)) {
                    if !out.iter().any(|(p, _)| p == &m.plot) {
                        out.push((m.plot.clone(), "you represent its holder"));
                    }
                }
                out
            })
            .unwrap_or_default()
    });
    let i_hold = Memo::new(move |_| {
        let p = plot.get();
        view.get().map(|v| v.holders.iter().any(|h| h.plot == p)).unwrap_or(false)
    });

    let flats: Vec<(String, String)> = load_map().phase0.iter().map(|f| (f.id.clone(), bare_name(&f.name))).collect();

    let p_wallet = move || match view.get() {
        None => view! {
            <p class="dl-note">"your wallet is your signature here. connect it, sign one line to prove the address, and every step after is an act you sign — committed, verifiable, yours."</p>
            <button type="button" class="dl-go" disabled=move || busy.get() on:click=move |_| enter(view, busy, err)>"CONNECT YOUR WALLET →"</button>
            <p class="dl-note">"MetaMask, Rabby or any wallet in this browser · nothing is paid and nothing is sent on a chain"</p>
        }.into_any(),
        Some(v) => {
            let role = match v.role.as_str() { "root" | "admin" => "an admin of the valley", "agent" => "an accredited agent", _ => "signed in" };
            view! {
                <div class="dl-rows">
                    <div><span class="dl-k">"address"</span><b class="mono">{v.address.clone()}</b></div>
                    <div><span class="dl-k">"here you are"</span><b>{role}</b></div>
                </div>
                <div class="dl-q">"the name and the contact buyers see"</div>
                <div class="sl-grid">
                    {field("name", "Ana · Bali Land Co.", p_name, "text")}
                    {field("telegram", "@handle", p_contact, "text")}
                </div>
                <button type="button" class="dl-go" disabled=move || busy.get() || p_name.get().trim().is_empty()
                    on:click=move |_| submit(view, busy, err, "profile", vec![("name", p_name.get_untracked()), ("contact", p_contact.get_untracked())])>
                    "SIGN YOUR NAME →"
                </button>
                <div class="dl-actions">
                    <button type="button" class="dl-mini" on:click=move |_| step.set(1)>"NEXT · THE FLAT →"</button>
                    <button type="button" class="dl-mini" on:click=move |_| { market::sign_out(); crate::eth::forget_eth(); view.set(None); step.set(0); }>"SIGN OUT"</button>
                </div>
            }.into_any()
        }
    };

    let flats_c = flats.clone();
    let p_flat = move || {
        let Some(v) = view.get() else {
            return view! { <p class="dl-note">"connect your wallet first"</p> }.into_any();
        };
        let list = mine.get();
        let claims = v.claims.clone();
        let flats_o = flats_c.clone();
        view! {
            {if list.is_empty() {
                view! { <p class="dl-note">"no flat is bound to this address yet. the valley binds a flat to its holder's wallet once — claim yours below and the valley confirms it against the lease book."</p> }.into_any()
            } else {
                view! {
                    <div class="dl-q">"pick the flat you offer"</div>
                    <div class="dl-opts">
                        {list.into_iter().map(|(id, how)| {
                            let (id_c, id_s) = (id.clone(), id.clone());
                            view! {
                                <button type="button" class=move || if plot.get() == id_c { "dl-opt is-on" } else { "dl-opt" }
                                    on:click=move |_| { plot.set(id_s.clone()); step.set(2); }>
                                    <span class="dl-opt-t">{plot_name(&id)}</span>
                                    <span class="dl-opt-s">{how}</span>
                                </button>
                            }
                        }).collect_view()}
                    </div>
                }.into_any()
            }}
            <div class="dl-q">"claim a flat as yours"</div>
            <div class="sl-grid">
                <label class="sl-field">
                    <span>"flat"</span>
                    <select on:change=move |ev| c_plot.set(event_target_value(&ev))>
                        <option value="">"choose…"</option>
                        {flats_o.into_iter().map(|(id, name)| view! { <option value=id>{name}</option> }).collect_view()}
                    </select>
                </label>
            </div>
            <button type="button" class="dl-go" disabled=move || busy.get() || c_plot.get().is_empty()
                on:click=move |_| submit(view, busy, err, "claim", vec![
                    ("plot", c_plot.get_untracked()),
                    ("name", { let n = p_name.get_untracked(); if n.trim().is_empty() { "holder".to_string() } else { n } }),
                    ("contact", p_contact.get_untracked()),
                ])>
                "SIGN THE CLAIM →"
            </button>
            {(!claims.is_empty()).then(|| view! {
                <div class="dl-ledger">
                    {claims.into_iter().map(|c| view! {
                        <div class=if c.status == "bound" { "ok" } else { "" }>
                            <span>{c.status.to_uppercase()}</span><b>{plot_name(&c.plot)}</b>
                            <i>{match c.status.as_str() {
                                "pending" => "the valley checks it against the lease book".to_string(),
                                "bound" => "bound to your address".to_string(),
                                _ => if c.reason.is_empty() { "not confirmed".to_string() } else { c.reason.clone() },
                            }}</i>
                        </div>
                    }).collect_view()}
                </div>
            })}
        }.into_any()
    };

    let p_offer = move || {
        if plot.get().is_empty() {
            return view! { <p class="dl-note">"pick the flat first"</p> }.into_any();
        }
        view! {
            <div class="dl-q">{move || format!("what you offer on {}", plot_name(&plot.get()))}</div>
            <div class="dl-opts">
                <button type="button" class=move || if deal.get() == "sale" { "dl-opt is-on" } else { "dl-opt" } on:click=move |_| deal.set("sale".into())>
                    <span class="dl-opt-t">"RESALE"</span><span class="dl-opt-s">"the leasehold passes to the buyer for the years it has left"</span>
                </button>
                <button type="button" class=move || if deal.get() == "lease" { "dl-opt is-on" } else { "dl-opt" } on:click=move |_| deal.set("lease".into())>
                    <span class="dl-opt-t">"SUBLEASE"</span><span class="dl-opt-s">"you stay the holder; a tenant takes the flat for a term"</span>
                </button>
            </div>
            <div class="sl-grid">
                {move || if deal.get() == "sale" {
                    field("price, USD", "42000", price, "number").into_any()
                } else {
                    view! {
                        {field("rent a year, USD", "3000", price, "number")}
                        {field("term, years", "5", term, "number")}
                    }.into_any()
                }}
                {field("telegram for buyers", "@handle", contact, "text")}
            </div>
            <label class="sl-field">
                <span>"a few words — what stands on it, the view, the access"</span>
                <textarea rows="3" maxlength="600" prop:value=move || words.get() on:input=move |ev| words.set(event_target_value(&ev))></textarea>
            </label>
            <button type="button" class="dl-mini" disabled=move || price.get().trim().is_empty() on:click=move |_| step.set(3)>"NEXT · WHO SELLS →"</button>
        }.into_any()
    };

    let list_it = move || {
        let lease = deal.get_untracked() == "lease";
        let mut f = vec![
            ("plot", plot.get_untracked()),
            ("deal", deal.get_untracked()),
            ("price_usd", price.get_untracked()),
        ];
        if lease {
            f.push(("rent_usd_year", price.get_untracked()));
            f.push(("term_years", term.get_untracked()));
        }
        f.push(("words", words.get_untracked()));
        f.push(("contact", contact.get_untracked()));
        submit(view, busy, err, "listing", f);
        step.set(4);
    };

    let p_who = move || {
        if plot.get().is_empty() || price.get().trim().is_empty() {
            return view! { <p class="dl-note">"write the offer first"</p> }.into_any();
        }
        let summary = move || {
            let p: f64 = price.get().parse().unwrap_or(0.0);
            match deal.get().as_str() {
                "lease" => format!("{} · sublease · {} a year · {} years", plot_name(&plot.get()), fmt_usd(p), term.get()),
                _ => format!("{} · resale · {}", plot_name(&plot.get()), fmt_usd(p)),
            }
        };
        view! {
            <div class="dl-sum"><div><span class="dl-k">"the offer"</span><b>{summary}</b></div></div>
            {move || i_hold.get().then(|| view! {
                <div class="dl-q">"who sells it"</div>
                <div class="dl-opts">
                    <button type="button" class=move || if who.get() == "me" { "dl-opt is-on" } else { "dl-opt" } on:click=move |_| who.set("me".into())>
                        <span class="dl-opt-t">"I SELL IT MYSELF"</span><span class="dl-opt-s">"buyers write to you"</span>
                    </button>
                    <button type="button" class=move || if who.get() == "agent" { "dl-opt is-on" } else { "dl-opt" } on:click=move |_| who.set("agent".into())>
                        <span class="dl-opt-t">"MY AGENT SELLS IT"</span><span class="dl-opt-s">"you sign a mandate; the agent lists and answers"</span>
                    </button>
                </div>
            })}
            {move || if i_hold.get() && who.get() == "agent" {
                view! {
                    <div class="sl-grid">
                        {field("the agent's wallet address", "0x…", agent, "text")}
                        {field("commission, %", "5", commission, "number")}
                        {field("the mandate ends", "2027-06-30", until, "date")}
                        <label class="sl-field">
                            <span>"the mandate covers"</span>
                            <select on:change=move |ev| scope.set(event_target_value(&ev))>
                                <option value="both">"resale and sublease"</option>
                                <option value="sale">"resale only"</option>
                                <option value="lease">"sublease only"</option>
                            </select>
                        </label>
                    </div>
                    <button type="button" class="dl-go" disabled=move || busy.get() || agent.get().trim().len() != 42 || until.get().is_empty()
                        on:click=move |_| {
                            submit(view, busy, err, "mandate", vec![
                                ("plot", plot.get_untracked()),
                                ("agent", agent.get_untracked().trim().to_string()),
                                ("scope", scope.get_untracked()),
                                ("commission", commission.get_untracked()),
                                ("until", until.get_untracked()),
                            ]);
                            step.set(4);
                        }>
                        "SIGN THE MANDATE →"
                    </button>
                    <p class="dl-note">"the mandate names one address, one flat, what it may offer, the commission and the day it ends. your agent then opens this page with that wallet and lists the flat. you revoke it with one signature, and their listing comes down with it."</p>
                }.into_any()
            } else {
                view! {
                    <button type="button" class="dl-go" disabled=move || busy.get() on:click=move |_| list_it()>
                        {move || if i_hold.get() { "SIGN AND SEND THE OFFER →" } else { "SIGN THE OFFER AS THE AGENT →" }}
                    </button>
                    <p class="dl-note">"the valley reads the offer and approves it; then it shows on the flat's page and on the map"</p>
                }.into_any()
            }}
        }.into_any()
    };

    let p_track = move || {
        let Some(v) = view.get() else {
            return view! { <p class="dl-note">"connect your wallet first"</p> }.into_any();
        };
        let (listings, mandates) = (v.listings.clone(), v.mandates.clone());
        let v2 = v.clone();
        if listings.is_empty() && mandates.is_empty() {
            return view! { <p class="dl-note">"nothing is signed yet — your offers and mandates appear here with their state"</p> }.into_any();
        }
        view! {
            {(!listings.is_empty()).then(|| view! {
                <div class="dl-q">"offers"</div>
                <div class="sl-cards">
                    {listings.into_iter().rev().map(|l| {
                        let open = l.status == "pending" || l.status == "live";
                        let id = l.id.clone();
                        let state = match l.status.as_str() {
                            "pending" => "waits for the valley's approval".to_string(),
                            "live" => "live on the map".to_string(),
                            "rejected" => if l.reason.is_empty() { "not approved".to_string() } else { format!("not approved · {}", l.reason) },
                            "superseded" => "replaced by a newer offer".to_string(),
                            _ => if l.reason.is_empty() { "withdrawn".to_string() } else { format!("withdrawn · {}", l.reason) },
                        };
                        view! {
                            <div class=format!("sl-card {}", l.status)>
                                <div class="sl-card-top">
                                    <b>{plot_name(&l.plot)}</b>
                                    <span class="sl-tag">{l.status.to_uppercase()}</span>
                                </div>
                                <div class="sl-card-line">{format!(
                                    "{} · {}{} · signed by {}",
                                    if l.deal == "lease" { "sublease" } else { "resale" },
                                    fmt_usd(l.price_usd),
                                    if l.deal == "lease" { " a year" } else { "" },
                                    if l.role == "agent" { format!("the agent {}", v2.name_of(&l.by)) } else { "the holder".to_string() }
                                )}</div>
                                <div class="sl-card-line dim">{state}</div>
                                <div class="dl-actions">
                                    {(l.status == "live").then(|| view! { <a class="dl-mini" href=format!("{BASE}/plot/{}", l.plot)>"SEE IT ON THE FLAT'S PAGE"</a> })}
                                    {open.then(|| view! {
                                        <button type="button" class="dl-mini" disabled=move || busy.get()
                                            on:click=move |_| submit(view, busy, err, "withdraw", vec![("listing", id.clone())])>"WITHDRAW"</button>
                                    })}
                                </div>
                            </div>
                        }
                    }).collect_view()}
                </div>
            })}
            {(!mandates.is_empty()).then(|| {
                let v3 = v.clone();
                view! {
                    <div class="dl-q">"mandates"</div>
                    <div class="sl-cards">
                        {mandates.into_iter().rev().map(|m| {
                            let id = m.id.clone();
                            let mine_given = v3.me(&m.holder);
                            view! {
                                <div class=format!("sl-card {}", m.status)>
                                    <div class="sl-card-top">
                                        <b>{plot_name(&m.plot)}</b>
                                        <span class="sl-tag">{m.status.to_uppercase()}</span>
                                    </div>
                                    <div class="sl-card-line">{if mine_given {
                                        format!("you gave it to {} ({})", v3.name_of(&m.agent), short(&m.agent))
                                    } else {
                                        format!("given to you by {} ({})", v3.name_of(&m.holder), short(&m.holder))
                                    }}</div>
                                    <div class="sl-card-line dim">{format!(
                                        "{} · {}% commission · until {}",
                                        match m.scope.as_str() { "sale" => "resale", "lease" => "sublease", _ => "resale and sublease" },
                                        m.commission,
                                        m.until
                                    )}</div>
                                    {(m.status == "live").then(|| view! {
                                        <div class="dl-actions">
                                            <button type="button" class="dl-mini" disabled=move || busy.get()
                                                on:click=move |_| submit(view, busy, err, "revoke", vec![("mandate", id.clone())])>
                                                {if mine_given { "REVOKE" } else { "GIVE IT BACK" }}
                                            </button>
                                        </div>
                                    })}
                                </div>
                            }
                        }).collect_view()}
                    </div>
                }
            })}
        }.into_any()
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
            <div class="dl" style="--ink: #fcf000;">
                <aside class="dl-track">
                    <a class="dl-back" href=format!("{BASE}/map")>"← the valley map"</a>
                    <div class="dl-flat">"Put your flat on the map"</div>
                    <div class="dl-flat-s">"for holders and their agents · resale or sublease · every step is a signature"</div>
                    <div class="dl-progress"><i style:width=move || format!("{:.0}%", (step.get() + 1) as f64 / STEPS.len() as f64 * 100.0)></i></div>
                    <div class="dl-steps">
                        {STEPS.iter().enumerate().map(|(i, (title, line))| view! {
                            <button type="button" class={move || if step.get() == i { "dl-step on" } else if i < step.get() { "dl-step done" } else { "dl-step" }}
                                on:click=move |_| step.set(i)>
                                <span class="dl-step-n">{move || if i < step.get() { "✓".to_string() } else { (i + 1).to_string() }}</span>
                                <span class="dl-step-c"><b>{*title}</b><i>{*line}</i></span>
                            </button>
                        }).collect_view()}
                    </div>
                    <a class="dl-help" href=support_href() target="_blank" rel="noopener">{format!("a person answers · @{SUPPORT_HANDLE}")}</a>
                </aside>
                <section class="dl-panel">
                    <div class="dl-panel-h">
                        <span class="dl-panel-n">{move || format!("STEP {} OF {}", step.get() + 1, STEPS.len())}</span>
                        <h2>{move || STEPS[step.get()].0}</h2>
                        <p>{move || STEPS[step.get()].1}</p>
                    </div>
                    <div class="dl-body">
                        {move || { let e = err.get(); (!e.is_empty()).then(|| view! { <p class="dl-note warn">{e}</p> }) }}
                        {move || busy.get().then(|| view! { <p class="dl-note">"waiting for your wallet…"</p> })}
                        {move || match step.get() {
                            0 => p_wallet().into_any(),
                            1 => p_flat().into_any(),
                            2 => p_offer().into_any(),
                            3 => p_who().into_any(),
                            _ => p_track().into_any(),
                        }}
                    </div>
                </section>
            </div>
        </div>
    }
}
