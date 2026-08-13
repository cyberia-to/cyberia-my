//! The robot portal — Garage · Gallery · Factory. Interface prototypes
//! for the cyb portal sections (cyb/root/product.md), wired to the real
//! local kernel: buying or assembling a robot mints a card + word and
//! commits a signed signal; factory runs queue ops intents.
//!
//! The model: 1 robot → 1 avatar → 1 body (none | machine | meat); two
//! axes (virtual↔embodied, alone↔collab) → four quadrants. Robots work,
//! humans govern.

use crate::erp::{create_intent_manual, load_cards};
use crate::land::FLAG_SVG;
use crate::nav::CyberiaNav;
use crate::signal::{emit_signal, mint_word, neuron, word_particle, Link};
use crate::wallet::{debit_cx, ensure_economy_boot, load_balance, load_profile, push_intent};
use leptos::prelude::*;

pub struct RobotModel {
    pub id: &'static str,
    pub name: &'static str,
    pub emoji: &'static str,
    /// none | machine | meat
    pub body: &'static str,
    /// default quadrant on delivery
    pub quadrant: &'static str,
    pub price: f64,
    pub blurb: &'static str,
}

/// The Gallery stock — типовые модели.
pub const MODELS: &[RobotModel] = &[
    RobotModel {
        id: "scribe",
        name: "SCRIBE",
        emoji: "✍️",
        body: "none",
        quadrant: "social ghost",
        price: 10.0,
        blurb: "Writes, files, remembers — a virtual clerk that never sleeps.",
    },
    RobotModel {
        id: "oracle",
        name: "ORACLE",
        emoji: "🔮",
        body: "none",
        quadrant: "social ghost",
        price: 25.0,
        blurb: "Research and signals — reads the graph, reports what matters.",
    },
    RobotModel {
        id: "keeper",
        name: "KEEPER",
        emoji: "🗝️",
        body: "none",
        quadrant: "dormant",
        price: 15.0,
        blurb: "Holds capital, schedules actions, guards the vault. Pure soul.",
    },
    RobotModel {
        id: "gardener",
        name: "GARDENER MK-1",
        emoji: "🌱",
        body: "machine",
        quadrant: "full actor",
        price: 120.0,
        blurb: "Pruning, planting, mulching — the orchard's tireless hands.",
    },
    RobotModel {
        id: "mason",
        name: "MASON MK-1",
        emoji: "🧱",
        body: "machine",
        quadrant: "full actor",
        price: 150.0,
        blurb: "Stone, mortar, terraces — builds what outlasts the lease.",
    },
    RobotModel {
        id: "hauler",
        name: "HAULER",
        emoji: "🚚",
        body: "machine",
        quadrant: "sovereign",
        price: 90.0,
        blurb: "Moves anything up the mountain — trail-rated, load-hungry.",
    },
    RobotModel {
        id: "warden",
        name: "WARDEN",
        emoji: "🛡️",
        body: "machine",
        quadrant: "sovereign",
        price: 110.0,
        blurb: "Patrols the perimeter, watches the night, wakes no one needlessly.",
    },
    RobotModel {
        id: "shepherd",
        name: "SHEPHERD",
        emoji: "🐑",
        body: "machine",
        quadrant: "full actor",
        price: 130.0,
        blurb: "Runs the flock — pasture rotation, headcount, gate discipline.",
    },
    RobotModel {
        id: "sexton",
        name: "SOLAR SEXTON",
        emoji: "☀️",
        body: "machine",
        quadrant: "sovereign",
        price: 80.0,
        blurb: "Keeps the arrays clean and the batteries honest.",
    },
    RobotModel {
        id: "wellkeeper",
        name: "WELLKEEPER",
        emoji: "💧",
        body: "machine",
        quadrant: "sovereign",
        price: 95.0,
        blurb: "Springs, tanks, pressure — the water always arrives.",
    },
    RobotModel {
        id: "docent",
        name: "DOCENT",
        emoji: "🥾",
        body: "meat",
        quadrant: "full actor",
        price: 45.0,
        blurb: "A bonded guide — stories, trails, sunrise timing. Biological presence.",
    },
    RobotModel {
        id: "apiarist",
        name: "APIARIST",
        emoji: "🐝",
        body: "meat",
        quadrant: "full actor",
        price: 55.0,
        blurb: "Bonded beekeeper — hives, harvest, stings taken on your behalf.",
    },
];

