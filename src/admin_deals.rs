//! DEALS — the valley's first sales. A deal is one card: the money received
//! against the money due at signing, the seven steps as buttons a press
//! confirms, the ledger of its receipts, where it stands. Every press is an
//! act the admin's wallet signs.

use crate::admin::{act, due_of, empty, field, in_motion, name_of, today, Desk, STEPS};
use crate::globe::BASE;
use crate::land::load_map;
use crate::market::{DealRec, View};
use crate::orders::PublishedDeal;
use crate::terms::{bare_name, fmt_usd};
use leptos::prelude::*;

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

pub fn deal_card(d: Desk, x: DealRec) -> AnyView {
    let plot = x.plot.clone();
    let (p_o, p_c) = (plot.clone(), plot.clone());
    let due = due_of(&x);
    let pct = due.map(|v| (x.paid_usd / v * 100.0).clamp(0.0, 100.0)).unwrap_or(0.0);
    let open = move || d.focus.get() == p_o;
    let steps = x.steps.clone();
    let payments = x.payments.clone();
    let status = x.status.clone();
    view! {
        <div class=if in_motion(&x) { "ad-deal" } else { "ad-deal done" }>
            <button type="button" class="ad-deal-h" on:click={ let p = plot.clone(); move |_| d.look(&p) }>
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
                                d.clear();
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

pub fn deals(d: Desk, v: &View) -> AnyView {
    match d.sub.get() {
        1 => {
            let open: Vec<&DealRec> = v.deals.iter().filter(|x| in_motion(x)).collect();
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
            list.sort_by(|a, b| in_motion(b).cmp(&in_motion(a)).then(b.updated.cmp(&a.updated)));
            let mine: Vec<String> = list.iter().map(|x| x.plot.clone()).collect();
            // the bot's file keeps a flat under any non-empty status: an `available` one carries no deal to take
            let from_bot: Vec<(String, PublishedDeal)> = d.bot.get().into_iter().filter(|(k, pd)| !mine.contains(k) && pd.status != "available").collect();
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
