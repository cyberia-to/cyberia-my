//! Steps four to seven of the deal’s track: the money and its control, the
//! papers made for this flat and signed, the notary, the handover. The
//! `Ctx` they read and the parts they share with the earlier steps are in
//! `deal_steps.rs`.

use crate::deal_docs::{annex_b, annex_c, annex_d, annex_e, open_sheet};
use crate::deal_steps::{pick, tg, Ctx};
use crate::deals::{fmt_ts, now_ms};
use crate::globe::BASE;
use crate::pay::{new_sent, record_sent, reference, sent_for, sent_text, Form, BANK, ETH_ADDRESS};
use crate::signal::{sign_document, verify_document};
use crate::terms::fmt_usd;
use leptos::prelude::*;

/// PAYMENT — the rail, the account, and the ledger of this deal.
pub fn payment_step(c: &Ctx) -> AnyView {
    let (cfg, sent, published, due) = (c.cfg, c.sent, c.published, c.due);
    let (id, name) = (c.id.clone(), c.name.clone());
    let p_pay = {
        let (id, name) = (id.clone(), name.clone());
        move || {
            let (id, name) = (id.clone(), name.clone());
            let Some(due_usd) = due.get() else {
                return view! { <p class="dl-note">"the price of this flat is set on request — the valley names it in the chat, then this step opens"</p> }.into_any();
            };
            let pd = published.get();
            let paid = pd.as_ref().map(|p| p.paid_usd).unwrap_or(0.0);
            let payments = pd.map(|p| p.payments).unwrap_or_default();
            let mine = sent_for(&sent.get(), &id);
            let pct = ((paid / due_usd) * 100.0).clamp(0.0, 100.0);
            let id_ref = id.clone();
            view! {
                <div class="dl-meter">
                    <div class="dl-meter-top"><span>{format!("{} received of {}", fmt_usd(paid), fmt_usd(due_usd))}</span><span>{format!("{pct:.0}%")}</span></div>
                    <div class="dl-meter-bar"><i style:width=format!("{pct:.1}%")></i></div>
                </div>
                <div class="dl-opts">
                    {pick(cfg, "rail", |c| &mut c.rail, "bank", "BANK TRANSFER", "USD · EUR · AUD · IDR")}
                    {pick(cfg, "rail", |c| &mut c.rail, "eth", "ETHEREUM", "ETH or USDT on mainnet")}
                </div>
                {move || if cfg.get().rail == "bank" {
                    let id_now = id_ref.clone();
                    view! {
                        <div class="dl-bank">
                            <div><span class="dl-k">"account name"</span><b>{BANK.holder}</b></div>
                            <div><span class="dl-k">"account number"</span><b class="mono">{BANK.number}</b></div>
                            <div><span class="dl-k">"SWIFT"</span><b class="mono">{BANK.swift}</b></div>
                            <div><span class="dl-k">"reference"</span><b class="mono">{move || reference(&id_now, cfg.get().share())}</b></div>
                            <div><span class="dl-k">"bank"</span><b>{format!("{} · {} · {}", BANK.bank, BANK.branch, BANK.bank_address)}</b></div>
                            <div><span class="dl-k">"currencies"</span><b>{BANK.currencies}</b></div>
                            <div><span class="dl-k">"holder address"</span><b>{BANK.holder_address}</b></div>
                        </div>
                        <p class="dl-note">"USD, EUR and AUD land as sent; IDR settles at Bank Indonesia's JISDOR rate on the invoice date"</p>
                    }.into_any()
                } else {
                    view! {
                        <div class="dl-bank">
                            <div><span class="dl-k">"address"</span><b class="mono">{ETH_ADDRESS.unwrap_or("given by support in a direct message")}</b></div>
                        </div>
                        <p class="dl-note">"ETH or USDT on Ethereum mainnet · the Company's address, named 2026-10-22"</p>
                    }.into_any()
                }}
                <button type="button" class="dl-go" on:click=move |_| {
                    let c = cfg.get_untracked();
                    let p = new_sent(&id, due_usd, c.share(), c.form, &c.rail);
                    let text = sent_text(&p, &name);
                    sent.update(|l| record_sent(l, p));
                    if let Some(w) = web_sys::window() {
                        let _ = w.open_with_url_and_target(&tg(&text), "_blank");
                    }
                }>{format!("I HAVE SENT {} →", fmt_usd(due_usd))}</button>
                <p class="dl-note">"tells the valley in the chat · the Company confirms the receipt and it shows here as received"</p>
                {(!mine.is_empty() || !payments.is_empty()).then(|| view! {
                    <div class="dl-q">"the ledger of this deal"</div>
                    <div class="dl-ledger">
                        {payments.into_iter().map(|p| view! {
                            <div class="ok"><span>"RECEIVED"</span><b>{fmt_usd(p.usd)}</b><i>{format!("{} · {} · {}", p.method, p.reference, p.on)}</i></div>
                        }).collect_view()}
                        {mine.into_iter().map(|p| view! {
                            <div><span>"SENT"</span><b>{fmt_usd(p.usd)}</b><i>{format!("{} · {} · {} · awaiting receipt", p.method, p.reference, fmt_ts(p.ts_ms))}</i></div>
                        }).collect_view()}
                    </div>
                })}
            }.into_any()
        }
    };
    p_pay().into_any()
}

