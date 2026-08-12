//! /studio — the Signal Studio hub (construction surface on the soft3
//! ladder) and /world — your presence: who you are in this world.
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
                        <CyberiaNav active="studio" />
                    </div>
                </div>
            </div>
            <div class="cities-stage">
                <div class="studio-breadcrumb">
                    <a href="/studio">"STUDIO"</a>
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

/// /studio — the Signal Studio hub: create words, links, signals.
#[component]
pub fn StudioPage() -> impl IntoView {
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
                <a class="cta-btn cta-event cta-lg" href="/world/cards/new" style="text-decoration:none;">
                    <span class="cta-copy">
                        <span class="cta-title">"+ CARD"</span>
                        <span class="cta-sub">"TSP-2 · unique entity"</span>
                    </span>
                </a>
                <a class="cta-btn cta-buy cta-lg" href="/world/coins/new" style="text-decoration:none;">
                    <span class="cta-copy">
                        <span class="cta-title">"+ COIN"</span>
                        <span class="cta-sub">"TSP-1 · divisible value"</span>
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

            // nature — the dialect's two token natures + the living layer.
            // every entity is one of these; the graph carries their relations
            <div class="studio-section-h" style="margin-top:12px;"><span>"NATURE — TSP-1 · TSP-2 · GENOME"</span></div>
            <div class="bank-kpi-grid me-counts">
                <a class="kpi me-kpi" href="/world/cards">
                    <div class="kpi-lab">"CARDS · TSP-2"</div>
                    <div class="kpi-val">{move || crate::erp::load_cards().len().to_string()}</div>
                    <div class="kpi-sub">"unique entities · owner_count = 1"</div>
                </a>
                <a class="kpi me-kpi" href="/world/coins">
                    <div class="kpi-lab">"COINS · TSP-1"</div>
                    <div class="kpi-val">{move || crate::wallet::load_stocks().len().to_string()}</div>
                    <div class="kpi-sub">"divisible value · Σ balances = supply"</div>
                </a>
                <a class="kpi me-kpi" href="/genetics">
                    <div class="kpi-lab">"GENETICS"</div>
                    <div class="kpi-val">{crate::genetics::SPECIES.len().to_string()}</div>
                    <div class="kpi-sub">"species — the living layer →"</div>
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

/// /world — your presence: who you are in this world. The neuron, your
/// word, your focus, and everything you hold — stocks, land, buildings,
/// genomes, signals. The studio creates; this page answers "who am I".
#[component]
pub fn WorldPage() -> impl IntoView {
    Effect::new(move |_| {
        document().set_title("Cyberia — my world");
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
                            {move || {
                                let handle = crate::wallet::load_profile().handle;
                                format!("MY WORLD · {}", handle.to_uppercase())
                            }}
                        </div>
                        <CyberiaNav active="world" />
                    </div>
                </div>
            </div>

            <div class="cities-stage">
                {move || {
                    use crate::signal::{links_touching, load_words, word_focus, word_name, word_particle};
                    use crate::wallet::{load_leases, load_profile, load_stocks};

                    let me = neuron();
                    let handle = load_profile().handle;
                    let my_word = word_particle("person", &handle);
                    let focus = word_focus(&my_word);
                    let my_links = links_touching(&my_word);
                    let words = load_words();
                    let my_words = words.iter().filter(|w| w.owner == me.bech32).count();
                    let my_species: Vec<_> = words
                        .iter()
                        .filter(|w| w.kind == "species" && w.owner == me.bech32)
                        .cloned()
                        .collect();
                    let my_signals = load_signals()
                        .into_iter()
                        .filter(|s| s.state == "committed" && s.neuron == me.bech32)
                        .count();
                    let leases = load_leases();
                    let buildings: Vec<_> = crate::erp::load_cards()
                        .into_iter()
                        .filter(|c| c.kind == "building" && c.owner == handle)
                        .collect();
                    let mut stocks = load_stocks();
                    stocks.retain(|s| s.qty > 0.0);
                    stocks.sort_by(|a, b| b.qty.partial_cmp(&a.qty).unwrap_or(std::cmp::Ordering::Equal));
                    let n_stocks = stocks.len();
                    stocks.truncate(8);
                    let word_href = format!("/world/word/{my_word}");

                    view! {
                        <div class="cities-hero">
                            <div>
                                <div class="cities-kicker">"WHO AM I IN THIS WORLD"</div>
                                <h2 class="cities-title" style="text-transform:none;">{handle.clone()}</h2>
                                <p class="cities-lead">
                                    "One neuron, one word, and everything the graph says about you. Creation happens in the "
                                    <a href="/studio" style="color: var(--cyber-green);">"Studio"</a>
                                    " — this is your standing in the world."
                                </p>
                            </div>
                        </div>

                        // identity
                        <div class="studio-section-h"><span>"IDENTITY — THE NEURON AND ITS WORD"</span></div>
                        <div class="bank-kpi-grid me-counts">
                            <div class="kpi me-kpi">
                                <div class="kpi-lab">"NEURON"</div>
                                <div class="kpi-val" style="font-size:11px; word-break:break-all;">{me.bech32.clone()}</div>
                                <div class="kpi-sub">"signs everything you assert"</div>
                            </div>
                            <a class="kpi me-kpi" href=word_href.clone()>
                                <div class="kpi-lab">"MY WORD"</div>
                                <div class="kpi-val" style="font-size:15px;">{format!("person:{handle}")}</div>
                                <div class="kpi-sub">"your particle in the graph →"</div>
                            </a>
                            <div class="kpi me-kpi">
                                <div class="kpi-lab">"FOCUS"</div>
                                <div class="kpi-val">{format!("φ {focus:.1}")}</div>
                                <div class="kpi-sub">"what the graph holds you at"</div>
                            </div>
                            <div class="kpi me-kpi">
                                <div class="kpi-lab">"CX"</div>
                                <div class="kpi-val">{fmt_qty(load_balance().cx)}</div>
                                <div class="kpi-sub">"soft balance"</div>
                            </div>
                        </div>

                        // standing
                        <div class="studio-section-h" style="margin-top:12px;"><span>"STANDING — WHAT YOU'VE PUT INTO THE WORLD"</span></div>
                        <div class="bank-kpi-grid me-counts">
                            <a class="kpi me-kpi" href="/world/signals">
                                <div class="kpi-lab">"SIGNALS"</div>
                                <div class="kpi-val">{my_signals.to_string()}</div>
                                <div class="kpi-sub">"committed under your key →"</div>
                            </a>
                            <a class="kpi me-kpi" href=word_href.clone()>
                                <div class="kpi-lab">"LINKS ON YOU"</div>
                                <div class="kpi-val">{my_links.len().to_string()}</div>
                                <div class="kpi-sub">"assertions touching your word →"</div>
                            </a>
                            <a class="kpi me-kpi" href="/world/words">
                                <div class="kpi-lab">"WORDS MINTED"</div>
                                <div class="kpi-val">{my_words.to_string()}</div>
                                <div class="kpi-sub">"vocabulary you coined →"</div>
                            </a>
                            <a class="kpi me-kpi" href="/map">
                                <div class="kpi-lab">"PLOTS"</div>
                                <div class="kpi-val">{leases.len().to_string()}</div>
                                <div class="kpi-sub">"land you lease →"</div>
                            </a>
                            <a class="kpi me-kpi" href="/world/cards">
                                <div class="kpi-lab">"BUILDINGS"</div>
                                <div class="kpi-val">{buildings.len().to_string()}</div>
                                <div class="kpi-sub">"constructed on your plots →"</div>
                            </a>
                            <a class="kpi me-kpi" href="/genetics">
                                <div class="kpi-lab">"GENOMES"</div>
                                <div class="kpi-val">{my_species.len().to_string()}</div>
                                <div class="kpi-sub">"species you seeded →"</div>
                            </a>
                        </div>

                        // holdings
                        <div class="studio-section-h" style="margin-top:12px;">
                            <span>{format!("HOLDINGS — {n_stocks} STOCKS")}</span>
                            <a class="chip chip-on" href="/elements">"ELEMENTS"</a>
                            <a class="chip" href="/products">"PRODUCTS"</a>
                        </div>
                        <div class="studio-list">
                            {if stocks.is_empty() {
                                view! { <div class="me-empty">"Nothing held yet — buy an element, run a motif, work the land."</div> }.into_any()
                            } else {
                                view! {
                                    {stocks.into_iter().map(|s| {
                                        view! {
                                            <div class="studio-row">
                                                <div class="studio-row-main">
                                                    <span class="studio-kind">"COIN"</span>
                                                    <div>
                                                        <div class="studio-title">{s.id.clone()}</div>
                                                        <div class="studio-meta">{format!("{} held", fmt_qty(s.qty))}</div>
                                                    </div>
                                                </div>
                                            </div>
                                        }
                                    }).collect_view()}
                                }.into_any()
                            }}
                        </div>

                        // the graph on you
                        <div class="studio-section-h" style="margin-top:16px;">
                            <span>"THE GRAPH ON YOU"</span>
                            <a class="chip chip-on" href="/studio">"OPEN STUDIO"</a>
                        </div>
                        <div class="studio-list">
                            {if my_links.is_empty() {
                                view! { <div class="me-empty">"No assertions yet — the world hasn't said anything about you. Say something first: " <a href="/world/links/new">"draft a link"</a>"."</div> }.into_any()
                            } else {
                                view! {
                                    {my_links.into_iter().take(10).map(|(sid, l)| {
                                        let href = format!("/world/signal/{sid}");
                                        view! {
                                            <a class="studio-row studio-row-link" href=href>
                                                <div class="studio-row-main">
                                                    <span class="studio-kind">"LINKED"</span>
                                                    <div>
                                                        <div class="studio-title">{format!("{} —[{}]→ {}", word_name(&l.from), word_name(&l.rel), word_name(&l.to))}</div>
                                                        <div class="studio-meta">{format!("{} · w={}", sid, l.weight)}</div>
                                                    </div>
                                                </div>
                                                <span class="chip">"SIGNAL →"</span>
                                            </a>
                                        }
                                    }).collect_view()}
                                }.into_any()
                            }}
                        </div>

                        <p class="bank-footnote">
                            "Identity is position: your word means what the graph links it to, weighted by focus. Wallet detail lives at " <a href="/me">"/me"</a> "; construction at " <a href="/studio">"/studio"</a> "."
                        </p>
                    }
                }}
            </div>
        </div>
    }
}
