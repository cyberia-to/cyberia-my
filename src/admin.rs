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
//! The page changes nothing by itself: every button is an act the admin's
//! wallet signs and the gate commits (`gate/acts.py`).

use crate::deals::{flat_status, load_deals, FlatStatus};
use crate::globe::BASE;
use crate::land::{area_m2, load_map, LandFlat, FLAG_SVG};
use crate::market::{self, short, DealRec, View};
use crate::nav::CyberiaNav;
use crate::orders::PublishedDeal;
use crate::sell::{enter, submit};
use crate::terms::{bare_name, fmt_usd, land_use, terms};
use leptos::prelude::*;
use std::collections::HashMap;

/// Title data every lease recites — from the Company's documents.
const TITLE: &[(&str, &str)] = &[
    ("Landowner", "PT CYBER VALLEY ESTATE · PT PMA · Gesing, Banjar, Buleleng, Bali"),
    ("Akta Pendirian", "No. 01 of 04-01-2022 · AHU-0001412-AH.01.01 of 07-01-2022"),
    ("NIB", "2504220016017"),
    ("NPWP", "53.898.326.3-902.000"),
    ("Title", "HGB No. 00048/Gesing · 40,000 m² · parcel NIB 22040417.00276 · Surat Ukur No. 00086/2002"),
    ("Granted", "08-02-2023 under KEPMEN ATR/KBPN No. 1339/SK-HK.02/X/2022 · expiring 02-02-2053 · AJB No. 81/2023"),
    ("PKKPR", "No. 10082310215108014 · OSS 09-08-2023 · KBLI 68111 · 40,000 m²"),
    ("Notices", "pt.cybervalley.estate@gmail.com · 15:00 WITA"),
];

const STEPS: &[(&str, &str)] = &[
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
struct Desk {
    view: RwSignal<Option<View>>,
    busy: RwSignal<bool>,
    err: RwSignal<String>,
    menu: RwSignal<usize>,
    sub: RwSignal<usize>,
    /// the flat or the deal opened in place
    focus: RwSignal<String>,
    /// the hard force bot's published deals, for the flats the desk has not taken yet
    bot: RwSignal<HashMap<String, PublishedDeal>>,
    q: RwSignal<String>,
    /// the titles opened in the register
    groups: RwSignal<Vec<String>>,
    // one set of fields, reused by the form in view
    a: RwSignal<String>,
    b: RwSignal<String>,
    c: RwSignal<String>,
    d: RwSignal<String>,
    e: RwSignal<String>,
}

impl Desk {
    fn sign(self, kind: &'static str, fields: Vec<(&'static str, String)>) {
        submit(self.view, self.busy, self.err, kind, fields);
    }

    fn go(self, menu: usize, sub: usize) {
        self.menu.set(menu);
        self.sub.set(sub);
        self.focus.set(String::new());
        for s in [self.a, self.b, self.c, self.d, self.e] {
            s.set(String::new());
        }
    }
}

fn today() -> String {
    let d = js_sys::Date::new_0();
    format!("{:04}-{:02}-{:02}", d.get_utc_full_year(), d.get_utc_month() + 1, d.get_utc_date())
}

fn name_of(id: &str) -> String {
    load_map().phase0.iter().find(|f| f.id == id).map(|f| bare_name(&f.name)).unwrap_or_else(|| id.to_uppercase())
}

/// The price a deal is counted at, and the share due at signing.
fn due_of(d: &DealRec) -> Option<f64> {
    let price = d.price_usd.or_else(|| terms(&d.plot).and_then(|t| t.price()))?;
    let share = if d.form == "index" { d.premium_pct.unwrap_or(100) } else { 100 };
    Some(price * share as f64 / 100.0)
}

fn next_step(d: &DealRec) -> Option<(&'static str, &'static str)> {
    STEPS.iter().find(|(k, _)| !d.steps.contains_key(*k)).copied()
}