pub fn body_color(body: &str) -> &'static str {
    match body {
        "none" => "var(--cyber-cyan)",
        "machine" => "var(--cyber-yellow)",
        _ => "var(--cyber-orange)",
    }
}

pub fn body_label(body: &str) -> &'static str {
    match body {
        "none" => "VIRTUAL",
        "machine" => "MACHINE BODY",
        _ => "MEAT BODY",
    }
}

fn my_robot_count() -> usize {
    load_cards().iter().filter(|c| c.kind == "robot").count()
}

/// Mint a robot: card (kind robot) + word + one signed signal
/// you —owns→ robot. Returns the card id.
fn mint_robot(name: &str, body: &str, quadrant: &str, note: &str) -> Result<String, String> {
    let handle = load_profile().handle;
    let me = neuron().bech32;
    let id = format!("robot-{}", name.to_lowercase().replace(' ', "-"));
    let card = crate::erp::Card {
        id: id.clone(),
        kind: "robot".into(),
        name: name.to_string(),
        owner: handle.clone(),
        parent: None,
        zone: String::new(),
        class: body.to_string(),
        locked: false,
        note: format!("{quadrant} · {note}"),
    };
    crate::erp::plumb_mint_card(card)?;
    let robot_w = word_particle("robot", name);
    let you_w = mint_word("person", &handle, "YOU", &me, false);
    let owns = mint_word("relation", "owns", "possession — card holds card", "", true);
    emit_signal(
        vec![Link {
            from: you_w,
            rel: owns,
            to: robot_w,
            weight: 1.0,
            note: body_label(body).to_lowercase(),
        }],
        &format!("robot minted · {name}"),
    )?;
    push_intent(&handle, "robot_mint", name);
    Ok(id)
}

/// Shared sub-nav for the portal surfaces.
#[component]
pub fn PortalNav(#[prop(into)] active: String) -> impl IntoView {
    let a = active;
    let tabs = [
        ("fleet", "/robots", "FLEET"),
        ("garage", "/garage", "GARAGE"),
        ("gallery", "/gallery", "GALLERY"),
        ("factory", "/factory", "FACTORY"),
    ];
    view! {
        <div class="list-filters" style="margin-bottom:16px;">
            {tabs.into_iter().map(|(key, href, label)| {
                view! {
                    <a class=if a == key { "chip chip-on" } else { "chip" } href=href style="text-decoration:none;">{label}</a>
                }
            }).collect_view()}
        </div>
    }
}