/// PAPERS — the set made for this flat, and the offer signed by the neuron.
pub fn papers_step(c: &Ctx) -> AnyView {
    let (cfg, cx) = (c.cfg, c.cx);
    let (id, flat) = (c.id.clone(), c.flat.clone());
    let offer = c.offer_fn();
    let p_docs = {
        let (flat, id) = (flat.clone(), id.clone());
        let offer = offer.clone();
        move || {
            let c = cfg.get();
            let index_form = c.form == Form::Index;
            let sig = c.offer_sig.clone();
            let signed_text = c.offer_text.clone();
            let valid = match (&signed_text, &sig) {
                (Some(t), Some(s)) => verify_document(t, s),
                _ => false,
            };
            // the offer changed after it was signed: the signature no longer covers it
            let stale = signed_text.as_ref().map(|t| *t != offer().canonical()).unwrap_or(false);
            let (f_b, o_d, o_e, o_sign) = (flat.clone(), offer.clone(), offer.clone(), offer.clone());
            // the sheet the notary reads carries the text the signature covers
            let signed_d = match (&signed_text, &sig) {
                (Some(t), Some(s)) if valid => Some((t.clone(), s.clone())),
                _ => None,
            };
            let row = |made: bool, title: &'static str, what: &'static str, key: &'static str| view! {
                <a class="dl-doc" href=format!("{BASE}/docs/{key}") target="_blank">
                    <span class=if made { "dl-doc-tag made" } else { "dl-doc-tag" }>{if made { "FOR THIS FLAT" } else { "TEMPLATE" }}</span>
                    <b>{title}</b><i>{what}</i>
                </a>
            };
            view! {
                <div class="dl-q">"the set for this deal"</div>
                <div class="dl-docs">
                    {row(false, "Lease Agreement", "Perjanjian Hak Sewa · the text the notary draws the deed from", "signable-pack")}
                    {row(false, "Annex A — Estate Registry", "the Title the flat sits on", "signable-04-annex-a")}
                    <button type="button" class="dl-doc" on:click=move |_| open_sheet(&annex_b(&f_b))>
                        <span class="dl-doc-tag made">"MADE FOR THIS FLAT"</span><b>"Annex B — Plot Boundary"</b><i>"corners, sides, neighbours, the drawing · open the filled sheet"</i>
                    </button>
                    <button type="button" class="dl-doc" on:click={ let f = flat.clone(); move |_| open_sheet(&annex_c(&f, cfg.get_untracked().dd_read_ms)) }>
                        <span class="dl-doc-tag made">"MADE FOR THIS DEAL"</span><b>"Annex C — Due Diligence Pack"</b><i>"every paper by its content hash, and your mark of reading"</i>
                    </button>
                    <button type="button" class="dl-doc" on:click=move |_| open_sheet(&annex_d(&o_d(), signed_d.as_ref().map(|(t, s)| (t.as_str(), s))))>
                        <span class="dl-doc-tag made">"MADE FOR THIS DEAL"</span><b>"Annex D — Offer of Record"</b><i>"the page and the price you were shown · carries your signature"</i>
                    </button>
                    {index_form.then(|| view! {
                        <button type="button" class="dl-doc" on:click=move |_| open_sheet(&annex_e(&o_e()))>
                            <span class="dl-doc-tag made">"MADE FOR THIS DEAL"</span><b>"Annex E — Century Index"</b><i>"your rent in CX and the rule that converts it"</i>
                        </button>
                    })}
                    {row(false, "Annex F — Utilities, Access and Mobility", "self-sufficiency norms · filled per flat by the valley", "signable-09-annex-f")}
                    {row(false, "Annex G — Handover Condition", "the record of handover · filled on the day", "signable-10-annex-g")}
                </div>
                <div class="dl-q">"sign the offer"</div>
                {match (sig, valid && !stale) {
                    (Some(s), true) => {
                        // what travels to the Company is the text under the
                        // signature, byte for byte — not one rebuilt at click
                        let (signed_body, pid) = (signed_text.clone().unwrap_or_default(), id.clone());
                        view! {
                            <div class="dl-signed">
                                <b>"✓ SIGNED BY YOUR NEURON · THE SIGNATURE VERIFIES"</b>
                                <span class="mono">{format!("particle {}…{}", &s.particle[..12], &s.particle[s.particle.len() - 8..])}</span>
                                <span>{format!("{} · {}", fmt_ts(s.ts_ms), s.neuron)}</span>
                            </div>
                            <button type="button" class="dl-go" on:click=move |_| {
                                let text = format!("SIGNED OFFER {}\nparticle: {}\nneuron: {}\npubkey: {}\nsignature: {}\n\n{}", pid, s.particle, s.neuron, s.pubkey_hex, s.sig_hex, signed_body);
                                if let Some(w) = web_sys::window() {
                                    let _ = w.open_with_url_and_target(&tg(&text), "_blank");
                                }
                            }>"SEND THE SIGNED OFFER →"</button>
                            <p class="dl-note">"the Company countersigns; the notary draws the Indonesian deed from the same figures"</p>
                        }
                        .into_any()
                    }
                    (had, _) => view! {
                        {had.is_some().then(|| view! { <p class="dl-note warn">"your terms changed after you signed — the old signature no longer covers the offer; sign it again"</p> })}
                        <button type="button" class="dl-go" on:click=move |_| {
                            // the moment and the index fix are frozen into the
                            // deal before the text is made, so the offer still
                            // reads the same a minute after it was signed
                            let (ts, lvl) = (now_ms(), cx.get_untracked());
                            cfg.update(|c| { c.offer_ts_ms = Some(ts); c.offer_cx = lvl; });
                            let text = o_sign().canonical();
                            let s = sign_document(&text);
                            cfg.update(|c| { c.offer_text = Some(text.clone()); c.offer_sig = Some(s.clone()); });
                        }>"SIGN THE OFFER WITH YOUR NEURON"</button>
                        <p class="dl-note">"an electronic signature made in this browser: the offer's text is hashed into a particle and signed with your key (ADR-036) · anyone can verify it · the deed itself is signed at the notary"</p>
                    }.into_any(),
                }}
            }
        }
    };
    p_docs().into_any()
}

