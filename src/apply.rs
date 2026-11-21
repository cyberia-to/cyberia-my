//! The ways into Cyberia that are not a flat's offer — land to add to the
//! map, an agent's accreditation, the franchise of a territory, a city to
//! found. One funnel for the four, the same five steps `/sell` has: the
//! wallet, the thing, the terms, one signature, the track. What is signed is
//! an `apply` act the gate commits (`gate/acts.py`); the valley answers it
//! on the desk, and the answer shows here.
//!
//! An accredited agent's track is their desk: the two links they work with
//! (one for holders, one for everyone they bring), the mandates they hold
//! and what the people they sent applied for.

use crate::apply_kinds::kind_of;
use crate::globe::BASE;
use crate::land::{load_map, FLAG_SVG};
use crate::market::{self, query_value, short, Application, Board, View};
use crate::nav::CyberiaNav;
use crate::orders::{support_href, SUPPORT_HANDLE};
use crate::sell::{enter, submit};
use crate::terms::bare_name;
use leptos::prelude::*;

fn field(label: &'static str, hint: &'static str, sig: RwSignal<String>, kind: &'static str) -> impl IntoView {
    view! {
        <label class="sl-field">
            <span>{label}</span>
            <input type=kind placeholder=hint prop:value=move || sig.get() on:input=move |ev| sig.set(event_target_value(&ev)) />
        </label>
    }
}

fn status_line(a: &Application, wait: &'static str) -> String {
    match a.status.as_str() {
        "pending" => wait.to_string(),
        "accepted" => "accepted by the valley".to_string(),
        "rejected" if a.reason.is_empty() => "not accepted".to_string(),
        "rejected" => format!("not accepted · {}", a.reason),
        "superseded" => "replaced by a newer application".to_string(),
        _ => "withdrawn".to_string(),
    }
}

/// Where this site stands, for a link an agent hands out.
fn origin() -> String {
    format!("{}{BASE}", window().location().origin().unwrap_or_default())
}

/// A link to copy: shown whole, selected by a tap.
fn link_row(label: &'static str, line: &'static str, url: String) -> impl IntoView {
    view! {
        <label class="sl-field ap-link">
            <span>{label}</span>
            <input type="text" readonly=true prop:value=url on:focus=move |ev| {
                use wasm_bindgen::JsCast;
                if let Some(i) = ev.target().and_then(|t| t.dyn_into::<web_sys::HtmlInputElement>().ok()) { i.select(); }
            } />
            <i>{line}</i>
        </label>
    }
}

/// The desk of an accredited agent: the links, the mandates, the people sent.
fn agent_desk(v: &View) -> AnyView {
    let me = v.address.clone();
    let held: Vec<_> = v.mandates.iter().filter(|m| m.status == "live" && v.me(&m.agent)).cloned().collect();
    let offers = v.listings.iter().filter(|l| v.me(&l.by) && (l.status == "live" || l.status == "pending")).count();
    let referred = v.referred.clone();
    let vv = v.clone();
    view! {
        <div class="ap-badge"><b>"ACCREDITED AGENT"</b><span>{format!("{} · {}", v.name_of(&me), short(&me))}</span></div>
        <div class="ap-nums">
            <div><b>{held.len()}</b><span>"mandates you hold"</span></div>
            <div><b>{offers}</b><span>"offers you signed, open"</span></div>
            <div><b>{referred.len()}</b><span>"applications you brought"</span></div>
        </div>
        <div class="dl-q">"1 · win a mandate — send a holder this link"</div>
        {link_row("for a holder", "it opens SELL with your address already named as the agent; the holder signs the mandate, you list the flat", format!("{}/sell?agent={me}", origin()))}
        <div class="dl-q">"2 · bring people — your link"</div>
        {link_row("for everyone else", "land, a franchise, a city, another agent: what they sign after this link names you as the one who sent them", format!("{}/?ref={me}", origin()))}
        <div class="dl-q">"3 · list the flats you hold a mandate for"</div>
        {if held.is_empty() {
            view! { <p class="dl-note">"no holder has signed a mandate for your address yet"</p> }.into_any()
        } else {
            view! {
                <div class="sl-cards">
                    {held.into_iter().map(|m| {
                        let name = load_map().phase0.iter().find(|f| f.id == m.plot).map(|f| bare_name(&f.name)).unwrap_or_else(|| m.plot.to_uppercase());
                        view! {
                            <div class="sl-card live">
                                <div class="sl-card-top"><b>{name}</b><span class="sl-tag">"MANDATE"</span></div>
                                <div class="sl-card-line">{format!("from {} · {} · {}% · until {}", vv.name_of(&m.holder), match m.scope.as_str() { "sale" => "resale", "lease" => "sublease", _ => "resale and sublease" }, m.commission, m.until)}</div>
                                <div class="dl-actions"><a class="dl-mini" href=format!("{BASE}/sell")>"LIST IT →"</a></div>
                            </div>
                        }
                    }).collect_view()}
                </div>
            }.into_any()
        }}
        {(!referred.is_empty()).then(|| view! {
            <div class="dl-q">"what the people you sent applied for"</div>
            <div class="dl-ledger">
                {referred.into_iter().rev().map(|r| view! {
                    <div class=if r.status == "accepted" { "ok" } else { "" }><span>{r.status.to_uppercase()}</span><b>{r.what.to_uppercase()}</b><i>{r.at.get(..10).unwrap_or("").to_string()}</i></div>
                }).collect_view()}
            </div>
        })}
    }.into_any()
}