/// Shared page chrome for portal pages.
#[component]
fn PortalShell(
    #[prop(into)] active: String,
    #[prop(into)] pill: String,
    #[prop(into)] kicker: String,
    #[prop(into)] title: String,
    #[prop(into)] lead: String,
    children: Children,
) -> impl IntoView {
    let t = title.clone();
    Effect::new(move |_| {
        document().set_title(&format!("Cyberia — {t}"));
        ensure_economy_boot();
        crate::erp::ensure_erp_boot();
        crate::signal::ensure_graph_boot();
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
                            {pill}
                        </div>
                        <CyberiaNav active="robots" />
                    </div>
                </div>
            </div>
            <div class="cities-stage">
                <div class="cities-hero">
                    <div>
                        <div class="cities-kicker">{kicker}</div>
                        <h2 class="cities-title">{title}</h2>
                        <p class="cities-lead">{lead}</p>
                    </div>
                </div>
                <PortalNav active=active />
                {children()}
            </div>
        </div>
    }
}

// ─── GALLERY — типовые модели, OpenSea-style ──────────────────────────

#[component]
pub fn GalleryPage() -> impl IntoView {
    let msg = RwSignal::new(None::<(bool, String)>);
    let tick = RwSignal::new(0u32);
    let body_filter = RwSignal::new(String::new());

    view! {
        <PortalShell
            active="gallery"
            pill=format!("{} MODELS IN STOCK", MODELS.len())
            kicker="THE ROBOT MARKET"
            title="Gallery"
            lead="Standard models, ready to own. Every robot is a capital asset: it earns, remembers, holds standing — and does not die. Buying mints the robot to your name with one signed signal."
        >
            {move || msg.get().map(|(ok, t)| view! {
                <div class=if ok { "eco-msg ok" } else { "eco-msg err" }>{t}</div>
            })}

            <div class="list-filters" style="margin-bottom:14px;">
                <button class=move || if body_filter.get().is_empty() { "chip chip-on" } else { "chip" }
                    on:click=move |_| body_filter.set(String::new())>"ALL"</button>
                <button class=move || if body_filter.get() == "none" { "chip chip-on" } else { "chip" }
                    on:click=move |_| body_filter.set("none".into())>"VIRTUAL"</button>
                <button class=move || if body_filter.get() == "machine" { "chip chip-on" } else { "chip" }
                    on:click=move |_| body_filter.set("machine".into())>"MACHINE"</button>
                <button class=move || if body_filter.get() == "meat" { "chip chip-on" } else { "chip" }
                    on:click=move |_| body_filter.set("meat".into())>"MEAT"</button>
            </div>

            <div class="bot-grid">
                {move || {
                    let _ = tick.get();
                    let f = body_filter.get();
                    MODELS.iter()
                        .filter(|m| f.is_empty() || m.body == f)
                        .map(|m| {
                            let color = body_color(m.body);
                            let buy = move |_| {
                                let n = my_robot_count() + 1;
                                let name = format!("{} #{n}", m.name);
                                if load_balance().cx + 1e-9 < m.price {
                                    msg.set(Some((false, format!("{:.0} CX needed — you hold {:.1}", m.price, load_balance().cx))));
                                } else {
                                    debit_cx(m.price, "robots", &format!("gallery · {}", m.name));
                                    match mint_robot(&name, m.body, m.quadrant, m.blurb) {
                                        Ok(id) => msg.set(Some((true, format!("{name} is yours · -{:.0} CX · card {id}", m.price)))),
                                        Err(e) => msg.set(Some((false, e))),
                                    }
                                }
                                tick.update(|x| *x += 1);
                            };
                            view! {
                                <div class="bot-card" style=format!("--bot-c:{color};")>
                                    <div class="bot-emoji">{m.emoji}</div>
                                    <div class="bot-name">{m.name}</div>
                                    <div class="bot-tags">
                                        <span class="chip" style="pointer-events:none;">{body_label(m.body)}</span>
                                        <span class="chip" style="pointer-events:none;">{m.quadrant.to_uppercase()}</span>
                                    </div>
                                    <div class="bot-blurb">{m.blurb}</div>
                                    <button class="chip chip-on bot-buy" on:click=buy>
                                        {format!("OWN · {:.0} CX", m.price)}
                                    </button>
                                </div>
                            }
                        }).collect_view()
                }}
            </div>

            <p class="bank-footnote">
                "1 robot → 1 avatar → 1 body. Virtual robots are already immortal; machine and meat bodies are upgrades. Custom builds live in the " <a href="/garage">"Garage"</a> "; batch runs at the " <a href="/factory">"Factory"</a> "."
            </p>
        </PortalShell>
    }
}

// ─── GARAGE — конструктор ─────────────────────────────────────────────

#[component]
pub fn GaragePage() -> impl IntoView {
    let name = RwSignal::new(String::new());
    let body = RwSignal::new("none".to_string());
    let mode = RwSignal::new("collab".to_string());
    let msg = RwSignal::new(None::<(bool, String)>);

    let quadrant = move || match (body.get().as_str(), mode.get().as_str()) {
        ("none", "alone") => "dormant",
        ("none", _) => "social ghost",
        (_, "alone") => "sovereign",
        _ => "full actor",
    };

    const ASSEMBLY_FEE: f64 = 5.0;

    let assemble = move |_| {
        let n = name.get();
        let n = n.trim().to_uppercase();
        if n.is_empty() {
            msg.set(Some((false, "name the robot — the name is the NFT".into())));
            return;
        }
        if load_balance().cx + 1e-9 < ASSEMBLY_FEE {
            msg.set(Some((false, format!("{ASSEMBLY_FEE:.0} CX assembly fee needed"))));
            return;
        }
        debit_cx(ASSEMBLY_FEE, "robots", &format!("garage assembly · {n}"));
        match mint_robot(&n, &body.get(), quadrant(), "garage build") {
            Ok(id) => {
                msg.set(Some((true, format!("{n} assembled · quadrant {} · card {id}", quadrant()))));
                name.set(String::new());
            }
            Err(e) => msg.set(Some((false, e))),
        }
    };

    view! {
        <PortalShell
            active="garage"
            pill="ROBOT CONSTRUCTOR"
            kicker="BUILD YOUR OWN"
            title="Garage"
            lead="The constructor. Pick the two axes — every robot lives in one of four quadrants. The soul is immediate; bodies attach later, the chain upgrades any time."
        >
            {move || msg.get().map(|(ok, t)| view! {
                <div class=if ok { "eco-msg ok" } else { "eco-msg err" }>{t}</div>
            })}

            <div class="studio-form-page">
                <label class="found-label">"NAME — the identity NFT"</label>
                <input class="found-input" type="text" prop:value=move || name.get()
                    on:input=move |ev| name.set(event_target_value(&ev))
                    placeholder="AURORA · TISHINA · BOB-9000 …" />

                <label class="found-label" style="margin-top:12px;">"AXIS 1 — BODY (virtual ↔ embodied)"</label>
                <div class="list-filters">
                    <button class=move || if body.get() == "none" { "chip chip-on" } else { "chip" }
                        on:click=move |_| body.set("none".into())>"VIRTUAL — no body, pure soul"</button>
                    <button class=move || if body.get() == "machine" { "chip chip-on" } else { "chip" }
                        on:click=move |_| body.set("machine".into())>"MACHINE — compute · sensors · actuators"</button>
                    <button class=move || if body.get() == "meat" { "chip chip-on" } else { "chip" }
                        on:click=move |_| body.set("meat".into())>"MEAT — bonded biology"</button>
                </div>

                <label class="found-label" style="margin-top:12px;">"AXIS 2 — MODE (alone ↔ collab)"</label>
                <div class="list-filters">
                    <button class=move || if mode.get() == "alone" { "chip chip-on" } else { "chip" }
                        on:click=move |_| mode.set("alone".into())>"ALONE — sovereign, no gas, private"</button>
                    <button class=move || if mode.get() == "collab" { "chip chip-on" } else { "chip" }
                        on:click=move |_| mode.set("collab".into())>"COLLAB — network, identity, reputation"</button>
                </div>

                <div class="bank-kpi-grid me-counts" style="margin-top:16px;">
                    <div class="kpi me-kpi">
                        <div class="kpi-lab">"QUADRANT"</div>
                        <div class="kpi-val" style="font-size:17px;">{move || quadrant().to_uppercase()}</div>
                        <div class="kpi-sub">{move || match quadrant() {
                            "dormant" => "holds capital, schedules, waits — minimal burn",
                            "social ghost" => "publishes, earns, governs — no body, full presence",
                            "sovereign" => "own compute, private network, full local power",
                            _ => "everything: earns, acts, governs — maximum leverage",
                        }}</div>
                    </div>
                    <div class="kpi me-kpi">
                        <div class="kpi-lab">"ASSEMBLY"</div>
                        <div class="kpi-val" style="font-size:17px;">{format!("{ASSEMBLY_FEE:.0} CX")}</div>
                        <div class="kpi-sub">"mints the card + the signed signal"</div>
                    </div>
                </div>

                <div class="studio-form-actions">
                    <button class="cta-btn cta-lease cta-lg" on:click=assemble>
                        <span class="cta-copy">
                            <span class="cta-title">"ASSEMBLE"</span>
                            <span class="cta-sub">"soul first · body optional"</span>
                        </span>
                    </button>
                    <a class="chip" href="/gallery">"OR PICK A STANDARD MODEL →"</a>
                </div>
            </div>
        </PortalShell>
    }
}

// ─── FACTORY — кастомные раны ─────────────────────────────────────────

#[component]
pub fn FactoryPage() -> impl IntoView {
    let chassis = RwSignal::new("gardener".to_string());
    let qty = RwSignal::new("10".to_string());
    let custom = RwSignal::new(String::new());
    let msg = RwSignal::new(None::<(bool, String)>);

    const SETUP_FEE: f64 = 50.0;
    const BATCH_DISCOUNT: f64 = 0.8;

    let quote = move || {
        let q: f64 = qty.get().trim().parse().unwrap_or(0.0);
        let m = MODELS.iter().find(|m| m.id == chassis.get()).unwrap_or(&MODELS[3]);
        SETUP_FEE + q * m.price * BATCH_DISCOUNT
    };

    let order = move |_| {
        let q: f64 = qty.get().trim().parse().unwrap_or(0.0);
        if q < 2.0 {
            msg.set(Some((false, "factory runs start at 2 units — singles live in the Gallery".into())));
            return;
        }
        let m = MODELS.iter().find(|m| m.id == chassis.get()).unwrap_or(&MODELS[3]);
        let total = quote();
        if load_balance().cx + 1e-9 < total {
            msg.set(Some((false, format!("{total:.0} CX needed — you hold {:.1}", load_balance().cx))));
            return;
        }
        let handle = load_profile().handle;
        let me = neuron().bech32;
        debit_cx(total, "factory", &format!("{} ×{}", m.name, q));
        let factory_w = mint_word("service", "robot factory", "customized robot runs", &me, true);
        let you_w = mint_word("person", &handle, "YOU", &me, false);
        let orders_rel = mint_word("relation", "orders", "service request", "", true);
        let note = custom.get();
        let _ = emit_signal(
            vec![Link {
                from: you_w,
                rel: orders_rel,
                to: factory_w,
                weight: q,
                note: format!("{} ×{} · -{total:.0} CX", m.name, q),
            }],
            &format!("factory run · {} ×{}", m.name, q),
        );
        let _ = create_intent_manual(
            "factory",
            m.id,
            None,
            &format!("{} ×{} · {} · custom: {}", m.name, q, body_label(m.body), if note.trim().is_empty() { "stock" } else { note.trim() }),
        );
        push_intent(&handle, "factory_order", &format!("{} x{}", m.id, q));
        msg.set(Some((true, format!("run queued · {} ×{} · -{total:.0} CX — delivery lands in the ops queue", m.name, q))));
    };

    view! {
        <PortalShell
            active="factory"
            pill="CUSTOMIZED RUNS"
            kicker="BATCH MANUFACTURING"
            title="Factory"
            lead="Custom robot runs from the line: pick a chassis, set the batch, describe the customization. Runs price at 20% under Gallery singles; delivery queues as an ops intent."
        >
            {move || msg.get().map(|(ok, t)| view! {
                <div class=if ok { "eco-msg ok" } else { "eco-msg err" }>{t}</div>
            })}

            <div class="studio-form-page">
                <label class="found-label">"CHASSIS"</label>
                <div class="list-filters">
                    {MODELS.iter().filter(|m| m.body != "meat").map(|m| {
                        let id = m.id;
                        view! {
                            <button class=move || if chassis.get() == id { "chip chip-on" } else { "chip" }
                                on:click=move |_| chassis.set(id.into())>
                                {format!("{} {}", m.emoji, m.name)}
                            </button>
                        }
                    }).collect_view()}
                </div>

                <label class="found-label" style="margin-top:12px;">"UNITS"</label>
                <input class="found-input el-qty" type="text" prop:value=move || qty.get()
                    on:input=move |ev| qty.set(event_target_value(&ev)) />

                <label class="found-label" style="margin-top:12px;">"CUSTOMIZATION"</label>
                <input class="found-input" type="text" prop:value=move || custom.get()
                    on:input=move |ev| custom.set(event_target_value(&ev))
                    placeholder="terrace-narrow wheelbase · coffee-picking grippers · night optics …" />

                <div class="bank-kpi-grid me-counts" style="margin-top:16px;">
                    <div class="kpi me-kpi">
                        <div class="kpi-lab">"QUOTE"</div>
                        <div class="kpi-val">{move || format!("{:.0} CX", quote())}</div>
                        <div class="kpi-sub">{move || format!("setup {SETUP_FEE:.0} + units × price × {BATCH_DISCOUNT}")}</div>
                    </div>
                    <div class="kpi me-kpi">
                        <div class="kpi-lab">"YOU HOLD"</div>
                        <div class="kpi-val">{move || format!("{:.1} CX", load_balance().cx)}</div>
                        <div class="kpi-sub">"trade elements · run motifs to earn"</div>
                    </div>
                </div>

                <div class="studio-form-actions">
                    <button class="cta-btn cta-lease cta-lg" on:click=order>
                        <span class="cta-copy">
                            <span class="cta-title">"ORDER THE RUN"</span>
                            <span class="cta-sub">"signed signal + ops intent"</span>
                        </span>
                    </button>
                    <a class="chip" href="/world/intents">"OPS QUEUE →"</a>
                </div>
            </div>
        </PortalShell>
    }
}
