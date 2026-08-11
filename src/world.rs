//! Signal Studio hub — the soft3 ladder as a construction surface.
//! word → link → sentence → signal → motif → dialect → lexicon.
//! Sub-routes are full pages (no overlays).

use crate::economy::fmt_qty;
use crate::erp::{ensure_erp_boot, sync_plot_cards_from_leases};
use crate::land::FLAG_SVG;
use crate::nav::CyberiaNav;
use crate::signal::{
    ensure_graph_boot, graph_view, lexicon, load_signals, neuron, open_draft, sentence_run,
};
use crate::wallet::load_balance;
use leptos::prelude::*;

/// Shared chrome for all /world/* pages.
#[component]
pub fn StudioShell(
    #[prop(into)] title: String,
    #[prop(into)] kicker: String,
    #[prop(optional, into)] lead: String,
    children: Children,
) -> impl IntoView {
    let t = title.clone();
    Effect::new(move |_| {
        document().set_title(&format!("Cyberia — {t}"));
        ensure_erp_boot();
        ensure_graph_boot();
        sync_plot_cards_from_leases();
    });
    view! {
        <div class="page-shell cities-shell">
            <div class="site-chrome cyberia-chrome">
                <div class="chrome-inner">
                    <div class="header-row1">
                        <div class="logo-zone">
                            <h1 class="logo">
                                <a href="/cities" class="brand-flag" title="home" inner_html=FLAG_SVG></a>
                                <span style="color: var(--cyber-green);">"cyber"</span>
                                <span style="color: var(--cyber-green); margin: 0 1px;">"•"</span>
                                <span style="color: #fff;">"ia"</span>
                            </h1>
                        </div>
                        <div class="cyberia-phase-pill">
                            <span class="phase-dot"></span>
                            "SIGNAL STUDIO"
                        </div>
                        <CyberiaNav active="world" />
                    </div>
                </div>
            </div>
            <div class="cities-stage">
                <div class="studio-breadcrumb">
                    <a href="/world">"SIGNAL"</a>
                    <span>" / "</span>
                    <span>{kicker.clone()}</span>
                </div>
                <div class="cities-hero">
                    <div>
                        <div class="cities-kicker">{kicker}</div>
                        <h2 class="cities-title">{title}</h2>
                        {(!lead.is_empty()).then(|| view! {
                            <p class="cities-lead">{lead}</p>
                        })}
                    </div>
                </div>
                {children()}
            </div>
            <div class="search-dock cyberia-dock cities-dock">
                <span class="dock-count">
                    {move || {
                        let v = graph_view();
                        format!(
                            "{} words · {} links ({} draft) · {} signals · {} CX",
                            v.words,
                            v.links,
                            v.drafts,
                            v.committed,
                            fmt_qty(load_balance().cx)
                        )
                    }}
                </span>
                <a class="cta-btn cta-lease cta-lg dock-found" href="/world/links/new" style="text-decoration:none;max-width:280px;">
                    <span class="cta-copy">
                        <span class="cta-title">"+ LINK"</span>
                        <span class="cta-sub">"word → word"</span>
                    </span>
                </a>
                <a href="/me" class="dock-credit" style="color: var(--cyber-cyan);">"YOU"</a>
            </div>
        </div>
    }
}