/// A button that signs one act.
fn act(d: Desk, label: impl Into<String>, class: &'static str, kind: &'static str, fields: Vec<(&'static str, String)>) -> impl IntoView {
    let label = label.into();
    view! {
        <button type="button" class=class disabled=move || d.busy.get() on:click=move |_| d.sign(kind, fields.clone())>{label}</button>
    }
}

fn field(label: &'static str, hint: &'static str, sig: RwSignal<String>) -> impl IntoView {
    view! {
        <label class="sl-field">
            <span>{label}</span>
            <input type="text" placeholder=hint prop:value=move || sig.get() on:input=move |ev| sig.set(event_target_value(&ev)) />
        </label>
    }
}

fn empty(text: &'static str) -> AnyView {
    view! { <p class="ad-empty">{text}</p> }.into_any()
}

// ── NOW ───────────────────────────────────────────────────────────────

fn waiting(v: &View) -> usize {
    v.listings.iter().filter(|l| l.status == "pending").count()
        + v.claims.iter().filter(|c| c.status == "pending").count()
        + v.deals.iter().filter(|d| d.status != "owned").count()
}

fn now(d: Desk, v: &View) -> AnyView {
    let offers: Vec<_> = v.listings.iter().filter(|l| l.status == "pending").cloned().collect();
    let claims: Vec<_> = v.claims.iter().filter(|c| c.status == "pending").cloned().collect();
    let deals: Vec<_> = v.deals.iter().filter(|x| x.status != "owned").cloned().collect();
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
            <div class="sl-cards">
                {offers.into_iter().map(|l| view! {
                    <div class="sl-card pending">
                        <div class="sl-card-top"><b>{name_of(&l.plot)}</b><span class="sl-tag">{if l.deal == "lease" { "SUBLEASE" } else { "RESALE" }}</span></div>
                        <div class="sl-card-line">{format!("{}{} · by {} {}", fmt_usd(l.price_usd), if l.deal == "lease" { " a year" } else { "" }, if l.role == "agent" { "the agent" } else { "the holder" }, vv.name_of(&l.by))}</div>
                        {(!l.words.is_empty()).then(|| view! { <div class="sl-card-line dim">{l.words.clone()}</div> })}
                        <div class="dl-actions">
                            {act(d, "APPROVE", "dl-go sm", "approve", vec![("listing", l.id.clone())])}
                            {act(d, "REJECT", "dl-mini", "reject", vec![("target", l.id.clone())])}
                        </div>
                    </div>
                }).collect_view()}
            </div>
        })}
        {(!claims.is_empty()).then(|| view! {
            <div class="dl-q">"claims — an address says a flat is theirs"</div>
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
                                {act(d, "REJECT", "dl-mini", "reject", vec![("target", c.id.clone())])}
                            </div>
                        </div>
                    }
                }).collect_view()}
            </div>
        })}
    }.into_any()
}

// ── FLATS ─────────────────────────────────────────────────────────────

