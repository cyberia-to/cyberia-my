//! Services — what the robot fleet can do for you. Each service maps to
//! a crew from /robots; ORDER debits CX, queues an ops intent, and
//! commits one signed signal: you —orders→ service.

use crate::land::FLAG_SVG;
use crate::nav::CyberiaNav;
use crate::signal::{emit_signal, mint_word, neuron, Link};
use crate::wallet::{debit_cx, ensure_economy_boot, load_balance, load_profile, push_intent};
use leptos::prelude::*;

pub struct ServiceDef {
    pub id: &'static str,
    pub name: &'static str,
    /// which robot crew performs it (names from /robots)
    pub crew: &'static str,
    /// hour | job
    pub unit: &'static str,
    pub price: f64,
    pub blurb: &'static str,
}

pub const SERVICES: &[ServiceDef] = &[
    ServiceDef {
        id: "svc_repair_power",
        name: "POWER & WATER REPAIR",
        crew: "SUTAR",
        unit: "hour",
        price: 6.0,
        blurb: "Solar arrays, pumps, plumbing — the lifelines stay on.",
    },
    ServiceDef {
        id: "svc_repair_electronics",
        name: "ELECTRONICS REPAIR",
        crew: "WITAYA",
        unit: "hour",
        price: 8.0,
        blurb: "Boards, radios, sensors — anything with a heartbeat of volts.",
    },
    ServiceDef {
        id: "svc_repair_mech",
        name: "MECHANICAL REPAIR",
        crew: "LUPUS",
        unit: "hour",
        price: 7.0,
        blurb: "Bearings, frames, drivetrains — the moving parts keep moving.",
    },
    ServiceDef {
        id: "svc_build_cube",
        name: "CUBE BUILD",
        crew: "SUDI · BUDI · SASTRA",
        unit: "hour",
        price: 12.0,
        blurb: "The construction crew — from kit to standing cube.",
    },
    ServiceDef {
        id: "svc_stove",
        name: "STOVE INSTALL",
        crew: "TIKA",
        unit: "job",
        price: 10.0,
        blurb: "Rocket stove set, sealed and drawing right.",
    },
    ServiceDef {
        id: "svc_road",
        name: "ROAD & TRAIL BASE",
        crew: "ANGGA",
        unit: "hour",
        price: 9.0,
        blurb: "Grading, drainage, compaction — the ground you can drive.",
    },
    ServiceDef {
        id: "svc_masonry",
        name: "MASONRY",
        crew: "DARMA",
        unit: "hour",
        price: 8.0,
        blurb: "Stone and mortar — walls that outlast the lease.",
    },
    ServiceDef {
        id: "svc_terrace",
        name: "TERRACING",
        crew: "DARSANA",
        unit: "hour",
        price: 9.0,
        blurb: "Contours cut into slopes — land that holds its water.",
    },
    ServiceDef {
        id: "svc_pruning",
        name: "PRUNING",
        crew: "ARIMA",
        unit: "hour",
        price: 5.0,
        blurb: "Canopy discipline — light in, deadwood out.",
    },
    ServiceDef {
        id: "svc_firewood",
        name: "FIREWOOD",
        crew: "DOPLANG",
        unit: "hour",
        price: 5.0,
        blurb: "Cut, split, stacked — dry heat on demand.",
    },
    ServiceDef {
        id: "svc_fodder",
        name: "FODDER RUN",
        crew: "SURYA",
        unit: "hour",
        price: 5.0,
        blurb: "Green cut daily for the flock and the goats.",
    },
];