/// NOTARY — in person or by proxy, and the date of the deed.
pub fn notary_step(c: &Ctx) -> AnyView {
    let cfg = c.cfg;
    let (id, name) = (c.id.clone(), c.name.clone());
    let p_notary = {
        let (id, name) = (id.clone(), name.clone());
        move || {
            let (id, name) = (id.clone(), name.clone());
            let asked = cfg.get().notary_asked_ms;
            view! {
                <div class="dl-q">"how you sign the deed"</div>
                <div class="dl-opts">
                    {pick(cfg, "notary", |c| &mut c.notary, "person", "IN PERSON", "at the notary's table in Bali, one visit")}
                    {pick(cfg, "notary", |c| &mut c.notary, "proxy", "BY PROXY", "a power of attorney, signed where you are")}
                </div>
                <ul class="dl-list">
                    {move || if cfg.get().notary == "proxy" {
                        view! {
                            <li>"you sign a power of attorney naming your proxy — notarised and, from abroad, legalised or apostilled"</li>
                            <li>"the original travels to Bali; the proxy signs the deed for you"</li>
                            <li>"the notary reads the deed in Bahasa Indonesia; your second-language copy is beside it"</li>
                        }.into_any()
                    } else {
                        view! {
                            <li>"bring the passport or KTP you sent for KYC"</li>
                            <li>"the notary reads the deed in Bahasa Indonesia; a sworn interpreter sits with you if you ask"</li>
                            <li>"both sides sign; you leave with your copy of the deed"</li>
                        }.into_any()
                    }}
                    <li>"the land office search is ordered by the notary within 30 days of signing and pinned to the dossier"</li>
                </ul>
                <a class="dl-go" href=tg(&format!("NOTARY {id}\n{name} — I am ready to set a date for the deed.\nhow: {}", if cfg.get_untracked().notary == "proxy" { "by proxy" } else { "in person" }))
                    target="_blank" rel="noopener" on:click=move |_| cfg.update(|c| c.notary_asked_ms = Some(now_ms()))>
                    {if asked.is_some() { "ASK AGAIN FOR A DATE →" } else { "SET A DATE WITH THE NOTARY →" }}
                </a>
            }
        }
    };
    p_notary().into_any()
}

/// YOURS — the handover, the flat’s token, the cabinet.
pub fn yours_step(c: &Ctx) -> AnyView {
    let published = c.published;
    let id = c.id.clone();
    let p_yours = {
        let id = id.clone();
        move || {
            let owned = published.get().map(|p| p.status == "owned").unwrap_or(false);
            view! {
                <div class="dl-rows">
                    <div><span class="dl-k">"handover"</span><b class=if owned { "" } else { "dim" }>{if owned { "the flat is handed over" } else { "Annex G is filled on the ground, with you or your proxy" }}</b></div>
                    <div><span class="dl-k">"the flat's token"</span><b class="dim">"minted to your Ethereum address once the deed is signed"</b>
                        <a class="dl-mini" href=format!("{BASE}/plot/{id}/metadata.json") target="_blank">"TOKEN METADATA"</a></div>
                    <div><span class="dl-k">"your room"</span><b class="dim">"every flat you hold, with its ledger"</b>
                        <a class="dl-mini" href=format!("{BASE}/cabinet")>"OPEN THE CABINET"</a></div>
                </div>
                {owned.then(|| view! { <p class="dl-note ok">"✓ the leasehold is yours for the term"</p> })}
            }
        }
    };
    p_yours().into_any()
}