/// /world — the Signal Studio hub.
#[component]
pub fn WorldPage() -> impl IntoView {
    Effect::new(move |_| {
        document().set_title("Cyberia — Signal Studio");
        ensure_erp_boot();
        ensure_graph_boot();
        sync_plot_cards_from_leases();
    });

    let v = move || graph_view();

    view! {
        <StudioShell
            title="Signal Studio"
            kicker="CONSTRUCTOR"
            lead="The universe constructor on the soft3 ladder. Words carry meaning, links assert it, signals commit it — signed by your neuron, verifiable by anyone. Motifs stamp recurring shapes; views project, never mutate."
        >
            <div class="chain-hint">
                <span class="chain-step">"WORD"</span>
                <span class="chain-arr">"→"</span>
                <span class="chain-step">"LINK"</span>
                <span class="chain-arr">"→"</span>
                <span class="chain-step">"SENTENCE"</span>
                <span class="chain-arr">"→"</span>
                <span class="chain-step">"SIGNAL"</span>
                <span class="chain-arr">"→"</span>
                <span class="chain-step">"MOTIF"</span>
                <span class="chain-arr">"→"</span>
                <span class="chain-step">"DIALECT"</span>
                <span class="chain-arr">"→"</span>
                <span class="chain-step">"LEXICON"</span>
            </div>

            // the neuron — the identity that signs every committed signal
            {move || {
                let me = neuron();
                view! {
                    <div class="studio-section-h" style="margin-top:8px;"><span>"YOUR NEURON — SIGNS EVERY SIGNAL"</span></div>
                    <div class="bank-kpi-grid me-counts">
                        <div class="kpi me-kpi">
                            <div class="kpi-lab">"NEURON"</div>
                            <div class="kpi-val" style="font-size:11px; word-break:break-all;">{me.bech32.clone()}</div>
                            <div class="kpi-sub">"mudra domain key · cyberia.my"</div>
                        </div>
                        <div class="kpi me-kpi">
                            <div class="kpi-lab">"PUBKEY"</div>
                            <div class="kpi-val" style="font-size:11px; word-break:break-all;">{me.pubkey_hex.clone()}</div>
                            <div class="kpi-sub">"secp256k1 · entropy → hemera KDF → d·G"</div>
                        </div>
                        <div class="kpi me-kpi">
                            <div class="kpi-lab">"NATIVE ID"</div>
                            <div class="kpi-val" style="font-size:11px; word-break:break-all;">{me.native_hex.clone()}</div>
                            <div class="kpi-sub">"hemera(pubkey)"</div>
                        </div>
                    </div>
                }
            }}

            // the open draft — the batch being composed right now
            {move || {
                let draft = open_draft();
                let n = draft.links.len();
                let run = sentence_run(&draft.links);
                let is_sentence = n >= 2 && run == n;
                let href = format!("/world/signal/{}", draft.id);
                view! {
                    <div class="studio-section-h" style="margin-top:12px;">
                        <span>"OPEN DRAFT SIGNAL"</span>
                        <a class="chip chip-on" href="/world/links/new">"+ LINK"</a>
                    </div>
                    <a class="studio-row studio-row-link" href=href>
                        <div class="studio-row-main">
                            <span class="studio-kind draft-kind">"DRAFT"</span>
                            <div>
                                <div class="studio-title">
                                    {format!("{} · {} link{} pending", draft.id, n, if n == 1 { "" } else { "s" })}
                                    {is_sentence.then(|| view! { <span class="sys-tag" style="margin-left:8px;">"SENTENCE"</span> })}
                                </div>
                                <div class="studio-meta">{if n == 0 { "empty — add the first link".to_string() } else { "open to commit · sign".to_string() }}</div>
                            </div>
                        </div>
                        <span class="chip">{if n == 0 { "OPEN →" } else { "COMMIT →" }}</span>
                    </a>
                }
            }}

            <div class="studio-create-bar" style="margin-top:14px;">
                <a class="cta-btn cta-lease cta-lg" href="/world/links/new" style="text-decoration:none;">
                    <span class="cta-copy">
                        <span class="cta-title">"+ LINK"</span>
                        <span class="cta-sub">"the atom · word → word"</span>
                    </span>
                </a>
                <a class="cta-btn cta-found cta-lg" href="/world/words/new" style="text-decoration:none;">
                    <span class="cta-copy">
                        <span class="cta-title">"+ WORD"</span>
                        <span class="cta-sub">"typed particle"</span>
                    </span>
                </a>
                <a class="cta-btn cta-event cta-lg" href="/world/templates/new" style="text-decoration:none;">
                    <span class="cta-copy">
                        <span class="cta-title">"+ MOTIF"</span>
                        <span class="cta-sub">"recipe · burns → mints"</span>
                    </span>
                </a>
                <a class="cta-btn cta-buy cta-lg" href="/world/schedules/new" style="text-decoration:none;">
                    <span class="cta-copy">
                        <span class="cta-title">"+ SCHEDULE"</span>
                        <span class="cta-sub">"fire motifs"</span>
                    </span>
                </a>
                <a class="cta-btn cta-found cta-lg" href="/world/views/new" style="text-decoration:none;">
                    <span class="cta-copy">
                        <span class="cta-title">"+ VIEW"</span>
                        <span class="cta-sub">"projection"</span>
                    </span>
                </a>
            </div>

            <div class="bank-kpi-grid me-counts">
                <a class="kpi me-kpi" href="/world/words">
                    <div class="kpi-lab">"WORDS"</div>
                    <div class="kpi-val">{move || v().words.to_string()}</div>
                    <div class="kpi-sub">{move || format!("{} relations", v().relations)}</div>
                </a>
                <a class="kpi me-kpi" href="/world/links">
                    <div class="kpi-lab">"LINKS"</div>
                    <div class="kpi-val">{move || v().links.to_string()}</div>
                    <div class="kpi-sub">{move || format!("{} draft", v().drafts)}</div>
                </a>
                <a class="kpi me-kpi" href="/world/signals">
                    <div class="kpi-lab">"SIGNALS"</div>
                    <div class="kpi-val">{move || v().committed.to_string()}</div>
                    <div class="kpi-sub">"signed batches →"</div>
                </a>
                <a class="kpi me-kpi" href="/world/templates">
                    <div class="kpi-lab">"MOTIFS"</div>
                    <div class="kpi-val">{move || crate::erp::load_templates().len().to_string()}</div>
                    <div class="kpi-sub">"recipes →"</div>
                </a>
                <a class="kpi me-kpi" href="/world/schedules">
                    <div class="kpi-lab">"SCHEDULES"</div>
                    <div class="kpi-val">{move || crate::erp::load_schedules().len().to_string()}</div>
                    <div class="kpi-sub">"recurring →"</div>
                </a>
                <a class="kpi me-kpi" href="/world/views">
                    <div class="kpi-lab">"VIEWS"</div>
                    <div class="kpi-val">{move || crate::erp::load_views().len().to_string()}</div>
                    <div class="kpi-sub">"projections →"</div>
                </a>
            </div>

            <div class="studio-section-h" style="margin-top:8px;">
                <span>"LEXICON — TOP WORDS BY FOCUS"</span>
                <a class="chip chip-on" href="/world/words">"ALL WORDS"</a>
            </div>
            <div class="studio-list">
                {move || {
                    let mut lex = lexicon();
                    lex.retain(|(_, f)| *f > 0.0);
                    lex.truncate(8);
                    if lex.is_empty() {
                        return view! {
                            <div class="me-empty">
                                "No focus yet — commit a signal and the lexicon wakes up."
                            </div>
                        }.into_any();
                    }
                    view! {
                        {lex.into_iter().map(|(w, f)| {
                            let href = format!("/world/word/{}", w.particle);
                            view! {
                                <a class="studio-row studio-row-link" href=href>
                                    <div class="studio-row-main">
                                        <span class="studio-kind">{w.kind.to_uppercase()}</span>
                                        <div>
                                            <div class="studio-title">{w.name.clone()}</div>
                                            <div class="studio-meta">{format!("φ {f:.1}")}</div>
                                        </div>
                                    </div>
                                    <span class="chip">"OPEN →"</span>
                                </a>
                            }
                        }).collect_view()}
                    }.into_any()
                }}
            </div>

            <div class="studio-section-h" style="margin-top:16px;">
                <span>"RECENT SIGNALS"</span>
                <a class="chip chip-on" href="/world/signals">"ALL"</a>
            </div>
            <div class="studio-list">
                {move || {
                    let mut signals = load_signals();
                    signals.retain(|s| s.state == "committed");
                    signals.reverse();
                    signals.truncate(6);
                    if signals.is_empty() {
                        return view! {
                            <div class="me-empty">
                                "Nothing committed yet. "
                                <a href="/world/links/new">"Draft a link"</a>
                                ", then commit the signal — it gets hashed and signed for real."
                            </div>
                        }.into_any();
                    }
                    view! {
                        {signals.into_iter().map(|s| {
                            let href = format!("/world/signal/{}", s.id);
                            let n = s.links.len();
                            view! {
                                <a class="studio-row studio-row-link" href=href>
                                    <div class="studio-row-main">
                                        <span class="studio-kind">"SIGNED"</span>
                                        <div>
                                            <div class="studio-title">{format!("{} · {} link{}", s.id, n, if n == 1 { "" } else { "s" })}</div>
                                            <div class="studio-meta">{if s.note.is_empty() { format!("body {}", &s.body_particle[..s.body_particle.len().min(16)]) } else { s.note.clone() }}</div>
                                        </div>
                                    </div>
                                    <span class="chip">"OPEN →"</span>
                                </a>
                            }
                        }).collect_view()}
                    }.into_any()
                }}
            </div>

            <p class="bank-footnote">
                "Everything here is real: particles are hemera (Poseidon2) hashes, the neuron is a mudra domain key, committed signals carry ADR-036 signatures you can re-verify. The dialect — cards, coins, motifs, PLUMB — is a convention over the same graph."
            </p>
        </StudioShell>
    }
}