fn flats(d: Desk, v: &View) -> AnyView {
    let map = load_map();
    let mut local = load_deals();
    let mut index = d.bot.get();
    for x in &v.deals {
        index.insert(x.plot.clone(), PublishedDeal { status: x.status.clone(), holder: x.holder.clone(), paid_usd: x.paid_usd, ..Default::default() });
    }
    crate::deals::merge_published(&mut local, &index);
    let want = d.sub.get();
    let q = d.q.get().to_lowercase();
    let rows: Vec<(LandFlat, FlatStatus)> = map
        .phase0
        .iter()
        .map(|f| (f.clone(), flat_status(&f.id, &local)))
        .filter(|(f, st)| match want {
            1 => *st == FlatStatus::Available,
            2 => *st == FlatStatus::Transfer,
            3 => *st == FlatStatus::Owned,
            4 => *st == FlatStatus::OnRequest,
            _ => true,
        } && (q.is_empty() || f.id.contains(&q) || bare_name(&f.name).to_lowercase().contains(&q)))
        .collect();
    let n = rows.len();
    let vv = v.clone();
    let row = move |(f, st): (LandFlat, FlatStatus)| {
                let id = f.id.clone();
                let (id_c, id_o) = (id.clone(), id.clone());
                let book = terms(&id);
                let lu = land_use(&id);
                let deal = vv.deals.iter().find(|x| x.plot == id).cloned();
                let bound = vv.holders.iter().find(|h| h.plot == id).cloned();
                let offer = vv.listings.iter().any(|l| l.plot == id && l.status == "live");
                let holder = deal.as_ref().map(|x| x.holder.clone()).filter(|h| !h.is_empty())
                    .or_else(|| bound.as_ref().map(|h| h.name.clone()).filter(|h| !h.is_empty()))
                    .or_else(|| book.and_then(|t| t.holder()).map(|h| h.to_string()))
                    .unwrap_or_else(|| "—".to_string());
                let open = move || d.focus.get() == id_c;
                let has_deal = deal.is_some();
                view! {
                    <button type="button" class=move || if d.focus.get() == id_o { "ad-row on" } else { "ad-row" } on:click={ let id = id.clone(); move |_| d.focus.update(|f| *f = if *f == id { String::new() } else { id.clone() }) }>
                        <b>{bare_name(&f.name)}</b>
                        <span style:color=lu.color()>{lu.short()}</span>
                        <span>{format!("{:.1} ares", area_m2(&f.coords) / 100.0)}</span>
                        <span>{book.and_then(|t| t.price()).map(fmt_usd).unwrap_or_else(|| "—".into())}</span>
                        <span class=match st { FlatStatus::Available => "ad-st open", FlatStatus::Transfer => "ad-st deal", FlatStatus::Owned => "ad-st held", _ => "ad-st" }>{st.label()}{offer.then_some(" · OFFERED")}</span>
                        <span>{holder}</span>
                        <span class="mono">{bound.as_ref().map(|h| short(&h.address)).unwrap_or_else(|| "—".into())}</span>
                    </button>
                    {move || open().then(|| {
                        let id = id.clone();
                        let (id_d, id_h) = (id.clone(), id.clone());
                        view! {
                            <div class="ad-open">
                                <div class="dl-actions">
                                    <a class="dl-mini" href=format!("{BASE}/plot/{id}") target="_blank">"THE FLAT'S PAGE"</a>
                                    <a class="dl-mini" href=format!("{BASE}/map?plot={id}") target="_blank">"ON THE MAP"</a>
                                    <button type="button" class="dl-mini" on:click=move |_| { d.go(2, if has_deal { 0 } else { 2 }); d.focus.set(id_d.clone()); d.a.set(id_d.clone()); }>
                                        {if has_deal { "ITS DEAL →" } else { "OPEN A DEAL ON IT →" }}
                                    </button>
                                </div>
                                <div class="dl-q">"bind the flat to its holder's wallet"</div>
                                <div class="sl-grid">
                                    {field("the holder's address", "0x…", d.b)}
                                    {field("the name shown", "Ana", d.c)}
                                </div>
                                <button type="button" class="dl-go sm" disabled=move || d.busy.get() || d.b.get().trim().len() != 42
                                    on:click=move |_| d.sign("holder", vec![("plot", id_h.clone()), ("address", d.b.get_untracked().trim().to_string()), ("name", d.c.get_untracked())])>
                                    "SIGN · BIND THE HOLDER"
                                </button>
                            </div>
                        }
                    })}
                }
    };
    // the upper level: the parcel a flat is a piece of. a group is open when the person opened it,
    // or when a search or a filter already narrowed the list
    let narrowed = want != 0 || !q.is_empty();
    let mut groups: Vec<(Option<&'static crate::titles::Title>, Vec<(LandFlat, FlatStatus)>)> = Vec::new();
    for (f, st) in rows {
        let t = crate::titles::title_of(&f.id);
        match groups.iter_mut().find(|g| g.0.map(|x| &x.name) == t.map(|x| &x.name)) {
            Some(g) => g.1.push((f, st)),
            None => groups.push((t, vec![(f, st)])),
        }
    }
    groups.sort_by(|a, b| a.0.is_none().cmp(&b.0.is_none()).then(b.1.len().cmp(&a.1.len())));
    let n_titles = groups.iter().filter(|g| g.0.is_some()).count();
    view! {
        <div class="ad-bar">
            <input class="ad-search" type="text" placeholder="find a flat: sinwood-7, avalon…" prop:value=move || d.q.get() on:input=move |ev| d.q.set(event_target_value(&ev)) />
            <span class="ad-count">{format!("{n} flats on {n_titles} titles")}</span>
            <button type="button" class="dl-mini" on:click=move |_| d.groups.update(|g| if g.is_empty() { *g = crate::titles::book().titles.iter().map(|t| t.name.clone()).chain(["—".to_string()]).collect() } else { g.clear() })>
                {move || if d.groups.get().is_empty() { "OPEN ALL" } else { "CLOSE ALL" }}
            </button>
        </div>
        <div class="ad-table">
            {groups.into_iter().map(|(t, list)| {
                let key = t.map(|t| t.name.clone()).unwrap_or_else(|| "—".to_string());
                let (k1, k2) = (key.clone(), key.clone());
                let (mut open_n, mut deal_n, mut held_n, mut ares) = (0, 0, 0, 0.0);
                for (f, st) in &list {
                    match st { FlatStatus::Available => open_n += 1, FlatStatus::Transfer => deal_n += 1, FlatStatus::Owned => held_n += 1, _ => {} }
                    ares += area_m2(&f.coords) / 100.0;
                }
                let total = list.len();
                let row = row.clone();
                view! {
                    <button type="button" class=move || if narrowed || d.groups.get().contains(&k1) { "ad-title on" } else { "ad-title" }
                        on:click=move |_| d.groups.update(|g| if let Some(i) = g.iter().position(|x| x == &k2) { g.remove(i); } else { g.push(k2.clone()); })>
                        <span class="ad-title-n">
                            <b>{t.map(|t| t.name.to_uppercase()).unwrap_or_else(|| "NO TITLE ON THE MAP".to_string())}</b>
                            <i>{t.map(|t| t.held_line()).unwrap_or_else(|| "the map layer draws no parcel under these flats".to_string())}</i>
                        </span>
                        <span class="ad-title-f">{t.map(|t| t.facts()).unwrap_or_default()}</span>
                        <span class="ad-title-c">
                            <b>{format!("{total} flats · {ares:.0} ares")}</b>
                            <i>
                                {(open_n > 0).then(|| view! { <em class="open">{format!("{open_n} open")}</em> })}
                                {(deal_n > 0).then(|| view! { <em class="deal">{format!("{deal_n} in deal")}</em> })}
                                {(held_n > 0).then(|| view! { <em class="held">{format!("{held_n} held")}</em> })}
                            </i>
                        </span>
                    </button>
                    {move || (narrowed || d.groups.get().contains(&key)).then(|| view! {
                        <div class="ad-title-body">
                            <div class="ad-row head"><span>"flat"</span><span>"use"</span><span>"size"</span><span>"price in the book"</span><span>"stands"</span><span>"holder"</span><span>"wallet"</span></div>
                            {list.clone().into_iter().map(|x| row(x)).collect_view()}
                        </div>
                    })}
                }
            }).collect_view()}
        </div>
    }.into_any()
}

// ── DEALS ─────────────────────────────────────────────────────────────

fn deal_card(d: Desk, x: DealRec) -> AnyView {
    let plot = x.plot.clone();
    let (p_o, p_c) = (plot.clone(), plot.clone());
    let due = due_of(&x);
    let pct = due.map(|v| (x.paid_usd / v * 100.0).clamp(0.0, 100.0)).unwrap_or(0.0);
    let open = move || d.focus.get() == p_o;
    let steps = x.steps.clone();
    let payments = x.payments.clone();
    let status = x.status.clone();
    view! {
        <div class=if x.status == "owned" { "ad-deal done" } else { "ad-deal" }>
            <button type="button" class="ad-deal-h" on:click={ let p = plot.clone(); move |_| d.focus.update(|f| *f = if *f == p { String::new() } else { p.clone() }) }>
                <b>{name_of(&plot)}</b>
                <span>{if x.holder.is_empty() { "holder not named".to_string() } else { x.holder.clone() }}</span>
                <span class="sl-tag">{x.status.to_uppercase()}</span>
                <span class="ad-money">{format!("{}{}", fmt_usd(x.paid_usd), due.map(|v| format!(" / {}", fmt_usd(v))).unwrap_or_default())}</span>
            </button>
            <div class="ad-meter"><i style:width=format!("{pct:.0}%")></i></div>
            <div class="ad-pips">
                {STEPS.iter().map(|(k, t)| {
                    let on = steps.get(*k).cloned();
                    let p = plot.clone();
                    let done = on.is_some();
                    view! {
                        <button type="button" class=if done { "ad-pip done" } else { "ad-pip" } disabled=move || d.busy.get() || done
                            title=if done { "confirmed" } else { "press to confirm it today" }
                            on:click=move |_| d.sign("step", vec![("plot", p.clone()), ("step", k.to_string()), ("on", today())])>
                            <i></i><b>{*t}</b><span>{on.unwrap_or_else(|| "confirm".into())}</span>
                        </button>
                    }
                }).collect_view()}
            </div>
            {move || open().then(|| {
                let p = p_c.clone();
                let (p1, p2) = (p.clone(), p.clone());
                let payments = payments.clone();
                let status = status.clone();
                view! {
                    <div class="ad-open">
                        <div class="dl-q">"record money received"</div>
                        <div class="sl-grid">
                            {field("USD", "7950", d.b)}
                            {field("reference", "LEASE … PREMIUM 30%", d.c)}
                            {field("received on", "today", d.d)}
                            <label class="sl-field"><span>"how"</span>
                                <select on:change=move |ev| d.e.set(event_target_value(&ev))>
                                    <option value="bank">"bank transfer"</option>
                                    <option value="eth">"Ethereum"</option>
                                </select>
                            </label>
                        </div>
                        <button type="button" class="dl-go sm" disabled=move || d.busy.get() || d.b.get().trim().is_empty()
                            on:click=move |_| {
                                d.sign("payment", vec![
                                    ("plot", p1.clone()), ("usd", d.b.get_untracked().trim().to_string()),
                                    ("method", { let m = d.e.get_untracked(); if m.is_empty() { "bank".to_string() } else { m } }),
                                    ("reference", d.c.get_untracked()),
                                    ("on", { let o = d.d.get_untracked(); if o.trim().is_empty() { today() } else { o.trim().to_string() } }),
                                ]);
                                d.b.set(String::new());
                            }>"SIGN · MONEY RECEIVED"</button>
                        {(!payments.is_empty()).then(|| view! {
                            <div class="dl-ledger">
                                {payments.iter().map(|pm| view! {
                                    <div class="ok"><span>{pm.on.clone()}</span><b>{fmt_usd(pm.usd)}</b><i>{format!("{} · {}", pm.method, pm.reference)}</i></div>
                                }).collect_view()}
                            </div>
                        })}
                        <div class="dl-q">"where the deal stands"</div>
                        <div class="dl-actions">
                            {["reserved", "transfer", "owned"].into_iter().map(|s| {
                                let on = status == s;
                                act(d, s.to_uppercase(), if on { "dl-mini on" } else { "dl-mini" }, "deal", vec![("plot", p2.clone()), ("status", s.to_string())])
                            }).collect_view()}
                            {act(d, "CLOSE THE DEAL · FLAT OPEN AGAIN", "dl-mini warn", "deal", vec![("plot", p.clone()), ("status", "available".to_string())])}
                            <a class="dl-mini" href=format!("{BASE}/plot/{p}/deal") target="_blank">"THE BUYER'S TRACK"</a>
                        </div>
                    </div>
                }
            })}
        </div>
    }.into_any()
}

fn deals(d: Desk, v: &View) -> AnyView {
    match d.sub.get() {
        1 => {
            let open: Vec<&DealRec> = v.deals.iter().filter(|x| x.status != "owned").collect();
            let received: f64 = v.deals.iter().map(|x| x.paid_usd).sum();
            let due: f64 = open.iter().filter_map(|x| due_of(x).map(|v| (v - x.paid_usd).max(0.0))).sum();
            let mut ledger: Vec<(String, String, f64, String, String)> = v
                .deals
                .iter()
                .flat_map(|x| x.payments.iter().map(move |p| (p.on.clone(), x.plot.clone(), p.usd, p.method.clone(), p.reference.clone())))
                .collect();
            ledger.sort_by(|a, b| b.0.cmp(&a.0));
            view! {
                <div class="ad-tiles">
                    <div class="ad-tile go"><span>"received"</span><b>{fmt_usd(received)}</b><i>{format!("{} payments", ledger.len())}</i></div>
                    <div class="ad-tile warn"><span>"still due at signing"</span><b>{fmt_usd(due)}</b><i>{format!("across {} open deals", open.len())}</i></div>
                    <div class="ad-tile"><span>"closed"</span><b>{v.deals.iter().filter(|x| x.status == "owned").count()}</b><i>"flats signed"</i></div>
                </div>
                <div class="dl-q">"the ledger — every receipt, newest first"</div>
                {if ledger.is_empty() { empty("no money is recorded yet") } else {
                    view! {
                        <div class="ad-table">
                            <div class="ad-row head l5"><span>"received on"</span><span>"flat"</span><span>"USD"</span><span>"how"</span><span>"reference"</span></div>
                            {ledger.into_iter().map(|(on, plot, usd, how, r)| view! {
                                <div class="ad-row l5"><span>{on}</span><b>{name_of(&plot)}</b><span class="ad-st open">{fmt_usd(usd)}</span><span>{how}</span><span>{r}</span></div>
                            }).collect_view()}
                        </div>
                    }.into_any()
                }}
            }.into_any()
        }
        2 => {
            let flats: Vec<(String, String)> = load_map().phase0.iter().map(|f| (f.id.clone(), bare_name(&f.name))).collect();
            let picked = d.a.get_untracked();
            view! {
                <p class="dl-note">"a buyer said yes outside the page, or the page's request needs the valley's terms: open the deal here. the flat turns reserved on the map."</p>
                <div class="sl-grid">
                    <label class="sl-field"><span>"flat"</span>
                        <select on:change=move |ev| d.a.set(event_target_value(&ev))>
                            <option value="">"choose…"</option>
                            {flats.into_iter().map(|(id, name)| { let sel = id == picked; view! { <option value=id selected=sel>{name}</option> } }).collect_view()}
                        </select>
                    </label>
                    {field("the name the holder agreed to show", "Ana", d.b)}
                    <label class="sl-field"><span>"form"</span>
                        <select on:change=move |ev| d.c.set(event_target_value(&ev))>
                            <option value="upfront">"all at signing"</option>
                            <option value="index">"premium + yearly rent in CX"</option>
                        </select>
                    </label>
                    {field("premium %, when not all at signing", "30", d.d)}
                    {field("price USD, when it differs from the book", "", d.e)}
                </div>
                <button type="button" class="dl-go" disabled=move || d.busy.get() || d.a.get().is_empty()
                    on:click=move |_| {
                        let form = { let f = d.c.get_untracked(); if f.is_empty() { "upfront".to_string() } else { f } };
                        d.sign("deal", vec![
                            ("plot", d.a.get_untracked()), ("status", "reserved".to_string()), ("holder", d.b.get_untracked()),
                            ("form", form), ("premium_pct", d.d.get_untracked().trim().to_string()), ("price_usd", d.e.get_untracked().trim().to_string()),
                        ]);
                        let p = d.a.get_untracked();
                        d.go(2, 0);
                        d.focus.set(p);
                    }>"SIGN · OPEN THE DEAL →"</button>
            }.into_any()
        }
        3 => view! {
            <p class="dl-note">"what every lease recites, for the notary's draft — from the Company's documents"</p>
            <div class="ad-table">
                {TITLE.iter().map(|(k, val)| view! { <div class="ad-row l2"><span>{*k}</span><b>{*val}</b></div> }).collect_view()}
            </div>
        }.into_any(),
        _ => {
            let mut list = v.deals.clone();
            list.sort_by(|a, b| (a.status == "owned").cmp(&(b.status == "owned")).then(b.updated.cmp(&a.updated)));
            let mine: Vec<String> = list.iter().map(|x| x.plot.clone()).collect();
            let from_bot: Vec<(String, PublishedDeal)> = d.bot.get().into_iter().filter(|(k, _)| !mine.contains(k)).collect();
            view! {
                {if list.is_empty() { empty("no deal is open — OPEN A DEAL starts one, or a buyer's request arrives in NOW") } else {
                    view! { <div class="ad-deals">{list.into_iter().map(|x| deal_card(d, x)).collect_view()}</div> }.into_any()
                }}
                {(!from_bot.is_empty()).then(|| view! {
                    <div class="dl-q">"published by the hard force bot — not yet on the desk"</div>
                    <div class="sl-cards">
                        {from_bot.into_iter().map(|(plot, pd)| view! {
                            <div class="sl-card">
                                <div class="sl-card-top"><b>{name_of(&plot)}</b><span class="sl-tag">{pd.status.to_uppercase()}</span></div>
                                <div class="sl-card-line dim">{format!("{} · {} received · written in the hard force chat", if pd.holder.is_empty() { "—" } else { pd.holder.as_str() }, fmt_usd(pd.paid_usd))}</div>
                                <div class="dl-actions">
                                    {act(d, "TAKE IT ONTO THE DESK", "dl-mini", "deal", vec![("plot", plot.clone()), ("status", pd.status.clone()), ("holder", pd.holder.clone())])}
                                </div>
                            </div>
                        }).collect_view()}
                    </div>
                })}
            }.into_any()
        }
    }
}

// ── MARKET ────────────────────────────────────────────────────────────

fn market_menu(d: Desk, v: &View) -> AnyView {
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

// ── PEOPLE ────────────────────────────────────────────────────────────

fn people(d: Desk, v: &View) -> AnyView {
    match d.sub.get() {
        1 => {
            let list: Vec<_> = v.claims.iter().rev().cloned().collect();
            if list.is_empty() { return empty("nobody has claimed a flat"); }
            view! {
                <div class="ad-table">
                    <div class="ad-row head l5"><span>"flat"</span><span>"who"</span><span>"contact"</span><span>"address"</span><span>"state"</span></div>
                    {list.into_iter().map(|c| view! {
                        <div class="ad-row l5"><b>{name_of(&c.plot)}</b><span>{c.name.clone()}</span><span>{c.contact.clone()}</span><span class="mono">{short(&c.from)}</span>
                            <span>{if c.status == "pending" {
                                view! {
                                    {act(d, "BIND", "dl-go sm", "holder", vec![("plot", c.plot.clone()), ("address", c.from.clone()), ("name", c.name.clone())])}
                                    {act(d, "REJECT", "dl-mini", "reject", vec![("target", c.id.clone())])}
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

// ── TRAIL ─────────────────────────────────────────────────────────────

fn trail(d: Desk, v: &View) -> AnyView {
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
            2 => v.deals.iter().filter(|x| x.status != "owned").count(),
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