#[component]
pub fn LandPage() -> impl IntoView {
    view! { <ApplyPage what="land" /> }
}

#[component]
pub fn AgentPage() -> impl IntoView {
    view! { <ApplyPage what="agent" /> }
}

#[component]
pub fn FranchisePage() -> impl IntoView {
    view! { <ApplyPage what="franchise" /> }
}

#[component]
pub fn FoundPage() -> impl IntoView {
    view! { <ApplyPage what="city" /> }
}

#[component]
fn ApplyPage(what: &'static str) -> impl IntoView {
    let k = kind_of(what);
    document().set_title(k.doc_title);
    let steps: [(&'static str, &'static str); 5] = [
        ("WALLET", "who you are"),
        k.thing,
        k.terms_step,
        ("SIGN", "one signature sends it"),
        ("TRACK", if what == "agent" { "the answer, then your desk" } else { "the valley's answer" }),
    ];

    let view = RwSignal::new(None::<View>);
    let busy = RwSignal::new(false);
    let err = RwSignal::new(String::new());
    let board = RwSignal::new(Board::default());
    leptos::task::spawn_local(async move {
        if let Some(v) = market::fetch_me().await {
            view.set(Some(v));
        }
    });
    if what == "franchise" {
        leptos::task::spawn_local(async move { board.set(market::fetch_board().await) });
    }

    // what the globe handed over: the point, the name, the territory
    let search = window().location().search().unwrap_or_default();
    let q = |key: &str| RwSignal::new(query_value(&search, key).unwrap_or_default());
    let step = RwSignal::new(0usize);
    let name = RwSignal::new(String::new());
    let contact = RwSignal::new(String::new());
    let (territory, place, city) = (q("territory"), q("place"), q("city"));
    let (lat, lon, code) = (q("lat"), q("lon"), q("code"));
    let level = RwSignal::new(query_value(&search, "level").unwrap_or_else(|| "state".to_string()));
    let ares = RwSignal::new(String::new());
    let right = RwSignal::new(String::new());
    let deal = RwSignal::new("both".to_string());
    let entity = RwSignal::new(String::new());
    let languages = RwSignal::new(String::new());
    let words = RwSignal::new(String::new());
    let accepted = RwSignal::new(false);

    let mine = move || view.get().map(|v| v.applications.iter().filter(|a| a.what == what && v.me(&a.from)).cloned().collect::<Vec<_>>()).unwrap_or_default();
    let is_agent = move || what == "agent" && view.get().map(|v| v.role == "agent").unwrap_or(false);

    // a signed-in visitor starts at the thing — or at the track, when something of theirs is there already
    Effect::new(move |seen: Option<bool>| {
        let Some(v) = view.get() else { return false };
        if seen != Some(true) {
            if let Some(p) = &v.profile {
                name.set(p.name.clone());
                contact.set(p.contact.clone());
            }
            if step.get_untracked() == 0 {
                let has = v.applications.iter().any(|a| a.what == what && v.me(&a.from) && (a.status == "pending" || a.status == "accepted"));
                step.set(if has || (what == "agent" && v.role == "agent") { 4 } else { 1 });
            }
        }
        true
    });

    let point_ok = move || lat.get().trim().parse::<f64>().is_ok() && lon.get().trim().parse::<f64>().is_ok();
    let ready = move || {
        let who = !name.get().trim().is_empty() && !contact.get().trim().is_empty();
        who && match what {
            "land" => !place.get().trim().is_empty(),
            "city" => !city.get().trim().is_empty() && point_ok(),
            _ => !territory.get().trim().is_empty(),
        }
    };
    let subject = move || match what {
        "land" => place.get(),
        "city" => city.get(),
        "agent" => format!("agent · {}", territory.get()),
        _ => format!("{} · {}", territory.get(), level.get()),
    };

    let p_wallet = move || match view.get() {
        None => view! {
            <p class="dl-note">"your wallet is your signature here. connect it, sign one line to prove the address, and the application is one more signature — committed, verifiable, yours."</p>
            <crate::wallet_ui::WalletConnect on_connected=Callback::new(move |w| enter(view, busy, err, w)) />
            <p class="dl-note">"MetaMask, Rabby or any wallet in this browser · nothing is paid and nothing is sent on a chain"</p>
        }.into_any(),
        Some(v) => view! {
            <div class="dl-rows">
                <div><span class="dl-k">"address"</span><b class="mono">{v.address.clone()}</b></div>
            </div>
            <div class="dl-actions">
                <button type="button" class="dl-go" on:click=move |_| step.set(1)>{format!("NEXT · {} →", k.thing.0)}</button>
                <button type="button" class="dl-mini" on:click=move |_| { market::sign_out(); crate::eth::forget_eth(); view.set(None); step.set(0); }>"SIGN OUT"</button>
            </div>
        }.into_any(),
    };

    let who_fields = move || view! {
        <div class="dl-q">"who the valley writes to"</div>
        <div class="sl-grid">
            {field(if what == "agent" { "the name buyers and holders see" } else { "your name" }, "Ana · Bali Land Co.", name, "text")}
            {field("telegram", "@handle", contact, "text")}
        </div>
    };
    let point_fields = move |mark: &'static str| view! {
        <div class="sl-grid">
            {field("latitude", "-8.5069", lat, "text")}
            {field("longitude", "115.2625", lon, "text")}
        </div>
        <div class="dl-actions"><a class="dl-mini" href=format!("{BASE}/?mark={mark}")>"MARK IT ON THE GLOBE →"</a></div>
    };
    let words_field = move |label: &'static str| view! {
        <label class="sl-field">
            <span>{label}</span>
            <textarea rows="3" maxlength="600" prop:value=move || words.get() on:input=move |ev| words.set(event_target_value(&ev))></textarea>
        </label>
    };

    let p_thing = move || {
        if view.get().is_none() {
            return view! { <p class="dl-note">"connect your wallet first"</p> }.into_any();
        }
        let form = match what {
            "land" => view! {
                <div class="dl-q">"the land"</div>
                <div class="sl-grid">
                    {field("where it is", "village, regency, island", place, "text")}
                    {field("size, ares", "25", ares, "number")}
                    <label class="sl-field">
                        <span>"the paper you hold"</span>
                        <select on:change=move |ev| right.set(event_target_value(&ev))>
                            <option value="">"choose…"</option>
                            <option value="SHM">"certificate of ownership · SHM"</option>
                            <option value="HGB">"right to build · HGB"</option>
                            <option value="leasehold">"a lease"</option>
                            <option value="other">"another paper"</option>
                        </select>
                    </label>
                    <label class="sl-field">
                        <span>"what you want for it"</span>
                        <select on:change=move |ev| deal.set(event_target_value(&ev))>
                            <option value="both">"sale or lease"</option>
                            <option value="sale">"sale"</option>
                            <option value="lease">"lease"</option>
                        </select>
                    </label>
                </div>
                {point_fields("land")}
                {words_field("a few words — the access, the view, what stands on it")}
            }.into_any(),
            "agent" => view! {
                <div class="dl-q">"where and how you sell"</div>
                <div class="sl-grid">
                    {field("territory — where your buyers are", "Bali · Jakarta · Europe", territory, "text")}
                    {field("languages", "English, Bahasa", languages, "text")}
                    {field("company, when there is one", "Bali Land Co.", entity, "text")}
                </div>
                {words_field("a few words — what you sold before, how buyers find you")}
            }.into_any(),
            "franchise" => view! {
                <div class="dl-q">"the territory"</div>
                <div class="dl-opts">
                    {[("state", "A STATE", "a whole country"), ("region", "A REGION", "a province, an island"), ("district", "A DISTRICT", "a regency, a city's land")].into_iter().map(|(v, t, s)| view! {
                        <button type="button" class=move || if level.get() == v { "dl-opt is-on" } else { "dl-opt" } on:click=move |_| level.set(v.to_string())>
                            <span class="dl-opt-t">{t}</span><span class="dl-opt-s">{s}</span>
                        </button>
                    }).collect_view()}
                </div>
                <div class="sl-grid">
                    {field("its name", "Portugal · Bali · Gianyar", territory, "text")}
                    {field("the company or the person who takes it", "Ana Lda.", entity, "text")}
                </div>
                <div class="dl-actions"><a class="dl-mini" href=format!("{BASE}/")>"PICK IT ON THE GLOBE →"</a></div>
                {move || {
                    let t = territory.get().trim().to_lowercase();
                    let given: Vec<_> = board.get().franchises.into_iter().filter(|f| !t.is_empty() && f.territory.to_lowercase() == t).collect();
                    (!given.is_empty()).then(|| view! {
                        <p class="dl-note warn">{format!("a franchise of {} is given already — to {}. an application still reaches the valley.", given[0].territory, given[0].by)}</p>
                    })
                }}
                {words_field("a few words — the land or the capital in hand, the people on the ground")}
            }.into_any(),
            _ => view! {
                <div class="dl-q">"the city"</div>
                <div class="sl-grid">
                    {field("the name of the city", "Nova", city, "text")}
                    {field("the state or the region it stands in", "Indonesia · Bali", territory, "text")}
                </div>
                {point_fields("city")}
                {words_field("a few words — the land you hold or are about to, who founds it with you")}
            }.into_any(),
        };
        view! {
            {form}
            {who_fields()}
            <button type="button" class="dl-go" disabled=move || !ready() on:click=move |_| step.set(2)>{format!("NEXT · {} →", k.terms_step.0)}</button>
        }.into_any()
    };

    let p_terms = move || view! {
        <div class="ap-terms">
            {k.terms.iter().enumerate().map(|(i, (t, line))| view! {
                <div class="ap-term"><span>{i + 1}</span><div><b>{*t}</b><i>{*line}</i></div></div>
            }).collect_view()}
        </div>
        <button type="button" class=move || if accepted.get() { "ap-accept on" } else { "ap-accept" } on:click=move |_| accepted.update(|a| *a = !*a)>
            <span class="ap-box">{move || if accepted.get() { "✓" } else { "" }}</span>{k.accept}
        </button>
        <button type="button" class="dl-go" disabled=move || !accepted.get() || !ready() on:click=move |_| step.set(3)>"NEXT · SIGN →"</button>
        {move || (!ready()).then(|| view! { <p class="dl-note">{format!("fill {} first", k.thing.0.to_lowercase())}</p> })}
    }.into_any();

    let send = move || {
        let g = |s: RwSignal<String>| s.get_untracked().trim().to_string();
        let mut f: Vec<(&'static str, String)> = vec![("what", what.to_string()), ("name", g(name)), ("contact", g(contact))];
        match what {
            "land" => f.extend([("place", g(place)), ("lat", g(lat)), ("lon", g(lon)), ("ares", g(ares)), ("right", g(right)), ("deal", g(deal))]),
            "agent" => f.extend([("territory", g(territory)), ("languages", g(languages)), ("entity", g(entity)), ("code", "accepted".to_string())]),
            "franchise" => f.extend([("territory", g(territory)), ("level", g(level)), ("code", g(code)), ("entity", g(entity))]),
            _ => f.extend([("city", g(city)), ("lat", g(lat)), ("lon", g(lon)), ("territory", g(territory))]),
        }
        f.push(("words", g(words)));
        f.push(("ref", market::referrer()));
        submit(view, busy, err, "apply", f);
        step.set(4);
    };

    let p_sign = move || {
        if !ready() || !accepted.get() {
            return view! { <p class="dl-note">{format!("fill {} and accept {} first", k.thing.0.to_lowercase(), k.terms_step.0.to_lowercase())}</p> }.into_any();
        }
        let sent_by = market::referrer();
        view! {
            <div class="dl-sum">
                <div><span class="dl-k">"you ask for"</span><b>{move || format!("{} · {}", k.title.to_lowercase(), subject())}</b></div>
                <div><span class="dl-k">"the valley writes to"</span><b>{move || format!("{} · {}", name.get(), contact.get())}</b></div>
                {move || point_ok().then(|| view! { <div><span class="dl-k">"the point"</span><b>{format!("{}, {}", lat.get(), lon.get())}</b></div> })}
                {(sent_by.len() == 42).then(|| view! { <div><span class="dl-k">"sent by the agent"</span><b class="mono">{short(&sent_by)}</b></div> })}
            </div>
            <button type="button" class="dl-go" disabled=move || busy.get() on:click=move |_| send()>"SIGN AND SEND →"</button>
            <p class="dl-note">"your wallet shows the application as plain text and signs it; nothing is paid and nothing is sent on a chain"</p>
        }.into_any()
    };

    let p_track = move || {
        let Some(v) = view.get() else {
            return view! { <p class="dl-note">"connect your wallet first"</p> }.into_any();
        };
        let list = mine();
        let desk = is_agent().then(|| agent_desk(&v));
        if list.is_empty() && desk.is_none() {
            return view! {
                <p class="dl-note">"nothing is signed yet — your application appears here with the valley's answer"</p>
                <button type="button" class="dl-mini" on:click=move |_| step.set(1)>{format!("START · {} →", k.thing.0)}</button>
            }.into_any();
        }
        view! {
            {desk}
            {(!list.is_empty()).then(|| view! {
                <div class="dl-q">"what you signed"</div>
                <div class="sl-cards">
                    {list.into_iter().rev().map(|a| {
                        let id = a.id.clone();
                        let class = match a.status.as_str() { "accepted" => "sl-card live", "pending" => "sl-card pending", _ => "sl-card rejected" };
                        let next = (a.status == "accepted").then(|| match what {
                            "land" => view! { <a class="dl-mini" href=format!("{BASE}/sell")>"PUT ITS FLATS UP FOR SALE →"</a> }.into_any(),
                            "franchise" => view! { <a class="dl-mini" href=format!("{BASE}/?mark=city")>"FOUND A CITY THERE →"</a> }.into_any(),
                            "city" => view! { <a class="dl-mini" href=format!("{BASE}/land")>"ADD ITS LAND →"</a> }.into_any(),
                            _ => view! { <a class="dl-mini" href=format!("{BASE}/sell")>"OPEN SELL →"</a> }.into_any(),
                        });
                        view! {
                            <div class=class>
                                <div class="sl-card-top"><b>{a.subject()}</b><span class="sl-tag">{a.status.to_uppercase()}</span></div>
                                <div class="sl-card-line dim">{status_line(&a, k.wait)}</div>
                                <div class="dl-actions">
                                    {next}
                                    {(a.status == "pending").then(|| view! {
                                        <button type="button" class="dl-mini" disabled=move || busy.get()
                                            on:click=move |_| submit(view, busy, err, "withdraw", vec![("application", id.clone())])>"WITHDRAW"</button>
                                    })}
                                </div>
                            </div>
                        }
                    }).collect_view()}
                </div>
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
                        <CyberiaNav active="earth" />
                    </div>
                </div>
            </div>
            <div class="dl" style="--ink: #fcf000;">
                <aside class="dl-track">
                    <a class="dl-back" href=format!("{BASE}/")>"← the globe"</a>
                    <div class="dl-flat">{k.title}</div>
                    <div class="dl-flat-s">{k.sub}</div>
                    <div class="dl-progress"><i style:width=move || format!("{:.0}%", (step.get() + 1) as f64 / 5.0 * 100.0)></i></div>
                    <div class="dl-steps">
                        {steps.into_iter().enumerate().map(|(i, (title, line))| view! {
                            <button type="button" class={move || if step.get() == i { "dl-step on" } else if i < step.get() { "dl-step done" } else { "dl-step" }}
                                on:click=move |_| step.set(i)>
                                <span class="dl-step-n">{move || if i < step.get() { "✓".to_string() } else { (i + 1).to_string() }}</span>
                                <span class="dl-step-c"><b>{title}</b><i>{line}</i></span>
                            </button>
                        }).collect_view()}
                    </div>
                    <a class="dl-help" href=support_href() target="_blank" rel="noopener">{format!("a person answers · @{SUPPORT_HANDLE}")}</a>
                </aside>
                <section class="dl-panel">
                    <div class="dl-panel-h">
                        <span class="dl-panel-n">{move || format!("STEP {} OF 5", step.get() + 1)}</span>
                        <h2>{move || if step.get() == 4 && is_agent() { "YOUR DESK" } else { steps[step.get()].0 }}</h2>
                        <p>{move || steps[step.get()].1}</p>
                    </div>
                    <div class="dl-body">
                        {move || { let e = err.get(); (!e.is_empty()).then(|| view! { <p class="dl-note warn">{e}</p> }) }}
                        {move || busy.get().then(|| view! { <p class="dl-note">"waiting for your wallet…"</p> })}
                        {move || match step.get() {
                            0 => p_wallet().into_any(),
                            1 => p_thing().into_any(),
                            2 => p_terms().into_any(),
                            3 => p_sign().into_any(),
                            _ => p_track().into_any(),
                        }}
                    </div>
                </section>
            </div>
        </div>
    }
}