/// Order a service: CX out, ops intent queued, one signed signal.
fn order_service(def: &ServiceDef, qty: f64) -> Result<String, String> {
    if qty <= 0.0 {
        return Err("qty must be positive".into());
    }
    let total = def.price * qty;
    if load_balance().cx + 1e-9 < total {
        return Err(format!(
            "{total:.1} CX needed — you hold {:.1}",
            load_balance().cx
        ));
    }
    let me = neuron().bech32;
    let handle = load_profile().handle;
    debit_cx(total, "services", &format!("{} ×{qty}", def.name));
    let svc = mint_word("service", def.name, &format!("crew {}", def.crew), &me, false);
    let you = mint_word("person", &handle, "YOU", &me, false);
    let rel = mint_word("relation", "orders", "service request", "", true);
    emit_signal(
        vec![Link {
            from: you,
            rel,
            to: svc,
            weight: qty,
            note: format!("-{total:.1} CX · {qty} {}", def.unit),
        }],
        &format!("order {} · {qty} {}", def.name, def.unit),
    )?;
    let _ = crate::erp::create_intent_manual(
        "service",
        def.id,
        None,
        &format!("{} · {qty} {} · crew {}", def.name, qty, def.crew),
    );
    push_intent(&handle, "svc_order", &format!("{} x{qty}", def.id));
    Ok(format!(
        "ordered {} · {qty} {} · -{total:.1} CX — crew {} dispatched",
        def.name, def.unit, def.crew
    ))
}

#[component]
pub fn ServicesPage() -> impl IntoView {
    let msg = RwSignal::new(None::<(bool, String)>);
    let qty = RwSignal::new("1".to_string());
    let tick = RwSignal::new(0u32);

    Effect::new(move |_| {
        document().set_title("Cyberia — services");
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
                            {move || {
                                let _ = tick.get();
                                format!("{} SERVICES · {} CX", SERVICES.len(), crate::economy::fmt_qty(load_balance().cx))
                            }}
                        </div>
                        <CyberiaNav active="services" />
                    </div>
                </div>
            </div>

            <div class="cities-stage">
                <div class="cities-hero">
                    <div>
                        <div class="cities-kicker">"WHAT THE FLEET CAN DO"</div>
                        <h2 class="cities-title">"Services"</h2>
                        <p class="cities-lead">
                            "Every service is a robot crew from the "
                            <a href="/robots" style="color: var(--cyber-green);">"fleet"</a>
                            ". Ordering queues an ops intent and commits one signed signal: you —orders→ service."
                        </p>
                    </div>
                </div>

                {move || msg.get().map(|(ok, t)| view! {
                    <div class=if ok { "eco-msg ok" } else { "eco-msg err" }>{t}</div>
                })}

                <div class="list-toolbar bank-toolbar" style="margin-bottom:12px;">
                    <div class="list-filters" style="align-items:center; gap:8px;">
                        <span style="color:#556; font-size:10px; letter-spacing:1px;">"QTY"</span>
                        <input class="found-input el-qty" type="text" prop:value=move || qty.get()
                            on:input=move |ev| qty.set(event_target_value(&ev)) />
                        <span class="studio-meta">"hours / jobs per order"</span>
                    </div>
                </div>

                <div class="studio-list">
                    {SERVICES.iter().map(|def| {
                        let order = move |_| {
                            let q: f64 = qty.get().trim().parse().unwrap_or(1.0);
                            match order_service(def, q) {
                                Ok(t) => msg.set(Some((true, t))),
                                Err(e) => msg.set(Some((false, e))),
                            }
                            tick.update(|n| *n += 1);
                        };
                        view! {
                            <div class="studio-row">
                                <div class="studio-row-main">
                                    <span class="studio-kind">"SVC"</span>
                                    <div>
                                        <div class="studio-title">{def.name}</div>
                                        <div class="studio-meta">{def.blurb}</div>
                                        <div class="studio-meta" style="color:var(--cyber-cyan);">{format!("crew {}", def.crew)}</div>
                                    </div>
                                </div>
                                <div class="studio-row-acts">
                                    <button class="chip chip-on" on:click=order>
                                        {format!("ORDER · {:.0} CX/{}", def.price, def.unit)}
                                    </button>
                                </div>
                            </div>
                        }
                    }).collect_view()}
                </div>

                <p class="bank-footnote">
                    "Ordered work shows up as intents in the "
                    <a href="/world/intents">"ops queue"</a>
                    " — reserve and mark done as the crew delivers. The fleet itself lives at "
                    <a href="/robots">"/robots"</a> "."
                </p>
            </div>
        </div>
    }
}
